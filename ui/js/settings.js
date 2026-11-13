import { h, icon } from './dom.js';
import { mountGradient } from './gradient.js';
import { has, LANGUAGE_NAMES, list, setLang, t } from './i18n.js';
import { applyAccent } from './theme.js';

const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);

const DEFAULT_WALLPAPER = 'deskwave.aurora';
const PAUSE_OPTIONS = [
  ['fullscreen', 'fullscreen'],
  ['covered', 'maximize'],
  ['battery', 'battery'],
  ['locked', 'lock'],
];
const PALETTES = [
  ['#0f2027', '#203a43', '#2c5364'],
  ['#ff7e5f', '#feb47b'],
  ['#654ea3', '#eaafc8'],
  ['#11998e', '#38ef7d'],
  ['#1d2b64', '#f8cdda'],
  ['#0b0f2b', '#5b3fa8', '#ff5c9a', '#ffb35c'],
];

let state = null;
let monitorIndex = 0;
let galleryKey = '';
let galleryRenderers = [];
let confirming = null;
let draft = null;
let draftPreview = null;

// ---- shell ----

function show(page) {
  for (const item of document.querySelectorAll('.nav-item')) {
    if (item.dataset.page === page) item.setAttribute('aria-current', 'page');
    else item.removeAttribute('aria-current');
  }
  for (const section of document.querySelectorAll('.page')) section.hidden = section.id !== `page-${page}`;
  document.querySelector('.content').scrollTop = 0;
  if (page === 'wallpaper') renderMonitors();
}

let noticeTimer;
async function run(command, args) {
  try {
    return await invoke(command, args);
  } catch (error) {
    const bar = $('notice');
    bar.querySelector('span').textContent = t('common.error', { error });
    bar.hidden = false;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (bar.hidden = true), 8000);
    throw error;
  }
}

async function load() {
  state = await run('settings_get');
  setLang(state.lang);
  applyAccent(state.accent);
  monitorIndex = Math.min(monitorIndex, Math.max(0, state.monitors.length - 1));
  $('version').textContent = state.version;
  $('startup-prompt').hidden = state.startupAsked || state.autostart;
  renderSummary();
  renderMonitors();
  renderGallery();
  renderWidgets();
  renderMods();
  renderGeneral();
  renderPrivacy();
}

function renderSummary() {
  const n = state.widgetCount;
  $('summary').textContent = `${t(state.internet ? 'summary.online' : 'summary.offline')} · ${t('summary.widgets', { n })}`;
}

const switchControl = (checked, label, onChange) => {
  const input = h('input', { type: 'checkbox', class: 'switch', role: 'switch', 'aria-label': label });
  input.checked = checked;
  const word = h('span', { 'aria-hidden': 'true' }, t(checked ? 'common.on' : 'common.off'));
  input.addEventListener('change', () => {
    word.textContent = t(input.checked ? 'common.on' : 'common.off');
    onChange(input);
  });
  return h('label', { class: 'toggle' }, word, input);
};

// ---- wallpaper ----

const displayName = (i) => t('wallpaper.display', { n: i + 1 });
const gradientLabel = (g) => `${t(`gradient.${g.style}`)} · ${t(`gradient.${g.motion}`)}`;

function wallpaperName(wallpaper) {
  if (wallpaper?.kind === 'file') return wallpaper.name;
  if (wallpaper?.kind === 'gradient') {
    const g = state.gradients.find((x) => x.id === wallpaper.id);
    return g ? gradientLabel(g) : t('wallpaper.unavailable');
  }
  const id = wallpaper?.id ?? DEFAULT_WALLPAPER;
  return state.mods.find((m) => m.id === id)?.name ?? t('wallpaper.unavailable');
}

function isCurrent(wallpaper) {
  const current = state.monitors[monitorIndex]?.wallpaper ?? { kind: 'mod', id: DEFAULT_WALLPAPER };
  return current.kind === wallpaper.kind && (current.id ?? current.name) === (wallpaper.id ?? wallpaper.name);
}

