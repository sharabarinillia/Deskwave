---
name: Deskwave
description: Animated wallpapers and desktop widgets that feel like part of Windows 11
colors:
  accent-fill: "#005fb8"
  ground: "#f3f3f3"
  layer: "#fbfbfb"
  flyout: "#f9f9f9"
  text: "rgb(0 0 0 / 0.896)"
  text-secondary: "rgb(0 0 0 / 0.606)"
  stroke: "rgb(0 0 0 / 0.0578)"
  ground-dark: "#202020"
  layer-dark: "#2b2b2b"
  text-dark: "#ffffff"
  text-secondary-dark: "rgb(255 255 255 / 0.786)"
  acrylic: "rgb(243 243 243 / 0.84)"
  acrylic-dark: "rgb(32 32 32 / 0.78)"
  caution: "#9d5d00"
  caution-bg: "#fff4ce"
  critical: "#c42b1c"
  critical-bg: "#fde7e9"
  success: "#0f7b0f"
  success-bg: "#dff6dd"
typography:
  title:
    fontFamily: "Segoe UI Variable Display, Segoe UI, system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 600
    lineHeight: "36px"
  subtitle:
    fontFamily: "Segoe UI Variable Display, Segoe UI, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: "28px"
  body:
    fontFamily: "Segoe UI Variable Text, Segoe UI, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
  body-strong:
    fontFamily: "Segoe UI Variable Text, Segoe UI, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 600
    lineHeight: "20px"
  caption:
    fontFamily: "Segoe UI Variable Text, Segoe UI, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: "16px"
  widget-numeral:
    fontFamily: "Segoe UI Variable Display, Segoe UI, system-ui, sans-serif"
    fontSize: "min(48cqh, 27cqw)"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "-0.02em"
rounded:
  control: "4px"
  tile: "6px"
  overlay: "8px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  xxl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.accent-fill}"
    textColor: "#ffffff"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "32px"
  button-standard:
    backgroundColor: "rgb(255 255 255 / 0.7)"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "32px"
  settings-row:
    backgroundColor: "{colors.layer}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "12px 16px"
  nav-item:
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "36px"
  widget-card:
    backgroundColor: "{colors.acrylic}"
    textColor: "{colors.text}"
    rounded: "{rounded.overlay}"
  switch-on:
    backgroundColor: "{colors.accent-fill}"
    width: "40px"
    height: "20px"
---

# Design System: Deskwave

## Overview

**Creative North Star: "The page Microsoft forgot to ship"**

Deskwave is Fluent 2 played straight. The Settings window is a Windows 11 Settings page; the widgets
are acrylic cards that could have shipped with the OS. Identity comes from three things only: the
to-scale monitor map, live wallpaper previews, and an Aurora that changes colour with the time of day.
The wallpaper is the star; everything else is quiet and legible over it.

**Key Characteristics:**
- System light/dark and the user's Windows accent drive every colour decision.
- Solid layer cards with hairline strokes on a Mica-tinted ground; no decorative gradients in chrome.
- Widgets are acrylic: dense tint plus fine grain, 8 px radius, soft two-layer shadow.
- One authored motion per moment: the edit toolbar drops in, the nav pill grows, pages fade up 8 px.

## Colors

The accent is never hard-coded: `ui/js/theme.js` derives `--accent-fill` (darker shade in light mode,
lighter in dark mode), `--accent-text` and `--on-accent` (black or white by contrast) from the Windows
accent. `#005fb8` is only the fallback.

### Primary
- **Windows accent** (`accent-fill`): selection pill, primary buttons, switches, selected monitor and tile rings.

### Neutral
- **Ground / Layer / Flyout** (`ground`, `layer`, `flyout`, and `-dark` variants): window background, setting cards, menus.
- **Text / Text secondary**: primary copy at 89.6 % black; descriptions and metadata at 60.6 %.

### Named Rules
**The Borrowed Accent Rule.** Accent comes from Windows at runtime. Never pick a brand hue for chrome.
**The Meaningful State Rule.** Caution, critical and success colours appear only where they carry
meaning: new unreviewed mods, errors, offline status, low or charging battery.

## Typography

Segoe UI Variable, as Windows ships it. The user pinned this explicitly; it is canon, not a fallback.

### Hierarchy
- **Title** 28/36 semibold: page titles.
- **Subtitle** 20/28 semibold: status headline (e.g. "Deskwave is offline").
- **Body** 14/20: rows, buttons, nav.
- **Body strong** 14/20 semibold: section headings inside a page.
- **Caption** 12/16 secondary: descriptions, metadata, hints.
- **Widget numeral** container-relative semibold with tabular numerals: clock time, battery percent.

## Layout

- **Settings:** 288 px navigation pane plus a content column of at most 1000 px. Groups of setting rows
  are 4 px apart; groups are 24 px apart; `h2` headings sit 32 px above and 8 px below their group.
- **Widgets:** snap to an 8 px grid. The edit-mode scrim shows 24 px dots.

## Elevation & Depth

Depth is quiet: cards carry a 1 px stroke and no shadow. Flyouts and the edit toolbar use
`0 8px 16px` shadow (14 % light, 26 % dark). Widgets float above the wallpaper with
`0 8px 24px` plus `0 1px 4px` and an inset 1 px light stroke. A lifted widget (dragging) deepens to
`0 18px 40px`.

## Shapes

Controls and setting rows have a 4 px radius. Widgets, flyouts and gallery tiles have 8 px (tile
media 6 px). Pills and circles take full rounding (`pill`): switches, the 3 × 16 px navigation
selection, count badges, meter and progress bars, the play button and status icons. Monitors in the map are 4 px rectangles at
true aspect ratio, with 4 px gaps.

## Components

### Buttons
Standard: translucent control fill, hairline stroke with a stronger bottom edge, 32 px tall. Accent:
`accent-fill` with `on-accent` text. Subtle: transparent until hovered.

### Cards / Containers
Setting row: icon (20 px) · title and caption description · actions on the right, at least 68 px tall.

### Inputs / Fields
Switches are native checkboxes with `role="switch"` and an "On"/"Off" label. The Notes text field
marks focus with a 2 px accent line along the bottom.

### Navigation
Transparent items with a subtle hover fill. The selected item gets the accent pill and a subtle fill.

### Monitor map (signature)
Displays drawn to scale from their real positions. The selected display fills with the accent and
shows its number and current wallpaper. It chooses which display the gallery below edits.

## Do's and Don'ts

### Do:
- Do take colour from the Windows accent and system theme; test both themes.
- Do build untrusted text (mod names, descriptions) with text nodes only.
- Do use authored line icons from `ui/img/icons.svg` (16 px, 1.4 stroke).
- Do keep mod pages transparent so the host card draws the chrome.

### Don't:
- Don't add eyebrows/kickers above headings, gradient text, or glass that is decoration rather than acrylic.
- Don't use emoji or Unicode glyphs as icons.
- Don't show internal ids or file paths where a name will do.
- Don't animate more than one thing per moment; respect reduced motion.
