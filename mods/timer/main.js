// Pomodoro-style timer. State is saved, so a running timer survives restarts of Deskwave.
const $ = (id) => document.getElementById(id);
const LENGTH = { focus: 25 * 60_000, break: 5 * 60_000 };
const t = deskwave.translate(STRINGS);

/** { phase, running, endsAt (ms, while running), left (ms, while paused) } */
let state = { phase: 'focus', running: false, left: LENGTH.focus };
let tick = 0;
let audio = null;

const remaining = () => (state.running ? Math.max(0, state.endsAt - Date.now()) : state.left);

function save() {
  deskwave.storage.set('state', state).catch(() => {});
}

function chime() {
  if (!audio) return;
  const t = audio.currentTime;
  [659, 880].forEach((hz, i) => {
    const osc = audio.createOscillator();
    const gain = audio.createGain();
    osc.frequency.value = hz;
    const at = t + i * 0.2;
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.18, at + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.7);
    osc.connect(gain).connect(audio.destination);
    osc.start(at);
    osc.stop(at + 0.75);
  });
}

function nextPhase(withSound) {
  state = { phase: state.phase === 'focus' ? 'break' : 'focus', running: false };
  state.left = LENGTH[state.phase];
  if (withSound) chime();
  save();
}

function render() {
  clearTimeout(tick);
  if (state.running && remaining() === 0) nextPhase(true);
  const ms = remaining();
  const total = Math.ceil(ms / 1000);
  $('clock').textContent = `${String(Math.floor(total / 60)).padStart(2, '0')}:${String(total % 60).padStart(2, '0')}`;
  $('phase').textContent = t(state.phase);
  $('root').className = `timer ${state.phase}`;
  $('bar').style.width = `${(1 - ms / LENGTH[state.phase]) * 100}%`;
  $('start').textContent = t(state.running ? 'pause' : ms < LENGTH[state.phase] ? 'resume' : 'start');
  if (state.running) tick = setTimeout(render, (ms % 1000) + 20);
}

$('start').addEventListener('click', () => {
  audio ??= new AudioContext(); // created on a click so the end-of-phase chime may play
  state = state.running
    ? { phase: state.phase, running: false, left: remaining() }
    : { phase: state.phase, running: true, endsAt: Date.now() + state.left };
  save();
  render();
});
$('reset').addEventListener('click', () => {
  state = { phase: state.phase, running: false, left: LENGTH[state.phase] };
  save();
  render();
});
$('skip').addEventListener('click', () => {
  nextPhase(false);
  render();
});

deskwave.ready
  .then(() => {
    deskwave.localize(t);
    return deskwave.storage.get('state');
  })
  .then((saved) => {
    const valid = saved?.phase in LENGTH && Number.isFinite(saved.running ? saved.endsAt : saved.left);
    if (valid) state = saved;
    // A phase that ended while Deskwave was closed just moves on, without a sound.
    if (state.running && state.endsAt <= Date.now()) nextPhase(false);
    for (const id of ['start', 'reset', 'skip']) $(id).disabled = false;
    render();
  })
  .catch(() => {});
