//! Per-widget key-value storage (`storage/<widget-id>.json`), e.g. the text of a note.

use crate::config::{valid_widget_id, write_atomic};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const MAX_BYTES: usize = 256 * 1024;
const MAX_KEY: usize = 64;

fn file(dir: &Path, widget: &str) -> Result<PathBuf, String> {
    if valid_widget_id(widget) {
        Ok(dir.join(format!("{widget}.json")))
    } else {
        Err("invalid widget id".into())
    }
}

pub fn read(dir: &Path, widget: &str) -> Result<Map<String, Value>, String> {
    match std::fs::read(file(dir, widget)?) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.to_string()),
        // Refuse instead of returning empty: a later write would erase the user's data.
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| "storage file is corrupt".into()),
    }
}

/// Sets `key` (a `null` value deletes it). Fails without writing if the result is too big.
pub fn set(dir: &Path, widget: &str, key: &str, value: Value) -> Result<(), String> {
    if key.is_empty() || key.len() > MAX_KEY {
        return Err(format!("key must be 1-{MAX_KEY} bytes"));
    }
    let mut map = read(dir, widget)?;
    if value.is_null() {
        map.remove(key);
    } else {
        map.insert(key.to_owned(), value);
    }
    let bytes = serde_json::to_vec(&map).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("storage full (256 KiB per widget)".into());
    }
    write_atomic(&file(dir, widget)?, &bytes).map_err(|e| e.to_string())
}

pub fn remove(dir: &Path, widget: &str) {
    if let Ok(path) = file(dir, widget) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn set_get_delete_and_limits() {
        let dir = crate::test_dir("storage");
        set(&dir, "w1", "text", json!("hello")).unwrap();
        assert_eq!(read(&dir, "w1").unwrap()["text"], "hello");
        set(&dir, "w1", "text", Value::Null).unwrap();
        assert!(read(&dir, "w1").unwrap().is_empty());

        assert!(set(&dir, "w1", "big", json!("x".repeat(MAX_BYTES))).is_err());
        assert!(set(&dir, "../evil", "k", json!(1)).is_err());
        assert!(set(&dir, "w1", "", json!(1)).is_err());

        std::fs::write(dir.join("w2.json"), "{oops").unwrap();
        assert!(set(&dir, "w2", "k", json!(1)).is_err(), "never overwrite corrupt data");
        remove(&dir, "w1");
        assert!(!dir.join("w1.json").exists());
    }
}
