//! Admin-Panel (nur mit `--admin`): Bytes und Upgrades geben, Events auslösen,
//! Zeitraffer, Reset – plus Stats.

use crate::format::{self, bar, grouped, trim_float};
use crate::game::UPGRADES;
use crate::game::events::EventKind;
use crate::theme::Theme;
use crate::ui::fx::Fx;
use crate::ui::keys;
use crate::ui::list::{self, ListScreen, cycle};
use crate::ui::msg::{Msg, Screen};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use crate::ui::widgets::{list_row, panel, separator};
use gpui::{AnyElement, Div, div, prelude::*, px};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Item {
    Grant(f64),
    Times10,
    UnlockAll,
    LevelAll,
    SpawnEvent,
    TimeWarp,
    Reset,
}

const ITEMS: [Item; 11] = [
    Item::Grant(1e3),
    Item::Grant(1e6),
    Item::Grant(1e9),
    Item::Grant(1e15),
    Item::Grant(1e21),
    Item::Times10,
    Item::UnlockAll,
    Item::LevelAll,
    Item::SpawnEvent,
    Item::TimeWarp,
    Item::Reset,
];

const TIME_SCALES: [f64; 4] = [1.0, 10.0, 100.0, 1000.0];
const RESET_CONFIRM_SECS: f64 = 4.0;

#[derive(Default)]
pub struct AdminScreen {
    selected: usize,
    /// `None` = zufälliges Event.
    event: Option<EventKind>,
    /// Sitzungszeit, zu der der Reset scharf geschaltet wurde.
    reset_armed_at: Option<f64>,
}

impl AdminScreen {
    fn reset_armed(&self, now: f64) -> bool {
        self.reset_armed_at
            .is_some_and(|t| now - t < RESET_CONFIRM_SECS)
    }

    fn event_choices() -> Vec<Option<EventKind>> {
        std::iter::once(None)
            .chain(EventKind::ALL.map(Some))
            .collect()
    }

    fn label(&self, item: Item, s: &Session) -> String {
        match item {
            Item::Grant(n) => format!("+{} B", grouped(n)),
            Item::Times10 => "bytes ×10".into(),
            Item::UnlockAll => "alle upgrades freischalten".into(),
            Item::LevelAll => "+1 level auf alle upgrades".into(),
            Item::SpawnEvent => {
                let name = self.event.map_or("zufällig".into(), |k| k.name());
                format!("event auslösen  <{name}>")
            }
            Item::TimeWarp => format!("zeitraffer  <×{}>", s.time_scale),
            Item::Reset if self.reset_armed(s.now()) => {
                format!(
                    "wirklich zurücksetzen? nochmal {}",
                    keys::label(&[Msg::Confirm])
                )
            }
            Item::Reset => "spielstand zurücksetzen".into(),
        }
    }

    fn render_stats(&self, s: &Session, v: &View) -> Div {
        let (theme, game) = (&v.theme, &s.game);
        let total_rate = game.per_second();
        let mut sources: Vec<(usize, f64)> = (0..UPGRADES.len())
            .map(|i| (i, game.source_rate(i)))
            .filter(|(_, r)| *r > 0.)
            .collect();
        sources.sort_by(|a, b| b.1.total_cmp(&a.1));

        panel(theme, "STATS")
            .w(px(420.))
            .flex_shrink()
            .min_w(px(0.))
            .gap(px(2.))
            .child(separator(theme))
            .child(stat(theme, "bytes", grouped(game.whole_bytes())))
            .child(stat(theme, "verdient gesamt", grouped(game.total_earned)))
            .child(stat(
                theme,
                "klicks gesamt",
                grouped(game.total_clicks as f64),
            ))
            .child(stat(theme, "B/klick", grouped(game.per_click())))
            .child(stat(
                theme,
                "klicks/s",
                format!("{:.1}", game.clicks_per_second(s.now())),
            ))
            .child(stat(theme, "B/s", trim_float(total_rate)))
            .child(stat(
                theme,
                "passiv-multiplikator",
                format!("×{}", trim_float(game.passive_multiplier())),
            ))
            .child(stat(
                theme,
                "upgrades gekauft",
                game.upgrades_bought().to_string(),
            ))
            .child(stat(
                theme,
                "freigeschaltet",
                format!("{}/{}", game.unlocked_count(), UPGRADES.len()),
            ))
            .child(stat(
                theme,
                "spielzeit",
                format::duration(game.played_seconds),
            ))
            .child(stat(theme, "sitzung", format::duration(s.now())))
            .child(stat(
                theme,
                "pakete aktiv",
                s.anim.count(v.now, Fx::is_packet).to_string(),
            ))
            .child(stat(theme, "zeitraffer", format!("×{}", s.time_scale)))
            .child(stat(
                theme,
                "krit-chance",
                format!("{} %", trim_float(game.crit_chance() * 100.)),
            ))
            .child(stat(
                theme,
                "offline-anteil",
                format!("{} %", trim_float(game.offline_fraction() * 100.)),
            ))
            .child(stat(
                theme,
                "kosten-faktor",
                format!("×{:.3}", game.cost_multiplier()),
            ))
            .child(stat(
                theme,
                "event-abstand",
                format!("×{:.2}", game.event_interval_multiplier()),
            ))
            .child(
                div()
                    .pt(px(12.))
                    .px(px(10.))
                    .text_color(theme.dim)
                    .child("einkommen nach quelle"),
            )
            .children(sources.into_iter().map(|(i, rate)| {
                div()
                    .flex()
                    .gap(px(10.))
                    .px(px(10.))
                    .child(
                        div()
                            .w(px(110.))
                            .flex_none()
                            .overflow_hidden()
                            .child(UPGRADES[i].id),
                    )
                    .child(
                        div()
                            .text_color(theme.dim)
                            .child(bar(rate / total_rate, 12)),
                    )
                    .child(
                        div()
                            .whitespace_nowrap()
                            .child(format!("{} B/s", trim_float(rate))),
                    )
            }))
    }
}

