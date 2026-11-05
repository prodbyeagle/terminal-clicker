//! Laden und Speichern als JSON im Datenordner des Systems
//! (macOS: ~/Library/Application Support/terminal-clicker/).

use crate::format::NumberFormat;
use crate::game::SaveData;
use crate::theme;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_DIR: &str = "terminal-clicker";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `"system"` oder die `id` eines Eintrags aus `theme::THEMES`.
    pub theme: String,
    pub ascii_background: bool,
    /// 0 = niedrig, 1 = mittel, 2 = hoch
    pub density: u8,
    /// 0 = langsam, 1 = mittel, 2 = schnell
    pub speed: u8,
    pub animations: bool,
    /// Zahlen voll ausschreiben oder in KB/MB bzw. KiB/MiB umrechnen.
    pub number_format: NumberFormat,
    /// Intro (Hilfe-Seite beim ersten Start) schon gesehen?
    pub intro_seen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: theme::SYSTEM.to_string(),
            ascii_background: false,
            density: 1,
            speed: 1,
            animations: true,
            number_format: NumberFormat::Full,
            intro_seen: false,
        }
    }
}

pub const LEVEL_LABELS: [&str; 3] = ["niedrig", "mittel", "hoch"];
pub const SPEED_LABELS: [&str; 3] = ["langsam", "mittel", "schnell"];

fn dir() -> Option<PathBuf> {
    // Für Tests/Entwicklung umlenkbar.
    if let Ok(p) = std::env::var("TERMINAL_CLICKER_DIR") {
        return Some(PathBuf::from(p));
    }
    dirs::data_dir().map(|d| d.join(APP_DIR))
}

fn read<T: for<'de> Deserialize<'de>>(name: &str) -> Option<T> {
    let path = dir()?.join(name);
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write<T: Serialize>(name: &str, value: &T) -> std::io::Result<()> {
    let dir = dir().ok_or_else(|| std::io::Error::other("kein Datenordner"))?;
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    // Erst temporär schreiben, dann umbenennen – kein halber Spielstand bei Absturz.
    let tmp = dir.join(format!("{name}.tmp"));
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, dir.join(name))
}

pub fn load_game() -> Option<SaveData> {
    read("save.json")
}

pub fn save_game(data: &SaveData) -> std::io::Result<()> {
    write("save.json", data)
}

pub fn load_settings() -> Settings {
    read("settings.json").unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> std::io::Result<()> {
    write("settings.json", settings)
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
