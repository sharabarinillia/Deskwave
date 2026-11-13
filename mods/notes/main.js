const t = deskwave.translate(STRINGS);
const note = document.getElementById('note');
const error = document.getElementById('error');

function fail(e) {
  error.textContent = t('notSaved', { error: e.message });
  error.hidden = false;
}

let timer;
function save() {
  clearTimeout(timer);
  deskwave.storage.set('text', note.value).then(() => (error.hidden = true), fail);
}

note.addEventListener('input', () => {
  clearTimeout(timer);
  timer = setTimeout(save, 500);
});
note.addEventListener('blur', () => {
  if (timer) save();
  timer = undefined;
});

deskwave.ready
  .then(() => {
    deskwave.localize(t);
    return deskwave.storage.get('text');
  })
  .then((text) => {
    note.value = text ?? '';
    note.disabled = false;
  })
  .catch(fail);
