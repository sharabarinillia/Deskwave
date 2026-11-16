# Deskwave — specification

Deskwave replaces the desktop wallpaper on Windows and macOS with an animated one and lets you
place widgets on the desktop. Anyone can write their own widgets ("mods") in HTML/CSS/JS.

Status: v0.4. Sections marked **(planned)** are designed but not built yet.

## Principles

1. **Local first.** Everything runs on the user's computer. No account, no server, no telemetry,
   no crash uploads, no automatic update checks.
2. **Offline by default.** The app makes no network requests unless the user turns the
   internet switch on, and then only for widgets they approved, to the hosts those
   widgets declared.
3. **Mods are untrusted.** A mod can only draw inside its own box and call the small API
   the host gives it. It cannot read files, other mods, the network or the Tauri IPC.
4. **Light on resources.** Animations pause when nobody can see them or the laptop is on
   battery.
5. **Auditable.** Small dependency list, no native plugin system, public source code
   ([PolyForm Noncommercial 1.0.0](../LICENSE): free for non-commercial use only).

## Platform

- Windows 10 (1809+) and Windows 11, including 24H2+ "raised desktop". Pages render in WebView2.
- macOS 12+, one universal app for Apple Silicon and Intel. Pages render in WKWebView.
- Tauri 2 + Rust. Everything that differs between the two lives in `src-tauri/src/platform`
  (`win.rs`, `mac.rs`), behind the same functions.
- Multi-monitor: each monitor has its own wallpaper and widget layout. Per-monitor DPI.

## Architecture

```
Deskwave (Rust, Tauri 2)
 ├─ poller thread ── pause state, battery, click-through, Explorer-restart watchdog
 ├─ dwmod:// protocol ── serves mod files + SDK, adds CSP / sandbox headers
 ├─ tray / menu bar icon ── click: Settings · menu: Open · Edit widgets · Pause · (Mac) Keep awake · Quit
 └─ web view windows (WebView2 on Windows, WKWebView on macOS)
     ├─ wallpaper-N   per monitor, on the desktop below the icons
     │    └─ <iframe sandbox> wallpaper mod, a gradient, an album, or <video>/<img> from the wallpapers folder
     ├─ widgets-N     per monitor, transparent, just above the desktop
     │    └─ <iframe sandbox> one per placed widget
     └─ settings      normal window, opened at launch and from the tray
```

Repository layout:

| Path | What |
|---|---|
| `crates/deskwave-core` | Platform-independent logic: manifest validation, CSP, config, storage, languages. Unit-tested on any OS. |
| `src-tauri` | The app: windows, desktop integration (`src/platform`), protocol, IPC commands, tray. |
| `ui` | Host pages (`wallpaper.html`, `widgets.html`, `settings.html`) and their translations. No bundler, no npm runtime deps. |
| `sdk` | `deskwave.js` / `deskwave.css` that mods include. Served at `<MOD_ORIGIN>/_sdk/` (`http://dwmod.localhost` on Windows, `dwmod://localhost` on macOS). |
| `mods` | Built-in mods. They use the same API and sandbox as third-party mods. |

### Desktop integration (Windows)

- **Wallpaper windows** are parented into the shell's desktop, below the icons.
  - Classic desktop (Win10, Win11 before 24H2): send `0x052C` to `Progman`, find the
    `WorkerW` that follows the window hosting `SHELLDLL_DefView`, and `SetParent` into it.
  - Raised desktop (24H2+; `Progman` has `WS_EX_NOREDIRECTIONBITMAP` and hosts
    `SHELLDLL_DefView`): make the window `WS_EX_LAYERED` (alpha 255) before
    `SetParent(Progman)`, then z-order it directly below `SHELLDLL_DefView`.
  - A watchdog re-attaches the windows if Explorer restarts. On exit the static wallpaper
    is repainted.
- **Widget layers** are transparent top-level windows covering each monitor's work area.
  Their *owner* is the desktop window, which keeps them above the wallpaper, behind normal
  apps, out of Alt+Tab and the taskbar, and visible on Show Desktop (Win+D).
  - Click-through: the layer ignores the mouse except over a widget. The poller checks the
    cursor against the widget rectangles about 30 times a second and toggles
    `set_ignore_cursor_events`. Desktop icons stay clickable everywhere else.
  - After a widget takes focus (for example to type in a note), the layer sinks back
    behind other apps.
