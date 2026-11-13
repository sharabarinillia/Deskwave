//! IPC commands. Which page may call which command is set in `capabilities/*.json`.
//! Every argument comes from a page that hosts untrusted mods, so it is validated here.

use crate::broker;
use crate::desktop::{self, Power};
use crate::layers::{self, label};
use crate::media::{Action, NowPlaying};
use crate::state::{AppState, Monitor, NetEvent, DEFAULT_WALLPAPER};
use crate::stats::Stats;
use deskwave_core::config::{new_widget_id, Gradient, Grant, PauseOptions, Wallpaper, Widget, MAX_GRADIENTS};
use deskwave_core::manifest::{InstalledMod, Kind, Permission};
use deskwave_core::sandbox::MOD_ORIGIN;
use deskwave_core::storage;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::atomic::Ordering::SeqCst;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

type Res<T> = Result<T, String>;

const VIDEO: &[&str] = &["mp4", "webm"];
const IMAGE: &[&str] = &["gif", "png", "apng", "jpg", "jpeg", "webp", "avif"];

fn ext(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

fn monitor_of(state: &AppState, window: &WebviewWindow) -> Res<(usize, Monitor)> {
    let i = layers::monitor_index(state, window.label()).ok_or("stale window")?;
    let monitor = state.monitors.lock().unwrap().get(i).cloned().ok_or("unknown monitor")?;
    Ok((i, monitor))
}

fn mod_url(m: &InstalledMod) -> String {
    format!("{MOD_ORIGIN}/{}/{}", m.manifest.id, m.manifest.entry)
}

/// Video and image files directly inside the wallpapers folder.
fn wallpaper_files(state: &AppState) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(&state.paths.wallpapers)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| {
            let e = ext(n);
            VIDEO.contains(&e.as_str()) || IMAGE.contains(&e.as_str())
        })
        .collect();
    names.sort();
    names
}

// ---- wallpaper window ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperInfo {
    mod_url: Option<String>,
    /// Absolute path inside the wallpapers folder; the page loads it via the asset protocol.
    file: Option<String>,
    video: bool,
    gradient: Option<Gradient>,
    paused: bool,
    accent: String,
    lang: &'static str,
}

#[tauri::command]
pub fn wallpaper_get(window: WebviewWindow, state: State<AppState>) -> Res<WallpaperInfo> {
    let (i, monitor) = monitor_of(&state, &window)?;
    let chosen = state.config.lock().unwrap().wallpapers.get(&monitor.key).cloned();
    let mut info = WallpaperInfo {
        mod_url: None,
        file: None,
        video: false,
        gradient: None,
        paused: state.paused.lock().unwrap().get(i).copied().unwrap_or(false),
        accent: desktop::accent_color(),
        lang: state.lang(),
    };
    let gradient = match &chosen {
        Some(Wallpaper::Gradient { id }) => state.config.lock().unwrap().gradients.iter().find(|g| &g.id == id).cloned(),
        _ => None,
    };
    if gradient.is_some() {
        info.gradient = gradient;
        return Ok(info);
    }
    match chosen {
        Some(Wallpaper::File { name }) if wallpaper_files(&state).contains(&name) => {
            info.video = VIDEO.contains(&ext(&name).as_str());
            info.file = Some(state.paths.wallpapers.join(name).to_string_lossy().into_owned());
        }
        chosen => {
            let id = match &chosen {
                Some(Wallpaper::Mod { id }) => id.as_str(),
                _ => DEFAULT_WALLPAPER,
            };
            info.mod_url = state
                .ready_kind(id, Kind::Wallpaper)
                .or_else(|| state.ready_kind(DEFAULT_WALLPAPER, Kind::Wallpaper))
                .map(|m| mod_url(&m));
        }
    }
    Ok(info)
}

// ---- widget layer ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetView {
    id: String,
    name: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// None when the mod is missing or not approved; the page shows a placeholder.
    url: Option<String>,
    permissions: Vec<Permission>,
}

#[derive(Serialize)]
pub struct ModChoice {
    id: String,
    name: String,
    description: Option<String>,
}

#[derive(Serialize)]
pub struct WidgetsInfo {
    widgets: Vec<WidgetView>,
    available: Vec<ModChoice>,
    editing: bool,
    accent: String,
    lang: &'static str,
}

