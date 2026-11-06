//! Die Tastenbelegung – einmal. Daraus entstehen die gpui-Actions, die
//! Tastenbindungen, die Zuordnung zu `Msg`s und die Tastennamen in
//! Statuszeile und Hilfe.

use crate::ui::msg::{Emit, Msg, Screen};
use gpui::{App, Div, KeyBinding, Stateful, prelude::*};

macro_rules! keymap {
    ($( $action:ident [$($stroke:literal),+] $label:literal => $msg:expr; )*) => {
        gpui::actions!(clicker, [$($action),*]);

        pub fn bind(cx: &mut App) {
            cx.bind_keys([$($(KeyBinding::new($stroke, $action, None)),+),*]);
        }

        pub fn attach(el: Stateful<Div>, emit: &Emit) -> Stateful<Div> {
            $(
                let send = emit.clone();
                let el = el.on_action(move |_: &$action, _, cx| send.send($msg, cx));
            )*
            el
        }

        const KEYS: &[(&str, Msg)] = &[$(($label, $msg)),*];
    };
}

keymap! {
    Click ["space"] "space" => Msg::Click;
    Quit ["q", "cmd-q"] "q" => Msg::Quit;
    OpenUpgrades ["u"] "u" => Msg::Open(Screen::Upgrades);
    OpenSettings ["s"] "s" => Msg::Open(Screen::Settings);
    OpenHelp ["h"] "h" => Msg::Open(Screen::Help);
    OpenAdmin ["a"] "a" => Msg::Open(Screen::Admin);
    CollectEvent ["e"] "e" => Msg::CollectEvent;
    Up ["up"] "↑" => Msg::Move(-1);
    Down ["down"] "↓" => Msg::Move(1);
    Left ["left"] "←" => Msg::AdjustSelected(-1);
    Right ["right"] "→" => Msg::AdjustSelected(1);
    Confirm ["enter"] "enter" => Msg::Confirm;
    Back ["escape"] "esc" => Msg::Back;
}

pub fn label(msgs: &[Msg]) -> String {
    msgs.iter()
        .filter_map(|m| KEYS.iter().find(|(_, k)| k == m).map(|(l, _)| *l))
        .collect()
}

/// Tasten für die Hilfe-Seite. Einträge für nicht verfügbare Ansichten blendet die Hilfe aus.
pub const HELP: &[(&[Msg], &str)] = &[
    (&[Msg::Click], "klicken"),
    (&[Msg::Open(Screen::Upgrades)], "upgrades"),
    (&[Msg::Open(Screen::Settings)], "settings"),
    (&[Msg::Open(Screen::Help)], "diese hilfe"),
    (&[Msg::CollectEvent], "event einsammeln"),
    (&[Msg::Move(-1), Msg::Move(1)], "auswählen"),
    (
        &[Msg::AdjustSelected(-1), Msg::AdjustSelected(1)],
        "wert ändern",
    ),
    (&[Msg::Confirm], "kaufen / umschalten"),
    (&[Msg::Back], "zurück"),
    (&[Msg::Quit], "speichern und beenden"),
    (&[Msg::Open(Screen::Admin)], "admin-panel"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_help_entry_has_a_key() {
        for (msgs, text) in HELP {
            assert!(!label(msgs).is_empty(), "{text}");
        }
        assert_eq!(label(&[Msg::Move(-1), Msg::Move(1)]), "↑↓");
    }
}