- **Edit mode** (tray → Edit widgets): layers come to the front with a scrim. Widgets can be
  dragged, resized (8 px grid), added and removed. Esc or Done exits.

### Desktop integration (macOS)

- **Window levels** do what parenting does on Windows (`platform/mac.rs`, through `msg_send!` on
  the `NSWindow` Tauri creates, always on the main thread):
  - **Wallpaper windows** sit at `kCGDesktopWindowLevel`, the level of the desktop picture, so
    they're below the desktop icons. They ignore the mouse, so clicks reach Finder's desktop.
  - **Widget layers** sit one level above `kCGDesktopIconWindowLevel`: above the icons, below
    every app. Clicking a widget doesn't raise it. Edit mode lifts them to the floating level.
  - Both join every Space, stay put in Mission Control and stay out of Cmd-\` cycling.
- **Transparent widget layers** need Tauri's `macOSPrivateApi`, so Deskwave can't go to the Mac
  App Store. It is distributed from GitHub.
- **No Dock icon** while only the desktop layers show (`ActivationPolicy::Accessory`). Opening
  Settings switches to `Regular`, so Settings appears in the Dock and Cmd-Tab, and closing it
  switches back.
- **Not available:** Now playing (macOS has no public API to read or control other apps'
  playback, so the media widget isn't offered) and media `permissions` therefore do nothing.

### Pausing

The poller re-evaluates once a second. Each monitor's wallpaper pauses (video paused, mods
receive `pause`) when any enabled condition holds:

| Setting (default on) | Windows | macOS |
|---|---|---|
| Fullscreen app or game | A visible window covers the whole monitor | The wallpaper window's `occlusionState` says it's hidden (windows, a full-screen app, another Space); one option covers both rows |
| Desktop covered | A maximized window is on the monitor | (same as above) |
| On battery | AC unplugged or Battery Saver on | On battery, or Low Power Mode |
| Locked | Workstation locked, or a Remote Desktop session | Screen locked, or another user's session in front |
| Tray → Pause animations | Manual, all monitors | Manual, all monitors |
| Keep awake is on | (macOS only) | No input for 10 minutes, so an always-on screen isn't animating all night |

Ignored when checking coverage on Windows: our own windows, the shell desktop, cloaked windows,
minimized windows and click-through/tool overlays (like the GPU driver overlay).

### Launch and startup

- **Opening Deskwave opens Settings,** from the Start menu or a shortcut, so nobody has to hunt
  for the tray icon.
- **One instance per user session.**
  - Windows: mutex `Local\Deskwave.SingleInstance`. A second launch signals the running one
    (event `Local\Deskwave.ShowSettings`) to show Settings, then exits.
  - macOS: opening the app again from Finder, Launchpad or the Dock reaches the running copy as
    `RunEvent::Reopen`, which shows Settings. A second process started another way finds the
    `flock` on `$TMPDIR/app.deskwave.desktop.lock` held and exits.
- **Start at sign-in is opt-in.** On first run Settings asks once. If the user agrees:
  - Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` gets the value `Deskwave` =
    `"<exe>" --background`;
  - macOS: a LaunchAgent, `~/Library/LaunchAgents/app.deskwave.desktop.plist`, runs
    `<exe> --background` (macOS shows it under Login Items as a background item).

  `--background` starts without opening Settings. It can be switched under General at any time.
- **Upgrade from ColorBy:** on start, `%APPDATA%\ColorBy` is moved to `%APPDATA%\Deskwave`,
  built-in ids `colorby.*` become `deskwave.*`, and the old `ColorBy` Run value is removed.

### Gradient wallpapers

- **Made in Settings → Wallpaper → New gradient** and stored in `config.json` under
  `gradients` (at most 50). A monitor's wallpaper can then be `{ "kind": "gradient", "id": … }`.

  ```json
  { "id": "g1", "colors": ["#0b0f2b", "#5b3fa8", "#ff5c9a"], "style": "flow", "angle": 135, "motion": "slow" }
  ```

- **Validated in Rust:** 2 to 5 `#rrggbb` colors, `style` is `linear`, `radial` or `flow`, `angle`
  is 0 to 359, `motion` is `still`, `slow`, `medium` or `fast`.