fn widgets_info(state: &AppState, monitor: &str) -> WidgetsInfo {
    let placed: Vec<Widget> = {
        let config = state.config.lock().unwrap();
        config.widgets.iter().filter(|w| w.monitor == monitor).cloned().collect()
    };
    let installed = state.mods.lock().unwrap().list.clone();
    let lang = state.lang();
    let widgets = placed
        .into_iter()
        .map(|w| {
            let ready = state.ready_kind(&w.mod_id, Kind::Widget);
            let name = installed
                .iter()
                .find(|m| m.manifest.id == w.mod_id)
                .map_or(w.mod_id.clone(), |m| m.manifest.name_in(lang).to_owned());
            WidgetView {
                id: w.id,
                name,
                x: w.x,
                y: w.y,
                w: w.w,
                h: w.h,
                url: ready.as_ref().map(mod_url),
                permissions: ready.map(|m| m.manifest.permissions).unwrap_or_default(),
            }
        })
        .collect();
    let available = installed
        .iter()
        .filter(|m| state.ready_kind(&m.manifest.id, Kind::Widget).is_some())
        .map(|m| ModChoice {
            id: m.manifest.id.clone(),
            name: m.manifest.name_in(lang).to_owned(),
            description: m.manifest.description_in(lang).map(str::to_owned),
        })
        .collect();
    WidgetsInfo {
        widgets,
        available,
        editing: state.editing.load(SeqCst),
        accent: desktop::accent_color(),
        lang,
    }
}

#[tauri::command]
pub fn widgets_get(window: WebviewWindow, state: State<AppState>) -> Res<WidgetsInfo> {
    let (_, monitor) = monitor_of(&state, &window)?;
    Ok(widgets_info(&state, &monitor.key))
}

#[derive(Deserialize)]
pub struct Placement {
    id: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[tauri::command]
pub fn widgets_save(window: WebviewWindow, state: State<AppState>, layout: Vec<Placement>) -> Res<()> {
    let (_, monitor) = monitor_of(&state, &window)?;
    {
        let mut config = state.config.lock().unwrap();
        for p in layout {
            let found = config.widgets.iter_mut().find(|w| w.id == p.id && w.monitor == monitor.key);
            if let Some(widget) = found {
                let moved = Widget { x: p.x, y: p.y, w: p.w, h: p.h, ..widget.clone() };
                if moved.rect_ok() {
                    *widget = moved;
                }
            }
        }
    }
    state.save();
    Ok(())
}

#[tauri::command]
pub fn widget_add(window: WebviewWindow, state: State<AppState>, mod_id: String) -> Res<WidgetsInfo> {
    let (_, monitor) = monitor_of(&state, &window)?;
    let size = state.ready_kind(&mod_id, Kind::Widget).ok_or("widget not available")?.manifest.default_size();
    {
        let mut config = state.config.lock().unwrap();
        let n = config.widgets.iter().filter(|w| w.monitor == monitor.key).count() % 8;
        let offset = 48.0 + 32.0 * n as f64; // cascade so new widgets don't stack exactly
        config.widgets.push(Widget {
            id: new_widget_id(),
            mod_id,
            monitor: monitor.key.clone(),
            x: offset,
            y: offset,
            w: size.w,
            h: size.h,
        });
    }
    state.save();
    Ok(widgets_info(&state, &monitor.key))
}

#[tauri::command]
pub fn widget_remove(window: WebviewWindow, state: State<AppState>, id: String) -> Res<WidgetsInfo> {
    let (_, monitor) = monitor_of(&state, &window)?;
    let removed = {
        let mut config = state.config.lock().unwrap();
        let before = config.widgets.len();
        config.widgets.retain(|w| !(w.id == id && w.monitor == monitor.key));
        before != config.widgets.len()
    };
    if removed {
        let _guard = state.storage_lock.lock().unwrap();
        storage::remove(&state.paths.storage, &id);
        drop(_guard);
        state.save();
    }
    Ok(widgets_info(&state, &monitor.key))
}

/// Widget rectangles in CSS px of the calling layer; stored in screen pixels for hit-testing.
#[tauri::command]
pub fn set_hit_rects(window: WebviewWindow, state: State<AppState>, rects: Vec<[f64; 4]>) -> Res<()> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let origin = window.inner_position().map_err(|e| e.to_string())?;
    let rects = rects
        .into_iter()
        .take(256)
        .filter(|r| r.iter().all(|v| v.is_finite()))
        .map(|[x, y, w, h]| [origin.x as f64 + x * scale, origin.y as f64 + y * scale, w * scale, h * scale])
        .collect();
    state.hit_rects.lock().unwrap().insert(window.label().to_owned(), rects);
    Ok(())
}

