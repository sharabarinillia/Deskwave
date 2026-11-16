//! `config.json`: wallpapers, widget layout, mod approvals and pause settings.

use crate::manifest::{Manifest, Permission};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Master internet switch; off by default. Approved mods also need their own grant.
    pub internet: bool,
    /// Whether the user already answered "start when you sign in?".
    pub startup_asked: bool,
    /// Chosen language code; `None` follows Windows.
    pub language: Option<String>,
    pub pause: PauseOptions,
    /// Monitor key (e.g. `\\.\DISPLAY1`) → wallpaper.
    pub wallpapers: BTreeMap<String, Wallpaper>,
    pub widgets: Vec<Widget>,
    /// Gradient wallpapers the user made.
    pub gradients: Vec<Gradient>,
    /// Albums: videos and photos that take turns as the wallpaper.
    pub albums: Vec<Album>,
    /// macOS: keeps the screen or the Mac awake. Off by default.
    pub keep_awake: KeepAwake,
    /// Mod id → what the user approved.
    pub grants: BTreeMap<String, Grant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct PauseOptions {
    pub fullscreen: bool,
    pub covered: bool,
    pub battery: bool,
    pub locked: bool,
}

impl Default for PauseOptions {
    fn default() -> Self {
        Self { fullscreen: true, covered: true, battery: true, locked: true }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Wallpaper {
    /// A wallpaper-kind mod, e.g. the built-in Aurora preset.
    Mod { id: String },
    /// A file name inside the wallpapers folder.
    File { name: String },
    /// One of `Config::gradients`.
    Gradient { id: String },
    /// One of `Config::albums`.
    Album { id: String },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeepAwake {
    #[default]
    Off,
    /// The screen stays on, so the Mac does too.
    Screen,
    /// The screen may turn off, but the Mac doesn't go to sleep.
    System,
}

pub const MAX_ALBUMS: usize = 50;
pub const MAX_ALBUM_ITEMS: usize = 200;
/// How long a photo stays up, in seconds. Videos play to the end.
pub const ALBUM_INTERVALS: [u32; 8] = [5, 10, 30, 60, 300, 900, 1800, 3600];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumOrder {
    InOrder,
    Shuffle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumLayout {
    /// One video or photo at a time, filling the display.
    Single,
    /// Up to four at once in a grid, each taking its own turns.
    Collage,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    /// File names inside the wallpapers folder, in play order.
    pub items: Vec<String>,
    /// Seconds each photo stays up; one of `ALBUM_INTERVALS`.
    pub interval: u32,
    pub order: AlbumOrder,
    pub layout: AlbumLayout,
}

impl Album {
    pub fn valid(&self) -> bool {
        let name = self.name.trim();
        valid_widget_id(&self.id)
            && (1..=60).contains(&name.chars().count())
            && name.len() == self.name.len()
            && !self.name.chars().any(char::is_control)
            && (1..=MAX_ALBUM_ITEMS).contains(&self.items.len())
            && self.items.iter().all(|n| crate::files::valid_file_name(n) && crate::files::is_media(n))
            && ALBUM_INTERVALS.contains(&self.interval)
    }
}

pub const MAX_GRADIENTS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GradientStyle {
    Linear,
    Radial,
    /// Soft blobs of each color drifting over the first one.
    Flow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    Still,
    Slow,
    Medium,
    Fast,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Gradient {
    pub id: String,
    /// 2-5 colors as `#rrggbb`.
    pub colors: Vec<String>,
    pub style: GradientStyle,
    /// Degrees, 0-359; used by linear gradients.
    pub angle: u16,
    pub motion: Motion,
}

impl Gradient {
    pub fn valid(&self) -> bool {
        let hex = |c: &String| c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit());
        valid_widget_id(&self.id) && (2..=5).contains(&self.colors.len()) && self.colors.iter().all(hex) && self.angle < 360
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    pub mod_id: String,
    pub monitor: String,
    /// CSS px relative to the monitor's work area.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Widget {
    pub fn rect_ok(&self) -> bool {
        [self.x, self.y, self.w, self.h].iter().all(|v| v.is_finite())
            && (40.0..=8000.0).contains(&self.w)
            && (40.0..=8000.0).contains(&self.h)
            && self.x.abs() <= 20000.0
            && self.y.abs() <= 20000.0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Grant {
    pub permissions: Vec<Permission>,
    /// Internet hosts the user approved for this mod.
    #[serde(default)]
    pub network: Vec<String>,
    pub enabled: bool,
}

impl Grant {
    pub fn all_requested(m: &Manifest) -> Self {
        Grant { permissions: m.permissions.clone(), network: m.network.clone(), enabled: true }
    }

    /// A mod may run only if enabled and every permission and host it asks for was approved,
    /// so an update that adds one needs approval again.
    pub fn allows(grant: Option<&Grant>, m: &Manifest) -> bool {
        grant.is_some_and(|g| {
            g.enabled
                && m.permissions.iter().all(|p| g.permissions.contains(p))
                && m.network.iter().all(|h| g.network.contains(h))
        })
    }
}

impl Config {
    /// `None` on first run. A corrupt file is moved to `config.json.bad` instead of being
    /// overwritten, so the user can recover their layout.
    pub fn load(path: &Path) -> io::Result<Option<Config>> {
        match std::fs::read(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(config) => Ok(Some(config)),
                Err(_) => {
                    std::fs::rename(path, path.with_extension("json.bad"))?;
                    Ok(None)
                }
            },
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        write_atomic(path, &serde_json::to_vec_pretty(self)?)
    }

    /// Renames mod ids starting with `from` (e.g. the built-ins from before the rename to
    /// Deskwave). Returns whether anything changed.
    pub fn rename_mod_prefix(&mut self, from: &str, to: &str) -> bool {
        let renamed = |id: &str| id.strip_prefix(from).map(|rest| format!("{to}{rest}"));
        let mut changed = false;
        for widget in &mut self.widgets {
            if let Some(id) = renamed(&widget.mod_id) {
                widget.mod_id = id;
                changed = true;
            }
        }
        for wallpaper in self.wallpapers.values_mut() {
            if let Wallpaper::Mod { id } = wallpaper {
                if let Some(new) = renamed(id) {
                    *id = new;
                    changed = true;
                }
            }
        }
        let old: Vec<String> = self.grants.keys().filter(|k| k.starts_with(from)).cloned().collect();
        for key in old {
            let grant = self.grants.remove(&key).expect("key from this map");
            self.grants.insert(renamed(&key).expect("filtered by prefix"), grant);
            changed = true;
        }
        changed
    }
}

/// Write to a temp file, flush to disk, then rename over the target, so a crash mid-write
/// never leaves a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

/// Unique, filename-safe widget id.
pub fn new_widget_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("w{nanos:x}{:x}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn valid_widget_id(id: &str) -> bool {
    (1..=40).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let dir = crate::test_dir("config");
        let path = dir.join("config.json");
        assert_eq!(Config::load(&path).unwrap(), None);

        let mut config = Config::default();
        assert!(config.pause.fullscreen && config.pause.battery);
        config.wallpapers.insert("m1".into(), Wallpaper::File { name: "rain.mp4".into() });
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), Some(config));

        // Older/newer files with missing or extra fields still load.
        std::fs::write(&path, r#"{"pause":{"battery":false},"future":1}"#).unwrap();
        let loaded = Config::load(&path).unwrap().unwrap();
        assert!(!loaded.pause.battery && loaded.pause.fullscreen);
    }

    #[test]
    fn gradients_are_validated() {
        let g = Gradient {
            id: "g1".into(),
            colors: vec!["#1d3557".into(), "#e63946".into()],
            style: GradientStyle::Flow,
            angle: 135,
            motion: Motion::Slow,
        };
        assert!(g.valid());
        assert!(!Gradient { colors: vec!["#1d3557".into()], ..g.clone() }.valid(), "one color");
        assert!(!Gradient { colors: vec!["#1d3557".into(), "red".into()], ..g.clone() }.valid(), "named color");
        assert!(!Gradient { colors: vec!["#1d3557".into(); 6], ..g.clone() }.valid(), "six colors");
        assert!(!Gradient { angle: 360, ..g.clone() }.valid());
        assert!(!Gradient { id: "../x".into(), ..g.clone() }.valid());
        let json = serde_json::to_string(&Wallpaper::Gradient { id: "g1".into() }).unwrap();
        assert_eq!(json, r#"{"kind":"gradient","id":"g1"}"#);
    }

    #[test]
    fn albums_are_validated() {
        let a = Album {
            id: "a1".into(),
            name: "Holiday".into(),
            items: vec!["beach.mp4".into(), "sunset.jpg".into()],
            interval: 30,
            order: AlbumOrder::Shuffle,
            layout: AlbumLayout::Collage,
        };
        assert!(a.valid());
        assert!(!Album { items: vec![], ..a.clone() }.valid(), "empty");
        assert!(!Album { items: vec!["../secret.jpg".into()], ..a.clone() }.valid(), "path");
        assert!(!Album { items: vec!["notes.txt".into()], ..a.clone() }.valid(), "not media");
        assert!(!Album { items: vec!["a.jpg".into(); MAX_ALBUM_ITEMS + 1], ..a.clone() }.valid(), "too many");
        assert!(!Album { interval: 7, ..a.clone() }.valid(), "interval");
        assert!(!Album { name: " ".into(), ..a.clone() }.valid(), "blank name");
        assert!(!Album { name: " padded ".into(), ..a.clone() }.valid(), "untrimmed");
        assert!(!Album { name: "x".repeat(61), ..a.clone() }.valid(), "long name");
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains(r#""order":"shuffle""#) && json.contains(r#""layout":"collage""#));
        let json = serde_json::to_string(&Wallpaper::Album { id: "a1".into() }).unwrap();
        assert_eq!(json, r#"{"kind":"album","id":"a1"}"#);
    }

    #[test]
    fn keep_awake_is_off_by_default() {
        assert_eq!(Config::default().keep_awake, KeepAwake::Off);
        let loaded: Config = serde_json::from_str(r#"{"keep_awake":"system"}"#).unwrap();
        assert_eq!(loaded.keep_awake, KeepAwake::System);
        assert_eq!(serde_json::to_string(&KeepAwake::Screen).unwrap(), r#""screen""#);
    }

    #[test]
    fn renames_old_builtin_ids() {
        let mut config = Config::default();
        config.wallpapers.insert("m".into(), Wallpaper::Mod { id: "colorby.aurora".into() });
        config.widgets.push(Widget { id: "w1".into(), mod_id: "colorby.clock".into(), monitor: "m".into(), x: 0.0, y: 0.0, w: 99.0, h: 99.0 });
        config.grants.insert("colorby.clock".into(), Grant::default());
        config.grants.insert("acme.thing".into(), Grant::default());
        assert!(config.rename_mod_prefix("colorby.", "deskwave."));
        assert_eq!(config.widgets[0].mod_id, "deskwave.clock");
        assert_eq!(config.wallpapers["m"], Wallpaper::Mod { id: "deskwave.aurora".into() });
        assert!(config.grants.contains_key("deskwave.clock") && config.grants.contains_key("acme.thing"));
        assert!(!config.rename_mod_prefix("colorby.", "deskwave."), "second run changes nothing");
    }

    #[test]
    fn corrupt_config_is_kept_aside() {
        let dir = crate::test_dir("corrupt");
        let path = dir.join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(Config::load(&path).unwrap(), None);
        assert_eq!(std::fs::read_to_string(dir.join("config.json.bad")).unwrap(), "{not json");
    }

    #[test]
    fn grants_cover_every_requested_permission() {
        let mut m = Manifest::parse(
            r#"{"id":"a.b","name":"B","version":"1","kind":"widget","entry":"i.html"}"#,
        )
        .unwrap();
        let old = Grant::all_requested(&m);
        assert!(Grant::allows(Some(&old), &m));
        assert!(!Grant::allows(None, &m));
        assert!(!Grant::allows(Some(&Grant { enabled: false, ..old.clone() }), &m));
        m.permissions.push(Permission::Battery);
        assert!(!Grant::allows(Some(&old), &m), "update added a permission");
        let with_battery = Grant::all_requested(&m);
        m.network.push("api.example.com".into());
        assert!(!Grant::allows(Some(&with_battery), &m), "update added a host");
        assert!(Grant::allows(Some(&Grant::all_requested(&m)), &m));
    }

    #[test]
    fn widget_ids_and_rects() {
        let (a, b) = (new_widget_id(), new_widget_id());
        assert!(a != b && valid_widget_id(&a));
        assert!(!valid_widget_id("../x") && !valid_widget_id(""));
        let w = Widget { id: a, mod_id: "a.b".into(), monitor: "m".into(), x: 0.0, y: 0.0, w: 200.0, h: 100.0 };
        assert!(w.rect_ok());
        assert!(!Widget { w: f64::NAN, ..w.clone() }.rect_ok());
        assert!(!Widget { h: 10.0, ..w }.rect_ok());
    }
}
