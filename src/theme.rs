//! Themes als erweiterbare Liste.
//!
//! Ein neues Theme = ein neuer Eintrag in [`THEMES`]. Es taucht dann
//! automatisch in den Settings auf und wird per `id` in `settings.json`
//! gespeichert. Die Auswahl „system“ folgt dem System-Theme und nimmt dafür
//! [`SYSTEM_LIGHT`] bzw. [`SYSTEM_DARK`].

use gpui::{Hsla, WindowAppearance, rgb};

/// Rohdefinition eines Themes. Farben als `0xRRGGBB`, damit Einträge `const` bleiben.
#[derive(Debug, Clone, Copy)]
pub struct ThemeDef {
    /// Stabile ID für `settings.json` – nach Veröffentlichung nicht mehr ändern.
    pub id: &'static str,
    /// Anzeigename in den Settings.
    pub label: &'static str,
    /// Hintergrund
    pub bg: u32,
    /// Text
    pub fg: u32,
    /// Nebentext
    pub dim: u32,
    /// Trennlinien
    pub line: u32,
}

/// Alle verfügbaren Themes in Anzeigereihenfolge.
pub const THEMES: &[ThemeDef] = &[
    ThemeDef {
        id: "light",
        label: "hell",
        bg: 0xffffff,
        fg: 0x000000,
        dim: 0x666666,
        line: 0xcccccc,
    },
    ThemeDef {
        id: "dark",
        label: "dunkel",
        bg: 0x000000,
        fg: 0xffffff,
        dim: 0x8c8c8c,
        line: 0x333333,
    },
    // Beispiel für ein zusätzliches Theme – einfach weitere Einträge anhängen.
    ThemeDef {
        id: "phosphor",
        label: "phosphor",
        bg: 0x000000,
        fg: 0x33ff66,
        dim: 0x1f9940,
        line: 0x0f3d1c,
    },
    ThemeDef {
        id: "amber",
        label: "amber",
        bg: 0x000000,
        fg: 0xffb000,
        dim: 0x9a6a00,
        line: 0x3a2800,
    },
    ThemeDef {
        id: "ice",
        label: "eis",
        bg: 0x050b14,
        fg: 0x9fe8ff,
        dim: 0x4f7f99,
        line: 0x16283a,
    },
    ThemeDef {
        id: "midnight",
        label: "mitternacht",
        bg: 0x0b1020,
        fg: 0xd6deeb,
        dim: 0x6b7a99,
        line: 0x1e2840,
    },
    ThemeDef {
        id: "alarm",
        label: "alarm",
        bg: 0x000000,
        fg: 0xff3b30,
        dim: 0x99231d,
        line: 0x3d0e0b,
    },
    ThemeDef {
        id: "paper",
        label: "papier",
        bg: 0xf4efe1,
        fg: 0x2b2418,
        dim: 0x7a6f5a,
        line: 0xd8cfb8,
    },
    ThemeDef {
        id: "blueprint",
        label: "blaupause",
        bg: 0x0a3d91,
        fg: 0xffffff,
        dim: 0x9db8e6,
        line: 0x2a5bb0,
    },
];

/// Pseudo-ID für „folgt dem System“.
pub const SYSTEM: &str = "system";
/// Theme, das „system“ im hellen Modus nimmt.
pub const SYSTEM_LIGHT: &str = "light";
/// Theme, das „system“ im dunklen Modus nimmt.
pub const SYSTEM_DARK: &str = "dark";

/// Aufgelöste Farben, wie die UI sie braucht.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Hsla,
    pub fg: Hsla,
    pub dim: Hsla,
    pub line: Hsla,
}

impl From<&ThemeDef> for Theme {
    fn from(def: &ThemeDef) -> Self {
        Self {
            bg: rgb(def.bg).into(),
            fg: rgb(def.fg).into(),
            dim: rgb(def.dim).into(),
            line: rgb(def.line).into(),
        }
    }
}

pub fn find(id: &str) -> Option<&'static ThemeDef> {
    THEMES.iter().find(|t| t.id == id)
}

fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

/// Theme-Definition zur gespeicherten Auswahl. Unbekannte IDs
/// (z. B. ein entferntes Theme) fallen auf „system“ zurück.
pub fn resolve_def(choice: &str, appearance: WindowAppearance) -> &'static ThemeDef {
    find(choice).unwrap_or_else(|| {
        let id = if is_dark(appearance) { SYSTEM_DARK } else { SYSTEM_LIGHT };
        find(id).unwrap_or(&THEMES[0])
    })
}

pub fn resolve(choice: &str, appearance: WindowAppearance) -> Theme {
    resolve_def(choice, appearance).into()
}

/// Alle Auswahlmöglichkeiten für die Settings: zuerst „system“, dann jedes Theme.
pub fn choices() -> impl Iterator<Item = &'static str> {
    std::iter::once(SYSTEM).chain(THEMES.iter().map(|t| t.id))
}

/// Anzeigename einer Auswahl.
pub fn label(choice: &str) -> &'static str {
    find(choice).map(|t| t.label).unwrap_or("system")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_not_system() {
        for (i, a) in THEMES.iter().enumerate() {
            assert_ne!(a.id, SYSTEM);
            for b in &THEMES[i + 1..] {
                assert_ne!(a.id, b.id, "doppelte Theme-ID");
            }
        }
    }

    #[test]
    fn system_targets_exist() {
        assert!(find(SYSTEM_LIGHT).is_some());
        assert!(find(SYSTEM_DARK).is_some());
    }

    #[test]
    fn system_follows_appearance() {
        assert_eq!(resolve_def(SYSTEM, WindowAppearance::Dark).id, SYSTEM_DARK);
        assert_eq!(resolve_def(SYSTEM, WindowAppearance::Light).id, SYSTEM_LIGHT);
    }

    #[test]
    fn explicit_and_unknown_choice() {
        assert_eq!(resolve_def("phosphor", WindowAppearance::Light).id, "phosphor");
        assert_eq!(resolve_def("gibts-nicht", WindowAppearance::Dark).id, SYSTEM_DARK);
    }

    #[test]
    fn choices_start_with_system() {
        let all: Vec<_> = choices().collect();
        assert_eq!(all[0], SYSTEM);
        assert_eq!(all.len(), THEMES.len() + 1);
    }
}
