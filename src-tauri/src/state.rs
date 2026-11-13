use crate::media::{Action, NowPlaying};
use crate::stats::Stats;
use deskwave_core::config::{Config, Grant, Wallpaper, Widget};
use deskwave_core::manifest::{self, InstalledMod, Kind, Permission};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const NET_LOG_LEN: usize = 50;
const NET_PER_MINUTE: usize = 30;

pub const DEFAULT_WALLPAPER: &str = "deskwave.aurora";

pub struct Paths {
    pub config: PathBuf,
    pub storage: PathBuf,
    pub mods: PathBuf,
    pub wallpapers: PathBuf,
    pub builtin_mods: PathBuf,
    pub sdk: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// Stable-ish id from Windows, e.g. `\\.\DISPLAY1`; the key in config.json.
    pub key: String,
    pub primary: bool,
    /// Physical pixels: (x, y, width, height).
    pub rect: (i32, i32, u32, u32),
    pub work: (i32, i32, u32, u32),
    pub scale: f64,
}

/// One internet request, shown in Settings → Privacy.
#[derive(Debug, Clone, Serialize)]
pub struct NetEvent {
    /// Unix seconds.
    pub time: u64,
    pub name: String,
    pub host: String,
    /// HTTP status, or the reason it failed.
    pub result: String,
}

#[derive(Default)]
pub struct Mods {
    pub list: Vec<InstalledMod>,
    pub errors: Vec<String>,
}

pub struct AppState {
    pub paths: Paths,
    pub config: Mutex<Config>,
    pub mods: Mutex<Mods>,
    pub monitors: Mutex<Vec<Monitor>>,
    /// Bumped on every rebuild; window labels carry it so stale windows are recognisable.
    pub generation: AtomicU32,
    /// Desktop window that owns the widget layers (changes when Explorer restarts).
    pub owner: Mutex<Option<isize>>,
    /// Widget-layer label → widget rectangles in physical screen pixels.
    pub hit_rects: Mutex<HashMap<String, Vec<[f64; 4]>>>,
    /// Per-monitor pause state last sent to the wallpapers.
    pub paused: Mutex<Vec<bool>>,
    pub editing: AtomicBool,
    pub user_paused: AtomicBool,
    pub storage_lock: Mutex<()>,
    pub stats: Mutex<Stats>,
    pub media: Mutex<NowPlaying>,
    pub media_actions: Mutex<Option<Sender<Action>>>,
    pub net_log: Mutex<VecDeque<NetEvent>>,
    /// Windows display languages, read once at startup.
    system_languages: Vec<String>,
    net_recent: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl AppState {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let roaming = app.path().data_dir()?;
        let data = roaming.join("Deskwave");
        // The app was called ColorBy before; keep its layout, notes and mods.
        let legacy = roaming.join("ColorBy");
        if !data.exists() && legacy.is_dir() {
            let _ = std::fs::rename(&legacy, &data);
        }
        let resources = app.path().resource_dir()?;
        let paths = Paths {
            config: data.join("config.json"),
            storage: data.join("storage"),
            mods: data.join("mods"),
            wallpapers: data.join("wallpapers"),
            builtin_mods: resources.join("mods"),
            sdk: resources.join("sdk"),
        };
        for dir in [&paths.mods, &paths.wallpapers, &paths.storage] {
            std::fs::create_dir_all(dir)?;
        }
        let mut loaded = Config::load(&paths.config)?;
        if let Some(config) = loaded.as_mut() {
            if config.rename_mod_prefix("colorby.", "deskwave.") {
                config.save(&paths.config)?;
            }
        }
        let state = AppState {
            config: Mutex::new(loaded.unwrap_or_default()),
            paths,
            mods: Mutex::default(),
            monitors: Mutex::default(),
            generation: AtomicU32::new(0),
            owner: Mutex::default(),
            hit_rects: Mutex::default(),
            paused: Mutex::default(),
            editing: AtomicBool::new(false),
            user_paused: AtomicBool::new(false),
            storage_lock: Mutex::default(),
            stats: Mutex::default(),
            media: Mutex::default(),
            media_actions: Mutex::default(),
            net_log: Mutex::default(),
            system_languages: crate::desktop::ui_languages(),
            net_recent: Mutex::default(),
        };
        state.rescan_mods();
        Ok(state)
    }

