//! Upgrade-Katalog: was es gibt, was es kostet, was es bewirkt.

use super::{CRIT_MULT, OFFLINE_CAP_SECS};
use crate::format::{self, NumberFormat, trim_float};

/// Was ein Upgrade pro Level bewirkt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// Passives Einkommen: +n Bytes pro Sekunde und Level.
    PerSecond(f64),
    /// Bytes pro Klick: +n pro Level (vor den Multiplikatoren).
    ClickAdd(f64),
    /// Multiplikator pro Klick: ×n pro Level (multiplikativ).
    ClickMul(f64),
    /// Multiplikator auf alles passive Einkommen: ×n pro Level (multiplikativ).
    PassiveMul(f64),
    /// Chance auf einen kritischen Klick (×[`super::CRIT_MULT`]): +n (0.02 = 2 %) pro Level.
    Crit(f64),
    /// Jeder Klick bringt zusätzlich n Sekunden passives Einkommen pro Level.
    ClickFromPassive(f64),
    /// Events kommen öfter und halten länger (siehe `Game::event_interval_multiplier`).
    EventBoost,
    /// Verdient offline weiter: +n (0.1 = 10 %) des passiven Einkommens pro Level.
    Offline(f64),
    /// Alle Upgrades werden billiger: Kosten ×n pro Level.
    Discount(f64),
}

impl Effect {
    /// Was ein Level bewirkt, z. B. `+450 B/s`.
    pub fn label(self, f: NumberFormat) -> String {
        match self {
            Effect::PerSecond(n) => format!("+{}", format::rate(n, f)),
            Effect::ClickAdd(n) => format!("+{}/klick", format::bytes(n, f)),
            Effect::ClickMul(m) => format!("×{} B/klick", trim_float(m)),
            Effect::PassiveMul(m) => format!("×{} B/s gesamt", trim_float(m)),
            Effect::Crit(c) => format!("+{} % krit-chance", trim_float(c * 100.)),
            Effect::ClickFromPassive(secs) => format!("klick +{} s B/s", trim_float(secs)),
            Effect::EventBoost => "events öfter & länger".into(),
            Effect::Offline(f) => format!("+{} % offline", trim_float(f * 100.)),
            Effect::Discount(m) => format!("-{} % kosten", trim_float((1. - m) * 100.)),
        }
    }

