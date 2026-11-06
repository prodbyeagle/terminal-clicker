//! Die Effekte der Oberfläche, die der Animator abspielt.

use crate::ascii::anim::{self, AnimKey};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fx {
    /// `+N` steigt über dem Button auf. `x` ist der horizontale Versatz in px.
    Float {
        x: f32,
        crit: bool,
    },
    ButtonPress,
    ButtonPulse,
    /// Upgrade-Zeile (Index in `UPGRADES`) blinkt nach dem Kauf.
    RowBlink(usize),
    /// Zähler-Ziffern beim Meilenstein.
    Scramble,
    /// Datenpaket eines passiven Upgrades mit seinem Betrag; `x`/`y` als Anteil der Hauptfläche.
    Packet {
        source: usize,
        value: f64,
        x: f32,
        y: f32,
    },
}

impl Fx {
    pub fn is_packet(&self) -> bool {
        matches!(self, Fx::Packet { .. })
    }
}

impl AnimKey for Fx {
    fn stacks(&self) -> bool {
        matches!(self, Fx::Float { .. } | Fx::Packet { .. })
    }
}

pub type Animator = anim::Animator<Fx>;
