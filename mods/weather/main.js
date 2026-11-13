// Weather from Open-Meteo (free, no account). All requests go through Deskwave's network
// broker, which only allows the two hosts in manifest.json and only while the internet is on.
const $ = (id) => document.getElementById(id);
const REFRESH_MS = 30 * 60 * 1000;
const FAHRENHEIT = /-(US|LR|MM|BS|BZ|KY|PW|FM|MH)$/i.test(navigator.language);
const t = deskwave.translate(STRINGS);

const WMO = {
  0: ['clear', 'clear'], 1: ['mostlyClear', 'clear'], 2: ['partly', 'partly'], 3: ['overcast', 'cloud'],
  45: ['fog', 'fog'], 48: ['freezingFog', 'fog'],
  51: ['lightDrizzle', 'drizzle'], 53: ['drizzle', 'drizzle'], 55: ['heavyDrizzle', 'drizzle'],
  56: ['freezingDrizzle', 'drizzle'], 57: ['freezingDrizzle', 'drizzle'],
  61: ['lightRain', 'rain'], 63: ['rain', 'rain'], 65: ['heavyRain', 'rain'],
  66: ['freezingRain', 'rain'], 67: ['freezingRain', 'rain'],
  71: ['lightSnow', 'snow'], 73: ['snow', 'snow'], 75: ['heavySnow', 'snow'], 77: ['snowGrains', 'snow'],
  80: ['lightShowers', 'rain'], 81: ['showers', 'rain'], 82: ['heavyShowers', 'rain'],
  85: ['snowShowers', 'snow'], 86: ['heavySnowShowers', 'snow'],
  95: ['storm', 'storm'], 96: ['stormHail', 'storm'], 99: ['stormHail', 'storm'],
};

let place = null;
let timer = 0;

function note(text, isError = false) {
  $('note').textContent = text ?? '';
  $('note').hidden = !text;
  $('note').classList.toggle('error', isError);
}

async function getJson(url) {
  const reply = await deskwave.net.fetch(url);
  if (reply.status !== 200) throw new Error(t('serviceError', { status: reply.status }));
  return JSON.parse(reply.body);
}

function showSetup() {
  $('now').hidden = true;
  $('setup').hidden = false;
  $('results').replaceChildren();
  $('query').focus();
}

async function refresh() {
  clearTimeout(timer);
  if (!place) return showSetup();
  const params = new URLSearchParams({
    latitude: place.lat,
    longitude: place.lon,
    current: 'temperature_2m,weather_code,is_day',
    daily: 'temperature_2m_max,temperature_2m_min',
    timezone: 'auto',
    forecast_days: '1',
  });
  if (FAHRENHEIT) params.set('temperature_unit', 'fahrenheit');
  try {
    const data = await getJson(`https://api.open-meteo.com/v1/forecast?${params}`);
    const [text, kind] = WMO[data.current.weather_code] ?? ['unknown', 'cloud'];
    const icon = kind === 'clear' || kind === 'partly' ? `${kind}-${data.current.is_day ? 'day' : 'night'}` : kind;
    $('sky').setAttribute('href', `#${icon}`);
    $('temp').textContent = Math.round(data.current.temperature_2m);
    $('desc').textContent = t(text);
    $('range').textContent = t('range', { hi: Math.round(data.daily.temperature_2m_max[0]), lo: Math.round(data.daily.temperature_2m_min[0]) });
    $('place').textContent = place.name;
    $('setup').hidden = true;
    $('now').hidden = false;
    note('');
  } catch (error) {
    note(error.message, true);
  }
  timer = setTimeout(refresh, REFRESH_MS);
}

// Not a <form>: sandboxed mods may not submit forms, so the submit event never fires.
async function search() {
  const name = $('query').value.trim();
  if (!name) return;
  note(t('searching'));
  try {
    const params = new URLSearchParams({ name, count: '5', language: deskwave.lang, format: 'json' });
    const data = await getJson(`https://geocoding-api.open-meteo.com/v1/search?${params}`);
    const found = data.results ?? [];
    note(found.length ? '' : t('notFound', { name }));
    $('results').replaceChildren(...found.map((r) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'dw-btn subtle';
      button.setAttribute('role', 'listitem');
      const where = [r.admin1, r.country].filter(Boolean).join(', ');
      button.append(r.name, ' ');
      const small = document.createElement('small');
      small.textContent = where;
      button.append(small);
      button.addEventListener('click', async () => {
        place = { name: r.name, lat: r.latitude, lon: r.longitude };
        await deskwave.storage.set('place', place);
        refresh();
      });
      return button;
    }));
  } catch (error) {
    note(error.message, true);
  }
}

$('search').addEventListener('click', search);
$('query').addEventListener('keydown', (event) => event.key === 'Enter' && search());
$('place').addEventListener('click', showSetup);
deskwave.on('internet', (on) => on && refresh());

deskwave.ready
  .then(() => {
    deskwave.localize(t);
    return deskwave.storage.get('place');
  })
  .then((saved) => {
    place = saved;
    refresh();
  })
  .catch((error) => note(error.message, true));