    pub fn save(&self) {
        if let Err(e) = self.config.lock().unwrap().save(&self.paths.config) {
            eprintln!("saving config failed: {e}");
        }
    }

    /// Re-reads all mod folders. Built-ins are approved the first time they are seen.
    pub fn rescan_mods(&self) {
        let mut mods = Mods::default();
        manifest::scan(&self.paths.builtin_mods, true, &mut mods.list, &mut mods.errors);
        manifest::scan(&self.paths.mods, false, &mut mods.list, &mut mods.errors);
        let mut config = self.config.lock().unwrap();
        let mut changed = false;
        for m in mods.list.iter().filter(|m| m.builtin) {
            if !config.grants.contains_key(&m.manifest.id) {
                config.grants.insert(m.manifest.id.clone(), Grant::all_requested(&m.manifest));
                changed = true;
            }
        }
        drop(config);
        *self.mods.lock().unwrap() = mods;
        if changed {
            self.save();
        }
    }

    /// The mod, if it exists and the user allows it to run.
    pub fn ready_mod(&self, id: &str) -> Option<InstalledMod> {
        let config = self.config.lock().unwrap();
        let mods = self.mods.lock().unwrap();
        mods.list
            .iter()
            .find(|m| m.manifest.id == id && Grant::allows(config.grants.get(id), &m.manifest))
            .cloned()
    }

    /// New monitor: default wallpaper. First run: clock and notes on the primary monitor.
    pub fn ensure_monitor_defaults(&self, monitor: &Monitor, first_run: bool) {
        let mut config = self.config.lock().unwrap();
        if config.wallpapers.contains_key(&monitor.key) {
            return;
        }
        config
            .wallpapers
            .insert(monitor.key.clone(), Wallpaper::Mod { id: DEFAULT_WALLPAPER.into() });
        if first_run && monitor.primary {
            let right = monitor.work.2 as f64 / monitor.scale - 24.0;
            for (mod_id, y, w, h) in
                [("deskwave.clock", 24.0, 300.0, 150.0), ("deskwave.notes", 190.0, 300.0, 260.0)]
            {
                config.widgets.push(Widget {
                    id: deskwave_core::config::new_widget_id(),
                    mod_id: mod_id.into(),
                    monitor: monitor.key.clone(),
                    x: right - w,
                    y,
                    w,
                    h,
                });
            }
        }
        drop(config);
        self.save();
    }

    /// The language the UI should use right now.
    pub fn lang(&self) -> &'static str {
        let chosen = self.config.lock().unwrap().language.clone();
        deskwave_core::i18n::resolve(chosen.as_deref(), &self.system_languages)
    }

    /// Whether any placed widget is a ready mod with `permission`; idle samplers skip work.
    pub fn placed_with(&self, permission: Permission) -> bool {
        let ids: Vec<String> = self.config.lock().unwrap().widgets.iter().map(|w| w.mod_id.clone()).collect();
        ids.iter().any(|id| self.ready_mod(id).is_some_and(|m| m.manifest.permissions.contains(&permission)))
    }

    /// Rate limit per mod: at most 30 internet requests a minute.
    pub fn admit_request(&self, mod_id: &str) -> Result<(), String> {
        let now = Instant::now();
        let mut recent = self.net_recent.lock().unwrap();
        let times = recent.entry(mod_id.to_owned()).or_default();
        while times.front().is_some_and(|t| now.duration_since(*t).as_secs() >= 60) {
            times.pop_front();
        }
        if times.len() >= NET_PER_MINUTE {
            return Err("rate-limited".into());
        }
        times.push_back(now);
        Ok(())
    }

    pub fn log_request(&self, name: &str, host: &str, result: String) {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let mut log = self.net_log.lock().unwrap();
        if log.len() >= NET_LOG_LEN {
            log.pop_front();
        }
        log.push_back(NetEvent { time, name: name.to_owned(), host: host.to_owned(), result });
    }

    /// The mod if it is ready and of `kind`.
    pub fn ready_kind(&self, id: &str, kind: Kind) -> Option<InstalledMod> {
        self.ready_mod(id).filter(|m| m.manifest.kind == kind)
    }
}
