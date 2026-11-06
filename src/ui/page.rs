//! Was jede Ansicht kann. Die App kennt nur dieses Interface.

use crate::game::GameEvent;
use crate::ui::msg::{Msg, Screen};
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use gpui::AnyElement;
use std::time::Instant;

pub trait Page {
    /// Kürzel für die Statuszeile (ohne „beenden“, das hängt die App an).
    fn hints(&self, s: &Session) -> Vec<Hint>;

    fn render(&self, s: &Session, v: &View) -> AnyElement;

    /// Eine Eingabe, die nicht global ist. Gibt zurück, wohin es danach geht.
    fn handle(&mut self, _msg: Msg, _s: &mut Session) -> Option<Screen> {
        None
    }

    /// Nach jeder Eingabe (`dt = 0`) und jedem Tick, solange die Ansicht offen ist.
    fn refresh(&mut self, _dt: f64, _s: &mut Session) {}

    /// Für jedes Spielereignis, egal welche Ansicht offen ist.
    fn on_event(&mut self, _event: &GameEvent, _s: &mut Session) {}

    /// Die Ansicht wird verlassen.
    fn leave(&mut self, _s: &mut Session) {}

    /// Braucht die Ansicht gerade weitere Frames (z. B. Tipp-Animation)?
    fn animating(&self, _s: &Session, _now: Instant) -> bool {
        false
    }

    /// Füllt die Fläche selbst, statt mittig als Kasten zu stehen.
    fn fills(&self) -> bool {
        false
    }
}
