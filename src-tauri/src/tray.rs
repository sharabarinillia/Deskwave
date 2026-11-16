use crate::layers;
use crate::state::AppState;
use deskwave_core::config::KeepAwake;
use deskwave_core::i18n;
use std::sync::atomic::Ordering::SeqCst;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let text = i18n::tray(state.lang());
    let paused = state.user_paused.load(SeqCst);
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", text.open, true, None::<&str>)?,
            &MenuItem::with_id(app, "edit", text.edit, true, None::<&str>)?,
            &CheckMenuItem::with_id(app, "pause", text.pause, true, paused, None::<&str>)?,
        ],
    )?;
    if crate::platform::PLATFORM == "macos" {
        let awake = state.config.lock().unwrap().keep_awake != KeepAwake::Off;
        menu.append(&CheckMenuItem::with_id(app, "awake", text.awake, true, awake, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", text.quit, true, None::<&str>)?)?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Deskwave")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false);
    // macOS: a one-color template image, so the menu bar can tint it for light and dark mode.
    #[cfg(target_os = "macos")]
    {
        // 44×44 RGBA, rendered from icons/tray-template.svg.
        let icon = tauri::image::Image::new(include_bytes!("../icons/tray-template.rgba"), 44, 44);
        tray = tray.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "open" => layers::open_settings(app),
        "edit" => layers::set_editing(app, true),
        "pause" => {
            app.state::<AppState>().user_paused.fetch_xor(true, SeqCst);
        }
        "awake" => {
            let on = app.state::<AppState>().config.lock().unwrap().keep_awake != KeepAwake::Off;
            let mode = if on { KeepAwake::Off } else { KeepAwake::Screen };
            let _ = crate::commands::apply_keep_awake(app, mode);
        }
        "quit" => app.exit(0),
        _ => {}
    })
    // Left click opens the window; right click shows the menu.
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            layers::open_settings(tray.app_handle());
        }
    })
    .build(app)?;
    Ok(())
}

/// Rebuilds the menu in the current language.
pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}