function renderMonitors() {
  const box = $('monitors');
  const monitors = state?.monitors ?? [];
  if (!box.clientWidth || !monitors.length) return;
  const left = Math.min(...monitors.map((m) => m.x));
  const top = Math.min(...monitors.map((m) => m.y));
  const width = Math.max(...monitors.map((m) => m.x + m.width)) - left;
  const height = Math.max(...monitors.map((m) => m.y + m.height)) - top;
  const pad = 24;
  const scale = Math.min((box.clientWidth - 2 * pad) / width, (box.clientHeight - 2 * pad) / height);
  const offsetX = (box.clientWidth - width * scale) / 2;
  const offsetY = (box.clientHeight - height * scale) / 2;
  box.replaceChildren(...monitors.map((m, i) => {
    const name = wallpaperName(m.wallpaper);
    const button = h('button', {
      class: 'monitor',
      role: 'radio',
      'aria-checked': String(i === monitorIndex),
      'aria-label': `${m.primary ? t('wallpaper.displayMain', { n: i + 1 }) : displayName(i)}: ${name}`,
      onclick: () => {
        monitorIndex = i;
        renderMonitors();
        renderChecks();
      },
    }, h('span', { class: 'num' }, String(i + 1)), h('span', { class: 'what' }, name));
    // A 4 px gap keeps neighbouring displays readable as separate screens.
    Object.assign(button.style, {
      left: `${offsetX + (m.x - left) * scale + 2}px`,
      top: `${offsetY + (m.y - top) * scale + 2}px`,
      width: `${m.width * scale - 4}px`,
      height: `${m.height * scale - 4}px`,
    });
    return button;
  }));
}

async function choose(wallpaper) {
  const monitor = state.monitors[monitorIndex];
  await run('set_wallpaper', { monitor: monitor.key, wallpaper });
  monitor.wallpaper = wallpaper;
  renderMonitors();
  renderChecks();
}

function tile(label, wallpaper, media) {
  const node = h('div', {
    class: 'tile',
    role: 'radio',
    tabindex: '0',
    'aria-label': label,
    onclick: () => choose(wallpaper),
    onkeydown: (event) => {
      if (event.target !== node || (event.key !== 'Enter' && event.key !== ' ')) return;
      event.preventDefault();
      choose(wallpaper);
    },
  }, h('span', { class: 'tile-media' }, media), h('span', { class: 'check' }, icon('check')),
  h('span', { class: 'tile-label', 'aria-hidden': 'true' }, label));
  node.wallpaper = wallpaper;
  return node;
}

function gradientTile(g) {
  const node = tile(gradientLabel(g), { kind: 'gradient', id: g.id });
  galleryRenderers.push(mountGradient(node.querySelector('.tile-media'), g));
  node.append(h('button', {
    class: 'edit',
    'aria-label': `${t('gradient.edit')}: ${gradientLabel(g)}`,
    title: t('gradient.edit'),
    onclick: (event) => {
      event.stopPropagation();
      openEditor(g);
    },
  }, icon('edit')));
  return node;
}

function newGradientTile() {
  const open = () => openEditor(null);
  return h('div', {
    class: 'tile new',
    role: 'button',
    tabindex: '0',
    onclick: open,
    onkeydown: (event) => {
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault();
      open();
    },
  }, h('span', { class: 'tile-media' }, icon('add')), h('span', { class: 'tile-label' }, t('wallpaper.newGradient')));
}

