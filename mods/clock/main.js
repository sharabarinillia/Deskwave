let timeFormat;
let dateFormat;
function useLanguage(lang) {
  timeFormat = new Intl.DateTimeFormat(lang, { hour: 'numeric', minute: '2-digit' });
  dateFormat = new Intl.DateTimeFormat(lang, { weekday: 'long', month: 'long', day: 'numeric' });
}
useLanguage(navigator.language);
const hm = document.getElementById('hm');
const period = document.getElementById('period');
const date = document.getElementById('date');

let timer = 0;
function tick() {
  clearTimeout(timer);
  const now = new Date();
  const parts = timeFormat.formatToParts(now);
  hm.textContent = parts.filter((p) => p.type !== 'dayPeriod').map((p) => p.value).join('').trim();
  period.textContent = parts.find((p) => p.type === 'dayPeriod')?.value ?? '';
  date.textContent = dateFormat.format(now);
  // Wake at the next minute, but at least every 15 s so the clock is right after sleep.
  const toNextMinute = 60_000 - (now.getSeconds() * 1000 + now.getMilliseconds()) + 50;
  timer = setTimeout(tick, Math.min(toNextMinute, 15_000));
}

tick();
deskwave.ready.then(() => {
  useLanguage(deskwave.lang);
  tick();
});
