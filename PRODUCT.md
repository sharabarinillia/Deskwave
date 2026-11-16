# Product

## Platform

web

(Desktop app for Windows 10/11 and macOS 12+; every surface is HTML/CSS rendered in WebView2 or
WKWebView inside Tauri, so web rules apply. The look follows Windows 11 (Fluent) on both.)

## Stack

Tauri 2 + Rust core; vanilla HTML/CSS/JS host pages with no bundler and no npm runtime
dependencies. Strict CSP: no inline scripts or styles in host pages; mods run in sandboxed
iframes and include `/_sdk/deskwave.css` and `/_sdk/deskwave.js`.

## Users

Everyday Windows users who want a nicer, more useful desktop without technical knowledge.
Mod authors (people who write their own widgets in HTML/JS) are a smaller secondary audience.

## Product Purpose

Replace the static Windows wallpaper with an animated one, and put widgets (clock and date,
notes, battery, later weather, system stats, media, to-do, timers) on the desktop, which Windows
itself does not offer. Success: the desktop looks alive and useful, the PC doesn't slow down, and
the user never has to worry about privacy.

## Positioning

Local-first and offline by default: no account, no telemetry, no network unless the user turns it
on per widget. Third-party widgets run in a strict sandbox and must be approved with visible
permissions. Built-in widgets use the same mod API as community ones.

## Operating Context

- Lives in the system tray all day.
- Widgets sit on the desktop layer, behind app windows, above the wallpaper.
- Users arrange widgets in an edit mode (tray → Edit widgets).
- Settings opens from the tray.
- Animations pause automatically for fullscreen apps, maximized windows, battery, and the locked screen.

## Capabilities and Constraints

- Windows 10 and 11, and macOS 12+ (Apple Silicon and Intel); multi-monitor, per-monitor DPI.
- macOS extra: keep the screen or the Mac awake for overnight work (off by default).
- Wallpapers: built-in animated presets, video files, GIF/images, and interactive web scenes (mods).
- Mods: folder with `manifest.json` plus web files, sandboxed. Permissions are approved in Settings.
- Undecided: mod zip import.

## Brand Commitments

- Name: Deskwave.
- Must feel like a native part of Windows 11 (Fluent-style), follow system light/dark and
  the accent color. The user explicitly chose this.
- Source available under PolyForm Noncommercial 1.0.0: free for non-commercial use; charging any kind of fee is prohibited.
- Author: sharabarinillia (https://github.com/sharabarinillia).

## Evidence on Hand

No users, testimonials, or benchmarks exist yet; do not invent any.

## Product Principles

1. Plain words for everyday users; never expose internals (ids, paths) where a name will do.
2. Safe by default: offline, sandboxed, approval before any third-party code runs.
3. The wallpaper is the star; widgets are quiet and legible over any background.
4. Respect the machine: pause when unseen, honor battery and reduced-motion settings.

## Accessibility & Inclusion

WCAG 2.2 AA: readable contrast (including widget text over animated wallpapers), full keyboard use
of Settings, respect Windows "Animation effects" off (prefers-reduced-motion) and text scaling.
