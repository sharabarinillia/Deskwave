const t = deskwave.translate(STRINGS);
const root = document.getElementById('root');
const level = document.getElementById('level');
const pct = document.getElementById('pct');
const unit = document.getElementById('unit');
const state = document.getElementById('state');

const duration = (seconds) => {
  const h = Math.floor(seconds / 3600);
  const m = Math.round((seconds % 3600) / 60);
  return h ? t('hm', { h, m }) : t('m', { m });
};

function render(b) {
  root.classList.toggle('none', !b.present);
  if (!b.present) {
    pct.textContent = t('noBattery');
    unit.hidden = true;
    state.textContent = t('mains');
    return;
  }
  const p = b.percent ?? 0;
  unit.hidden = b.percent == null;
  pct.textContent = b.percent ?? '–';
  level.style.width = `${Math.max(1.5, (32 * p) / 100)}px`;
  root.classList.toggle('charging', b.charging);
  root.classList.toggle('low', p <= 15 && !b.onAc);
  let text = b.charging ? t('charging')
    : b.onAc ? t(p >= 100 ? 'full' : 'plugged')
    : b.secondsLeft ? t('left', { time: duration(b.secondsLeft) })
    : t('onBattery');
  if (b.saver) text += ` · ${t('saver')}`;
  state.textContent = text;
}

deskwave.on('battery', render);
deskwave.ready
  .then(() => {
    deskwave.localize(t);
    return deskwave.battery.get();
  })
  .then(render)
  .catch((e) => (state.textContent = e.message));
