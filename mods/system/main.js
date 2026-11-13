const $ = (id) => document.getElementById(id);
const t = deskwave.translate(STRINGS);
let percent;
let unit;
function useLanguage(lang) {
  percent = new Intl.NumberFormat(lang, { style: 'percent', maximumFractionDigits: 0 });
  unit = (name) => new Intl.NumberFormat(lang, { style: 'unit', unit: name, unitDisplay: 'short', maximumFractionDigits: 1 });
}
useLanguage(navigator.language);

const UNITS = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'];
function bytes(n, perSecond = false) {
  let i = 0;
  while (n >= 1000 && i < UNITS.length - 1) {
    n /= 1000;
    i++;
  }
  return unit(perSecond ? `${UNITS[i]}-per-second` : UNITS[i]).format(n);
}

function bar(id, fraction, warnAt) {
  const fill = $(`${id}-bar`);
  fill.style.width = `${Math.min(100, Math.max(0, fraction * 100))}%`;
  fill.classList.toggle('high', fraction >= warnAt);
}

function render(s) {
  $('cpu').textContent = percent.format(s.cpu / 100);
  bar('cpu', s.cpu / 100, 0.9);
  const memory = s.memoryTotal ? s.memoryUsed / s.memoryTotal : 0;
  $('memory').textContent = t('of', { used: bytes(s.memoryUsed), total: bytes(s.memoryTotal) });
  bar('memory', memory, 0.9);
  const used = s.diskTotal ? 1 - s.diskFree / s.diskTotal : 0;
  $('disk').textContent = t('free', { free: bytes(s.diskFree) });
  bar('disk', used, 0.95);
  $('net').textContent = t('net', { down: bytes(s.netDown, true), up: bytes(s.netUp, true) });
}

deskwave.on('system', render);
deskwave.ready
  .then(() => {
    useLanguage(deskwave.lang);
    deskwave.localize(t);
    return deskwave.system.get();
  })
  .then(render)
  .catch(() => {});
