const $ = (id) => document.getElementById(id);
const t = deskwave.translate(STRINGS);
const MAX_ITEMS = 200;
let items = [];

function save() {
  deskwave.storage.set('items', items).catch((e) => ($('count').textContent = t('notSaved', { error: e.message })));
}

function render() {
  $('list').replaceChildren(...items.map((item) => {
    const li = document.createElement('li');
    li.className = item.done ? 'item done' : 'item';
    const box = document.createElement('input');
    box.type = 'checkbox';
    box.className = 'dw-check';
    box.id = `t${item.id}`;
    box.checked = item.done;
    box.addEventListener('change', () => {
      item.done = box.checked;
      save();
      render();
    });
    const label = document.createElement('label');
    label.htmlFor = box.id;
    label.textContent = item.text;
    const remove = document.createElement('button');
    remove.className = 'dw-btn subtle icon remove';
    remove.setAttribute('aria-label', t('remove', { text: item.text }));
    remove.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/></svg>';
    remove.addEventListener('click', () => {
      items = items.filter((x) => x !== item);
      save();
      render();
    });
    li.append(box, label, remove);
    return li;
  }));
  const left = items.filter((x) => !x.done).length;
  $('empty').hidden = items.length > 0;
  $('list').hidden = items.length === 0;
  $('count').textContent = items.length ? t('left', { n: left }) : '';
  $('clear').hidden = left === items.length;
}

function add() {
  const text = $('new').value.trim();
  if (!text || items.length >= MAX_ITEMS) return;
  items.push({ id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6), text, done: false });
  $('new').value = '';
  save();
  render();
}

$('add').addEventListener('click', add);
$('new').addEventListener('keydown', (event) => event.key === 'Enter' && add());
$('clear').addEventListener('click', () => {
  items = items.filter((x) => !x.done);
  save();
  render();
});

deskwave.ready
  .then(() => {
    deskwave.localize(t);
    return deskwave.storage.get('items');
  })
  .then((saved) => {
    items = Array.isArray(saved) ? saved : [];
    $('new').disabled = false;
    $('add').disabled = false;
    render();
  })
  .catch((e) => ($('count').textContent = e.message));
