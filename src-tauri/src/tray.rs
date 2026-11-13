use crate::layers;
use crate::state::AppState;
use deskwave_core::i18n;
use std::sync::atomic::Ordering::SeqCst;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let text = i18n::tray(state.lang());
    let paused = state.user_paused.load(SeqCst);
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", text.open, true, None::<&str>)?,
            &MenuItem::with_id(app, "edit", text.edit, true, None::<&str>)?,
            &CheckMenuItem::with_id(app, "pause", text.pause, true, paused, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", text.quit, true, None::<&str>)?,
        ],
    )
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Deskwave")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "open" => layers::open_settings(app),
        "edit" => layers::set_editing(app, true),
        "pause" => {
            app.state::<AppState>().user_paused.fetch_xor(true, SeqCst);
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
