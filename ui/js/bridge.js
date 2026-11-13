// Host side of the mod bridge. A mod runs in a sandboxed iframe and only receives a
// MessagePort. Every request is checked here against what the user approved for that mod.
import { has, t } from './i18n.js';

const { invoke } = window.__TAURI__.core;

const WRITES_PER_10S = 30;

export function mountMod(parent, { url, title, accent, lang = 'en', widgetId = null, permissions = [] }) {
  const frame = document.createElement('iframe');
  frame.sandbox = 'allow-scripts';
  frame.referrerPolicy = 'no-referrer';
  frame.title = title;
  frame.src = url;

  let port = null;
  let queued = [];
  const writes = [];

  frame.addEventListener('load', () => {
    port?.close();
    const channel = new MessageChannel();
    port = channel.port1;
    port.onmessage = (event) => handle(event.data);
    frame.contentWindow.postMessage(
      { type: 'deskwave:init', context: { widget: widgetId }, accent, lang },
      '*', // the frame's origin is opaque, so it cannot be named
      [channel.port2],
    );
    for (const message of queued) port.postMessage(message);
    queued = [];
  });

  async function handle(message) {
    if (message?.type !== 'request' || !Number.isInteger(message.id)) return;
    try {
      const result = await call(String(message.method), message.params ?? {});
      port.postMessage({ type: 'response', id: message.id, ok: true, result });
    } catch (error) {
      // Rust returns short codes like "internet-off"; mods get a sentence in the user's language.
      const text = String(error?.message ?? error);
      port.postMessage({ type: 'response', id: message.id, ok: false, error: has(`net.${text}`) ? t(`net.${text}`) : text });
    }
  }

  function need(permission) {
    if (!permissions.includes(permission)) throw new Error(`This mod is not allowed to use: ${permission}.`);
  }

  async function call(method, params) {
    switch (method) {
      case 'storage.get': {
        if (!widgetId) throw new Error('Storage is only available to widgets.');
        const all = await invoke('storage_get', { widget: widgetId });
        return all[String(params.key)] ?? null;
      }
      case 'storage.set': {
        if (!widgetId) throw new Error('Storage is only available to widgets.');
        const now = Date.now();
        while (writes.length && now - writes[0] > 10_000) writes.shift();
        if (writes.length >= WRITES_PER_10S) throw new Error('Too many writes; save less often.');
        writes.push(now);
        return invoke('storage_set', { widget: widgetId, key: String(params.key), value: params.value ?? null });
      }
      case 'battery.get':
        need('battery');
        return invoke('battery_get');
      case 'system.get':
        need('system');
        return invoke('system_get');
      case 'media.get':
        need('media');
        return invoke('media_get');
      case 'media.control':
        need('media');
        if (!widgetId) throw new Error('Only widgets can control media.');
        return invoke('media_control', { widget: widgetId, action: String(params.action) });
      case 'net.fetch':
        // Rust checks the internet switch, this mod's approved hosts and the URL itself.
        return invoke('net_fetch', { widget: widgetId, url: String(params.url) });
      default:
        throw new Error(`Unknown method: ${method}`);
    }
  }

  parent.append(frame);
  return {
    frame,
    emit(name, data) {
      if (['battery', 'system', 'media'].includes(name) && !permissions.includes(name)) return;
      const message = { type: 'event', name, data };
      if (port) port.postMessage(message);
      else queued.push(message);
    },
  };
}
