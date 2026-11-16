//! The windows Deskwave shows: per monitor a wallpaper and a widget layer, plus Settings.

use crate::platform;
use crate::state::{AppState, Monitor};
use deskwave_core::sandbox::BROWSER_ARGS;
use std::sync::atomic::Ordering::SeqCst;
use tauri::{
    AppHandle, DragDropEvent, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder, Window, WindowEvent,
};

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
    let owner = platform::desktop_owner();
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
    platform::harden_webview(&window);
    if let Err(e) = platform::place_wallpaper(&window, m) {
        eprintln!("attaching wallpaper to the desktop failed: {e}");
    }
    Ok(())
}

fn create_widgets(app: &AppHandle, label: &str, m: &Monitor, owner: Option<isize>) -> tauri::Result<()> {
    let window = layer_builder(app, label, "widgets.html").transparent(true).build()?;
    platform::harden_webview(&window);
    window.set_position(PhysicalPosition::new(m.work.0, m.work.1))?;
    window.set_size(PhysicalSize::new(m.work.2, m.work.3))?;
    window.set_ignore_cursor_events(true)?;
    platform::place_widgets(&window, owner)
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
        platform::set_editing(&window, on, owner);
        if on && label == self::label("widgets", generation, primary) {
            let _ = window.set_focus(); // so Esc reaches it
        }
    }
    let _ = app.emit("edit-mode", on);
}

pub fn open_settings(app: &AppHandle) {
    // A Dock icon (and Cmd-Tab) only while Settings is open.
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
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
    match built {
        Ok(window) => platform::harden_webview(&window),
        Err(e) => eprintln!("settings window failed: {e}"),
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
            if let (false, Some(webview)) = (state.editing.load(SeqCst), window.get_webview_window(label)) {
                platform::sink(&webview, owner);
            }
        }
        // Remember what the user dropped onto Settings. The page then asks to import it, but
        // never names paths itself, so it cannot make Deskwave copy files of its choosing.
        // This handler runs before the page hears about the drop.
        WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) if label == "settings" => {
            *window.state::<AppState>().dropped.lock().unwrap() = Some((paths.clone(), std::time::Instant::now()));
        }
        #[cfg(target_os = "macos")]
        WindowEvent::Destroyed if label == "settings" => {
            let _ = window.app_handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
        _ => {}
    }
}
