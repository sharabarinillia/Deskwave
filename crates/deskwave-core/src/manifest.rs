//! Mod manifests: parsing, validation, discovery and safe file resolution.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Widget,
    Wallpaper,
}

/// Capabilities a mod can ask for beyond drawing and its own storage.
/// Unknown names fail to parse, so a mod can never request something the host would ignore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Battery,
    /// CPU, memory, disk and network throughput.
    System,
    /// Now-playing info and play/pause/next/previous.
    Media,
}

pub const MAX_HOSTS: usize = 8;

/// A mod's name and description in another language.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalText {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: Kind,
    pub entry: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Internet hosts the mod may fetch from (HTTPS only, exact names) once approved and
    /// while the internet switch is on.
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub size: Option<Size>,
    /// Language code (`uk`, `fr`, ...) → translated name and description.
    #[serde(default)]
    pub translations: BTreeMap<String, LocalText>,
}

impl Manifest {
    pub fn parse(json: &str) -> Result<Self, String> {
        let m: Manifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if !valid_id(&m.id) {
            return Err(format!(
                "invalid id {:?}: use lowercase dot-separated parts like \"you.widget\"",
                m.id
            ));
        }
        check_text("name", Some(&m.name), 64)?;
        check_text("version", Some(&m.version), 32)?;
        check_text("author", m.author.as_deref(), 64)?;
        check_text("description", m.description.as_deref(), 280)?;
        if safe_rel_path(&m.entry).is_none() {
            return Err(format!("invalid entry {:?}", m.entry));
        }
        if m.network.len() > MAX_HOSTS {
            return Err(format!("network may list at most {MAX_HOSTS} hosts"));
        }
        if let Some(bad) = m.network.iter().find(|h| !valid_host(h)) {
            return Err(format!("invalid network host {bad:?}: use a public name like \"api.example.com\""));
        }
        if m.translations.len() > 32 {
            return Err("too many translations".into());
        }
        for (lang, text) in &m.translations {
            if lang.len() != 2 || !lang.bytes().all(|b| b.is_ascii_lowercase()) {
                return Err(format!("invalid translation language {lang:?}: use a code like \"fr\""));
            }
            check_text("name", text.name.as_deref(), 64)?;
            check_text("description", text.description.as_deref(), 280)?;
        }
        if let Some(s) = m.size {
            if !((40.0..=4000.0).contains(&s.w) && (40.0..=4000.0).contains(&s.h)) {
                return Err("size must be between 40 and 4000 px".into());
            }
        }
        Ok(m)
    }

    pub fn name_in(&self, lang: &str) -> &str {
        self.translations.get(lang).and_then(|t| t.name.as_deref()).unwrap_or(&self.name)
    }

    pub fn description_in(&self, lang: &str) -> Option<&str> {
        self.translations
            .get(lang)
            .and_then(|t| t.description.as_deref())
            .or(self.description.as_deref())
    }

    pub fn default_size(&self) -> Size {
        self.size.unwrap_or(Size { w: 240.0, h: 140.0 })
    }
}

fn check_text(field: &str, value: Option<&str>, max: usize) -> Result<(), String> {
    match value {
        Some(v) if v.trim().is_empty() || v.chars().count() > max || v.chars().any(char::is_control) => {
            Err(format!("{field} must be 1-{max} characters"))
        }
        _ => Ok(()),
    }
}

/// `you.widget`, `acme.weather-pro`: lowercase, at least two dot-separated parts.
pub fn valid_id(id: &str) -> bool {
    id.len() <= 64
        && id.split('.').count() >= 2
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// A public DNS name such as `api.open-meteo.com`. IP literals, `localhost` and local-network
/// suffixes are refused so an approved mod cannot be pointed at the user's own network by name.
pub fn valid_host(host: &str) -> bool {
    const LOCAL: [&str; 7] = [".localhost", ".local", ".internal", ".lan", ".home", ".arpa", ".test"];
    let labels: Vec<&str> = host.split('.').collect();
    let label_ok = |l: &&str| {
        (1..=63).contains(&l.len())
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    host.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(label_ok)
        // A numeric last label would make an IPv4 literal (or something parsed as one).
        && labels.last().is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_lowercase()))
        && !LOCAL.iter().any(|suffix| host.ends_with(suffix))
}

/// Accepts `a/b.js`. Rejects anything that could escape the folder or need decoding:
/// empty or dot-leading segments (`..`, `.git`), backslashes, `%`, `:` (drives, NTFS streams).
pub fn safe_rel_path(path: &str) -> Option<PathBuf> {
    if path.is_empty() || path.len() > 255 {
        return None;
    }
    let mut out = PathBuf::new();
    for seg in path.split('/') {
        let ok_chars = seg
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        if seg.is_empty() || seg.starts_with('.') || !ok_chars {
            return None;
        }
        out.push(seg);
    }
    Some(out)
}

/// Resolves `rel` inside `root`. The canonical result must still be inside `root`, which also
/// rejects symlinks pointing out and Windows device names such as `CON`.
pub fn resolve_in(root: &Path, rel: &str) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let full = root.join(safe_rel_path(rel)?).canonicalize().ok()?;
    (full.starts_with(&root) && full.is_file()).then_some(full)
}

#[derive(Debug, Clone)]
pub struct InstalledMod {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub builtin: bool,
}

