//! The wallpapers folder and importing what the user drops onto Settings: video and image
//! files are copied into the wallpapers folder, mod folders into the mods folder.

use std::io;
use std::path::{Path, PathBuf};

pub const VIDEO: &[&str] = &["mp4", "webm"];
pub const IMAGE: &[&str] = &["gif", "png", "apng", "jpg", "jpeg", "webp", "avif"];

/// A mod folder may hold at most this many files and bytes, and nest this deep.
pub const MOD_MAX_FILES: usize = 2000;
pub const MOD_MAX_BYTES: u64 = 100 * 1024 * 1024;
const MOD_MAX_DEPTH: usize = 16;

pub fn ext(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

pub fn is_video(name: &str) -> bool {
    VIDEO.contains(&ext(name).as_str())
}

pub fn is_media(name: &str) -> bool {
    let e = ext(name);
    VIDEO.contains(&e.as_str()) || IMAGE.contains(&e.as_str())
}

/// A plain file name: no folders, no `..`, not hidden, no control or reserved characters.
pub fn valid_file_name(name: &str) -> bool {
    (1..=255).contains(&name.len())
        && !name.starts_with('.')
        && !name.ends_with([' ', '.'])
        && !name.chars().any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
}

/// Video and image files directly inside `dir`, sorted.
pub fn media_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| valid_file_name(n) && is_media(n))
        .collect();
    names.sort();
    names
}

/// `name`, or `stem (2).ext`, `stem (3).ext`, … — the first that doesn't exist in `dir`.
pub fn unique_name(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_owned();
    }
    let (stem, dot_ext) = match name.rsplit_once('.') {
        Some((stem, e)) if !stem.is_empty() => (stem, format!(".{e}")),
        _ => (name, String::new()),
    };
    (2..)
        .map(|n| format!("{stem} ({n}){dot_ext}"))
        .find(|candidate| !dir.join(candidate).exists())
        .expect("an unused name exists")
}

#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub struct Imported {
    /// New names inside the destination folder.
    pub added: Vec<String>,
    /// Names of dropped items that were not imported.
    pub skipped: Vec<String>,
}

fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// Copies the video and image files among `paths` into `dir` under unused names. Folders,
/// links to folders and other files are skipped. Existing files are never overwritten.
pub fn import_media(paths: &[PathBuf], dir: &Path) -> Imported {
    let mut out = Imported::default();
    for path in paths {
        let name = display_name(path);
        let ok = valid_file_name(&name) && is_media(&name) && path.is_file();
        let copied = ok.then(|| {
            let target = unique_name(dir, &name);
            copy_new(path, &dir.join(&target)).map(|_| target)
        });
        match copied {
            Some(Ok(target)) => out.added.push(target),
            _ => out.skipped.push(name),
        }
    }
    out
}

/// Copies `from` to `to`, failing if `to` already exists.
fn copy_new(from: &Path, to: &Path) -> io::Result<()> {
    let mut source = std::fs::File::open(from)?;
    let mut target = std::fs::OpenOptions::new().write(true).create_new(true).open(to)?;
    let copied = io::copy(&mut source, &mut target).and_then(|_| target.sync_all());
    if copied.is_err() {
        drop(target);
        let _ = std::fs::remove_file(to);
    }
    copied
}

/// Copies each dropped folder that contains a `manifest.json` into `mods` under an unused
/// folder name. Symbolic links are left out, and folders over the size limits are refused,
/// so a drop cannot pull in files from elsewhere or fill the disk.
pub fn import_mods(paths: &[PathBuf], mods: &Path) -> Imported {
    let mut out = Imported::default();
    for path in paths {
        let name = display_name(path);
        let is_mod = valid_file_name(&name)
            && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
            && path.join("manifest.json").is_file();
        let target = mods.join(unique_name(mods, &name));
        let copied = is_mod && plan_copy(path).is_ok_and(|files| copy_tree(path, &target, &files).is_ok());
        if copied {
            out.added.push(display_name(&target));
        } else {
            if target.exists() && is_mod {
                let _ = std::fs::remove_dir_all(&target);
            }
            out.skipped.push(name);
        }
    }
    out
}

/// Relative paths of the regular files under `root`, checked against the limits first.
fn plan_copy(root: &Path) -> io::Result<Vec<PathBuf>> {
    let too_big = || io::Error::other("mod folder is too large");
    let mut files = Vec::new();
    let mut bytes = 0u64;
    let mut pending = vec![(PathBuf::new(), 0usize)];
    while let Some((relative, depth)) = pending.pop() {
        if depth > MOD_MAX_DEPTH {
            return Err(too_big());
        }
        for entry in std::fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let kind = entry.file_type()?; // does not follow links
            let path = relative.join(entry.file_name());
            if kind.is_dir() {
                pending.push((path, depth + 1));
            } else if kind.is_file() {
                bytes += entry.metadata()?.len();
                files.push(path);
                if files.len() > MOD_MAX_FILES || bytes > MOD_MAX_BYTES {
                    return Err(too_big());
                }
            }
        }
    }
    Ok(files)
}

