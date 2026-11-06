//! Hilfe-Seite. Beim allerersten Start erscheint sie als Intro.
//!
//! Tasten, Upgrade-Arten, Events und Zahlenformate werden aus den Daten erzeugt,
//! die sie beschreiben – so kann die Hilfe nicht veralten.

use crate::format::NumberFormat;
use crate::game::UPGRADES;
use crate::game::events::EventKind;
use crate::theme::Theme;
use crate::ui::keys;
use crate::ui::msg::{Msg, Screen};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use gpui::{AnyElement, Div, div, prelude::*, px};
use std::mem::discriminant;

const KEY_W: f32 = 76.;
/// Event-Namen sind länger als Tastennamen.
const EVENT_KEY_W: f32 = 140.;

/// Eine Zeile: Kurzform links (darf leer sein), Erklärung rechts.
type Row = (String, String);

const MOUSE: &[(&str, &str)] = &[
    ("klick", "button, zeilen, reiter, kürzel"),
    ("rechts", "wert zurückschalten"),
    ("ziehen", "titelleiste verschiebt fenster"),
];

/// Solange das Intro nicht gesehen wurde, ist die Hilfe die Begrüßung.
#[derive(Default)]
pub struct HelpScreen;

fn is_intro(s: &Session) -> bool {
    !s.settings.intro_seen
}

impl Page for HelpScreen {
    fn hints(&self, s: &Session) -> Vec<Hint> {
        if is_intro(s) {
            vec![Hint::new(Msg::Confirm, "los geht's")]
        } else {
            vec![Hint::new(Msg::Back, "zurück")]
        }
    }

    fn handle(&mut self, msg: Msg, _s: &mut Session) -> Option<Screen> {
        (msg == Msg::Confirm).then_some(Screen::Main)
    }

    /// Wer die Hilfe einmal verlassen hat, bekommt kein Intro mehr.
    fn leave(&mut self, s: &mut Session) {
        if is_intro(s) {
            s.settings.intro_seen = true;
            s.save_settings = true;
        }
    }

    fn render(&self, s: &Session, v: &View) -> AnyElement {
        let theme = &v.theme;
        let (title, lead, footer) = if is_intro(s) {
            (
                "WILLKOMMEN",
                "du bist ein terminal. du sammelst bytes. so geht's:",
                format!(
                    "[{}] los geht's · diese seite gibt's jederzeit mit [{}]",
                    keys::label(&[Msg::Confirm]),
                    keys::label(&[Msg::Open(Screen::Help)])
                ),
            )
        } else {
            (
                "HILFE",
                "alles, was man wissen muss.",
                format!("[{}] zurück", keys::label(&[Msg::Back])),
            )
        };
        let formats: Vec<_> = NumberFormat::ALL.iter().map(|f| f.label()).collect();

        let left = div()
            .flex()
            .flex_col()
            .w(px(340.))
            .flex_shrink()
            .min_w(px(0.))
            .child(section(
                theme,
                "ZIEL",
                vec![
                    text_row("bytes sammeln. upgrades kaufen."),
                    text_row("noch mehr bytes sammeln."),
                ],
                KEY_W,
            ))
            .child(section(theme, "TASTEN", key_rows(s.admin), KEY_W))
            .child(section(
                theme,
                "MAUS",
                MOUSE
                    .iter()
                    .map(|(k, t)| (k.to_string(), t.to_string()))
                    .collect(),
                KEY_W,
            ))
            .child(section(
                theme,
                "SETTINGS",
                vec![
                    text_row("ascii-hintergrund, animationen, themes,"),
                    text_row(&format!("zahlenformat ({}).", formats.join(" / "))),
                ],
                KEY_W,
            ));
        let right = div()
            .flex()
            .flex_col()
            .w(px(420.))
            .flex_shrink()
            .min_w(px(0.))
            .child(section(theme, "UPGRADES", upgrade_rows(), KEY_W))
            .child(section(theme, "EVENTS", event_rows(), EVENT_KEY_W));

        div()
            .id("help")
            .flex()
            .flex_col()
            .max_w_full()
            .max_h_full()
            .overflow_y_scroll()
            .p(px(16.))
            .bg(theme.bg)
            .border_1()
            .border_color(theme.line)
            .child(div().pb(px(4.)).child(title))
            .child(div().pb(px(10.)).text_color(theme.dim).child(lead))
            .child(div().border_b_1().border_color(theme.line).mb(px(12.)))
            .child(div().flex().gap(px(32.)).child(left).child(right))
            .child(div().pt(px(10.)).child(footer))
            .into_any_element()
    }
}

fn text_row(text: &str) -> Row {
    (String::new(), text.to_string())
}

fn key_rows(admin: bool) -> Vec<Row> {
    let available = |msg: &Msg| !matches!(msg, Msg::Open(screen) if !screen.available(admin));
    keys::HELP
        .iter()
        .filter(|(msgs, _)| msgs.iter().all(available))
        .map(|(msgs, text)| (keys::label(msgs), text.to_string()))
        .collect()
}

/// Jede Effekt-Art einmal, in der Reihenfolge, in der sie im Spiel auftaucht.
fn upgrade_rows() -> Vec<Row> {
    let mut seen = Vec::new();
    let mut rows = Vec::new();
    for def in UPGRADES {
        if !seen.contains(&discriminant(&def.effect)) {
            seen.push(discriminant(&def.effect));
            let (key, text) = def.effect.kind_help();
            rows.push((key.to_string(), text));
        }
    }
    rows.push(("???".into(), "wird mit mehr bytes sichtbar".into()));
    rows
}

fn event_rows() -> Vec<Row> {
    let intro = text_row(&format!(
        "ab und zu ploppt ein kasten auf – klick oder [{}]:",
        keys::label(&[Msg::CollectEvent])
    ));
    std::iter::once(intro)
        .chain(EventKind::ALL.iter().map(|k| (k.name(), k.description())))
        .collect()
}

/// `key_w`: Breite der Kurzform-Spalte.
fn section(theme: &Theme, title: &'static str, rows: Vec<Row>, key_w: f32) -> Div {
    div()
        .flex()
        .flex_col()
        .pb(px(14.))
        .child(div().pb(px(4.)).child(title))
        .children(rows.into_iter().map(|(key, text)| {
            div()
                .flex()
                .when(!key.is_empty(), |d| {
                    d.child(div().w(px(key_w)).flex_none().child(key))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_color(theme.dim)
                        .child(text),
                )
        }))
}
