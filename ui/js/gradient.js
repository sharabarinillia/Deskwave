// Gradient wallpapers. Still ones are plain CSS: nothing runs per frame. Moving ones draw at a
// tenth of the size on a canvas, which the browser scales up smoothly, at 20 frames a second.
const SCALE = 10;
const FPS = 20;
const SPEED = { slow: 1 / 40, medium: 1 / 16, fast: 1 / 6 };
const BLOBS = [[0.22, 0.28], [0.78, 0.3], [0.32, 0.78], [0.75, 0.74]];

const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const rgb = (c, alpha = 1) => `rgb(${c.join(' ')} / ${alpha})`;

/** The still version as a CSS background. */
export function css(g) {
  const stops = g.colors.join(', ');
  if (g.style === 'linear') return `linear-gradient(${g.angle}deg, ${stops})`;
  if (g.style === 'radial') return `radial-gradient(circle at 50% 45%, ${stops})`;
  const blobs = g.colors.slice(1).map((c, i) => {
    const [x, y] = BLOBS[i % BLOBS.length];
    return `radial-gradient(ellipse at ${x * 100}% ${y * 100}%, ${c} 0%, transparent 62%)`;
  });
  return [...blobs, g.colors[0]].join(', ');
}

function draw(ctx, g, colors, t) {
  const { width: w, height: h } = ctx.canvas;
  const n = colors.length;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  if (g.style === 'linear') {
    // CSS angles: 0deg points up, clockwise. Sway ±25° around the chosen angle.
    const a = ((g.angle + 25 * Math.sin(t)) * Math.PI) / 180;
    const [dx, dy] = [Math.sin(a), -Math.cos(a)];
    const half = (Math.abs(w * dx) + Math.abs(h * dy)) / 2;
    const fill = ctx.createLinearGradient(w / 2 - dx * half, h / 2 - dy * half, w / 2 + dx * half, h / 2 + dy * half);
    colors.forEach((c, i) => fill.addColorStop(i / (n - 1), rgb(c)));
    ctx.fillStyle = fill;
  } else if (g.style === 'radial') {
    const x = w * (0.5 + 0.18 * Math.sin(t));
    const y = h * (0.45 + 0.15 * Math.cos(t * 0.8));
    const r = Math.max(w, h) * 0.9 * (1 + 0.08 * Math.sin(t * 1.7));
    const fill = ctx.createRadialGradient(x, y, 0, x, y, r);
    colors.forEach((c, i) => fill.addColorStop(i / (n - 1), rgb(c)));
    ctx.fillStyle = fill;
  } else {
    ctx.fillStyle = rgb(colors[0]);
    ctx.fillRect(0, 0, w, h);
    // At least three blobs so two colors still flow.
    const others = colors.slice(1);
    const count = Math.max(3, others.length);
    for (let i = 0; i < count; i++) {
      const c = others[i % others.length];
      const [bx, by] = BLOBS[i % BLOBS.length];
      const x = w * (bx + 0.16 * Math.sin(t * (0.9 + i * 0.23) + i * 2.1));
      const y = h * (by + 0.14 * Math.cos(t * (0.7 + i * 0.19) + i * 1.3));
      // Kept small and soft so the first color still shows between the blobs.
      const r = Math.max(w, h) * (0.32 + 0.05 * Math.sin(t * 1.3 + i));
      ctx.setTransform(1.5, 0, 0, 1, x, y);
      const blob = ctx.createRadialGradient(0, 0, 0, 0, 0, r);
      blob.addColorStop(0, rgb(c, 0.85));
      blob.addColorStop(0.5, rgb(c, 0.35));
      blob.addColorStop(1, rgb(c, 0));
      ctx.fillStyle = blob;
      ctx.fillRect(-r, -r, r * 2, r * 2);
    }
    return;
  }
  ctx.fillRect(0, 0, w, h);
}

/** Renders `g` into `container`. Returns { pause, resume, destroy }. */
export function mountGradient(container, g) {
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (g.motion === 'still' || reduced) {
    const fill = document.createElement('div');
    fill.className = 'gradient-fill';
    fill.style.background = css(g);
    container.append(fill);
    return { pause() {}, resume() {}, destroy: () => fill.remove() };
  }
  const canvas = document.createElement('canvas');
  canvas.className = 'gradient-fill';
  container.append(canvas);
  const ctx = canvas.getContext('2d', { alpha: false });
  const colors = g.colors.map(hex);
  const speed = SPEED[g.motion] ?? SPEED.slow;
  const now = () => (performance.now() / 1000) * speed * Math.PI * 2;

  let timer = 0;
  let paused = false;
  const loop = () => {
    clearTimeout(timer);
    if (paused) return;
    requestAnimationFrame(() => {
      draw(ctx, g, colors, now());
      timer = setTimeout(loop, 1000 / FPS);
    });
  };
  // Measure the canvas, not the container: the wallpaper stage has no height of its own (its
  // children are fixed). CSS sizes the canvas, so setting its pixel size here doesn't loop.
  const resize = new ResizeObserver(() => {
    canvas.width = Math.max(16, Math.ceil(canvas.clientWidth / SCALE));
    canvas.height = Math.max(9, Math.ceil(canvas.clientHeight / SCALE));
    draw(ctx, g, colors, now());
  });
  resize.observe(canvas);
  loop();
  return {
    pause() {
      paused = true;
      clearTimeout(timer);
    },
    resume() {
      if (!paused) return;
      paused = false;
      loop();
    },
    destroy() {
      paused = true;
      clearTimeout(timer);
      resize.disconnect();
      canvas.remove();
    },
  };
}