- **Deleting a gradient in use** switches those monitors back to Aurora.
- **Rendering** (`ui/js/gradient.js`):
  - **Still:** a plain CSS gradient, so nothing runs per frame.
  - **Moving:** a canvas at a tenth of the size, scaled up smoothly by the browser, at 20 fps.
  - It pauses with the wallpaper, and reduced motion shows the still version.
- **Gradients are drawn by the host page,** not a mod, so they need no sandbox or permissions.

### Albums

- **Made in Settings → Wallpaper → New album** and stored in `config.json` under `albums` (at
  most 50). A monitor's wallpaper can then be `{ "kind": "album", "id": … }`.

  ```json
  { "id": "a1", "name": "Summer", "items": ["beach.mp4", "sunset.jpg"], "interval": 30, "order": "shuffle", "layout": "collage" }
  ```

- **Validated in Rust:** a trimmed name of 1-60 characters; 1-200 items, each a plain file name
  of a video or image (no folders, no `..`); `interval` (seconds a photo stays up) is one of 5,
  10, 30, 60, 300, 900, 1800 or 3600; `order` is `inorder` or `shuffle`; `layout` is `single` or
  `collage`. Saving drops items that are no longer in the wallpapers folder.
- **Playing** (`ui/js/album.js`, also used for the live preview in Settings):
  - photos stay up for `interval` seconds; videos play to the end (a single-video album loops);
  - each tile crossfades between two layers, so at most two items per tile are loaded; reduced
    motion swaps without fading;
  - a collage shows up to four tiles, each starting at a different point in the album and
    changing at a different moment;
  - files that are missing or unreadable are skipped, and after a whole round of failures the
    tile waits 30 s before trying again;
  - pausing stops videos and photo timers and keeps the remaining time.
- **Deleting an album** doesn't delete its files; monitors that used it go back to Aurora.

### Drag and drop

- **What can be dropped onto Settings:** videos and images (copied into the wallpapers folder,
  and into the album being edited, if any) or, on the Mods page, mod folders (copied into the
  mods folder, where they stay off until approved, like any new mod).
- **The page never names the paths.** Rust records the paths from the OS drop event
  (`WindowEvent::DragDrop`), which reaches Rust before the page hears of it. The page then calls
  `drop_import` with only a target (`wallpapers` or `mods`). So the page cannot make Deskwave
  copy files of its choosing. Paths are used once and expire after 60 s.
- **Copying** (`deskwave_core::files`, unit-tested):
  - media: only regular files with a video or image extension and a plain name; never
    overwrites, picking `name (2).ext` instead;
  - mods: only a folder with a `manifest.json` at its top; symbolic links are left out, and
    folders over 2,000 files, 100 MB or 16 levels deep are refused, leaving nothing behind;
  - at most 500 paths per drop.
- **Tauri adds dropped paths to the asset protocol scope** for the session. Deskwave doesn't use
  that (it serves only the copies in its own folder); mod pages can't use the asset protocol at
  all (their CSP allows only their own folder).

### Keeping awake (macOS)

- **Off by default.** General → Keep awake, or the menu bar icon's **Keep screen awake** item:
  - **Keep the screen on:** an IOKit `PreventUserIdleDisplaySleep` assertion (the Mac stays
    awake too);
  - **Keep the Mac awake:** `PreventUserIdleSystemSleep`; the screen may turn off.
- The assertion is named "Deskwave: Keep awake is on", so it shows in Activity Monitor and
  `pmset -g assertions`. It's released when turned off and ends with the process.
- The choice is saved (`keep_awake` in `config.json`) and applied again at start.
- Closing a MacBook's lid still sleeps it unless it's in clamshell mode with power and an
  external display; Settings says so.
- While it's on, wallpapers pause after 10 minutes without input (see Pausing).

### Languages

- **Supported:** English, Ukrainian, French, Dutch, German, Polish and Spanish
  (`en`, `uk`, `fr`, `nl`, `de`, `pl`, `es`).
- **Rust picks the language:**
  1. the user's choice under General → Language;
  2. otherwise the first supported system language (Windows: `GetUserPreferredUILanguages`;
     macOS: `CFLocaleCopyPreferredLanguages`);
  3. otherwise English.
