// Writes the text both installers show on their license page ("I Agree" is required):
// the official-source warning, the no-liability notice, then the full LICENSE.
// Runs before bundling (tauri.conf.json > build > beforeBundleCommand).
import { writeFileSync } from 'node:fs';
import { OFFICIAL_SOURCE, NO_LIABILITY, FREE_ONLY, LICENSE, crlf, assertAscii } from './notice.mjs';

const text = `PLEASE READ BEFORE INSTALLING DESKWAVE

1. ONLY INSTALL DESKWAVE FROM THE OFFICIAL PAGE

${OFFICIAL_SOURCE}

2. NO WARRANTY, NO LIABILITY

${NO_LIABILITY}

3. FREE FOR NON-COMMERCIAL USE ONLY

${FREE_ONLY} The full license follows.

------------------------------------------------------------------------------------

${LICENSE}`;

assertAscii('Installer license text', text);
const out = new URL('../src-tauri/installer/license.txt', import.meta.url);
writeFileSync(out, crlf(text));
console.log('wrote src-tauri/installer/license.txt');
