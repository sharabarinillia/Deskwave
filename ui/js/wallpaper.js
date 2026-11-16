import { mountAlbum } from './album.js';
import { mountMod } from './bridge.js';
import { mountGradient } from './gradient.js';
import { setLang } from './i18n.js';

const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const stage = document.getElementById('stage');
let current = {};
let paused = false;

function setPaused(value) {
  paused = value;
  const { media, mod, gradient, album } = current;
  for (const player of [gradient, album]) {
    if (paused) player?.pause();
    else player?.resume();
  }
  if (media instanceof HTMLVideoElement) {
    if (paused) media.pause();
    else media.play().catch(() => {});
  }
  // shortcut: GIFs and images keep animating while paused; pausing them needs a canvas copy.
  mod?.emit(paused ? 'pause' : 'resume');
}

async function load() {
  const info = await invoke('wallpaper_get');
  setLang(info.lang);
  current.gradient?.destroy();
  current.album?.destroy();
  stage.replaceChildren();
  current = {};
  if (info.gradient) {
    current = { gradient: mountGradient(stage, info.gradient) };
  } else if (info.album) {
    current = { album: mountAlbum(stage, info.album, convertFileSrc) };
  } else if (info.file) {
    const media = document.createElement(info.video ? 'video' : 'img');
    if (info.video) Object.assign(media, { muted: true, loop: true, playsInline: true });
    else media.alt = '';
    media.src = convertFileSrc(info.file);
    stage.append(media);
    current = { media };
  } else if (info.modUrl) {
    current = { mod: mountMod(stage, { url: info.modUrl, title: 'Wallpaper', accent: info.accent, lang: info.lang }) };
  }
  setPaused(info.paused);
}

// Listen before the first load so no pause change slips through in between.
await Promise.all([
  listen('pause', (event) => setPaused(event.payload)),
  listen('wallpaper-changed', load),
  listen('mods-changed', load),
  listen('theme', (event) => current.mod?.emit('theme', event.payload)),
  listen('internet', (event) => current.mod?.emit('internet', event.payload)),
  listen('language', load),
]);
await load();