- **Host pages** receive `lang` from their `*_get` command:
  - texts live in `ui/js/strings.js`;
  - plurals use `Intl.PluralRules`;
  - dates, times and units use `Intl`.
- **The tray menu** comes from `deskwave_core::i18n`.
- **macOS wording:** where a text names Windows things (the taskbar, Win+D, "maximized"), a
  `<key>@mac` variant replaces it on a Mac (`setPlatform` in `ui/js/i18n.js`).
- **Mods** receive the language too:
  - a manifest can carry `translations` of its name and description;
  - the SDK's `translate` and `localize` helpers do the rest (see SDK).
- **Changing the language** reloads the pages and restarts widgets.
- **`npm run check-strings`** (also in CI) fails if any language misses a text or a `{placeholder}`.

## Mods

A mod is a folder containing `manifest.json` and its web files. Built-in mods live in
the app's resources. User mods go in `%APPDATA%\Deskwave\mods\<folder>\`.

```json
{
  "id": "acme.pomodoro",
  "name": "Pomodoro",
  "version": "1.0.0",
  "kind": "widget",
  "entry": "index.html",
  "author": "Acme",
  "description": "25/5 focus timer",
  "permissions": ["battery"],
  "network": ["api.example.com"],
  "size": { "w": 240, "h": 160 },
  "translations": { "uk": { "name": "Помодоро", "description": "Фокус 25/5" } }
}
```

| Field | Rules |
|---|---|
| `id` | Lowercase, dot-separated, at least two parts: `[a-z0-9-]+(\.[a-z0-9-]+)+`, max 64 chars. Unique. |
| `kind` | `widget` (placed on the widget layer) or `wallpaper` (fills a monitor). |
| `entry` | Relative path inside the folder. Path segments may contain only `A-Z a-z 0-9 - _ .`. |
| `permissions` | Subset of the table below. Unknown permissions or fields reject the manifest. |
| `network` | Up to 8 exact public host names the mod may fetch from. IP addresses, `localhost` and local-network names (`.local`, `.lan`, `.internal`, ...) are refused. |
| `size` | Default widget size in CSS px (widgets only). |
| `translations` | Optional. Per language code, a localized `name` and/or `description`. |

### Permissions

| Permission | Grants |
|---|---|
| *(always)* | Draw in its box, `deskwave.storage` (its own key-value store, 256 KiB per placed widget), theme and pause events |
| `battery` | `deskwave.battery.get()` and `battery` events |
| `system` | `deskwave.system.get()` and `system` events: CPU, RAM, disk and network throughput |
| `media` | `deskwave.media.get()`, `toggle()`, `next()`, `previous()` and `media` events |
| hosts in `network` | `deskwave.net.fetch(url)` to exactly those hosts, only while the internet switch is on |

Approval:

- A user mod is **inactive until approved** in Settings → Mods. The approval screen lists
  its permissions and hosts.
- If an update requests a permission or host that wasn't approved, the mod deactivates again
  until re-approved.
- Built-in mods are approved on first run but declare their permissions like everyone else.
- Ids starting with `deskwave.` are reserved for built-in mods.
- The protocol refuses to serve files of a mod that is not approved.

### Sandbox (defence in depth)

1. **Opaque origin.** Each mod runs in `<iframe sandbox="allow-scripts">`. The response
   CSP also contains `sandbox allow-scripts`, so the mod is sandboxed even if loaded
   another way. It has no cookies, `localStorage`, parent DOM access or top-level
   navigation.
2. **CSP on every mod file.** Scripts, styles, images, fonts and media may only come from the
   mod's own folder or the SDK. No inline scripts, no `eval`, `connect-src 'none'`, no
   frames, no workers, no forms.
3. **Host page CSP** has `frame-src http://dwmod.localhost dwmod:` (the Windows and macOS forms
   of the mod origin), so a mod frame cannot navigate itself anywhere else.
4. **No IPC.** Tauri's IPC is only injected into our host pages. Each host page has its own
   capability file listing exactly the commands it may call.
