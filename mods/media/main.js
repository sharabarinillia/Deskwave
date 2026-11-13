const $ = (id) => document.getElementById(id);
const t = deskwave.translate(STRINGS);
const buttons = ['previous', 'toggle', 'next'];

function render(now) {
  $('root').className = `media${now.active ? '' : ' idle'}${now.playing ? ' playing' : ''}`;
  $('title').textContent = now.active ? now.title || t('unknown') : t('nothing');
  $('artist').textContent = now.active ? [now.artist, now.app].filter(Boolean).join(' · ') : t('hint');
  $('toggle').setAttribute('aria-label', t(now.playing ? 'pause' : 'play'));
  for (const id of buttons) $(id).disabled = !now.active;
}

$('toggle').addEventListener('click', () => deskwave.media.toggle().catch(() => {}));
$('next').addEventListener('click', () => deskwave.media.next().catch(() => {}));
$('previous').addEventListener('click', () => deskwave.media.previous().catch(() => {}));

deskwave.on('media', render);
deskwave.ready
  .then(() => {
    deskwave.localize(t);
    return deskwave.media.get();
  })
  .then(render)
  .catch(() => {});
