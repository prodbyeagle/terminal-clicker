//! Einstellungen des Spielers (gespeichert in `settings.json`).

use crate::ascii::background::{Charset, Pattern};
use crate::format::NumberFormat;
use crate::theme;
use serde::{Deserialize, Serialize};

/// Dichte des ASCII-Hintergrunds. Gespeichert als 0/1/2 (ältere Dateien ebenso).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "u8", into = "u8")]
pub enum Density {
    Low,
    #[default]
    Mid,
    High,
}

/// Wie schnell der ASCII-Hintergrund wabert. Gespeichert als 0/1/2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "u8", into = "u8")]
pub enum Speed {
    Slow,
    #[default]
    Mid,
    Fast,
}

/// Wie deutlich der Hintergrund durchscheint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strength {
    Faint,
    #[default]
    Low,
    Mid,
    High,
}

/// Größe der Formen im Hintergrund.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scale {
    Fine,
    #[default]
    Mid,
    Coarse,
}

impl Strength {
    pub const ALL: [Strength; 4] = [
        Strength::Faint,
        Strength::Low,
        Strength::Mid,
        Strength::High,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Strength::Faint => "hauch",
            Strength::Low => "leicht",
            Strength::Mid => "mittel",
            Strength::High => "stark",
        }
    }
}

impl Scale {
    pub const ALL: [Scale; 3] = [Scale::Fine, Scale::Mid, Scale::Coarse];

    pub fn label(self) -> &'static str {
        match self {
            Scale::Fine => "fein",
            Scale::Mid => "normal",
            Scale::Coarse => "grob",
        }
    }
}

impl Density {
    pub const ALL: [Density; 3] = [Density::Low, Density::Mid, Density::High];

    pub fn label(self) -> &'static str {
        match self {
            Density::Low => "niedrig",
            Density::Mid => "mittel",
            Density::High => "hoch",
        }
    }
}

impl Speed {
    pub const ALL: [Speed; 3] = [Speed::Slow, Speed::Mid, Speed::Fast];

    pub fn label(self) -> &'static str {
        match self {
            Speed::Slow => "langsam",
            Speed::Mid => "mittel",
            Speed::Fast => "schnell",
        }
    }
}

impl From<u8> for Density {
    fn from(v: u8) -> Self {
        Self::ALL[(v as usize).min(Self::ALL.len() - 1)]
    }
}

impl From<Density> for u8 {
    fn from(v: Density) -> Self {
        v as u8
    }
}

impl From<u8> for Speed {
    fn from(v: u8) -> Self {
        Self::ALL[(v as usize).min(Self::ALL.len() - 1)]
    }
}

impl From<Speed> for u8 {
    fn from(v: Speed) -> Self {
        v as u8
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `"system"` oder die `id` eines Eintrags aus `theme::THEMES`.
    pub theme: String,
    pub ascii_background: bool,
    pub density: Density,
    pub speed: Speed,
    pub pattern: Pattern,
    pub charset: Charset,
    pub strength: Strength,
    pub scale: Scale,
    pub animations: bool,
    pub number_format: NumberFormat,
    /// Ob das Intro (Hilfe-Seite beim ersten Start) schon gezeigt wurde.
    pub intro_seen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: theme::SYSTEM.to_string(),
            ascii_background: false,
            density: Density::default(),
            speed: Speed::default(),
            pattern: Pattern::default(),
            charset: Charset::default(),
            strength: Strength::default(),
            scale: Scale::default(),
            animations: true,
            number_format: NumberFormat::Full,
            intro_seen: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_numeric_values_still_load() {
        let s: Settings = serde_json::from_str(r#"{"density": 2, "speed": 0}"#).unwrap();
        assert_eq!((s.density, s.speed), (Density::High, Speed::Slow));
        let broken: Settings = serde_json::from_str(r#"{"density": 9}"#).unwrap();
        assert_eq!(broken.density, Density::High);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""density":2"#));
    }

    #[test]
    fn background_style_roundtrips_and_defaults() {
        let old: Settings = serde_json::from_str(r#"{"density": 1}"#).unwrap();
        assert_eq!(
            (old.pattern, old.charset),
            (Pattern::Clouds, Charset::Classic)
        );
        let s = Settings {
            pattern: Pattern::Rain,
            charset: Charset::Binary,
            scale: Scale::Coarse,
            ..Settings::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""pattern":"rain""#), "{json}");
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
    }
}
