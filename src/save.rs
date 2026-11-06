//! Dateien als JSON im Datenordner des Systems
//! (macOS: ~/Library/Application Support/terminal-clicker/).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_DIR: &str = "terminal-clicker";
pub const GAME_FILE: &str = "save.json";
pub const SETTINGS_FILE: &str = "settings.json";

fn dir() -> Option<PathBuf> {
    // Für Tests und Entwicklung umlenkbar.
    if let Ok(p) = std::env::var("TERMINAL_CLICKER_DIR") {
        return Some(PathBuf::from(p));
    }
    dirs::data_dir().map(|d| d.join(APP_DIR))
}

#[derive(Debug, PartialEq)]
pub struct LoadError {
    pub message: String,
    /// Die unlesbare Datei liegt noch an ihrem Platz (Beiseitelegen ging nicht) –
    /// dann darf sie nicht überschrieben werden.
    pub still_in_place: bool,
}

/// `Ok(None)`: Datei gibt es nicht. Ist sie unlesbar, wird sie als
/// `<name>.broken` beiseitegelegt, damit das nächste Speichern sie nicht überschreibt.
pub fn load<T: for<'de> Deserialize<'de>>(name: &str) -> Result<Option<T>, LoadError> {
    match dir() {
        Some(dir) => load_from(&dir, name),
        None => Ok(None),
    }
}

fn load_from<T: for<'de> Deserialize<'de>>(
    dir: &std::path::Path,
    name: &str,
) -> Result<Option<T>, LoadError> {
    let path = dir.join(name);
    let parsed = match std::fs::read_to_string(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string()),
    };
    parsed.map(Some).map_err(|e| {
        let backup = path.with_file_name(format!("{name}.broken"));
        match std::fs::rename(&path, &backup) {
            Ok(()) => LoadError {
                message: format!("{name} defekt ({e}), gesichert als {name}.broken"),
                still_in_place: false,
            },
            Err(_) => LoadError {
                message: format!("{name} defekt ({e}) – wird nicht überschrieben"),
                still_in_place: true,
            },
        }
    })
}

pub fn store<T: Serialize>(name: &str, value: &T) -> std::io::Result<()> {
    let dir = dir().ok_or_else(|| std::io::Error::other("kein Datenordner"))?;
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    // Erst temporär schreiben, dann umbenennen: kein halber Spielstand bei Absturz.
    let tmp = dir.join(format!("{name}.tmp"));
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, dir.join(name))
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_file_is_set_aside_instead_of_overwritten() {
        let dir = std::env::temp_dir().join(format!("tc-save-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.json"), "{ kaputt").unwrap();
        assert!(load_from::<serde_json::Value>(&dir, "x.json").is_err());
        assert!(!dir.join("x.json").exists());
        assert_eq!(
            std::fs::read_to_string(dir.join("x.json.broken")).unwrap(),
            "{ kaputt"
        );
        assert_eq!(load_from::<serde_json::Value>(&dir, "x.json"), Ok(None));
        std::fs::write(dir.join("y.json"), [0xff, 0xfe]).unwrap();
        assert!(load_from::<serde_json::Value>(&dir, "y.json").is_err());
        assert!(dir.join("y.json.broken").exists());
        std::fs::remove_dir_all(dir).ok();
    }
}