5. **Dead proxy.** If a CSP gap appeared, a request would still have nowhere to go:
   - Windows: every WebView2 starts with `--proxy-server=127.0.0.1:9` (nothing listens there),
     plus WebRTC limited to proxied UDP and background networking off;
   - macOS 14+: WebKit's shared data store gets a proxy on 127.0.0.1:9, and WebRTC is turned
     off in every web view's preferences (`platform::harden_webview`). These calls are looked
     up at run time; on macOS 12-13 the proxy isn't available, so layers 1-4 and 6-7 carry it.
6. **Permissions-Policy** denies camera, microphone, geolocation, display capture and USB/HID/serial.
7. **Validated bridge.** Mods talk to the host only through a `MessagePort` handed over at
   load. The host checks the method and permission on every request. Rust re-validates
   sizes and IDs.

### SDK

```html
<link rel="stylesheet" href="/_sdk/deskwave.css">
<script src="/_sdk/deskwave.js"></script>
```

```js
const ctx = await deskwave.ready;           // { instance, kind }
deskwave.on('pause', () => {});             // wallpapers: stop animating
deskwave.on('resume', () => {});
deskwave.on('theme', ({ accent }) => {});   // also sets --dw-accent on :root
await deskwave.storage.set('text', 'hi');   // JSON values, per widget instance
await deskwave.storage.get('text');
await deskwave.battery.get();               // needs "battery": { percent, charging, onAc, saver, present }
deskwave.on('battery', (b) => {});
await deskwave.net.fetch('https://api.example.com/x'); // host in "network": { status, contentType, body }
deskwave.on('internet', (on) => {});        // the user flipped the internet switch
await deskwave.system.get();                // needs "system": { cpu, memoryUsed, memoryTotal, diskFree, diskTotal, netDown, netUp }
await deskwave.media.get();                 // needs "media": { active, playing, title, artist, app }
await deskwave.media.toggle();              // also next(), previous()

deskwave.lang;                              // 'en', 'uk', 'fr', 'nl', 'de', 'pl' or 'es'
const t = deskwave.translate({ en: { hi: 'Hi {name}' }, uk: { hi: 'Привіт, {name}' } });
t('hi', { name: 'Sam' });                   // missing texts fall back to English
// Plurals: { one: '{n} task', other: '{n} tasks' }, picked with Intl.PluralRules by params.n.
deskwave.localize(t);                       // fills data-t, data-t-placeholder, data-t-label, data-t-title
```

The SDK stylesheet also offers Fluent controls: `.dw-btn` (with `primary`, `subtle` and `icon`),
`.dw-input` and `.dw-check`.

Things the sandbox rules out, so mods must avoid them:
- **External SVG sprites** (`<use href="icons.svg#x">`): the mod's origin is opaque, so these are
  blocked as cross-origin. Put the `<symbol>`s inline in the page.
- **`<form>` submission**: it is disabled. Handle the button click and the Enter key instead.
- **`localStorage` and cookies**: use `deskwave.storage`.

Mod pages should keep a transparent background. The host draws the widget card (background,
radius, shadow), so all widgets match. Light/dark: use `prefers-color-scheme`.

## Network broker

- **The broker (`src-tauri/src/broker.rs`) is the only HTTP client in the app.** WebViews never
  reach the internet (see dead proxy).
- A request succeeds only if all of these hold:
  - the internet switch is on (Settings → Privacy & about, off by default);
  - the mod is approved, including the host;
  - the URL is `https://`, has no credentials and no non-default port, and its host is exactly
    one of the approved hosts. The check uses the same `http::Uri` parser as the HTTP client.
- **The mod is identified by Rust,** from the calling window and widget id, not trusted from
  the page.
- **No private addresses:** every address a name resolves to must be public. A name that
  resolves into the user's own network is refused, which also stops DNS rebinding.
- **No redirects, no proxy.** A redirect could leave the approved hosts, and a proxy would
  resolve names itself and skip the private-address check.
- **Limits:** GET only, 10 s timeout, 1 MiB response, 30 requests a minute per mod, no
  cookies or credentials.
- **Settings → Privacy & about shows the last requests** (time, mod, host, result).
- **Weather** uses Open-Meteo with a city the user types, never IP geolocation.

## Data

