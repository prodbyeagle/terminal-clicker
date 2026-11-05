//! Hilfe-Seite. Beim allerersten Start erscheint sie als Intro.

use crate::theme::Theme;
use gpui::{Div, div, prelude::*, px};

/// Ein Abschnitt: Überschrift und Zeilen (`Taste`, `Erklärung`).
type Section = (&'static str, &'static [(&'static str, &'static str)]);

const LEFT: &[Section] = &[
    (
        "ZIEL",
        &[
            ("", "bytes sammeln. upgrades kaufen."),
            ("", "noch mehr bytes sammeln."),
        ],
    ),
    (
        "TASTEN",
        &[
            ("space", "klicken"),
            ("u", "upgrades"),
            ("s", "settings"),
            ("h", "diese hilfe"),
            ("e", "event einsammeln"),
            ("↑↓", "auswählen"),
            ("←→", "wert ändern"),
            ("enter", "kaufen / umschalten"),
            ("esc", "zurück"),
            ("q", "speichern und beenden"),
        ],
    ),
    (
        "MAUS",
        &[
            ("klick", "button, zeilen, reiter, kürzel"),
            ("rechts", "wert zurückschalten"),
            ("ziehen", "titelleiste verschiebt fenster"),
        ],
    ),
    (
        "SETTINGS",
        &[
            ("", "ascii-hintergrund, animationen,"),
            ("", "zahlenformat (voll / kb / kib), themes."),
        ],
    ),
];

const RIGHT: &[Section] = &[
    (
        "UPGRADES",
        &[
            ("+B/s", "bytes von allein, jede sekunde"),
            ("+B/klick", "mehr pro klick"),
            ("×", "klickwert / passives vervielfachen"),
            ("krit", "chance auf klick ×10"),
            ("klick+s", "jeder klick bringt sekunden B/s"),
            ("events", "kommen öfter, halten länger"),
            ("offline", "verdient weiter, wenn's zu ist"),
            ("-kosten", "alle upgrades billiger"),
            ("???", "wird mit mehr bytes sichtbar"),
        ],
    ),
    (
        "EVENTS",
        &[
            ("", "ab und zu ploppt ein kasten auf – klick oder [e]:"),
            ("boost", "×4 auf alles, 30 s"),
            ("rausch", "×10 pro klick, 15 s"),
            ("fund", "sofort ein batzen bytes"),
            ("kette", "jeder klick = 10 s passiv, 15 s"),
            ("rabatt", "alle upgrades halber preis, 30 s"),
            ("glitch", "meist ×7 auf alles. manchmal lag."),
            ("ddos", "abwehren! ignoriert = ×0.5 B/s"),
        ],
    ),
];

fn section(theme: &Theme, (title, rows): &Section) -> Div {
    div()
        .flex()
        .flex_col()
        .pb(px(14.))
        .child(div().pb(px(4.)).child(*title))
        .children(rows.iter().map(|(key, text)| {
            div()
                .flex()
                // Zeilen ohne Taste nutzen die volle Breite (Fließtext).
                .when(!key.is_empty(), |d| d.child(div().w(px(76.)).flex_none().child(*key)))
                .child(div().flex_1().min_w(px(0.)).text_color(theme.dim).child(*text))
        }))
}

fn column(theme: &Theme, sections: &[Section]) -> Div {
    div()
        .flex()
        .flex_col()
        .w(px(340.))
        .flex_shrink()
        .min_w(px(0.))
        .children(sections.iter().map(|s| section(theme, s)))
}

/// `intro`: erster Start – mit Begrüßung und „enter = los“.
pub fn render_help(theme: &Theme, intro: bool, admin: bool) -> impl IntoElement {
    let title = if intro { "WILLKOMMEN" } else { "HILFE" };
    let lead = if intro {
        "du bist ein terminal. du sammelst bytes. so geht's:"
    } else {
        "alles, was man wissen muss."
    };
    let footer = if intro {
        "[enter] los geht's · diese seite gibt's jederzeit mit [h]"
    } else {
        "[esc] zurück"
    };

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
        .child(
            div()
                .flex()
                .gap(px(32.))
                .child(column(theme, LEFT))
                .child(column(theme, RIGHT)),
        )
        .when(admin, |d| {
            d.child(
                div()
                    .text_color(theme.dim)
                    .child("admin-modus aktiv: [a] öffnet das admin-panel."),
            )
        })
        .child(div().pt(px(10.)).child(footer))
}
