//! Upgrade-Ansicht: Tabelle und Kaufen.

use crate::format;
use crate::game::{BuyError, GameEvent, UPGRADES};
use crate::ui::fx::Fx;
use crate::ui::list::{self, ListScreen};
use crate::ui::msg::{Msg, Screen};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use crate::ui::widgets::{list_row, panel};
use gpui::{AnyElement, Div, ScrollHandle, div, prelude::*, px, transparent_black};
use std::time::Instant;

const W_NAME: f32 = 200.;
const W_LVL: f32 = 70.;
const W_COST: f32 = 150.;
/// Abstand links, damit die Spalten mit dem `>`-Marker der Zeilen fluchten.
const W_MARK: f32 = 24.;

fn cell(w: f32) -> Div {
    div()
        .w(px(w))
        .flex_none()
        .whitespace_nowrap()
        .overflow_hidden()
}

#[derive(Default)]
pub struct UpgradesScreen {
    selected: usize,
    scroll: ScrollHandle,
}

impl ListScreen for UpgradesScreen {
    fn len(&self) -> usize {
        UPGRADES.len()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, index: usize) {
        self.selected = index;
        self.scroll.scroll_to_item(index);
    }

    fn activate(&mut self, index: usize, s: &mut Session) {
        match s.game.buy(index, &mut s.happened) {
            Ok(_) => s.save_game = true,
            Err(BuyError::Locked) => s.log.warn("gesperrt"),
            Err(BuyError::TooExpensive { cost }) => s.log.warn(format!(
                "zu teuer: {} nötig",
                format::bytes_padded(cost, s.settings.number_format)
            )),
        }
    }
}

impl Page for UpgradesScreen {
    fn hints(&self, _s: &Session) -> Vec<Hint> {
        vec![
            Hint::info(&[Msg::Move(-1), Msg::Move(1)], "wählen"),
            Hint::new(Msg::Confirm, "kaufen"),
            Hint::open(Screen::Settings),
            Hint::new(Msg::Back, "zurück"),
        ]
    }

    fn handle(&mut self, msg: Msg, s: &mut Session) -> Option<Screen> {
        list::handle(self, msg, s);
        None
    }

    fn on_event(&mut self, event: &GameEvent, s: &mut Session) {
        if let GameEvent::Bought { index, .. } = event
            && s.settings.animations
        {
            s.anim.blink(Fx::RowBlink(*index), 4, 70, Instant::now());
        }
    }

    fn render(&self, s: &Session, v: &View) -> AnyElement {
        let (theme, game, fmt) = (&v.theme, &s.game, s.settings.number_format);
        // Nur das nächste gesperrte Upgrade ist schon lesbar, alles danach bleibt „???“.
        let next_locked = (0..UPGRADES.len()).find(|&i| !game.is_unlocked(i));
        let header = div()
            .flex()
            .px(px(10.))
            .py(px(6.))
            .text_color(theme.dim)
            .border_b_1()
            .border_color(theme.line)
            .child(cell(W_MARK))
            .child(cell(W_NAME).child("NAME"))
            .child(cell(W_LVL).child("LVL"))
            .child(cell(W_COST).child("KOSTEN"))
            .child(div().flex_1().child("EFFEKT"));

        let rows = UPGRADES.iter().enumerate().map(|(i, def)| {
            let unlocked = game.is_unlocked(i);
            let is_sel = i == self.selected;
            let inverted = match s.anim.find(Fx::RowBlink(i), v.now) {
                Some(blink) => blink.blink_on(v.now),
                None => is_sel && unlocked,
            };
            let muted = if inverted { theme.bg } else { theme.dim };
            let row = list_row("upgrade", i, is_sel, inverted, v)
                .py(px(5.))
                .border_1();

            if unlocked {
                row.border_color(transparent_black())
                    .child(cell(W_NAME).child(def.id))
                    .child(cell(W_LVL).child(format!("{:03}", game.level(i))))
                    .child(
                        cell(W_COST)
                            .when(!game.can_afford(i), |d| d.text_color(muted))
                            .child(format::bytes_padded(game.cost(i), fmt)),
                    )
                    .child(div().flex_1().child(def.effect.label(fmt)))
            } else {
                let known = next_locked == Some(i);
                let name = if known {
                    def.id.to_string()
                } else {
                    "?".repeat(def.id.len())
                };
                let unlock = if known {
                    format!("ab {}", format::bytes(def.unlock_at, fmt))
                } else {
                    "ab ??? B".to_string()
                };
                row.border_dashed()
                    .border_color(theme.dim)
                    .text_color(theme.dim)
                    .child(cell(W_NAME).child(name))
                    .child(cell(W_LVL).child("---"))
                    .child(cell(W_COST).child(unlock))
                    .child(div().flex_1().child("gesperrt"))
            }
        });

        let sel_def = &UPGRADES[self.selected];
        let info = if game.is_unlocked(self.selected) {
            sel_def.about.to_string()
        } else if next_locked == Some(self.selected) {
            format!(
                "freischalten: insgesamt {} verdienen.",
                format::bytes(sel_def.unlock_at, fmt)
            )
        } else {
            "noch unbekannt.".to_string()
        };

        panel(theme, "UPGRADES")
            .w(px(680.))
            .max_w_full()
            .max_h_full()
            .child(header)
            .child(
                div()
                    .id("upgrade-rows")
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .flex_shrink()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(rows),
            )
            .child(
                div()
                    .pt(px(12.))
                    .text_color(theme.dim)
                    .child(format!("# {info}  (klick = kaufen)")),
            )
            .into_any_element()
    }
}