/// Reads every `<dir>/<folder>/manifest.json`. A broken mod is reported, never fatal.
/// Scan built-ins first: later duplicates are rejected, so a user mod cannot take over a
/// built-in id (and its approval). The `deskwave.` prefix is reserved for built-ins.
pub fn scan(dir: &Path, builtin: bool, mods: &mut Vec<InstalledMod>, errors: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().into_owned();
        let parsed = std::fs::read_to_string(path.join("manifest.json"))
            .map_err(|e| format!("manifest.json: {e}"))
            .and_then(|s| Manifest::parse(&s));
        match parsed {
            Ok(m) if !builtin && m.id.starts_with("deskwave.") => {
                errors.push(format!("{folder}: ids starting with \"deskwave.\" are reserved"))
            }
            Ok(m) if mods.iter().any(|x| x.manifest.id == m.id) => {
                errors.push(format!("{folder}: another mod already uses id {}", m.id))
            }
            Ok(manifest) => mods.push(InstalledMod { manifest, dir: path, builtin }),
            Err(e) => errors.push(format!("{folder}: {e}")),
        }
    }
    mods.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOCK: &str = r#"{"id":"acme.clock","name":"Clock","version":"1.0","kind":"widget","entry":"index.html","permissions":["battery"]}"#;

    #[test]
    fn parses_valid_manifest() {
        let m = Manifest::parse(CLOCK).unwrap();
        assert_eq!(m.kind, Kind::Widget);
        assert_eq!(m.permissions, vec![Permission::Battery]);
    }

    #[test]
    fn rejects_unknown_fields_and_permissions() {
        assert!(Manifest::parse(&CLOCK.replace("\"kind\"", "\"extra\":1,\"kind\"")).is_err());
        assert!(Manifest::parse(&CLOCK.replace("battery", "filesystem")).is_err());
    }

    #[test]
    fn rejects_bad_ids_and_entries() {
        for id in ["clock", "Acme.clock", "acme..clock", "acme/clock", "acme.cl ock"] {
            assert!(Manifest::parse(&CLOCK.replace("acme.clock", id)).is_err(), "{id}");
        }
        for entry in ["../x.html", "/x.html", "a\\\\b.html", "C:x.html", "a/%2e%2e/b", ""] {
            assert!(Manifest::parse(&CLOCK.replace("index.html", entry)).is_err(), "{entry}");
        }
    }

    #[test]
    fn translated_names_fall_back_to_english() {
        let json = CLOCK.replace("\"permissions\"", r#""translations":{"uk":{"name":"Годинник"}},"permissions""#);
        let m = Manifest::parse(&json).unwrap();
        assert_eq!(m.name_in("uk"), "Годинник");
        assert_eq!(m.name_in("fr"), "Clock");
        assert!(Manifest::parse(&CLOCK.replace("\"permissions\"", r#""translations":{"UKR":{"name":"x"}},"permissions""#)).is_err());
    }

    #[test]
    fn network_hosts_must_be_public_names() {
        let with = |hosts: &str| CLOCK.replace("\"permissions\"", &format!("\"network\":[{hosts}],\"permissions\""));
        assert_eq!(Manifest::parse(&with("\"api.open-meteo.com\"")).unwrap().network, ["api.open-meteo.com"]);
        for bad in ["localhost", "127.0.0.1", "192.168.1.1", "printer.local", "x.localhost", "API.example.com",
                    "example", "-a.com", "a..com", "https://a.com", "a.com:443", "router.home.arpa", "[::1]"] {
            assert!(Manifest::parse(&with(&format!("{bad:?}"))).is_err(), "{bad}");
        }
        let many = (0..=MAX_HOSTS).map(|i| format!("\"h{i}.example.com\"")).collect::<Vec<_>>().join(",");
        assert!(Manifest::parse(&with(&many)).is_err());
    }

    #[test]
    fn safe_paths() {
        assert_eq!(safe_rel_path("js/main.js"), Some(PathBuf::from("js").join("main.js")));
        for bad in ["..", "a/../b", "./a", "a//b", ".hidden", "a/", "a b", "x:y", "a\\b"] {
            assert_eq!(safe_rel_path(bad), None, "{bad}");
        }
    }

    #[test]
    fn resolve_stays_inside_root() {
        let dir = crate::test_dir("resolve");
        let root = dir.join("mod");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.js"), "").unwrap();
        std::fs::write(dir.join("secret.txt"), "").unwrap();
        assert!(resolve_in(&root, "a.js").is_some());
        assert!(resolve_in(&root, "missing.js").is_none());
        assert!(resolve_in(&root, "../secret.txt").is_none());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("secret.txt"), root.join("link.txt")).unwrap();
            assert!(resolve_in(&root, "link.txt").is_none());
        }
    }

    #[test]
    fn scan_reports_errors_and_protects_builtin_ids() {
        let dir = crate::test_dir("scan");
        let write = |base: &str, folder: &str, json: &str| {
            let d = dir.join(base).join(folder);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("manifest.json"), json).unwrap();
        };
        write("builtin", "clock", &CLOCK.replace("acme.clock", "deskwave.clock"));
        write("user", "fake", &CLOCK.replace("acme.clock", "deskwave.clock"));
        write("user", "good", CLOCK);
        write("user", "broken", "{");
        let (mut mods, mut errors) = (vec![], vec![]);
        scan(&dir.join("builtin"), true, &mut mods, &mut errors);
        scan(&dir.join("user"), false, &mut mods, &mut errors);
        let ids: Vec<_> = mods.iter().map(|m| (m.manifest.id.as_str(), m.builtin)).collect();
        assert!(ids.contains(&("deskwave.clock", true)) && ids.contains(&("acme.clock", false)));
        assert_eq!(mods.len(), 2);
        assert_eq!(errors.len(), 2, "{errors:?}");
    }
}
