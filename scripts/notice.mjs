// Texts shown before installing: on the installers' license page and in "READ ME FIRST.txt".
// Plain ASCII on purpose: the MSI turns this into a Windows-1252 RTF document.
import { readFileSync } from 'node:fs';

const root = new URL('..', import.meta.url);
export const VERSION = JSON.parse(readFileSync(new URL('src-tauri/tauri.conf.json', root), 'utf8')).version;
export const LICENSE = readFileSync(new URL('LICENSE', root), 'utf8');
export const RELEASES = 'https://github.com/sharabarinillia/Deskwave/releases';
export const AUTHOR = 'sharabarinillia (https://github.com/sharabarinillia)';

export const OFFICIAL_SOURCE = `The only official place to download Deskwave is:
${RELEASES}
If you got this file anywhere else (another website, a file-sharing service, an email,
a chat message or a USB stick), it may have been changed and is NOT safe. Do not
install it. Delete it and download Deskwave again from the page above.`;

export const NO_LIABILITY = `Deskwave is provided "as is", without any warranty of any kind. You install and use
it at your own risk. To the fullest extent permitted by law, the author is not liable
for any damage or loss caused by this program or by its use, including damage to your
computer, loss of data, and damage caused by copies obtained anywhere other than the
official page above.`;

export const FREE_ONLY = `ONLY FOR UNPAID USAGE. CHARGING ANY KIND OF FEES IS PROHIBITED.
Deskwave is free for non-commercial use only, under the PolyForm Noncommercial
License 1.0.0.`;

/** Windows line endings, so Notepad and the installers show the lines as written. */
export const crlf = (text) => text.replace(/\r?\n/g, '\r\n');

export function assertAscii(name, text) {
  const bad = text.match(/[^\x09\x0a\x0d\x20-\x7e]/);
  if (bad) throw new Error(`${name} contains a non-ASCII character: ${JSON.stringify(bad[0])}`);
}
