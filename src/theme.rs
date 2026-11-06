//! Themes als erweiterbare Liste.
//!
//! Ein neues Theme = ein neuer Eintrag in [`THEMES`]. Es taucht dann
//! automatisch in den Settings auf und wird per `id` in `settings.json`
//! gespeichert. Die Auswahl „system“ folgt dem System-Theme und nimmt dafür
//! [`SYSTEM_LIGHT`] bzw. [`SYSTEM_DARK`].

use gpui::{Hsla, WindowAppearance, rgb};

/// Rohdefinition eines Themes. Farben als `0xRRGGBB`, damit Einträge `const` bleiben.
#[derive(Debug, Clone, Copy)]
struct ThemeDef {
    /// Stabile ID für `settings.json` – nach Veröffentlichung nicht mehr ändern.
    id: &'static str,
    /// Anzeigename in den Settings.
    label: &'static str,
    bg: u32,
    fg: u32,
    dim: u32,
    line: u32,
}

const THEMES: &[ThemeDef] = &[
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
    ThemeDef {
        id: "dracula",
        label: "dracula",
        bg: 0x282a36,
        fg: 0xf8f8f2,
        dim: 0x6272a4,
        line: 0x44475a,
    },
    ThemeDef {
        id: "nord",
        label: "nord",
        bg: 0x2e3440,
        fg: 0xeceff4,
        dim: 0x81a1c1,
        line: 0x434c5e,
    },
    ThemeDef {
        id: "monokai",
        label: "monokai",
        bg: 0x272822,
        fg: 0xf8f8f2,
        dim: 0x75715e,
        line: 0x49483e,
    },
    ThemeDef {
        id: "gruvbox",
        label: "gruvbox",
        bg: 0x282828,
        fg: 0xebdbb2,
        dim: 0xa89984,
        line: 0x504945,
    },
    ThemeDef {
        id: "solarized",
        label: "solarized",
        bg: 0x002b36,
        fg: 0xeee8d5,
        dim: 0x839496,
        line: 0x073642,
    },
    ThemeDef {
        id: "tokyo",
        label: "tokyo night",
        bg: 0x1a1b26,
        fg: 0xc0caf5,
        dim: 0x565f89,
        line: 0x292e42,
    },
    ThemeDef {
        id: "catppuccin",
        label: "catppuccin",
        bg: 0x1e1e2e,
        fg: 0xcdd6f4,
        dim: 0x9399b2,
        line: 0x45475a,
    },
    ThemeDef {
        id: "rosepine",
        label: "rose pine",
        bg: 0x191724,
        fg: 0xe0def4,
        dim: 0x908caa,
        line: 0x403d52,
    },
    ThemeDef {
        id: "synthwave",
        label: "synthwave",
        bg: 0x241b2f,
        fg: 0xff7edb,
        dim: 0x9d72b0,
        line: 0x593d70,
    },
    ThemeDef {
        id: "cyberpunk",
        label: "cyberpunk",
        bg: 0x0d0221,
        fg: 0x00f5ff,
        dim: 0xb967ff,
        line: 0x3b1764,
    },
    ThemeDef {
        id: "forest",
        label: "forest",
        bg: 0x14251c,
        fg: 0xc5e1c5,
        dim: 0x729b78,
        line: 0x294b35,
    },
    ThemeDef {
        id: "ocean",
        label: "ozean",
        bg: 0x071e2d,
        fg: 0xb8e3f5,
        dim: 0x5b91aa,
        line: 0x153c52,
    },
    ThemeDef {
        id: "coffee",
        label: "kaffee",
        bg: 0x211a16,
        fg: 0xe8d5c4,
        dim: 0xa58b76,
        line: 0x49352a,
    },
    ThemeDef {
        id: "lavender",
        label: "lavendel",
        bg: 0x242038,
        fg: 0xe6ddff,
        dim: 0xa99bc9,
        line: 0x49415f,
    },
    ThemeDef {
        id: "crimson",
        label: "crimson",
        bg: 0x200b12,
        fg: 0xffd6df,
        dim: 0xb56b7d,
        line: 0x51202f,
    },
    ThemeDef {
        id: "terminal",
        label: "terminal",
        bg: 0x101010,
        fg: 0x00ff00,
        dim: 0x008f00,
        line: 0x263b26,
    },
];

/// Pseudo-ID für „folgt dem System“.
pub const SYSTEM: &str = "system";
/// Theme, das „system“ im hellen Modus nimmt.
const SYSTEM_LIGHT: &str = "light";
/// Theme, das „system“ im dunklen Modus nimmt.
const SYSTEM_DARK: &str = "dark";

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

fn find(id: &str) -> Option<&'static ThemeDef> {
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
fn resolve_def(choice: &str, appearance: WindowAppearance) -> &'static ThemeDef {
    find(choice).unwrap_or_else(|| {
        let id = if is_dark(appearance) {
            SYSTEM_DARK
        } else {
            SYSTEM_LIGHT
        };
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
        assert_eq!(
            resolve_def(SYSTEM, WindowAppearance::Light).id,
            SYSTEM_LIGHT
        );
    }

    #[test]
    fn explicit_and_unknown_choice() {
        assert_eq!(
            resolve_def("phosphor", WindowAppearance::Light).id,
            "phosphor"
        );
        assert_eq!(
            resolve_def("gibts-nicht", WindowAppearance::Dark).id,
            SYSTEM_DARK
        );
    }
}
