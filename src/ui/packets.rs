//! Datenpakete: macht passives Einkommen sichtbar. Jedes aktive Upgrade
//! schickt in seinem eigenen Takt ein Paket mit dem seitdem verdienten Betrag.

use crate::ascii::glitch::decode_text;
use crate::format;
use crate::game::UPGRADES;
use crate::ui::fx::Fx;
use crate::ui::session::Session;
use crate::ui::view::View;
use gpui::{Div, div, prelude::*, px, relative};
use std::time::Instant;

const LIFETIME_MS: u64 = 1800;
/// Höchstens so viele Pakete zugleich; was darüber hinausgeht, kommt ins
/// nächste Paket desselben Upgrades – so bleibt es auch bei riesigem Einkommen ruhig.
const MAX_ON_SCREEN: usize = 12;
const RISE_PX: f32 = 40.;

pub struct Packets {
    earned: Vec<f64>,
    waited: Vec<f64>,
}

impl Default for Packets {
    fn default() -> Self {
        Self {
            earned: vec![0.; UPGRADES.len()],
            waited: vec![0.; UPGRADES.len()],
        }
    }
}

/// Mehr Level = schnellerer Takt, aber nie schneller als alle 0,6 s.
fn interval(level: u32) -> f64 {
    (2.4 / (1. + level as f64).sqrt()).max(0.6)
}

impl Packets {
    /// Sammelt das Einkommen des letzten Ticks (`s.income`) und schickt fällige
    /// Pakete los. Ohne Animationen wird nichts aufgestaut, sonst käme später ein Riesenpaket.
    pub fn tick(&mut self, dt: f64, s: &mut Session) {
        let now = Instant::now();
        let dt = dt * s.time_scale;
        let mut on_screen = s.anim.count(now, Fx::is_packet);
        for (i, earned) in std::mem::take(&mut s.income) {
            self.earned[i] += earned;
            self.waited[i] += dt;
            if self.waited[i] < interval(s.game.level(i)) {
                continue;
            }
            if !s.settings.animations {
                self.earned[i] = 0.;
                self.waited[i] = 0.;
                continue;
            }
            if on_screen >= MAX_ON_SCREEN || self.earned[i] < 1. {
                continue;
            }
            let value = self.earned[i].floor();
            self.earned[i] -= value;
            self.waited[i] = 0.;
            on_screen += 1;
            launch(s, i, value, now);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Links oder rechts vom Zähler, nie mitten drauf.
fn launch(s: &mut Session, source: usize, value: f64, now: Instant) {
    let rng = &mut s.rng;
    let x = if rng.chance(0.5) {
        rng.range(0.03, 0.18)
    } else {
        rng.range(0.72, 0.86)
    };
    let y = rng.range(0.12, 0.9);
    let label = format!("+{}", format::bytes(value, s.settings.number_format));
    s.anim.play(
        Fx::Packet {
            source,
            value,
            x,
            y,
        },
        vec![label],
        LIFETIME_MS,
        now,
    );
}

/// Pakete tauchen aus ASCII-Rauschen auf, steigen langsam und verblassen.
pub fn render(s: &Session, v: &View) -> Vec<Div> {
    s.anim
        .iter(v.now)
        .filter_map(|a| {
            let Fx::Packet { source, x, y, .. } = a.kind else {
                return None;
            };
            let p = a.progress(v.now);
            let decode = (p / 0.22).min(1.);
            let opacity = if p < 0.6 { 1. } else { (1. - p) / 0.4 };
            Some(
                div()
                    .absolute()
                    .left(relative(x))
                    .top(relative(y))
                    .mt(px(-p * RISE_PX))
                    .flex()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .whitespace_nowrap()
                    .opacity(opacity)
                    .child(div().text_color(v.theme.fg).child(decode_text(
                        a.frames.first()?,
                        decode,
                        a.age(v.now),
                    )))
                    .child(div().text_color(v.theme.dim).child(decode_text(
                        UPGRADES[source].id,
                        decode,
                        a.age(v.now),
                    ))),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::settings::Settings;
    use crate::ui::log::Log;

    fn session(animations: bool) -> Session {
        let mut game = Game::new();
        game.bytes = 15.0;
        game.buy(0, &mut Vec::new()).unwrap();
        let settings = Settings {
            animations,
            ..Settings::default()
        };
        Session::new(game, settings, Log::default(), false, 0)
    }

    /// Ein Spieltick wie in der App: Einkommen buchen, dann Pakete.
    fn run(p: &mut Packets, s: &mut Session, ticks: usize) -> f64 {
        let mut earned = 0.;
        for _ in 0..ticks {
            s.income = s.game.tick(0.1, s.time_scale, 0., &mut Vec::new());
            earned += s.income.iter().map(|(_, e)| e).sum::<f64>();
            p.tick(0.1, s);
        }
        earned
    }

    fn sent(s: &Session) -> Vec<f64> {
        s.anim
            .iter(Instant::now())
            .filter_map(|a| match a.kind {
                Fx::Packet { value, .. } => Some(value),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn packets_carry_the_real_income_up_to_a_small_rest() {
        let mut s = session(true);
        let mut p = Packets::default();
        let earned = run(&mut p, &mut s, 40);
        let total: f64 = sent(&s).iter().sum();
        assert!(total > 0. && total <= earned, "{total} von {earned}");
        let longest_wait = interval(0);
        assert!(
            earned - total <= s.game.per_second() * longest_wait + 1.,
            "{total} von {earned}"
        );
    }

    #[test]
    fn nothing_piles_up_without_animations() {
        let mut s = session(false);
        let mut p = Packets::default();
        run(&mut p, &mut s, 100);
        assert!(sent(&s).is_empty());
        s.settings.animations = true;
        run(&mut p, &mut s, 30);
        let first = sent(&s)[0];
        assert!(first <= s.game.per_second() * interval(0) + 1., "{first}");
    }

    #[test]
    fn respects_the_screen_limit() {
        let mut s = session(true);
        s.game.admin_level_all(&mut Vec::new());
        let mut p = Packets::default();
        run(&mut p, &mut s, 200);
        assert!(s.anim.count(Instant::now(), Fx::is_packet) <= MAX_ON_SCREEN);
    }
}