fn copy_tree(from: &Path, to: &Path, files: &[PathBuf]) -> io::Result<()> {
    std::fs::create_dir(to)?;
    for relative in files {
        let target = to.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        copy_new(&from.join(relative), &target)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(valid_file_name("Rainy window.mp4") && valid_file_name("Озеро.jpg"));
        for bad in ["", ".hidden.png", "../x.png", "a/b.png", "a\\b.png", "c:x.png", "x.png ", "x."] {
            assert!(!valid_file_name(bad), "{bad:?}");
        }
        assert!(is_media("A.MP4") && is_video("a.webm") && !is_video("a.gif") && !is_media("a.exe"));
    }

    #[test]
    fn unique_names_never_overwrite() {
        let dir = crate::test_dir("unique");
        assert_eq!(unique_name(&dir, "lake.jpg"), "lake.jpg");
        std::fs::write(dir.join("lake.jpg"), "1").unwrap();
        std::fs::write(dir.join("lake (2).jpg"), "2").unwrap();
        assert_eq!(unique_name(&dir, "lake.jpg"), "lake (3).jpg");
        std::fs::write(dir.join("notes"), "").unwrap();
        assert_eq!(unique_name(&dir, "notes"), "notes (2)");
    }

    #[test]
    fn imports_only_media_files() {
        let dir = crate::test_dir("import-media");
        let (src, dest) = (dir.join("src"), dir.join("wallpapers"));
        std::fs::create_dir_all(src.join("folder.mp4")).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        for name in ["rain.mp4", "lake.JPG", "setup.exe"] {
            std::fs::write(src.join(name), name).unwrap();
        }
        std::fs::write(dest.join("rain.mp4"), "older").unwrap();
        let paths: Vec<PathBuf> = ["rain.mp4", "lake.JPG", "setup.exe", "folder.mp4", "missing.png"]
            .iter()
            .map(|n| src.join(n))
            .collect();
        let result = import_media(&paths, &dest);
        assert_eq!(result.added, ["rain (2).mp4", "lake.JPG"]);
        assert_eq!(result.skipped, ["setup.exe", "folder.mp4", "missing.png"]);
        assert_eq!(std::fs::read_to_string(dest.join("rain.mp4")).unwrap(), "older", "not overwritten");
        assert_eq!(media_files(&dest), ["lake.JPG", "rain (2).mp4", "rain.mp4"]);
    }

    #[test]
    fn imports_mod_folders_within_limits() {
        let dir = crate::test_dir("import-mods");
        let (src, mods) = (dir.join("src"), dir.join("mods"));
        std::fs::create_dir_all(src.join("pomodoro/img")).unwrap();
        std::fs::write(src.join("pomodoro/manifest.json"), "{}").unwrap();
        std::fs::write(src.join("pomodoro/img/a.png"), "png").unwrap();
        std::fs::create_dir_all(src.join("not-a-mod")).unwrap();
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(mods.join("pomodoro")).unwrap();
        let paths = [src.join("pomodoro"), src.join("not-a-mod"), src.join("pomodoro/img/a.png")];
        let result = import_mods(&paths, &mods);
        assert_eq!(result.added, ["pomodoro (2)"]);
        assert_eq!(result.skipped, ["not-a-mod", "a.png"]);
        assert_eq!(std::fs::read_to_string(mods.join("pomodoro (2)/img/a.png")).unwrap(), "png");

        // Too many files: refused, and nothing is left behind.
        let big = src.join("big");
        std::fs::create_dir_all(&big).unwrap();
        std::fs::write(big.join("manifest.json"), "{}").unwrap();
        for i in 0..MOD_MAX_FILES {
            std::fs::write(big.join(format!("{i}.txt")), "").unwrap();
        }
        let result = import_mods(&[big], &mods);
        assert_eq!(result.skipped, ["big"]);
        assert!(!mods.join("big").exists());
    }

    #[cfg(unix)]
    #[test]
    fn mod_import_leaves_links_out() {
        let dir = crate::test_dir("import-links");
        let (src, mods) = (dir.join("src/linked"), dir.join("mods"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(src.join("manifest.json"), "{}").unwrap();
        std::fs::write(dir.join("secret.txt"), "secret").unwrap();
        std::os::unix::fs::symlink(dir.join("secret.txt"), src.join("secret.txt")).unwrap();
        std::os::unix::fs::symlink(&dir, src.join("everything")).unwrap();
        assert_eq!(import_mods(&[src], &mods).added, ["linked"]);
        assert!(mods.join("linked/manifest.json").exists());
        assert!(!mods.join("linked/secret.txt").exists() && !mods.join("linked/everything").exists());
    }
}
