// Fails if any language misses a key, has an extra one, or changes a {placeholder}.
// Checks the host pages' strings and every built-in mod's i18n.js. Run: node scripts/check-strings.mjs
import { readdirSync, existsSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const placeholders = (v) => JSON.stringify([...new Set(JSON.stringify(v).match(/\{\w+\}/g) ?? [])].sort());
let problems = 0;

function check(name, table) {
  const en = table.en;
  for (const [lang, strings] of Object.entries(table)) {
    for (const key of Object.keys(en)) {
      if (!(key in strings)) {
        console.error(`${name} ${lang}: missing "${key}"`);
        problems++;
      } else if (placeholders(strings[key]) !== placeholders(en[key])) {
        console.error(`${name} ${lang}: placeholders differ in "${key}"`);
        problems++;
      }
    }
    for (const key of Object.keys(strings)) {
      if (!(key in en)) {
        console.error(`${name} ${lang}: unknown key "${key}"`);
        problems++;
      }
    }
  }
  return Object.keys(table).length;
}

const { STRINGS } = await import(pathToFileURL('ui/js/strings.js'));
const langs = check('ui', STRINGS);
for (const dir of readdirSync('mods')) {
  const file = `mods/${dir}/i18n.js`;
  if (!existsSync(file)) continue;
  // Mod files assign `const STRINGS = {...}` for the page; evaluate them the same way.
  const source = (await import('node:fs')).readFileSync(file, 'utf8');
  const table = new Function(`${source}; return STRINGS;`)();
  if (check(dir, table) !== langs) {
    console.error(`${dir}: has ${Object.keys(table).length} languages, expected ${langs}`);
    problems++;
  }
}
if (problems) process.exit(1);
console.log(`strings ok: ${langs} languages`);
