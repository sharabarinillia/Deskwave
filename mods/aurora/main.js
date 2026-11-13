// Aurora: soft light curtains whose colours follow the time of day.
// Drawn at a tenth of the screen's resolution; the browser's upscale is the blur, so it costs
// almost nothing. 20 fps, stops on 'pause', one frame a minute with reduced motion.
const canvas = document.getElementById('sky');
const ctx = canvas.getContext('2d', { alpha: false });
const SCALE = 10;
const FPS = 20;
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');

const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const ANCHORS = [
  { hour: 0, base: '#060918', glow: ['#13a37e', '#4b2bb8', '#0e5a8a', '#7a2a8c', '#1d8f9e'] },
  { hour: 6, base: '#1c1030', glow: ['#ff8a5c', '#d0468f', '#6a46c8', '#ffb35c', '#3a6fd0'] },
  { hour: 12, base: '#0d2a52', glow: ['#2fd3c0', '#3a7bff', '#8a5cf6', '#22b8e6', '#5fe0a8'] },
  { hour: 18, base: '#1d0d33', glow: ['#ff5c9a', '#8b5cf6', '#ff9a3c', '#3456d6', '#c03fd0'] },
  { hour: 24, base: '#060918', glow: ['#13a37e', '#4b2bb8', '#0e5a8a', '#7a2a8c', '#1d8f9e'] },
].map((a) => ({ hour: a.hour, base: hex(a.base), glow: a.glow.map(hex) }));

// Periods of 2-4 minutes keep the motion calm enough to work next to.
const BLOBS = [
  { ax: 0.35, ay: 0.22, sx: 1 / 23, sy: 1 / 31, sr: 1 / 17, phase: 0, r: 0.46 },
  { ax: 0.4, ay: 0.28, sx: 1 / 29, sy: 1 / 19, sr: 1 / 23, phase: 2.1, r: 0.5 },
  { ax: 0.3, ay: 0.3, sx: 1 / 37, sy: 1 / 27, sr: 1 / 29, phase: 4.2, r: 0.42 },
  { ax: 0.45, ay: 0.2, sx: 1 / 31, sy: 1 / 41, sr: 1 / 19, phase: 1.3, r: 0.38 },
  { ax: 0.25, ay: 0.26, sx: 1 / 43, sy: 1 / 23, sr: 1 / 31, phase: 5.5, r: 0.36 },
];

const mix = (a, b, k) => a.map((v, i) => Math.round(v + (b[i] - v) * k));
const rgb = (c, alpha = 1) => `rgb(${c.join(' ')} / ${alpha})`;
const smooth = (k) => k * k * (3 - 2 * k);

let accent = null;
function readAccent() {
  const value = getComputedStyle(document.documentElement).getPropertyValue('--dw-accent-raw').trim();
  accent = /^#[0-9a-f]{6}$/i.test(value) ? hex(value) : null;
}

function palette(date) {
  const hours = date.getHours() + date.getMinutes() / 60;
  const i = ANCHORS.findIndex((a) => a.hour > hours) - 1;
  const [a, b] = [ANCHORS[i], ANCHORS[i + 1]];
  const k = smooth((hours - a.hour) / (b.hour - a.hour));
  const glow = a.glow.map((c, j) => mix(c, b.glow[j], k));
  // One curtain leans towards the Windows accent so the desktop feels like yours.
  if (accent) glow[0] = mix(glow[0], accent, 0.5);
  return { base: mix(a.base, b.base, k), glow };
}

function resize() {
  canvas.width = Math.max(16, Math.ceil(innerWidth / SCALE));
  canvas.height = Math.max(9, Math.ceil(innerHeight / SCALE));
}

let colors = palette(new Date());
let colorsAt = 0;

function draw(seconds) {
  if (seconds - colorsAt > 5) {
    colors = palette(new Date());
    colorsAt = seconds;
  }
  const { width: w, height: h } = canvas;
  ctx.globalCompositeOperation = 'source-over';
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.fillStyle = rgb(colors.base);
  ctx.fillRect(0, 0, w, h);
  ctx.globalCompositeOperation = 'screen';
  BLOBS.forEach((b, i) => {
    const x = (0.5 + b.ax * Math.sin(seconds * b.sx + b.phase)) * w;
    const y = (0.42 + b.ay * Math.cos(seconds * b.sy + b.phase * 1.7)) * h;
    const r = b.r * Math.max(w, h) * (1 + 0.12 * Math.sin(seconds * b.sr + b.phase));
    // Wide, flat ellipses read as curtains rather than bubbles.
    ctx.setTransform(1.7, 0, 0.25 * Math.sin(seconds * b.sr), 0.75, x, y);
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, r);
    g.addColorStop(0, rgb(colors.glow[i], 0.85));
    g.addColorStop(0.55, rgb(colors.glow[i], 0.32));
    g.addColorStop(1, rgb(colors.glow[i], 0));
    ctx.fillStyle = g;
    ctx.fillRect(-r, -r, r * 2, r * 2);
  });
}

let paused = false;
let timer = 0;
function loop() {
  clearTimeout(timer);
  if (paused) return;
  requestAnimationFrame(() => {
    draw(Date.now() / 1000 % 100000);
    timer = setTimeout(loop, reducedMotion.matches ? 60_000 : 1000 / FPS);
  });
}

addEventListener('resize', () => {
  resize();
  draw(Date.now() / 1000 % 100000);
});
reducedMotion.addEventListener('change', loop);
deskwave.on('pause', () => {
  paused = true;
  clearTimeout(timer);
});
deskwave.on('resume', () => {
  paused = false;
  loop();
});
deskwave.on('theme', () => {
  readAccent();
  colorsAt = -Infinity;
});
deskwave.ready.then(() => {
  readAccent();
  colorsAt = -Infinity;
});

resize();
loop();