function renderGallery() {
  const presets = state.mods.filter((m) => m.kind === 'wallpaper' && m.url);
  const key = JSON.stringify([state.lang, presets.map((m) => [m.url, m.name]), state.gradients, state.files]);
  if (key !== galleryKey) {
    // Rebuild only when the choices change, so live previews keep running.
    galleryKey = key;
    galleryRenderers.forEach((r) => r.destroy());
    galleryRenderers = [];
    $('presets').replaceChildren(...(presets.length
      ? presets.map((m) => tile(m.name, { kind: 'mod', id: m.id },
          h('iframe', { src: m.url, sandbox: 'allow-scripts', referrerpolicy: 'no-referrer', loading: 'lazy', tabindex: '-1', 'aria-hidden': 'true', title: m.name })))
      : [h('p', { class: 'empty' }, t('wallpaper.noAnimated'))]));
    $('gradients').replaceChildren(...state.gradients.map(gradientTile), newGradientTile());
    $('files').replaceChildren(...(state.files.length
      ? state.files.map((name) => {
          const src = convertFileSrc(`${state.wallpapersDir}\\${name}`);
          const isVideo = /\.(mp4|webm)$/i.test(name);
          const media = isVideo ? h('video', { src, muted: true, loop: true, preload: 'metadata' }) : h('img', { src, alt: '' });
          const node = tile(name, { kind: 'file', name }, media);
          if (isVideo) {
            const play = () => media.play().catch(() => {});
            const stop = () => media.pause();
            node.addEventListener('pointerenter', play);
            node.addEventListener('focus', play);
            node.addEventListener('pointerleave', stop);
            node.addEventListener('blur', stop);
          }
          return node;
        })
      : [h('p', { class: 'empty' }, t('wallpaper.noFiles'))]));
  }
  renderChecks();
}

function renderChecks() {
  for (const node of document.querySelectorAll('.tile[role="radio"]')) {
    node.setAttribute('aria-checked', String(isCurrent(node.wallpaper)));
  }
}

// ---- gradient editor ----

function openEditor(g) {
  draft = structuredClone(g ?? { id: '', colors: [...PALETTES[5]], style: 'flow', angle: 135, motion: 'slow' });
  $('editor-title').textContent = t(g ? 'gradient.titleEdit' : 'gradient.titleNew');
  $('gradient-delete').hidden = !g;
  $('gradient-editor').hidden = false;
  renderEditor();
  $('gradient-editor').scrollIntoView({ block: 'nearest' });
  $('gradient-use').focus({ preventScroll: true });
}

function closeEditor() {
  draftPreview?.destroy();
  draftPreview = null;
  draft = null;
  $('gradient-editor').hidden = true;
}

function updatePreview() {
  draftPreview?.destroy();
  draftPreview = mountGradient($('editor-preview'), draft);
}

function segmented(id, field, values) {
  $(id).replaceChildren(...values.map((value) => {
    const input = h('input', { type: 'radio', name: field, value });
    input.checked = draft[field] === value;
    input.addEventListener('change', () => {
      draft[field] = value;
      $('angle-field').hidden = draft.style !== 'linear';
      updatePreview();
    });
    return h('label', {}, input, t(`gradient.${value}`));
  }));
}

function renderEditor() {
  $('swatches').replaceChildren(
    ...draft.colors.map((color, i) => {
      const input = h('input', { type: 'color', value: color, 'aria-label': t('gradient.color', { n: i + 1 }) });
      input.addEventListener('input', () => {
        draft.colors[i] = input.value;
        updatePreview();
      });
      const remove = draft.colors.length > 2 && h('button', {
        class: 'remove',
        'aria-label': t('gradient.removeColor', { n: i + 1 }),
        onclick: () => {
          draft.colors.splice(i, 1);
          renderEditor();
        },
      }, icon('dismiss'));
      return h('div', { class: 'swatch' }, input, remove);
    }),
    draft.colors.length < 5 && h('button', {
      class: 'btn add-color',
      'aria-label': t('gradient.addColor'),
      title: t('gradient.addColor'),
      onclick: () => {
        draft.colors.push(draft.colors.at(-1));
        renderEditor();
      },
    }, icon('add')),
  );
  $('palettes').replaceChildren(...PALETTES.map((colors, i) => {
    const button = h('button', {
      class: 'palette',
      'aria-label': t('gradient.palette', { n: i + 1 }),
      onclick: () => {
        draft.colors = [...colors];
        renderEditor();
      },
    });
    button.style.background = `linear-gradient(90deg, ${colors.join(', ')})`;
    return button;
  }));
  segmented('style-group', 'style', ['linear', 'radial', 'flow']);
  segmented('motion-group', 'motion', ['still', 'slow', 'medium', 'fast']);
  $('angle-field').hidden = draft.style !== 'linear';
  $('angle').value = String(draft.angle);
  $('angle-value').textContent = `${draft.angle}°`;
  updatePreview();
}

