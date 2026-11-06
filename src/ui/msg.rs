//! Alles, was der Spieler tun kann, als Nachricht. Tasten, Maus, Reiter und
//! Statuszeile erzeugen dieselben `Msg`s; nur die App verarbeitet sie.

use gpui::{App, MouseDownEvent, Window};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Boot,
    Main,
    Upgrades,
    Settings,
    Help,
    Admin,
}

impl Screen {
    /// Alle Ansichten; ohne Boot in dieser Reihenfolge als Reiter.
    pub const ALL: [Screen; 6] = [
        Screen::Boot,
        Screen::Main,
        Screen::Upgrades,
        Screen::Settings,
        Screen::Help,
        Screen::Admin,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Screen::Boot => "boot",
            Screen::Main => "main",
            Screen::Upgrades => "upgrades",
            Screen::Settings => "settings",
            Screen::Help => "hilfe",
            Screen::Admin => "admin",
        }
    }

    /// Admin nur mit `--admin`.
    pub fn available(self, admin: bool) -> bool {
        self != Screen::Admin || admin
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Msg {
    Open(Screen),
    Back,
    Quit,
    Click,
    Confirm,
    CollectEvent,
    /// Auswahl in der aktuellen Liste verschieben.
    Move(i32),
    /// Wert des ausgewählten Eintrags ändern (`←`/`→`).
    AdjustSelected(i32),
    /// Maus über Listeneintrag.
    Select(usize),
    /// Klick auf Listeneintrag.
    Activate(usize),
    /// Rechtsklick (−1) oder Pfeil im Eintrag.
    Adjust(usize, i32),
    PickTheme(&'static str),
}

/// Schickt Nachrichten an die App. Views bauen damit ihre Listener,
/// ohne die App selbst zu kennen.
#[derive(Clone)]
pub struct Emit(Rc<Send>);

type Send = dyn Fn(Msg, &mut App);

impl Emit {
    pub fn new(send: impl Fn(Msg, &mut App) + 'static) -> Self {
        Self(Rc::new(send))
    }

    pub fn send(&self, msg: Msg, cx: &mut App) {
        (self.0)(msg, cx);
    }

    /// Mausklick-Listener. Stoppt die Weitergabe, damit z. B. ein Reiter das
    /// Fenster nicht zusätzlich zieht.
    pub fn on_press(&self, msg: Msg) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        let send = self.0.clone();
        move |_, _, cx| {
            cx.stop_propagation();
            send(msg, cx);
        }
    }

    pub fn on_hover(&self, msg: Msg) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        let send = self.0.clone();
        move |hovered, _, cx| {
            if *hovered {
                send(msg, cx);
            }
        }
    }
}
