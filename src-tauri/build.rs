fn main() {
    // Listing the commands makes each one a permission (`allow-<name>`), so every window only
    // gets the commands its capability file grants instead of all of them.
    let commands = tauri_build::AppManifest::new().commands(&[
        "wallpaper_get",
        "widgets_get",
        "widgets_save",
        "widget_add",
        "widget_remove",
        "set_hit_rects",
        "storage_get",
        "storage_set",
        "battery_get",
        "set_edit_mode",
        "settings_get",
        "set_wallpaper",
        "set_mod_approved",
        "set_pause_options",
        "open_folder",
        "net_fetch",
        "system_get",
        "media_get",
        "media_control",
        "set_internet",
        "set_autostart",
        "set_language",
        "gradient_save",
        "gradient_delete",
        "album_save",
        "album_delete",
        "drop_import",
        "set_keep_awake",
        "open_link",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(commands))
        .expect("tauri-build failed");
}