fn check_widget(state: &AppState, id: &str) -> Res<()> {
    let exists = state.config.lock().unwrap().widgets.iter().any(|w| w.id == id);
    exists.then_some(()).ok_or_else(|| "unknown widget".into())
}

#[tauri::command]
pub fn storage_get(state: State<AppState>, widget: String) -> Res<Map<String, Value>> {
    check_widget(&state, &widget)?;
    let _guard = state.storage_lock.lock().unwrap();
    storage::read(&state.paths.storage, &widget)
}

#[tauri::command]
pub fn storage_set(state: State<AppState>, widget: String, key: String, value: Value) -> Res<()> {
    check_widget(&state, &widget)?;
    let _guard = state.storage_lock.lock().unwrap();
    storage::set(&state.paths.storage, &widget, &key, value)
}

#[tauri::command]
pub fn battery_get() -> Power {
    desktop::power()
}

/// The mod behind a request: a widget on the calling layer, or the calling wallpaper's mod.
/// Resolved here rather than trusted from the page.
fn calling_mod(state: &AppState, window: &WebviewWindow, widget: Option<&str>) -> Res<String> {
    let (_, monitor) = monitor_of(state, window)?;
    let config = state.config.lock().unwrap();
    if window.label().starts_with("widgets-") {
        let id = widget.ok_or("missing widget")?;
        config
            .widgets
            .iter()
            .find(|w| w.id == id && w.monitor == monitor.key)
            .map(|w| w.mod_id.clone())
            .ok_or_else(|| "unknown widget".into())
    } else {
        match config.wallpapers.get(&monitor.key) {
            Some(Wallpaper::Mod { id }) => Ok(id.clone()),
            _ => Ok(DEFAULT_WALLPAPER.into()),
        }
    }
}

#[tauri::command]
pub async fn net_fetch(
    window: WebviewWindow,
    state: State<'_, AppState>,
    widget: Option<String>,
    url: String,
) -> Res<broker::Reply> {
    let mod_id = calling_mod(&state, &window, widget.as_deref())?;
    let installed = state.ready_mod(&mod_id).ok_or("mod-off")?;
    let (internet, approved) = {
        let config = state.config.lock().unwrap();
        (config.internet, config.grants.get(&mod_id).map(|g| g.network.clone()).unwrap_or_default())
    };
    if !internet {
        return Err("internet-off".into());
    }
    let allowed: Vec<String> = installed.manifest.network.iter().filter(|h| approved.contains(h)).cloned().collect();
    let host = deskwave_core::net::check_url(&url, &allowed)?;
    state.admit_request(&mod_id)?;
    let reply = tauri::async_runtime::spawn_blocking(move || broker::fetch(&url))
        .await
        .map_err(|e| e.to_string())?;
    let result = match &reply {
        Ok(r) => r.status().to_string(),
        Err(e) => e.clone(),
    };
    state.log_request(&installed.manifest.name, &host, result);
    reply
}

#[tauri::command]
pub fn system_get(state: State<AppState>) -> Stats {
    *state.stats.lock().unwrap()
}

#[tauri::command]
pub fn media_get(state: State<AppState>) -> NowPlaying {
    state.media.lock().unwrap().clone()
}

