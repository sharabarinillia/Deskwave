import { mountMod } from './bridge.js';
import { h, icon } from './dom.js';
import { setLang, t } from './i18n.js';
import { applyAccent } from './theme.js';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const GRID = 8;
const MIN_W = 96;
const MIN_H = 64;

const layer = document.getElementById('widgets');
const toolbar = document.getElementById('toolbar');
const addButton = document.getElementById('add');
const menu = document.getElementById('menu');
/** id → { el, mod, key, source (saved rect), data (rect as shown) } */
const mounted = new Map();
let available = [];
let editing = false;

const snap = (v) => Math.round(v / GRID) * GRID;
const clamp = (v, lo, hi) => Math.min(Math.max(v, lo), Math.max(lo, hi));

function fit({ x, y, w, h: height }) {
  const W = clamp(snap(w), MIN_W, innerWidth);
  const H = clamp(snap(height), MIN_H, innerHeight);
  return { x: clamp(snap(x), 0, innerWidth - W), y: clamp(snap(y), 0, innerHeight - H), w: W, h: H };
}

function place(entry) {
  Object.assign(entry.el.style, {
    left: `${entry.data.x}px`,
    top: `${entry.data.y}px`,
    width: `${entry.data.w}px`,
    height: `${entry.data.h}px`,
  });
}

function reportHitRects() {
  const rects = [...mounted.values()].map(({ data }) => [data.x, data.y, data.w, data.h]);
  invoke('set_hit_rects', { rects }).catch(console.error);
}

let saveTimer;
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    const layout = [...mounted.values()].map(({ data: { id, x, y, w, h: height } }) => ({ id, x, y, w, h: height }));
    invoke('widgets_save', { layout }).catch(console.error);
  }, 250);
  reportHitRects();
}

let lang = 'en';

function render(info) {
  applyAccent(info.accent);
  lang = info.lang;
  setLang(lang);
  available = info.available;
  const ids = new Set(info.widgets.map((w) => w.id));
  for (const [id, entry] of mounted) {
    if (!ids.has(id)) {
      entry.el.remove();
      mounted.delete(id);
    }
  }
  for (const w of info.widgets) {
    // Running mods keep their state; recreate only when what they may do changed.
    const key = `${w.url}|${w.permissions.join()}|${w.name}|${lang}`;
    let entry = mounted.get(w.id);
    if (entry && entry.key !== key) {
      entry.el.remove();
      entry = null;
    }
    if (!entry) {
      entry = create(w, key, info.accent);
      mounted.set(w.id, entry);
    }
    // `source` is the saved position; `data` is it fitted to the current window size, so a
    // window that is briefly smaller while loading never moves widgets for good.
    entry.source = { id: w.id, x: w.x, y: w.y, w: w.w, h: w.h };
    entry.data = { ...fit(entry.source), id: w.id };
    place(entry);
  }
  setEditing(info.editing);
  reportHitRects();
}

function create(w, key, accent) {
  const el = h('div', { class: 'widget' });
  const entry = { el, key, data: null, mod: null };
  if (w.url) {
    entry.mod = mountMod(el, { url: w.url, title: w.name, accent, lang, widgetId: w.id, permissions: w.permissions });
  } else {
    el.append(h('div', { class: 'widget-off' }, h('strong', {}, w.name), t('layer.off')));
  }
  el.append(handle(entry, w.name));
  layer.append(el);
  return entry;
}

