//! The windows Deskwave shows: per monitor a wallpaper and a widget layer, plus Settings.

use crate::desktop;
use crate::state::{AppState, Monitor};
use deskwave_core::sandbox::BROWSER_ARGS;
use std::sync::atomic::Ordering::SeqCst;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    Window, WindowEvent,
};
use windows::Win32::Foundation::RECT;

pub fn label(kind: &str, generation: u32, index: usize) -> String {
    format!("{kind}-g{generation}-{index}")
}

/// Monitor index of a layer window from the current generation (labels: `widgets-g3-1`).
pub fn monitor_index(state: &AppState, label: &str) -> Option<usize> {
    let mut parts = label.split('-').skip(1);
    let generation: u32 = parts.next()?.strip_prefix('g')?.parse().ok()?;
    let index = parts.next()?.parse().ok()?;
    (generation == state.generation.load(SeqCst)).then_some(index)
}

pub fn read_monitors(app: &AppHandle) -> Vec<Monitor> {
    let primary = app.primary_monitor().ok().flatten().map(|m| *m.position());
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let (pos, size, work) = (m.position(), m.size(), m.work_area());
            Monitor {
                key: m.name().cloned().unwrap_or_else(|| format!("monitor-{i}")),
                primary: primary == Some(*pos),
                rect: (pos.x, pos.y, size.width, size.height),
                work: (work.position.x, work.position.y, work.size.width, work.size.height),
                scale: m.scale_factor(),
            }
        })
        .collect()
}

/// (Re)creates every layer window. Runs at startup and when monitors change or Explorer
/// restarts (which destroys windows parented to or owned by the desktop).
pub fn rebuild(app: &AppHandle) {
    let state = app.state::<AppState>();
    for (label, window) in app.webview_windows() {
        if label.starts_with("wallpaper-") || label.starts_with("widgets-") {
            let _ = window.destroy();
        }
    }
    let generation = state.generation.fetch_add(1, SeqCst) + 1;
    let monitors = read_monitors(app);
    let first_run = state.config.lock().unwrap().wallpapers.is_empty();
    for m in &monitors {
        state.ensure_monitor_defaults(m, first_run);
    }
    *state.monitors.lock().unwrap() = monitors.clone();
    *state.paused.lock().unwrap() = vec![false; monitors.len()];
    state.hit_rects.lock().unwrap().clear();

    // Wallpapers first: attaching splits the desktop, which can change the owner below.
    for (i, m) in monitors.iter().enumerate() {
        if let Err(e) = create_wallpaper(app, &label("wallpaper", generation, i), m) {
            eprintln!("wallpaper window {i} failed: {e}");
        }
    }
    let owner = desktop::desktop_owner();
    *state.owner.lock().unwrap() = owner;
    for (i, m) in monitors.iter().enumerate() {
        if let Err(e) = create_widgets(app, &label("widgets", generation, i), m, owner) {
            eprintln!("widget window {i} failed: {e}");
        }
    }
}

fn layer_builder<'a>(app: &'a AppHandle, label: &str, page: &str) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title("Deskwave")
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .shadow(false)
        .additional_browser_args(BROWSER_ARGS)
}

fn create_wallpaper(app: &AppHandle, label: &str, m: &Monitor) -> tauri::Result<()> {
    let window = layer_builder(app, label, "wallpaper.html").build()?;
    let rect = RECT {
        left: m.rect.0,
        top: m.rect.1,
        right: m.rect.0 + m.rect.2 as i32,
        bottom: m.rect.1 + m.rect.3 as i32,
    };
    // Shown by attach_wallpaper via Win32: tao's show() would restore the popup styles.
    if let Err(e) = desktop::attach_wallpaper(window.hwnd()?.0 as isize, rect) {
        eprintln!("attaching wallpaper to the desktop failed: {e}");
    }
    Ok(())
}

fn create_widgets(app: &AppHandle, label: &str, m: &Monitor, owner: Option<isize>) -> tauri::Result<()> {
    let window = layer_builder(app, label, "widgets.html").transparent(true).build()?;
    window.set_position(PhysicalPosition::new(m.work.0, m.work.1))?;
    window.set_size(PhysicalSize::new(m.work.2, m.work.3))?;
    window.set_ignore_cursor_events(true)?;
    let hwnd = window.hwnd()?.0 as isize;
    if let Some(owner) = owner {
        desktop::set_owner(hwnd, owner);
    }
    window.show()?;
    if let Some(owner) = owner {
        desktop::sink(hwnd, owner);
    }
    Ok(())
}

/// Edit mode lifts the widget layers above all windows; leaving it sinks them back.
pub fn set_editing(app: &AppHandle, on: bool) {
    let state = app.state::<AppState>();
    state.editing.store(on, SeqCst);
    let owner = *state.owner.lock().unwrap();
    let primary = state.monitors.lock().unwrap().iter().position(|m| m.primary).unwrap_or(0);
    let generation = state.generation.load(SeqCst);
    for (label, window) in app.webview_windows() {
        if !label.starts_with("widgets-") {
            continue;
        }
        let _ = window.set_always_on_top(on);
        if on && label == self::label("widgets", generation, primary) {
            let _ = window.set_focus(); // so Esc reaches it
        }
        if let (false, Some(owner), Ok(hwnd)) = (on, owner, window.hwnd()) {
            desktop::sink(hwnd.0 as isize, owner);
        }
    }
    let _ = app.emit("edit-mode", on);
}

pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Deskwave")
        .inner_size(1000.0, 720.0)
        .min_inner_size(760.0, 540.0)
        .center()
        .additional_browser_args(BROWSER_ARGS)
        .build();
    if let Err(e) = built {
        eprintln!("settings window failed: {e}");
    }
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let label = window.label();
    match event {
        // Alt+F4 on a focused widget must not close the whole layer.
        WindowEvent::CloseRequested { api, .. }
            if label.starts_with("widgets-") || label.starts_with("wallpaper-") =>
        {
            api.prevent_close()
        }
        // Clicking a widget activates its layer, which raises it over other apps: undo that.
        WindowEvent::Focused(true) if label.starts_with("widgets-") => {
            let state = window.state::<AppState>();
            let owner = *state.owner.lock().unwrap();
            if let (false, Some(owner), Ok(hwnd)) = (state.editing.load(SeqCst), owner, window.hwnd()) {
                desktop::sink(hwnd.0 as isize, owner);
            }
        }
        _ => {}
    }
}