    /// Kurzform und Erklärung der Effekt-Art (für die Hilfe-Seite).
    pub fn kind_help(self) -> (&'static str, String) {
        match self {
            Effect::PerSecond(_) => ("+B/s", "bytes von allein, jede sekunde".into()),
            Effect::ClickAdd(_) => ("+B/klick", "mehr pro klick".into()),
            Effect::ClickMul(_) => ("×klick", "klickwert vervielfachen".into()),
            Effect::PassiveMul(_) => ("×B/s", "alles passive vervielfachen".into()),
            Effect::Crit(_) => ("krit", format!("chance auf klick ×{CRIT_MULT}")),
            Effect::ClickFromPassive(_) => ("klick+s", "jeder klick bringt sekunden B/s".into()),
            Effect::EventBoost => ("events", "kommen öfter, halten länger".into()),
            Effect::Offline(_) => (
                "offline",
                format!(
                    "verdient weiter, wenn's zu ist (max. {} h)",
                    OFFLINE_CAP_SECS / 3600.
                ),
            ),
            Effect::Discount(_) => ("-kosten", "alle upgrades billiger".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UpgradeDef {
    pub id: &'static str,
    pub base_cost: f64,
    /// Kostenfaktor pro Level.
    pub growth: f64,
    /// Freigeschaltet, sobald insgesamt so viele Bytes verdient wurden.
    pub unlock_at: f64,
    pub effect: Effect,
    pub about: &'static str,
}

/// Alle Upgrades in Anzeigereihenfolge, sortiert nach `unlock_at`. Der Spielstand
/// speichert Level per `id`, deshalb bleiben alte Saves beim Anhängen gültig.
pub const UPGRADES: &[UpgradeDef] = &[
    up(
        "auto_click.sh",
        15.0,
        1.15,
        0.0,
        Effect::PerSecond(1.0),
        "ein shell-skript, das für dich klickt.",
    ),
    up(
        "macro.kbd",
        40.0,
        1.25,
        20.0,
        Effect::ClickAdd(1.0),
        "ein tastatur-makro. jeder klick zählt etwas mehr.",
    ),
    up(
        "double.bin",
        100.0,
        3.0,
        50.0,
        Effect::ClickMul(2.0),
        "verdoppelt die bytes pro klick.",
    ),
    up(
        "cron.d",
        500.0,
        1.15,
        400.0,
        Effect::PerSecond(10.0),
        "geplante jobs, die nebenbei bytes schaufeln.",
    ),
    up(
        "lucky.bit",
        1_200.0,
        2.2,
        1_000.0,
        Effect::Crit(0.02),
        "manchmal kippt ein bit richtig: kritischer klick.",
    ),
    up(
        "gzip.d",
        2_500.0,
        3.0,
        2_000.0,
        Effect::PassiveMul(1.5),
        "komprimiert alles passive einkommen.",
    ),
    up(
        "fork_daemon",
        6_000.0,
        1.15,
        5_000.0,
        Effect::PerSecond(80.0),
        "forkt sich selbst. sehr effizient. sehr laut.",
    ),
    up(
        "notify.d",
        9_000.0,
        3.0,
        7_500.0,
        Effect::EventBoost,
        "lauscht auf events: kommen öfter, halten länger.",
    ),
    up(
        "hotkey.sh",
        12_000.0,
        1.3,
        10_000.0,
        Effect::ClickAdd(25.0),
        "globale hotkeys für noch mehr bytes pro klick.",
    ),
    up(
        "systemd.service",
        25_000.0,
        2.5,
        20_000.0,
        Effect::Offline(0.1),
        "läuft weiter, auch wenn das spiel zu ist.",
    ),
    up(
        "mmap.so",
        40_000.0,
        1.15,
        30_000.0,
        Effect::PerSecond(450.0),
        "blendet fremden speicher ein. bytes fließen einfach so.",
    ),
    up(
        "overclock.cfg",
        100_000.0,
        5.0,
        80_000.0,
        Effect::ClickMul(3.0),
        "taktet den klick-finger hoch.",
    ),
    up(
        "pipe.sh",
        250_000.0,
        2.5,
        200_000.0,
        Effect::ClickFromPassive(0.5),
        "leitet passives einkommen in jeden klick um.",
    ),
    up(
        "kernel.ko",
        500_000.0,
        1.15,
        400_000.0,
        Effect::PerSecond(2_500.0),
        "läuft im ring 0. niemand hält es auf.",
    ),
    up(
        "compiler.o3",
        1.5e6,
        4.0,
        1.2e6,
        Effect::Discount(0.95),
        "optimiert alles: alle upgrades werden billiger.",
    ),
    up(
        "botnet.py",
        6e6,
        1.15,
        5e6,
        Effect::PerSecond(18_000.0),
        "viele kleine rechner klicken für dich.",
    ),
    up(
        "cluster.yaml",
        8e7,
        1.15,
        6e7,
        Effect::PerSecond(140_000.0),
        "ein ganzer cluster. kubernetes inklusive.",
    ),
    up(
        "neural.link",
        3e8,
        8.0,
        2.5e8,
        Effect::ClickMul(5.0),
        "der finger ist jetzt direkt am kernel.",
    ),
    up(
        "quantum.qpu",
        1e9,
        1.15,
        8e8,
        Effect::PerSecond(1.2e6),
        "klickt in allen zuständen gleichzeitig.",
    ),
    up(
        "ai.model",
        4e9,
        6.0,
        3e9,
        Effect::PassiveMul(2.0),
        "ein modell, das bytes vorhersagt.",
    ),
    up(
        "dyson.sphere",
        1.5e10,
        1.15,
        1.2e10,
        Effect::PerSecond(9e6),
        "eine sonne als netzteil.",
    ),
    up(
        "simulation.exe",
        2e11,
        1.15,
        1.5e11,
        Effect::PerSecond(7e7),
        "simuliert ein universum, das für dich klickt.",
    ),
    up(
        "multiverse.fork",
        3e12,
        1.15,
        2.5e12,
        Effect::PerSecond(5e8),
        "fork() auf realitätsebene.",
    ),
    up(
        "blackhole.dll",
        5e13,
        1.15,
        4e13,
        Effect::PerSecond(4e9),
        "unendliche dichte, unendlicher speicher.",
    ),
    up(
        "heat_death.sh",
        1e15,
        1.15,
        8e14,
        Effect::PerSecond(3.5e10),
        "läuft bis zum wärmetod. und danach.",
    ),
    up(
        "big_bang.iso",
        2e16,
        1.15,
        1.6e16,
        Effect::PerSecond(3e11),
        "startet das universum neu. mit mehr bytes.",
    ),
    up(
        "singularity.ai",
        1e17,
        10.0,
        8e16,
        Effect::PassiveMul(3.0),
        "denkt schneller als du klickst.",
    ),
    up(
        "omega.point",
        5e17,
        1.15,
        4e17,
        Effect::PerSecond(2.8e12),
        "jede information im universum, gleichzeitig.",
    ),
    up(
        "infinity.loop",
        1e19,
        1.15,
        8e18,
        Effect::PerSecond(2.5e13),
        "while(true) { bytes++; }",
    ),
    up(
        "beyond.null",
        3e20,
        1.15,
        2.4e20,
        Effect::PerSecond(2.2e14),
        "was nach dem ende kommt. ziemlich viel.",
    ),
];

const fn up(
    id: &'static str,
    base_cost: f64,
    growth: f64,
    unlock_at: f64,
    effect: Effect,
    about: &'static str,
) -> UpgradeDef {
    UpgradeDef {
        id,
        base_cost,
        growth,
        unlock_at,
        effect,
        about,
    }
}