function handle(entry, name) {
  const remove = h('button', {
    class: 'remove',
    'aria-label': t('layer.remove', { name }),
    onclick: async () => render(await invoke('widget_remove', { id: entry.data.id })),
  }, icon('dismiss'));
  const grip = h('span', { class: 'grip', 'aria-hidden': 'true' }, icon('grip'));
  const node = h('div', {
    class: 'handle',
    tabindex: editing ? '0' : '-1',
    role: 'group',
    'aria-label': t('layer.handle', { name }),
  }, remove, grip);

  node.addEventListener('pointerdown', (event) => {
    if (event.button !== 0 || event.target.closest('.remove')) return;
    const resizing = Boolean(event.target.closest('.grip'));
    const start = { ...entry.data, px: event.clientX, py: event.clientY };
    node.setPointerCapture(event.pointerId);
    entry.el.classList.add('lifted');
    const move = (e) => {
      const dx = e.clientX - start.px;
      const dy = e.clientY - start.py;
      const next = resizing
        ? { x: start.x, y: start.y, w: start.w + dx, h: start.h + dy }
        : { x: start.x + dx, y: start.y + dy, w: start.w, h: start.h };
      entry.data = { ...fit(next), id: start.id };
      place(entry);
    };
    const end = () => {
      node.removeEventListener('pointermove', move);
      entry.el.classList.remove('lifted');
      entry.source = entry.data;
      save();
    };
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerup', end, { once: true });
    node.addEventListener('pointercancel', end, { once: true });
  });

  node.addEventListener('keydown', (event) => {
    const step = { ArrowLeft: [-GRID, 0], ArrowRight: [GRID, 0], ArrowUp: [0, -GRID], ArrowDown: [0, GRID] }[event.key];
    if (!step) return;
    event.preventDefault();
    const [dx, dy] = step;
    const d = entry.data;
    const next = event.shiftKey ? { ...d, w: d.w + dx, h: d.h + dy } : { ...d, x: d.x + dx, y: d.y + dy };
    entry.data = { ...fit(next), id: d.id };
    entry.source = entry.data;
    place(entry);
    save();
  });
  return node;
}

function setEditing(on) {
  editing = on;
  document.body.classList.toggle('editing', on);
  for (const node of layer.querySelectorAll('.handle')) node.tabIndex = on ? 0 : -1;
  if (!on) closeMenu();
}

function openMenu() {
  menu.replaceChildren(
    ...(available.length
      ? available.map((m) => h('button', {
          class: 'menu-item',
          role: 'menuitem',
          onclick: async () => {
            closeMenu();
            render(await invoke('widget_add', { modId: m.id }));
          },
        }, m.name, m.description && h('small', {}, m.description)))
      : [h('div', { class: 'menu-empty' }, t('layer.noWidgets'))]),
  );
  const bar = toolbar.getBoundingClientRect();
  Object.assign(menu.style, { left: `${bar.left}px`, top: `${bar.bottom + 4}px` });
  menu.hidden = false;
  addButton.setAttribute('aria-expanded', 'true');
  menu.querySelector('button')?.focus();
}

function closeMenu() {
  if (menu.hidden) return;
  menu.hidden = true;
  addButton.setAttribute('aria-expanded', 'false');
}

addButton.addEventListener('click', () => (menu.hidden ? openMenu() : closeMenu()));
document.getElementById('done').addEventListener('click', () => invoke('set_edit_mode', { on: false }));
document.addEventListener('pointerdown', (event) => {
  if (!menu.hidden && !menu.contains(event.target) && !addButton.contains(event.target)) closeMenu();
});
document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape' || !editing) return;
  if (!menu.hidden) {
    closeMenu();
    addButton.focus();
  } else {
    invoke('set_edit_mode', { on: false });
  }
});
addEventListener('resize', () => {
  for (const entry of mounted.values()) {
    entry.data = { ...fit(entry.source), id: entry.source.id };
    place(entry);
  }
  reportHitRects();
});

const load = async () => render(await invoke('widgets_get'));

await Promise.all([
  listen('edit-mode', (event) => {
    setEditing(event.payload);
    if (event.payload && document.hasFocus()) addButton.focus();
  }),
  listen('mods-changed', load),
  listen('language', load),
  ...['battery', 'system', 'media', 'internet'].map((name) =>
    listen(name, (event) => mounted.forEach((m) => m.mod?.emit(name, event.payload)))),
  listen('theme', (event) => {
    applyAccent(event.payload);
    mounted.forEach((m) => m.mod?.emit('theme', event.payload));
  }),
]);
await load();
