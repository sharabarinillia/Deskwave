// Translations for the host pages. Rust decides the language (the user's choice, else the
// system's display language) and passes it to every page. On a Mac, a key's `@mac` variant
// (e.g. 'general.system@mac') replaces it where Windows wording wouldn't fit.
import { STRINGS } from './strings.js';

export const LANGUAGE_NAMES = {
  en: 'English',
  uk: 'Українська',
  fr: 'Français',
  nl: 'Nederlands',
  de: 'Deutsch',
  pl: 'Polski',
  es: 'Español',
};

let lang = 'en';
let mac = false;

export const getLang = () => lang;

/** `windows` or `macos`, from Rust. Call before `setLang` so the page is translated once. */
export function setPlatform(platform) {
  mac = platform === 'macos';
}

export function setLang(code) {
  lang = code in STRINGS ? code : 'en';
  document.documentElement.lang = lang;
  translateDom();
}

/** `t('key', { n: 3, name: 'x' })`. Plural entries are objects keyed by Intl.PluralRules category. */
export function t(key, params = {}) {
  if (mac && `${key}@mac` in STRINGS.en) key = `${key}@mac`;
  let text = STRINGS[lang]?.[key] ?? STRINGS.en[key] ?? key;
  if (typeof text === 'object') {
    text = text[new Intl.PluralRules(lang).select(params.n ?? 0)] ?? text.other;
  }
  return text.replace(/\{(\w+)\}/g, (_, name) => String(params[name] ?? ''));
}

export const has = (key) => key in STRINGS.en;

export const list = (items) => new Intl.ListFormat(lang, { type: 'conjunction' }).format(items);

/** Fills `data-i18n="key"` text and `data-i18n-attr="attr:key;attr:key"` attributes. */
export function translateDom(root = document) {
  for (const el of root.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
  for (const el of root.querySelectorAll('[data-i18n-attr]')) {
    for (const pair of el.dataset.i18nAttr.split(';')) {
      const [attr, key] = pair.split(':');
      el.setAttribute(attr, t(key));
    }
  }
}
