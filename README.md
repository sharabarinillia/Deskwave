<!--
  The pictures in docs/images are rendered from Deskwave's real interface with sample data.
  To swap in real screenshots, replace the files in docs/images and keep the same file names.
-->

<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="112" height="112" alt="Deskwave logo: a wave on a screen">
</p>

<h1 align="center">Deskwave</h1>

<p align="center">
  <b>Animated wallpapers and desktop widgets for Windows 10 and 11.</b><br>
  Private, offline by default, free for non-commercial use.
</p>

<p align="center">
  <a href="https://github.com/sharabarinillia/Deskwave/releases/latest/download/Deskwave-Setup.exe"><b>Download for Windows</b></a> ·
  <a href="https://github.com/sharabarinillia/Deskwave/releases/latest">All downloads</a> ·
  <a href="#install">Install guide</a> ·
  <a href="#features">Features</a> ·
  <a href="#privacy-and-security">Privacy and security</a> ·
  <a href="#write-your-own-widget">Write a widget</a>
</p>

<p align="center">
  <a href="https://github.com/sharabarinillia/Deskwave/actions/workflows/ci.yml"><img src="https://github.com/sharabarinillia/Deskwave/actions/workflows/ci.yml/badge.svg" alt="Build status"></a>
</p>

![Deskwave on a Windows desktop: a moving wallpaper behind clock, weather, notes, system, focus timer, to-do, now playing and battery widgets](docs/images/desktop.jpg)

Windows has no desktop widgets and no moving wallpapers. Deskwave adds both: it plays an
animation behind your desktop icons and lets you pin the widgets you need to the desktop itself.
Everything runs on your PC. Deskwave has no account, sends no data about you, and doesn't touch
the internet unless you switch it on.

## Contents