#[tauri::command]
pub fn media_control(window: WebviewWindow, state: State<AppState>, widget: String, action: String) -> Res<()> {
    let mod_id = calling_mod(&state, &window, Some(&widget))?;
    let allowed = state.ready_mod(&mod_id).is_some_and(|m| m.manifest.permissions.contains(&Permission::Media));
    if !allowed {
        return Err("not-allowed".into());
    }
    let action = match action.as_str() {
        "toggle" => Action::Toggle,
        "next" => Action::Next,
        "previous" => Action::Previous,
        _ => return Err("unknown media action".into()),
    };
    let sender = state.media_actions.lock().unwrap().clone().ok_or("media is unavailable")?;
    sender.send(action).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_edit_mode(app: AppHandle, on: bool) {
    layers::set_editing(&app, on);
}

// ---- settings window ----

#[derive(Serialize)]
pub struct ModInfo {
    id: String,
    name: String,
    version: String,
    author: Option<String>,
    description: Option<String>,
    kind: Kind,
    permissions: Vec<Permission>,
    network: Vec<String>,
    builtin: bool,
    approved: bool,
    /// False until the user has made a decision about this mod.
    reviewed: bool,
    /// Set only when approved; used for live previews.
    url: Option<String>,
}

#[derive(Serialize)]
pub struct MonitorInfo {
    key: String,
    primary: bool,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    wallpaper: Option<Wallpaper>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInfo {
    pause: PauseOptions,
    monitors: Vec<MonitorInfo>,
    mods: Vec<ModInfo>,
    errors: Vec<String>,
    files: Vec<String>,
    wallpapers_dir: String,
    widget_count: usize,
    accent: String,
    internet: bool,
    autostart: bool,
    startup_asked: bool,
    /// The language in use, and the user's choice (`None` = follow Windows).
    lang: &'static str,
    language: Option<String>,
    gradients: Vec<Gradient>,
    /// Newest first.
    net_log: Vec<NetEvent>,
    version: &'static str,
}

#[tauri::command]
pub fn settings_get(state: State<AppState>) -> SettingsInfo {
    state.rescan_mods();
    let config = state.config.lock().unwrap().clone();
    let mods = state.mods.lock().unwrap();
    SettingsInfo {
        pause: config.pause,
        monitors: state
            .monitors
            .lock()
            .unwrap()
            .iter()
            .map(|m| MonitorInfo {
                key: m.key.clone(),
                primary: m.primary,
                x: m.rect.0,
                y: m.rect.1,
                width: m.rect.2,
                height: m.rect.3,
                wallpaper: config.wallpapers.get(&m.key).cloned(),
            })
            .collect(),
        mods: mods
            .list
            .iter()
            .map(|m| {
                let approved = Grant::allows(config.grants.get(&m.manifest.id), &m.manifest);
                let lang = state.lang();
                ModInfo {
                    id: m.manifest.id.clone(),
                    name: m.manifest.name_in(lang).to_owned(),
                    version: m.manifest.version.clone(),
                    author: m.manifest.author.clone(),
                    description: m.manifest.description_in(lang).map(str::to_owned),
                    kind: m.manifest.kind,
                    permissions: m.manifest.permissions.clone(),
                    network: m.manifest.network.clone(),
                    builtin: m.builtin,
                    approved,
                    reviewed: config.grants.contains_key(&m.manifest.id),
                    url: approved.then(|| mod_url(m)),
                }
            })
            .collect(),
        errors: mods.errors.clone(),
        files: wallpaper_files(&state),
        wallpapers_dir: state.paths.wallpapers.to_string_lossy().into_owned(),
        widget_count: config.widgets.len(),
        accent: desktop::accent_color(),
        internet: config.internet,
        autostart: desktop::autostart_enabled(),
        startup_asked: config.startup_asked,
        lang: state.lang(),
        language: config.language.clone(),
        gradients: config.gradients.clone(),
        net_log: state.net_log.lock().unwrap().iter().rev().cloned().collect(),
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
pub fn set_wallpaper(app: AppHandle, state: State<AppState>, monitor: String, wallpaper: Wallpaper) -> Res<()> {
    let valid = match &wallpaper {
        Wallpaper::Mod { id } => state.ready_kind(id, Kind::Wallpaper).is_some(),
        Wallpaper::File { name } => wallpaper_files(&state).contains(name),
        Wallpaper::Gradient { id } => state.config.lock().unwrap().gradients.iter().any(|g| &g.id == id),
    };
    let index = state.monitors.lock().unwrap().iter().position(|m| m.key == monitor);
    let (true, Some(index)) = (valid, index) else {
        return Err("unknown wallpaper or monitor".into());
    };
    state.config.lock().unwrap().wallpapers.insert(monitor, wallpaper);
    state.save();
    let generation = state.generation.load(SeqCst);
    let _ = app.emit_to(label("wallpaper", generation, index), "wallpaper-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_mod_approved(app: AppHandle, state: State<AppState>, id: String, approved: bool) -> Res<()> {
    let manifest = state
        .mods
        .lock()
        .unwrap()
        .list
        .iter()
        .find(|m| m.manifest.id == id)
        .map(|m| m.manifest.clone())
        .ok_or("unknown mod")?;
    let grant = if approved { Grant::all_requested(&manifest) } else { Grant::default() };
    state.config.lock().unwrap().grants.insert(id, grant);
    state.save();
    let _ = app.emit("mods-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_language(app: AppHandle, state: State<AppState>, language: Option<String>) -> Res<()> {
    if language.as_deref().is_some_and(|l| !deskwave_core::i18n::LANGUAGES.contains(&l)) {
        return Err("unknown language".into());
    }
    state.config.lock().unwrap().language = language;
    state.save();
    crate::tray::refresh(&app);
    let _ = app.emit("language", state.lang());
    Ok(())
}

/// Creates or updates a gradient; returns its id.
#[tauri::command]
pub fn gradient_save(state: State<AppState>, mut gradient: Gradient) -> Res<String> {
    if gradient.id.is_empty() {
        gradient.id = new_widget_id();
    }
    if !gradient.valid() {
        return Err("invalid gradient".into());
    }
    let mut config = state.config.lock().unwrap();
    match config.gradients.iter().position(|g| g.id == gradient.id) {
        Some(i) => config.gradients[i] = gradient.clone(),
        None if config.gradients.len() >= MAX_GRADIENTS => return Err("too many gradients".into()),
        None => config.gradients.push(gradient.clone()),
    }
    drop(config);
    state.save();
    Ok(gradient.id)
}

/// Deletes a gradient; displays that used it go back to the default wallpaper.
#[tauri::command]
pub fn gradient_delete(app: AppHandle, state: State<AppState>, id: String) {
    {
        let mut config = state.config.lock().unwrap();
        config.gradients.retain(|g| g.id != id);
        for wallpaper in config.wallpapers.values_mut() {
            if matches!(wallpaper, Wallpaper::Gradient { id: used } if *used == id) {
                *wallpaper = Wallpaper::Mod { id: DEFAULT_WALLPAPER.into() };
            }
        }
    }
    state.save();
    let _ = app.emit("wallpaper-changed", ());
}

/// Only these addresses can be opened, so the page cannot launch arbitrary URLs.
#[tauri::command]
pub fn open_link(which: String) -> Res<()> {
    let url = match which.as_str() {
        "author" => "https://github.com/sharabarinillia",
        "source" => "https://github.com/sharabarinillia/Deskwave",
        "license" => "https://polyformproject.org/licenses/noncommercial/1.0.0",
        _ => return Err("unknown link".into()),
    };
    desktop::open_url(url);
    Ok(())
}

#[tauri::command]
pub fn set_internet(app: AppHandle, state: State<AppState>, on: bool) {
    state.config.lock().unwrap().internet = on;
    state.save();
    let _ = app.emit("internet", on);
}

#[tauri::command]
pub fn set_autostart(state: State<AppState>, on: bool) -> Res<()> {
    desktop::set_autostart(on)?;
    state.config.lock().unwrap().startup_asked = true;
    state.save();
    Ok(())
}

#[tauri::command]
pub fn set_pause_options(state: State<AppState>, options: PauseOptions) {
    state.config.lock().unwrap().pause = options;
    state.save();
}

#[tauri::command]
pub fn open_folder(state: State<AppState>, which: String) -> Res<()> {
    let dir = match which.as_str() {
        "mods" => &state.paths.mods,
        "wallpapers" => &state.paths.wallpapers,
        _ => return Err("unknown folder".into()),
    };
    std::process::Command::new("explorer.exe").arg(dir).spawn().map(drop).map_err(|e| e.to_string())
}
