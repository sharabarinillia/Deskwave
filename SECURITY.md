# Security

Deskwave runs all day on people's desktops and loads code written by strangers (mods), so its
security model is part of the product. The full design is in
[docs/SPEC.md](docs/SPEC.md#sandbox-defence-in-depth).

## What Deskwave promises

- **Local network:** no open ports, no local server, and nothing on the network can reach the app.
- **Privacy:** no telemetry, no crash uploads, no account and no automatic update checks.
- **Offline by default:** no network requests at all.
  - With the internet switch on, only widgets the user approved can reach the internet, and only
    to the public hosts they declared. Requests are HTTPS-only, never redirected, and never sent
    to addresses on the user's own network.
- **Mods** run in a sandbox. They cannot read files, other mods, the network or the app's internal
  commands, and they stay off until the user approves them and the permissions they ask for.
- **No hidden persistence:** Deskwave starts at sign-in only if the user says yes, needs no
  administrator rights, and installs no services, scheduled tasks or drivers.

## Verifying a download

Only download Deskwave from this repository's
[Releases page](https://github.com/sharabarinillia/Deskwave/releases). Each release is built by
[`.github/workflows/release.yml`](.github/workflows/release.yml) on GitHub's runners, from a
tagged commit. It comes with:

- `SHA256SUMS.txt`: compare it with `(Get-FileHash <file> -Algorithm SHA256).Hash`;
- a signed build provenance attestation: check it with
  `gh attestation verify <file> --repo sharabarinillia/Deskwave`.

The installers are not code-signed yet, so SmartScreen and sometimes Microsoft Defender warn about
them. The README explains
[what to do](README.md#windows-defender-or-smartscreen-warns-about-the-file). Never disable your
antivirus for Deskwave.

## Reporting a vulnerability

Please report privately through GitHub's **Security → Report a vulnerability** form on this
repository. Don't open a public issue. Include the steps to reproduce and the Windows version.

In scope, among others:

- a mod escaping its sandbox (reading files, reaching the network, calling app commands, reading
  another widget's storage);
- path traversal through the `dwmod` protocol;
- any way for Deskwave to make a network request while offline;
- any way for a mod to reach a host it wasn't approved for, or the user's local network;
- a release file whose checksum or attestation doesn't match.