$('angle').addEventListener('input', () => {
  draft.angle = Number($('angle').value);
  $('angle-value').textContent = `${draft.angle}°`;
  updatePreview();
});
$('gradient-cancel').addEventListener('click', closeEditor);
$('gradient-use').addEventListener('click', async () => {
  const id = await run('gradient_save', { gradient: draft }).catch(() => null);
  if (!id) return;
  closeEditor();
  await load();
  await choose({ kind: 'gradient', id });
});
$('gradient-delete').addEventListener('click', async () => {
  await run('gradient_delete', { id: draft.id }).catch(() => {});
  closeEditor();
  await load();
});

// ---- widgets ----

function renderWidgets() {
  const widgets = state.mods.filter((m) => m.kind === 'widget' && m.approved);
  $('widget-list').replaceChildren(...(widgets.length
    ? widgets.map((m) => h('div', { class: 'row' }, icon('widgets', 'icon row-icon'),
        h('div', { class: 'row-text' }, h('div', {}, m.name), m.description && h('div', { class: 'row-desc' }, m.description))))
    : [h('p', { class: 'empty' }, t('widgets.noneOn'))]));
}

// ---- mods ----

/** Everything a mod asks for, as infinitive phrases. */
const asks = (mod) => [
  ...mod.permissions.map((p) => (has(`perm.${p}`) ? t(`perm.${p}`) : p)),
  ...(mod.network.length ? [t('perm.network', { hosts: list(mod.network) })] : []),
];

async function setApproved(mod, approved) {
  confirming = null;
  await run('set_mod_approved', { id: mod.id, approved }).catch(() => {});
  await load();
}

function modRow(mod) {
  const meta = [t(mod.kind === 'wallpaper' ? 'mods.wallpaper' : 'mods.widget'), mod.author && t('mods.by', { author: mod.author }), t('mods.version', { version: mod.version })]
    .filter(Boolean).join(' · ');
  const can = asks(mod).length ? t('mods.can', { list: list(asks(mod)) }) : t('mods.none');
  const toggle = switchControl(mod.approved, t('mods.turnOn', { name: mod.name }), (input) => {
    if (input.checked && asks(mod).length && !mod.approved) {
      // Approving a permission is a decision, not a side effect of a switch.
      confirming = mod.id;
      renderMods();
      return;
    }
    setApproved(mod, input.checked);
  });
  return h('div', { class: 'row mod' },
    icon(mod.kind === 'wallpaper' ? 'image' : 'widgets', 'icon row-icon'),
    h('div', { class: 'row-text' },
      h('div', {}, mod.name),
      h('div', { class: 'meta' }, meta),
      mod.description && h('div', { class: 'row-desc' }, mod.description),
      h('div', { class: mod.reviewed ? 'can' : 'can waiting' },
        icon(mod.reviewed ? 'shield' : 'info'),
        mod.reviewed ? can : `${t('mods.new')} ${can}`)),
    h('div', { class: 'row-actions' }, mod.builtin && h('span', { class: 'pill' }, t('mods.builtIn')), toggle),
    confirming === mod.id && h('div', { class: 'confirm' },
      h('p', {}, t('mods.confirm', { name: mod.name, list: list(asks(mod)) })),
      h('div', { class: 'row-actions' },
        h('button', { class: 'btn primary', onclick: () => setApproved(mod, true) }, t('mods.allow')),
        h('button', { class: 'btn', onclick: () => { confirming = null; renderMods(); } }, t('common.cancel')))));
}

function renderMods() {
  const fresh = state.mods.filter((m) => !m.reviewed).length;
  $('mods-badge').hidden = !fresh;
  $('mods-badge').textContent = String(fresh);
  $('mods-pending').hidden = !fresh;
  $('mods-pending').querySelector('span').textContent = t('mods.pending', { n: fresh });
  $('mod-list').replaceChildren(...state.mods.map(modRow));
  const errors = $('mod-errors');
  errors.hidden = !state.errors.length;
  errors.replaceChildren(
    h('h2', {}, t('mods.couldntLoad')),
    ...state.errors.map((text) => h('div', { class: 'row' }, icon('warning', 'icon row-icon error-row'),
      h('div', { class: 'row-text' }, h('div', { class: 'selectable' }, text)))),
  );
}