- [Features](#features)
- [Install](#install): [step by step](#step-by-step) · [advanced](#advanced-install)
- [Windows Defender or SmartScreen warns about the file](#windows-defender-or-smartscreen-warns-about-the-file)
- [Using Deskwave](#using-deskwave)
- [Privacy and security](#privacy-and-security)
- [Write your own widget](#write-your-own-widget)
- [Uninstall](#uninstall)
- [No warranty](#no-warranty)
- [Build from source](#build-from-source)
- [License](#license)

## Features

### Wallpapers that move

- **Aurora**, a built-in animated wallpaper: soft northern lights that change color through the day.
- **Your own videos and images:** `.mp4`, `.webm`, `.gif`, `.png`, `.jpg` and `.webp`.
- **Gradient creator.** Make your own still or moving gradient:
  - 2 to 5 colors, or start from a palette;
  - three styles: linear (with an angle), radial and flow;
  - four speeds: still, slow, medium and fast.
- **A different wallpaper on each display.**

| Make a gradient | On the desktop |
|---|---|
| ![The gradient editor in Settings: color swatches, palettes, style and motion controls next to a live preview](docs/images/settings-gradient.png) | ![A moving "flow" gradient in deep blue, violet, pink and amber behind the widgets](docs/images/gradient-desktop.jpg) |

### Widgets on your desktop

| Widget | What it shows | Uses the internet |
|---|---|---|
| Clock | Time and date in your language | No |
| Notes | A note that stays on your desktop | No |
| To-do | A short list of things to get done | No |
| Focus timer | 25 minutes of focus, then a 5-minute break | No |
| Battery | Charge level and time left (laptops) | No |
| System | Processor, memory, disk and network use | No |
| Now playing | What's playing in any app, with play, pause and skip | No |
| Weather | Current weather and today's high and low | Yes, only when you allow it |

Move, resize, add and remove widgets right on the desktop:

![Edit mode: each widget has a frame, a remove button and a resize corner, with an "Add widget" toolbar at the top](docs/images/edit-mode.jpg)

### Made to stay out of your way

- **Light on your PC.** Animations pause when you can't see them:
  - while a game or video runs full screen;
  - while a window is maximized;
  - on battery;
  - while the PC is locked or used through Remote Desktop.
- **Feels like Windows.** It follows light and dark mode and your accent color, works with the
  keyboard and screen readers, and calms its animations when Windows is set to show fewer.
- **Speaks your language.** English, Українська, Français, Nederlands, Deutsch, Polski and
  Español. It follows your Windows language, or you can pick one in Settings.
- **Extendable.** Anyone can write widgets and wallpapers ("mods") in HTML, CSS and JavaScript.
  Each one runs in a sandbox and stays off until you approve it.

| Mods, in dark mode | Privacy and about |
|---|---|
| ![Settings, Mods page in dark mode: built-in mods and a new third-party mod waiting for approval, with its permissions listed](docs/images/settings-mods-dark.png) | ![Settings, Privacy and about page: the internet switch, recent internet activity and the About section](docs/images/settings-about.png) |

## Install

**You need:** Windows 10 or 11, 64-bit.

> [!WARNING]
> **Only download Deskwave from this repository's
> [Releases page](https://github.com/sharabarinillia/Deskwave/releases).** Copies from anywhere
> else may have been changed and are not safe.

Every release has these files under **Assets**:

| File | Who it's for |
|---|---|
| **[Deskwave-Setup.exe](https://github.com/sharabarinillia/Deskwave/releases/latest/download/Deskwave-Setup.exe)** | **Most people.** Installs for your account only. No administrator rights needed. |
| [Deskwave.msi](https://github.com/sharabarinillia/Deskwave/releases/latest/download/Deskwave.msi) | IT admins and managed PCs. Installs for all users. Needs administrator rights. |
| [Deskwave-Windows.zip](https://github.com/sharabarinillia/Deskwave/releases/latest/download/Deskwave-Windows.zip) | Both installers in clearly named folders, with a READ ME FIRST file. |
| [SHA256SUMS.txt](https://github.com/sharabarinillia/Deskwave/releases/latest/download/SHA256SUMS.txt) | Fingerprints to check your download is genuine (see [advanced](#advanced-install)). |

Inside `Deskwave-Windows.zip`:

```text
1 - Setup (recommended)/Deskwave-Setup.exe
2 - MSI (for IT admins)/Deskwave.msi
READ ME FIRST.txt
SHA256SUMS.txt
```

### Step by step

1. Select **[Download Deskwave-Setup.exe](https://github.com/sharabarinillia/Deskwave/releases/latest/download/Deskwave-Setup.exe)**. It downloads to your
   **Downloads** folder.
2. Open the file you downloaded.
3. Windows may show a blue box that says **"Windows protected your PC"**. This is normal for new
   apps that aren't signed with a paid certificate (see
   [why](#windows-defender-or-smartscreen-warns-about-the-file)). If you downloaded the file with
   the link in step 1:
   1. select **More info**;
   2. check that the app name starts with **Deskwave**;
   3. select **Run anyway**.

   If you got the file anywhere else, delete it instead.
4. The installer shows a warning and the license. Read it, then select **I Agree**.
5. Select **Next**, then **Install**, then **Finish**.
6. Deskwave opens its Settings window. It asks whether to start Deskwave when you sign in: select
   **Turn on** to keep your wallpaper and widgets after a restart, or **Not now**.

   <img src="docs/images/settings-wallpaper.png" width="640" alt="The Settings window on first start: a 'Start Deskwave when you sign in?' bar with Turn on and Not now buttons, above the Wallpaper page">

Next time, open Deskwave from the **Start menu**. You'll also find its icon in the taskbar corner
(select the **^** arrow next to the clock if you don't see it).

### Advanced install

**Check the download is genuine.** Compare the file's SHA-256 checksum with the line for that file
in `SHA256SUMS.txt` from the same release. Upper or lower case doesn't matter.

```powershell
(Get-FileHash .\Deskwave-Setup.exe -Algorithm SHA256).Hash
```

**Check where it was built.** Releases of the public repository carry a signed
[build provenance attestation](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations).
It proves the file was built by this repository's release workflow on GitHub's own servers, from a
tagged commit. With the [GitHub CLI](https://cli.github.com):

```bash
gh attestation verify Deskwave-Setup.exe --repo sharabarinillia/Deskwave
```

**Install without questions** (both installers):

```powershell
.\Deskwave-Setup.exe /S
```

```powershell
msiexec /i Deskwave.msi /qn
```

The MSI command needs an administrator terminal.

**Start without opening Settings:** run `deskwave.exe --background`. The sign-in entry uses this.

**Where things are:**

| What | Where |
|---|---|
| App (setup.exe) | `%LOCALAPPDATA%\Deskwave` |
| Settings and widget data | `%APPDATA%\Deskwave\config.json` and `storage\` |
| Your wallpapers | `%APPDATA%\Deskwave\wallpapers` |
| Third-party mods | `%APPDATA%\Deskwave\mods` |
| Start at sign-in (only if you turned it on) | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, value `Deskwave` |

Deskwave installs no services, scheduled tasks or drivers. The app itself never asks for
administrator rights.

## Windows Defender or SmartScreen warns about the file

**Why it happens.**
- **SmartScreen** trusts files by reputation. A new app that isn't signed with a paid
  code-signing certificate starts with none, so Windows asks before running it.
- **Microsoft Defender** sometimes flags unsigned apps built with Rust and Tauri, the tools Deskwave
  uses, with a "machine learning" detection whose name ends in `!ml` (often
  `Trojan:Win32/Wacatac.B!ml`). This is a guess from the file's shape, not a match with known
  malware, and it's a
  [known false positive](https://github.com/tauri-apps/tauri/issues?q=is%3Aissue+wacatac) for apps
  built this way.

**What Deskwave does about it:**
- The installer downloads nothing. If your PC lacks Microsoft's WebView2, which Windows 11 always
  has, Microsoft's own signed setup inside the installer adds it.
- Deskwave doesn't add itself to startup unless you say yes.
- It needs no administrator rights, opens no ports and runs no background services.
- Every release is built on GitHub's servers from this public source code, with checksums and a
  signed build provenance attestation.

**What you can safely do:**
1. Only download Deskwave from this repository's
   [Releases page](https://github.com/sharabarinillia/Deskwave/releases).
2. [Check the checksum](#advanced-install). If it matches, the file is exactly what was built here.
3. For a second opinion, you can upload the file to [VirusTotal](https://www.virustotal.com), which
   checks it with about 70 virus scanners.
4. If Defender quarantined the file and the checksum matches, report the mistake to Microsoft at
   [microsoft.com/wdsi/filesubmission](https://www.microsoft.com/en-us/wdsi/filesubmission).
   Choose **Incorrectly detected as malware/malicious**. Once Microsoft clears it, update Defender
   (Windows Security → Virus & threat protection → Protection updates) and download the file again.

**Never** turn off Microsoft Defender or add exclusions to get Deskwave running. Never run a copy
from another website, even if it has the same name.

## Using Deskwave

- **Open Settings:** open Deskwave from the Start menu, or select its icon in the taskbar corner.
- **Taskbar icon menu** (right-click): Open Deskwave, Edit widgets, Pause animations, Quit Deskwave.
- **See your widgets:** they sit on the desktop behind your apps. Press **Windows logo key + D**.
- **Arrange widgets:** select **Edit widgets**, then:
  - drag a widget to move it;
  - drag its corner to resize it;
  - select **×** to remove it, or **Add widget** for more;
  - press **Esc** or select **Done** when you're finished.

  With the keyboard, arrow keys move the focused widget and Shift + arrow keys resize it.
- **Change the wallpaper:** **Wallpaper** page. Pick a display, then select a wallpaper.
  - For your own files, select **Open folder**, copy your videos or images into it, then select
    **Refresh**.
  - For a gradient, select **New gradient**. Choose colors, a style and a motion, then select
    **Use this gradient**. To change or remove one later, select **Edit** on its tile.
- **Language, startup and pausing:** **General** page.
- **Internet and recent activity:** **Privacy & about** page.

## Privacy and security

- **Offline by default.** Deskwave makes no network requests until you turn on **Let widgets use the
  internet** under Privacy & about.
- **When the internet is on:**
  - only widgets you approved can connect, and only to the sites they list;
  - connections are HTTPS-only, never redirected, never sent to your home or office network, and
    limited to 30 a minute for each widget;
  - every request appears under **Recent internet activity**;
  - the built-in Weather widget only talks to Open-Meteo: `geocoding-api.open-meteo.com` to find the
    city you type, `api.open-meteo.com` for the forecast. No API key, no account.
- **No telemetry.** No analytics, crash uploads or automatic update checks, and no account.
- **Nothing can connect in.** Deskwave runs no server and opens no ports.
- **Sandboxed mods.** Each mod runs in a locked-down sandbox:
  - it can't read your files, other widgets' data or the app's internals;
  - it can't use the camera, microphone or location;
  - new mods stay off until you approve them, and the approval screen lists exactly what each one
    can do.

The full design is in [docs/SPEC.md](docs/SPEC.md#sandbox-defence-in-depth). To report a
vulnerability privately, see [SECURITY.md](SECURITY.md).

## Write your own widget

A widget is a folder with a `manifest.json` and ordinary web files. Here is a complete one that
counts clicks and remembers the count.

`hello/manifest.json`:

```json
{
  "id": "yourname.hello",
  "name": "Hello",
  "version": "1.0.0",
  "kind": "widget",
  "entry": "index.html",
  "author": "Your name",
  "description": "Counts clicks",
  "size": { "w": 240, "h": 120 }
}
```

`hello/index.html`:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="/_sdk/deskwave.css">
  <script src="/_sdk/deskwave.js" defer></script>
  <script src="main.js" defer></script>
</head>
<body>
  <button class="dw-btn primary" id="hello">Clicked 0 times</button>
</body>
</html>
```

`hello/main.js`:

```js
const button = document.getElementById('hello');
let count = 0;

deskwave.ready.then(async () => {
  count = (await deskwave.storage.get('count')) ?? 0;
  button.textContent = `Clicked ${count} times`;
});

button.addEventListener('click', () => {
  count += 1;
  button.textContent = `Clicked ${count} times`;
  deskwave.storage.set('count', count);
});
```

To try it:
1. In Settings → Mods, select **Open folder**.
2. Copy the `hello` folder in, then select **Refresh**.
3. Turn the mod on.
4. Under **Edit widgets**, select **Add widget**.

Widgets can also:
- read the battery, system stats or media (with the matching permission);
- fetch from sites they declare;
- translate themselves into all seven languages.

The [mod reference](docs/SPEC.md#mods) has the details, and the built-in widgets in [`mods/`](mods)
are working examples. Mod ids starting with `deskwave.` are reserved for built-ins.

## Uninstall

1. Right-click the Deskwave icon in the taskbar corner and select **Quit Deskwave**.
2. Open **Settings → Apps → Installed apps**, find **Deskwave**, then select **⋯ → Uninstall**.
3. In the setup.exe uninstaller, tick **Delete the application data** to also remove your settings
   and widget data. Your own wallpapers and mods in `%APPDATA%\Deskwave` are kept. Delete that
   folder yourself if you don't need them.

The uninstaller also removes the start-at-sign-in entry. If you used the MSI, turn off
**Start Deskwave when you sign in** (General page) before uninstalling.

## No warranty

Deskwave is provided "as is", without any warranty of any kind. You install and use it at your
own risk. To the fullest extent permitted by law, the author is not liable for any damage or loss
caused by this program or by its use, including damage caused by copies obtained anywhere other
than this repository's [Releases page](https://github.com/sharabarinillia/Deskwave/releases).

## Build from source

On Windows, with [Rust](https://rustup.rs) and [Node.js](https://nodejs.org) 22 installed:

```bash
npm ci
```

```bash
npm run dev
```

```bash
npm run build
```

`npm run build` writes both installers to `target/release/bundle/`. `npm run package` then
gathers them into `dist/`, with the same names and folders as a release.

The core logic builds and tests on any OS. It covers manifest checks, sandbox headers, config,
storage and languages.

```bash
cargo test -p deskwave-core
```

To check that every language has every text:

```bash
npm run check-strings
```

| Path | What |
|---|---|
| `crates/deskwave-core` | Platform-independent logic, unit-tested |
| `src-tauri` | The Windows app (Tauri 2 and Rust) |
| `ui` | Settings window, widget layer and wallpaper pages, with all translations |
| `sdk` | Script and styles that mods include |
| `mods` | Built-in widgets and wallpapers |
| `docs/SPEC.md` | Architecture, security model and roadmap |

Releases are built and published by
[`.github/workflows/release.yml`](.github/workflows/release.yml) when a version tag such as
`v0.3.0` is pushed, or when a release with a new tag is published on the Releases page. The title
and notes come from `docs/releases/<tag>.md`, so write that file first.

## License

**ONLY FOR UNPAID USAGE. CHARGING ANY KIND OF FEES IS PROHIBITED.**

Deskwave is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE). You may use, share
and change it for any non-commercial purpose. You may not sell it, charge for it, or use it to
make money in any other way.

Made by [sharabarinillia](https://github.com/sharabarinillia).
