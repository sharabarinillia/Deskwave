// Albums: videos and photos taking turns. Photos stay up for `interval` seconds; videos play
// to the end. Each tile crossfades between two layers, so at most two items are loaded at a
// time. A collage shows up to four tiles, each walking the album from its own starting point.
const FADE_MS = 700;
/** After this many failed items in a row (missing or unreadable files), wait before retrying. */
const RETRY_MS = 30_000;

function shuffled(items) {
  const copy = [...items];
  for (let i = copy.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [copy[i], copy[j]] = [copy[j], copy[i]];
  }
  return copy;
}

function load(item, src) {
  return new Promise((resolve, reject) => {
    if (item.video) {
      const video = Object.assign(document.createElement('video'), { muted: true, playsInline: true, preload: 'auto' });
      video.addEventListener('loadeddata', () => resolve(video), { once: true });
      video.addEventListener('error', () => reject(new Error(item.name)), { once: true });
      video.src = src(item.path);
    } else {
      const img = Object.assign(document.createElement('img'), { alt: '', decoding: 'async' });
      img.src = src(item.path);
      img.decode().then(() => resolve(img), () => reject(new Error(item.name)));
    }
  });
}

function unload(el) {
  if (el instanceof HTMLVideoElement) {
    el.pause();
    el.removeAttribute('src');
    el.load(); // frees the decoder
  }
  el.remove();
}

function tile(root, album, src, index, count, reduced) {
  const box = document.createElement('div');
  box.className = 'album-tile';
  root.append(box);
  const fade = reduced ? 0 : FADE_MS;
  const single = album.items.length === 1;
  let list = album.order === 'shuffle' ? shuffled(album.items) : album.items;
  // Start each collage tile at a different point in the album.
  let position = Math.floor((index * list.length) / count) - 1;
  let current = null;
  let timer = 0;
  let due = 0;
  let remaining = 0;
  let failures = 0;
  let first = true;
  let paused = false;
  let gone = false;

  const next = () => {
    position += 1;
    if (position >= list.length) {
      position = 0;
      if (album.order === 'shuffle') list = shuffled(album.items);
    }
    return list[position];
  };

  /** Advances after `ms`; while paused, keeps the time for when playback resumes. */
  function wait(ms) {
    clearTimeout(timer);
    timer = 0;
    if (paused) {
      remaining = ms;
      return;
    }
    due = performance.now() + ms;
    timer = setTimeout(advance, ms);
  }

  async function advance() {
    clearTimeout(timer);
    if (paused || gone) return;
    const item = next();
    let el;
    try {
      el = await load(item, src);
    } catch {
      if (gone) return;
      failures += 1;
      wait(failures >= album.items.length ? RETRY_MS : 50);
      return;
    }
    if (gone) return unload(el);
    failures = 0;
    el.className = 'album-media';
    el.style.transitionDuration = `${fade}ms`;
    box.append(el);
    const previous = current;
    current = el;
    requestAnimationFrame(() => el.classList.add('shown'));
    if (previous) setTimeout(() => unload(previous), fade + 50);
    if (el instanceof HTMLVideoElement) {
      el.loop = single;
      el.addEventListener('ended', advance);
      if (!paused) el.play().catch(() => {});
    } else if (!single) {
      // Collage tiles change at different moments, not all at once.
      const stagger = first ? (album.interval * 1000 * index) / count : 0;
      wait(album.interval * 1000 + stagger);
    }
    first = false;
  }

  advance();
  return {
    pause() {
      if (paused) return;
      paused = true;
      if (timer) remaining = Math.max(0, due - performance.now());
      clearTimeout(timer);
      timer = 0;
      if (current instanceof HTMLVideoElement) current.pause();
    },
    resume() {
      if (!paused) return;
      paused = false;
      const left = remaining;
      remaining = 0;
      if (current instanceof HTMLVideoElement) current.play().catch(() => {});
      if (left) wait(left);
      else if (!current) advance();
    },
    destroy() {
      gone = true;
      clearTimeout(timer);
      if (current) unload(current);
      box.remove();
    },
  };
}

/** Plays `album` ({ items: [{ name, path, video }], interval, order, layout }) in `container`.
 *  `src` turns a file path into a URL. Returns { pause, resume, destroy }. */
export function mountAlbum(container, album, src) {
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const count = album.layout === 'collage' ? Math.min(4, album.items.length) : 1;
  const root = document.createElement('div');
  root.className = `album album-${count}`;
  container.append(root);
  const tiles = Array.from({ length: count }, (_, i) => tile(root, album, src, i, count, reduced));
  return {
    pause: () => tiles.forEach((t) => t.pause()),
    resume: () => tiles.forEach((t) => t.resume()),
    destroy() {
      tiles.forEach((t) => t.destroy());
      root.remove();
    },
  };
}
