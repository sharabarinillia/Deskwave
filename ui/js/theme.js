// Windows accent → the shades Fluent uses: darker fills in light mode, lighter in dark mode,
// with black or white text on top, whichever contrasts more.
const scheme = matchMedia('(prefers-color-scheme: dark)');
let accent = '#0067c0';

function luminance(rgb) {
  const [r, g, b] = rgb.map((v) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function applyAccent(hex = accent) {
  if (/^#[0-9a-f]{6}$/i.test(hex)) accent = hex;
  const rgb = [1, 3, 5].map((i) => parseInt(accent.slice(i, i + 2), 16));
  const towards = (target, k) => rgb.map((v) => Math.round(v + (target - v) * k));
  const dark = scheme.matches;
  const fill = dark ? towards(255, 0.45) : towards(0, 0.2);
  const text = dark ? towards(255, 0.55) : towards(0, 0.35);
  const style = document.documentElement.style;
  style.setProperty('--accent-fill', `rgb(${fill})`);
  style.setProperty('--accent-text', `rgb(${text})`);
  style.setProperty('--on-accent', luminance(fill) > 0.179 ? '#000' : '#fff');
}

scheme.addEventListener('change', () => applyAccent());
