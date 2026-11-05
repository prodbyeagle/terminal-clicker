use crate::format::{self, NumberFormat};
use crate::game::{Effect, Game, UPGRADES};
use crate::theme::Theme;
use crate::ui::main_view::ClickerApp;
use gpui::{Context, Div, MouseButton, ScrollHandle, div, prelude::*, px, transparent_black};

fn description(id: &str) -> &'static str {
    match id {
        "auto_click.sh" => "ein shell-skript, das für dich klickt.",
        "double.bin" => "verdoppelt die bytes pro klick. pro level.",
        "cron.d" => "geplante jobs, die nebenbei bytes schaufeln.",
        "fork_daemon" => "forkt sich selbst. sehr effizient. sehr laut.",
        "macro.kbd" => "ein tastatur-makro. jeder klick zählt etwas mehr.",
        "gzip.d" => "komprimiert alles passive einkommen. pro level ×1.5.",
        "hotkey.sh" => "globale hotkeys. +25 bytes pro klick und level.",
        "mmap.so" => "blendet fremden speicher ein. bytes fließen einfach so.",
        "overclock.cfg" => "taktet den klick-finger hoch. ×3 pro level.",
        "kernel.ko" => "läuft im ring 0. niemand hält es auf.",
        "botnet.py" => "viele kleine rechner klicken für dich.",
        "cluster.yaml" => "ein ganzer cluster. kubernetes inklusive.",
        "quantum.qpu" => "klickt in allen zuständen gleichzeitig.",
        "lucky.bit" => "manchmal kippt ein bit richtig: kritischer klick ×10.",
        "notify.d" => "lauscht auf events: kommen öfter, halten länger.",
        "systemd.service" => "läuft weiter, auch wenn das spiel zu ist (max. 8 h).",
        "pipe.sh" => "leitet passives einkommen in jeden klick um.",
        "compiler.o3" => "optimiert alles: alle upgrades werden billiger.",
        "neural.link" => "der finger ist jetzt direkt am kernel. ×5 pro level.",
        "ai.model" => "ein modell, das bytes vorhersagt. ×2 auf alles passive.",
        "dyson.sphere" => "eine sonne als netzteil.",
        "simulation.exe" => "simuliert ein universum, das für dich klickt.",
        "multiverse.fork" => "fork() auf realitätsebene.",
        "blackhole.dll" => "unendliche dichte, unendlicher speicher.",
        "heat_death.sh" => "läuft bis zum wärmetod. und danach.",
        "big_bang.iso" => "startet das universum neu. mit mehr bytes.",
        "singularity.ai" => "denkt schneller als du klickst. ×3 auf alles passive.",
        "omega.point" => "jede information im universum, gleichzeitig.",
        "infinity.loop" => "while(true) { bytes++; }",
        "beyond.null" => "was nach dem ende kommt. ziemlich viel.",
        _ => "",
    }
}

pub use crate::format::trim_float;

pub fn effect_label(effect: Effect, f: NumberFormat) -> String {
    match effect {
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

const W_MARK: f32 = 24.;
const W_NAME: f32 = 200.;
const W_LVL: f32 = 70.;
const W_COST: f32 = 150.;

fn cell(w: f32) -> Div {
    div().w(px(w)).flex_none().whitespace_nowrap().overflow_hidden()
}

/// Upgrade-Tabelle. `blink` überschreibt während der Kauf-Animation die Invertierung.
/// Maus: Zeile überfahren = auswählen, klicken = kaufen.
pub fn render_upgrades(
    game: &Game,
    selected: usize,
    blink: Option<(usize, bool)>,
    theme: &Theme,
    scroll: &ScrollHandle,
    fmt: NumberFormat,
    cx: &mut Context<ClickerApp>,
) -> Div {
    // Nur das nächste gesperrte Upgrade ist schon lesbar, alles danach bleibt „???“.
    let next_locked = (0..UPGRADES.len()).find(|&i| !game.is_unlocked(i));
    let header = div()
        .flex()
        .px(px(10.))
        .py(px(6.))
        .text_color(theme.dim)
        .border_b_1()
        .border_color(theme.line)
        .child(cell(W_MARK).child(""))
        .child(cell(W_NAME).child("NAME"))
        .child(cell(W_LVL).child("LVL"))
        .child(cell(W_COST).child("KOSTEN"))
        .child(div().flex_1().child("EFFEKT"));

    let rows = UPGRADES.iter().enumerate().map(|(i, def)| {
        let unlocked = game.is_unlocked(i);
        let is_sel = i == selected;
        // Während des Kauf-Blinkens bestimmt die Animation die Invertierung.
        let inverted = match blink {
            Some((row, on)) if row == i => on,
            _ => is_sel && unlocked,
        };
        let cost = game.cost(i);
        let affordable = game.bytes >= cost;

        let (fg, bg) = if inverted { (theme.bg, theme.fg) } else { (theme.fg, theme.bg) };
        let muted = if inverted { theme.bg } else { theme.dim };

        let mut row = div()
            .id(("upgrade", i))
            .flex()
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    this.hover_upgrade(i, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.click_upgrade(i, cx)),
            )
            .flex_none()
            .px(px(10.))
            .py(px(5.))
            .border_1()
            .bg(bg)
            .text_color(fg);

        if unlocked {
            row = row
                .border_color(transparent_black())
                .child(cell(W_MARK).child(if is_sel { ">" } else { "" }))
                .child(cell(W_NAME).child(def.id))
                .child(cell(W_LVL).child(format!("{:03}", game.level(i))))
                .child(
                    cell(W_COST)
                        .when(!affordable, |d| d.text_color(muted))
                        .child(format::bytes_padded(cost, fmt)),
                )
                .child(div().flex_1().child(effect_label(def.effect, fmt)));
        } else {
            // Gesperrt: gestrichelt und grau. Nur das nächste verrät schon etwas.
            let known = next_locked == Some(i);
            let name = if known { def.id.to_string() } else { "?".repeat(def.id.len()) };
            row = row
                .border_dashed()
                .border_color(theme.dim)
                .text_color(theme.dim)
                .child(cell(W_MARK).child(if is_sel { ">" } else { "" }))
                .child(cell(W_NAME).child(name))
                .child(cell(W_LVL).child("---"))
                .child(cell(W_COST).child(if known {
                    format!("ab {}", format::bytes(def.unlock_at, fmt))
                } else {
                    "ab ??? B".to_string()
                }))
                .child(div().flex_1().child("gesperrt"));
        }
        row
    });

    let sel_def = &UPGRADES[selected.min(UPGRADES.len() - 1)];
    let info = if game.is_unlocked(selected) {
        description(sel_def.id).to_string()
    } else if next_locked == Some(selected) {
        format!("freischalten: insgesamt {} verdienen.", format::bytes(sel_def.unlock_at, fmt))
    } else {
        "noch unbekannt.".to_string()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .w(px(680.))
        .max_w_full()
        .max_h_full()
        .p(px(16.))
        .bg(theme.bg)
        .border_1()
        .border_color(theme.line)
        .child(div().pb(px(10.)).child("UPGRADES"))
        .child(header)
        // Liste scrollt, wenn das Fenster zu niedrig ist; die Auswahl wird mitgeführt.
        .child(
            div()
                .id("upgrade-rows")
                .flex()
                .flex_col()
                .gap(px(4.))
                .flex_shrink()
                .min_h(px(0.))
                .overflow_y_scroll()
                .track_scroll(scroll)
                .children(rows),
        )
        .child(
            div()
                .pt(px(12.))
                .text_color(theme.dim)
                .child(format!("# {info}  (klick = kaufen)")),
        )
}