Everything lives in `%APPDATA%\Deskwave\` on Windows and
`~/Library/Application Support/Deskwave/` on macOS:

| Path | What |
|---|---|
| `config.json` | Layouts, wallpapers, gradients, albums, approvals, the internet switch, language, pause settings, keep awake. Written atomically. A corrupt file is moved to `config.json.bad` and defaults are used. |
| `storage\<widget-id>.json` | Each widget's key-value store (e.g. note text). |
| `mods\` | User mods. |
| `wallpapers\` | Video (`.mp4`, `.webm`) and image (`.gif`, `.png`, `.jpg`, `.webp`, `.avif`, `.apng`) wallpapers. |

On Windows, the setup.exe uninstaller removes the Run value. If the user ticks "Delete the application data",
it also removes `config.json` and `storage\` (`src-tauri/installer/hooks.nsh`), but never `mods\`
or `wallpapers\`, which hold the user's own files. On macOS, the README tells users to turn off
start at sign-in before dragging the app to the Trash. Deleting the whole folder resets Deskwave.

## Threat model (summary)

| Threat | Mitigation |
|---|---|
| Malicious mod steals files or data | No filesystem or network API; opaque origin; CSP; storage scoped per widget instance |
| Malicious mod phones home | `connect-src 'none'`, `frame-src` lock, dead proxy; the broker only reaches approved hosts while the internet switch is on |
| Approved mod probes the home network | Hosts must be public names; resolved addresses must be public; no redirects |
| Mod path traversal (`../`) | Strict path charset, canonicalised path must stay inside the mod folder |
| Mod reaches Tauri IPC | IPC only in host pages; per-window capabilities |
| Remote attacker reaches the app | Deskwave opens no ports and starts no server |
| Supply chain | Few crates, lockfiles committed, no npm runtime deps, CI builds from source |
| Tampered download | Releases built on GitHub's runners from a tag, with `SHA256SUMS.txt` and a signed build provenance attestation (`gh attestation verify`) |
| Installer looks like a dropper | Installer embeds Microsoft's signed WebView2 setup instead of downloading one; no admin rights, services or autostart without consent |
| A page makes Deskwave copy files | Drop imports take paths only from the OS drop event, never from the page; only media files or mod folders, with size limits and no links |
| A Mac is kept awake without the user knowing | Off by default; a named power assertion visible in Activity Monitor and `pmset`; released when turned off or on quit |

## Roadmap

- **v0.1:**
  - wallpaper layer: built-in "Aurora" preset, video/GIF/image files;
  - widget layer with edit mode;
  - widgets: clock & date, notes, battery;
  - mod loading, approval and sandbox;
  - auto-pause, tray and settings;
  - CI building an installer.
- **v0.2:**
  - internet switch and network broker, with an activity list;
  - widgets: weather, system stats, to-do, focus timer, now playing (Windows media session);
  - start with Windows, on by default and switchable in Settings;
  - opening Deskwave again opens Settings;
  - Fluent controls in the mod SDK.
- **v0.3:**
  - renamed from ColorBy to Deskwave, with automatic data migration;
  - gradient creator: linear, radial and flow, still or moving;
  - seven languages, for the app, tray and built-in widgets, plus translation helpers for mods;
  - opening Deskwave opens Settings; start at sign-in is asked, not assumed;
  - new icon; About with author, license and the no-fees notice;
  - PolyForm Noncommercial license;
  - releases with an MSI, checksums and build provenance, published from a version tag;
  - installers open with a warning page: only the official Releases page is safe, no warranty,
    then the license (`scripts/installer-license.mjs`);
  - one download zip with clearly named folders and a READ ME FIRST file (`scripts/package.mjs`).
- **v0.4 (now):**
  - macOS version: universal app for Apple Silicon and Intel, menu bar icon, LaunchAgent for
    sign-in, disk image with a READ ME FIRST file;
  - keep awake on macOS (off by default): screen on, or the Mac only;
  - albums: videos and photos taking turns, one at a time or as a collage;
  - drag and drop of videos, photos and mod folders onto Settings.
- **Later:**
  - mod packaging (`.zip` import) and a mod author guide;
  - per-widget settings UI;
  - more wallpaper presets;
  - widget themes;
  - a global hotkey for edit mode;
  - album art for Now playing;
  - a code-signing certificate for the installers and Apple notarization;
  - reordering album items by dragging;
  - a translated installer.
