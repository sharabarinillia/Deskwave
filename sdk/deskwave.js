// Deskwave mod SDK. Include with <script src="/_sdk/deskwave.js"></script>.
// The host hands this page a MessagePort; everything goes through it. See docs/SPEC.md.
(() => {
  'use strict';
  let port = null;
  let nextId = 1;
  let lang = (navigator.language || 'en').split('-')[0];
  const pending = new Map();
  const handlers = new Map();
  let resolveReady;
  const ready = new Promise((resolve) => (resolveReady = resolve));

  // Fluent shades of the Windows accent: darker fill in light mode, lighter in dark mode,
  // with black or white text on top, whichever contrasts more.
  const scheme = matchMedia('(prefers-color-scheme: dark)');
  let base = null;
  function applyAccent(hex) {
    if (typeof hex === 'string' && /^#[0-9a-f]{6}$/i.test(hex)) base = hex;
    if (!base) return;
    const rgb = [1, 3, 5].map((i) => parseInt(base.slice(i, i + 2), 16));
    const fill = rgb.map((v) => Math.round(scheme.matches ? v + (255 - v) * 0.45 : v * 0.8));
    const [r, g, b] = fill.map((v) => {
      const c = v / 255;
      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    const style = document.documentElement.style;
    style.setProperty('--dw-accent-raw', base);
    style.setProperty('--dw-accent', `rgb(${fill.join(' ')})`);
    style.setProperty('--dw-on-accent', 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.179 ? '#000' : '#fff');
  }
  scheme.addEventListener('change', () => applyAccent());

  addEventListener('message', (event) => {
    const data = event.data;
    if (port || event.source !== parent || data?.type !== 'deskwave:init' || !event.ports[0]) return;
    port = event.ports[0];
    port.onmessage = ({ data: message }) => {
      if (message?.type === 'response') {
        const call = pending.get(message.id);
        if (!call) return;
        pending.delete(message.id);
        if (message.ok) call.resolve(message.result);
        else call.reject(new Error(message.error));
      } else if (message?.type === 'event') {
        if (message.name === 'theme') applyAccent(message.data);
        for (const fn of handlers.get(message.name) ?? []) fn(message.data);
      }
    };
    applyAccent(data.accent);
    if (typeof data.lang === 'string') lang = data.lang;
    document.documentElement.lang = lang;
    resolveReady(Object.freeze({ ...data.context, lang }));
  });

  const call = (method, params) => ready.then(() => new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    port.postMessage({ type: 'request', id, method, params });
  }));

  window.deskwave = Object.freeze({
    /** Resolves with { widget, lang } once the host has connected. Never resolves in previews. */
    ready,
    /** The user's language code (en, uk, fr, nl, de, pl, es). The host restarts mods when it changes. */
    get lang() {
      return lang;
    },
    /** `const t = deskwave.translate({ en: {...}, uk: {...} })`, then `t('key', { n: 2 })`.
     *  Plural entries are objects keyed by Intl.PluralRules category (one, few, many, other). */
    translate(table) {
      return (key, params = {}) => {
        let text = table[lang]?.[key] ?? table.en?.[key] ?? key;
        if (typeof text === 'object') text = text[new Intl.PluralRules(lang).select(params.n ?? 0)] ?? text.other;
        return String(text).replace(/\{(\w+)\}/g, (_, name) => String(params[name] ?? ''));
      };
    },
    /** Fills data-t="key" text, and data-t-placeholder / data-t-label (aria-label) / data-t-title. */
    localize(t, root = document) {
      for (const el of root.querySelectorAll('[data-t]')) el.textContent = t(el.dataset.t);
      for (const [data, attr] of [['tPlaceholder', 'placeholder'], ['tLabel', 'aria-label'], ['tTitle', 'title']]) {
        for (const el of root.querySelectorAll(`[data-${attr === 'aria-label' ? 't-label' : `t-${attr}`}]`)) {
          el.setAttribute(attr, t(el.dataset[data]));
        }
      }
    },
    /** Events: 'pause', 'resume', 'theme' (accent "#rrggbb"), 'internet' (on/off),
     *  and with permission: 'battery', 'system', 'media'. */
    on(name, fn) {
      if (!handlers.has(name)) handlers.set(name, []);
      handlers.get(name).push(fn);
    },
    storage: Object.freeze({
      get: (key) => call('storage.get', { key }),
      set: (key, value) => call('storage.set', { key, value }),
    }),
    battery: Object.freeze({
      get: () => call('battery.get'),
    }),
    /** GET an https URL on a host listed in the manifest's "network". Resolves with
     *  { status, contentType, body } (body is text); rejects with a readable message. */
    net: Object.freeze({
      fetch: (url) => call('net.fetch', { url: String(url) }),
    }),
    /** Needs "system": { cpu, memoryUsed, memoryTotal, diskFree, diskTotal, netDown, netUp }. */
    system: Object.freeze({
      get: () => call('system.get'),
    }),
    /** Needs "media": { active, playing, title, artist, app } and playback control. */
    media: Object.freeze({
      get: () => call('media.get'),
      toggle: () => call('media.control', { action: 'toggle' }),
      next: () => call('media.control', { action: 'next' }),
      previous: () => call('media.control', { action: 'previous' }),
    }),
  });
})();
