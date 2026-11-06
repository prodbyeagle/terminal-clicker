//! Gemeinsame Bedienung aller Listen-Ansichten (Upgrades, Settings, Admin).

use crate::ui::msg::Msg;
use crate::ui::session::Session;

pub trait ListScreen {
    fn len(&self) -> usize;
    fn selected(&self) -> usize;
    fn set_selected(&mut self, index: usize);
    /// Klick, `enter` oder `space` auf einem Eintrag.
    fn activate(&mut self, index: usize, s: &mut Session);
    /// `←`/`→`, Rechtsklick oder Pfeil im Eintrag.
    fn adjust(&mut self, _index: usize, _delta: i32, _s: &mut Session) {}
}

/// `delta` Schritte weiter in `all`, mit Umlauf. Ist `current` nicht in der Liste
/// (z. B. ein entferntes Theme), geht es vom ersten Eintrag aus weiter.
pub fn cycle<T: Copy + PartialEq>(all: &[T], current: T, delta: i32) -> T {
    let i = all.iter().position(|v| *v == current).unwrap_or(0) as i32;
    all[(i + delta).rem_euclid(all.len() as i32) as usize]
}

pub fn handle(list: &mut dyn ListScreen, msg: Msg, s: &mut Session) {
    match msg {
        Msg::Move(delta) => {
            let len = list.len() as i32;
            list.set_selected((list.selected() as i32 + delta).rem_euclid(len) as usize);
        }
        Msg::Select(i) => {
            if i != list.selected() {
                list.set_selected(i);
            }
        }
        Msg::Activate(i) => {
            list.set_selected(i);
            list.activate(i, s);
        }
        Msg::Adjust(i, delta) => {
            list.set_selected(i);
            list.adjust(i, delta, s);
        }
        Msg::AdjustSelected(delta) => list.adjust(list.selected(), delta, s),
        // `space` aktiviert in Listen wie `enter` (kaufen, umschalten).
        Msg::Click | Msg::Confirm => list.activate(list.selected(), s),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_both_ways() {
        assert_eq!(cycle(&[1, 2, 3], 3, 1), 1);
        assert_eq!(cycle(&[1, 2, 3], 1, -1), 3);
    }
}
