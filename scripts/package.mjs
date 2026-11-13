// Turns Tauri's bundle output (nsis/ and msi/ folders with versioned names) into a download
// that explains itself:
//
//   dist/Deskwave-Setup.exe, dist/Deskwave.msi           release assets, stable names
//   dist/Deskwave-Windows/                               becomes Deskwave-Windows.zip
//     1 - Setup (recommended)/Deskwave-Setup.exe
//     2 - MSI (for IT admins)/Deskwave.msi
//     READ ME FIRST.txt
//     SHA256SUMS.txt
//
// Usage: node scripts/package.mjs [bundle dir, default target/release/bundle]
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { VERSION, RELEASES, AUTHOR, OFFICIAL_SOURCE, NO_LIABILITY, FREE_ONLY, crlf } from './notice.mjs';

const bundle = process.argv[2] ?? 'target/release/bundle';
const SETUP = { from: 'nsis', ext: '.exe', name: 'Deskwave-Setup.exe', folder: '1 - Setup (recommended)' };
const MSI = { from: 'msi', ext: '.msi', name: 'Deskwave.msi', folder: '2 - MSI (for IT admins)' };

function find({ from, ext }) {
  const files = readdirSync(join(bundle, from)).filter((f) => f.endsWith(ext));
  if (files.length !== 1) throw new Error(`expected one ${ext} in ${join(bundle, from)}, found ${files.length}`);
  return join(bundle, from, files[0]);
}

const sha256 = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');

rmSync('dist', { recursive: true, force: true });
const pack = join('dist', 'Deskwave-Windows');
mkdirSync(pack, { recursive: true });
const sums = [];
for (const kind of [SETUP, MSI]) {
  const source = find(kind);
  copyFileSync(source, join('dist', kind.name));
  mkdirSync(join(pack, kind.folder), { recursive: true });
  copyFileSync(source, join(pack, kind.folder, kind.name));
  sums.push(`${sha256(source)}  ${kind.name}`);
}
writeFileSync(join(pack, 'SHA256SUMS.txt'), crlf(sums.join('\n') + '\n'));

const title = `DESKWAVE ${VERSION} FOR WINDOWS 10 AND 11 (64-BIT)`;
writeFileSync(join(pack, 'READ ME FIRST.txt'), crlf(`${title}
${'='.repeat(title.length)}

WARNING: ONLY USE FILES FROM THE OFFICIAL PAGE

${OFFICIAL_SOURCE}

WHICH INSTALLER TO USE

  ${SETUP.folder}\\${SETUP.name}
      For most people. Installs Deskwave for your Windows account only.
      Does not need administrator rights.

  ${MSI.folder}\\${MSI.name}
      Installs Deskwave for every user of this PC. Needs administrator rights.
      For managed PCs and silent installs:  msiexec /i ${MSI.name} /qn

"WINDOWS PROTECTED YOUR PC"?

Deskwave is new and not signed with a paid certificate yet, so Windows may warn you.
If, and only if, you downloaded it from the official page above: select "More info",
check that the app name starts with "Deskwave", then select "Run anyway".
Never turn off your antivirus to install Deskwave.

CHECK THE FILES (OPTIONAL)

SHA256SUMS.txt lists a fingerprint of each installer. In PowerShell, run for example:
  Get-FileHash ".\\${SETUP.folder}\\${SETUP.name}"
The Hash it prints must match the line for that file in SHA256SUMS.txt.

NO WARRANTY, NO LIABILITY

${NO_LIABILITY}

LICENSE

${FREE_ONLY}
Full text: ${RELEASES.replace('/releases', '/blob/main/LICENSE')}

Made by ${AUTHOR}
`));

console.log(`packaged Deskwave ${VERSION}:`);
for (const line of sums) console.log(`  ${line}`);