// ---- general ----

function renderGeneral() {
  const select = $('language');
  select.replaceChildren(
    h('option', { value: '' }, t('general.system')),
    ...Object.entries(LANGUAGE_NAMES).map(([code, name]) => h('option', { value: code, lang: code }, name)),
  );
  select.value = state.language ?? '';
  $('autostart-toggle').replaceChildren(switchControl(state.autostart, t('startup.title'), (input) => {
    run('set_autostart', { on: input.checked }).catch(() => (input.checked = !input.checked));
  }));
  $('pause-options').replaceChildren(...PAUSE_OPTIONS.map(([key, iconName]) => h('div', { class: 'row' },
    icon(iconName, 'icon row-icon'),
    h('div', { class: 'row-text' }, h('div', {}, t(`pause.${key}`)), h('div', { class: 'row-desc' }, t(`pause.${key}Text`))),
    switchControl(state.pause[key], t(`pause.${key}`), (input) => {
      state.pause[key] = input.checked;
      run('set_pause_options', { options: state.pause }).catch(() => {});
    }))));
}

// ---- privacy & about ----

const clock = () => new Intl.DateTimeFormat(state.lang, { hour: 'numeric', minute: '2-digit' });

function renderPrivacy() {
  const online = state.internet;
  $('status').classList.toggle('online', online);
  $('status-icon').setAttribute('href', `img/icons.svg#i-${online ? 'globe' : 'shield'}`);
  $('status-title').textContent = t(online ? 'privacy.onlineTitle' : 'privacy.offlineTitle');
  $('status-desc').textContent = t(online ? 'privacy.onlineText' : 'privacy.offlineText');
  $('internet-toggle').replaceChildren(switchControl(online, t('privacy.switchTitle'), async (input) => {
    try {
      await run('set_internet', { on: input.checked });
      state.internet = input.checked;
      renderPrivacy();
      renderSummary();
    } catch {
      input.checked = !input.checked;
    }
  }));
  const format = clock();
  $('net-log').replaceChildren(...(state.netLog.length
    ? state.netLog.slice(0, 20).map((e) => {
        const ok = /^2\d\d$/.test(e.result);
        const result = ok ? t('privacy.ok') : has(`net.${e.result}`) ? t(`net.${e.result}`) : e.result;
        return h('div', { class: 'row log-row' },
          h('span', { class: 'log-time' }, format.format(new Date(e.time * 1000))),
          h('div', { class: 'row-text' }, h('div', {}, e.name), h('div', { class: 'row-desc selectable' }, e.host)),
          h('span', { class: ok ? 'log-result' : 'log-result failed' }, result));
      })
    : [h('p', { class: 'empty' }, t('privacy.noActivity'))]));
}

// ---- wiring ----

for (const item of document.querySelectorAll('.nav-item')) item.addEventListener('click', () => show(item.dataset.page));
for (const button of document.querySelectorAll('[data-refresh]')) button.addEventListener('click', load);
for (const button of document.querySelectorAll('[data-open]')) {
  button.addEventListener('click', () => run('open_folder', { which: button.dataset.open }).catch(() => {}));
}
for (const button of document.querySelectorAll('[data-link]')) {
  button.addEventListener('click', () => run('open_link', { which: button.dataset.link }).catch(() => {}));
}
$('edit-widgets').addEventListener('click', () => run('set_edit_mode', { on: true }).catch(() => {}));
$('language').addEventListener('change', () => run('set_language', { language: $('language').value || null }).catch(() => {}));
for (const [id, on] of [['startup-yes', true], ['startup-no', false]]) {
  $(id).addEventListener('click', async () => {
    await run('set_autostart', { on }).catch(() => {});
    await load();
  });
}
addEventListener('resize', renderMonitors);
await listen('theme', (event) => applyAccent(event.payload));
await listen('language', () => load());
await load();
