#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("Deskwave runs on Windows and macOS. The core crate (crates/deskwave-core) builds anywhere.");

mod broker;
mod commands;
mod layers;
mod media;
mod platform;
mod poller;
mod protocol;
mod state;
mod stats;
mod tray;

use tauri::Manager;

fn main() {
    let Some(instance) = platform::single_instance() else {
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
            commands::album_save,
            commands::album_delete,
            commands::drop_import,
            commands::set_keep_awake,
            commands::open_link,
        ])
        .on_window_event(layers::on_window_event)
        .setup(move |app| {
            let state = state::AppState::load(app.handle())?;
            app.asset_protocol_scope().allow_directory(&state.paths.wallpapers, false)?;
            platform::refresh_autostart();
            let keep_awake = state.config.lock().unwrap().keep_awake;
            if let Err(e) = platform::set_keep_awake(keep_awake) {
                eprintln!("keeping awake failed: {e}");
            }
            // macOS: no Dock icon while only the desktop layers are showing.
            #[cfg(target_os = "macos")]
            let _ = app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory);
            *state.media_actions.lock().unwrap() = Some(media::start(app.handle().clone()));
            app.manage(state);
            tray::create(app.handle())?;
            layers::rebuild(app.handle());
            poller::start(app.handle().clone(), instance);
            // Started from a shortcut: show the window. Started at sign-in: stay in the tray.
            if !std::env::args().any(|arg| arg == platform::BACKGROUND_FLAG) {
                layers::open_settings(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Deskwave")
        .run(|_app, event| match event {
            // Explorer restarts destroy our desktop windows; the watchdog recreates them.
            tauri::RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            tauri::RunEvent::Exit => platform::repaint_static_wallpaper(),
            // macOS: opening Deskwave again (Finder, Launchpad, Dock) while it runs.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => layers::open_settings(_app),
            _ => {}
        });
}
