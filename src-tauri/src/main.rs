#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("Deskwave is a Windows app. The core crate (crates/deskwave-core) builds anywhere.");

mod broker;
mod commands;
mod desktop;
mod layers;
mod media;
mod poller;
mod protocol;
mod state;
mod stats;
mod tray;

use tauri::Manager;

fn main() {
    let Some(show_settings) = desktop::single_instance() else {
        return; // already running; that copy opens Settings
    };
    tauri::Builder::default()
        .register_uri_scheme_protocol("dwmod", protocol::handle)
        .invoke_handler(tauri::generate_handler![
            commands::wallpaper_get,
            commands::widgets_get,
            commands::widgets_save,
            commands::widget_add,
            commands::widget_remove,
            commands::set_hit_rects,
            commands::storage_get,
            commands::storage_set,
            commands::battery_get,
            commands::set_edit_mode,
            commands::settings_get,
            commands::set_wallpaper,
            commands::set_mod_approved,
            commands::set_pause_options,
            commands::open_folder,
            commands::net_fetch,
            commands::system_get,
            commands::media_get,
            commands::media_control,
            commands::set_internet,
            commands::set_autostart,
            commands::set_language,
            commands::gradient_save,
            commands::gradient_delete,
            commands::open_link,
        ])
        .on_window_event(layers::on_window_event)
        .setup(move |app| {
            let state = state::AppState::load(app.handle())?;
            app.asset_protocol_scope().allow_directory(&state.paths.wallpapers, false)?;
            desktop::refresh_autostart();
            *state.media_actions.lock().unwrap() = Some(media::start(app.handle().clone()));
            app.manage(state);
            tray::create(app.handle())?;
            layers::rebuild(app.handle());
            poller::start(app.handle().clone(), show_settings);
            // Started from a shortcut: show the window. Started at sign-in: stay in the tray.
            if !std::env::args().any(|arg| arg == desktop::BACKGROUND_FLAG) {
                layers::open_settings(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Deskwave")
        .run(|_, event| match event {
            // Explorer restarts destroy our desktop windows; the watchdog recreates them.
            tauri::RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            tauri::RunEvent::Exit => desktop::repaint_static_wallpaper(),
            _ => {}
        });
}