impl ListScreen for AdminScreen {
    fn len(&self) -> usize {
        ITEMS.len()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, index: usize) {
        if index != self.selected {
            self.reset_armed_at = None;
        }
        self.selected = index;
    }

    fn activate(&mut self, index: usize, s: &mut Session) {
        let now = s.now();
        let (game, out) = (&mut s.game, &mut s.happened);
        match ITEMS[index] {
            Item::Grant(n) => {
                game.admin_grant(n, out);
                s.log.event(format!("admin: +{} B", grouped(n)));
            }
            Item::Times10 => {
                let add = game.bytes * 9.;
                game.admin_grant(add, out);
                s.log.event("admin: bytes ×10");
            }
            Item::UnlockAll => {
                game.admin_unlock_all(out);
                s.log.event("admin: alles freigeschaltet");
            }
            Item::LevelAll => {
                game.admin_level_all(out);
                s.log.event("admin: +1 level auf alles");
            }
            Item::SpawnEvent => {
                let kind = self.event.unwrap_or_else(|| s.events.random_kind());
                s.events.spawn(kind, now);
                s.log.event("admin: event ausgelöst (in main)");
            }
            Item::TimeWarp => self.adjust(index, 1, s),
            Item::Reset if self.reset_armed(now) => {
                game.admin_reset(out);
                s.log.event("admin: spielstand zurückgesetzt");
            }
            Item::Reset => {
                self.reset_armed_at = Some(now);
                s.log.warn(format!(
                    "nochmal {} zum zurücksetzen",
                    keys::label(&[Msg::Confirm])
                ));
                return;
            }
        }
        self.reset_armed_at = None;
        s.save_game = true;
    }

    fn adjust(&mut self, index: usize, delta: i32, s: &mut Session) {
        match ITEMS[index] {
            Item::TimeWarp => {
                s.time_scale = cycle(&TIME_SCALES, s.time_scale, delta);
                s.log.event(format!("admin: zeitraffer ×{}", s.time_scale));
            }
            Item::SpawnEvent => self.event = cycle(&Self::event_choices(), self.event, delta),
            _ => {}
        }
    }
}

impl Page for AdminScreen {
    fn hints(&self, _s: &Session) -> Vec<Hint> {
        vec![
            Hint::info(&[Msg::Move(-1), Msg::Move(1)], "wählen"),
            Hint::new(Msg::Confirm, "ausführen"),
            Hint::info(
                &[Msg::AdjustSelected(-1), Msg::AdjustSelected(1)],
                "auswahl",
            ),
            Hint::new(Msg::Back, "zurück"),
        ]
    }

    fn handle(&mut self, msg: Msg, s: &mut Session) -> Option<Screen> {
        list::handle(self, msg, s);
        None
    }

    fn render(&self, s: &Session, v: &View) -> AnyElement {
        let theme = &v.theme;
        let actions = ITEMS.iter().enumerate().map(|(i, item)| {
            let is_sel = i == self.selected;
            list_row("admin", i, is_sel, is_sel, v)
                .py(px(5.))
                .when(*item == Item::Reset && !is_sel, |d| d.text_color(theme.dim))
                .child(self.label(*item, s))
        });
        let footer = format!(
            "# {}/klick = ausführen · {} = auswahl ändern",
            keys::label(&[Msg::Confirm]),
            keys::label(&[Msg::AdjustSelected(-1), Msg::AdjustSelected(1)])
        );

        div()
            .flex()
            .gap(px(16.))
            .items_start()
            .max_w_full()
            .child(
                panel(theme, "ADMIN")
                    .w(px(420.))
                    .flex_shrink()
                    .min_w(px(0.))
                    .gap(px(2.))
                    .child(separator(theme))
                    .children(actions)
                    .child(div().pt(px(12.)).text_color(theme.dim).child(footer)),
            )
            .child(self.render_stats(s, v))
            .into_any_element()
    }
}

fn stat(theme: &Theme, name: &str, value: String) -> Div {
    div()
        .flex()
        .justify_between()
        .gap(px(12.))
        .px(px(10.))
        .child(div().text_color(theme.dim).child(name.to_string()))
        .child(div().whitespace_nowrap().child(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::settings::Settings;
    use crate::ui::log::Log;

    fn session_with_bytes() -> Session {
        let mut s = Session::new(Game::new(), Settings::default(), Log::default(), true, 0);
        s.game.bytes = 500.0;
        s
    }

    fn reset_index() -> usize {
        ITEMS.iter().position(|i| *i == Item::Reset).unwrap()
    }

    #[test]
    fn reset_needs_a_second_confirmation() {
        let mut s = session_with_bytes();
        let mut admin = AdminScreen::default();
        list::handle(&mut admin, Msg::Activate(reset_index()), &mut s);
        assert_eq!(s.game.bytes, 500.0);
        list::handle(&mut admin, Msg::Activate(reset_index()), &mut s);
        assert_eq!(s.game.bytes, 0.0);
    }

    #[test]
    fn moving_away_disarms_the_reset() {
        let mut s = session_with_bytes();
        let mut admin = AdminScreen::default();
        list::handle(&mut admin, Msg::Activate(reset_index()), &mut s);
        list::handle(&mut admin, Msg::Move(-1), &mut s);
        list::handle(&mut admin, Msg::Activate(reset_index()), &mut s);
        assert_eq!(s.game.bytes, 500.0);
    }
}
