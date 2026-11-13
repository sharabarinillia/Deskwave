//! Platform-independent Deskwave logic. Everything that decides what a mod may do lives
//! here so it can be reviewed in one place and tested on any OS.

pub mod config;
pub mod i18n;
pub mod manifest;
pub mod net;
pub mod sandbox;
pub mod storage;

#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("deskwave-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
