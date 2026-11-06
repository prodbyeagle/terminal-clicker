//! Boot-Screen: Fake-Log, das sich Zeichen für Zeichen eintippt.

use crate::format;
use crate::game::Game;
use crate::ui::keys;
use crate::ui::msg::{Msg, Screen};
use crate::ui::page::Page;
use crate::ui::session::Session;
use crate::ui::status_bar::Hint;
use crate::ui::view::View;
use gpui::{AnyElement, MouseButton, div, prelude::*, px};
use std::time::Instant;

const CHARS_PER_SEC: f64 = 420.;

struct Line {
    text: String,
    dim: bool,
}

pub struct Boot {
    lines: Vec<Line>,
    started: Instant,
}

impl Boot {
    pub fn new(game: &Game, loaded: bool, fmt: format::NumberFormat) -> Self {
        let save = if loaded {
            format!("ok ({})", format::bytes_padded(game.whole_bytes(), fmt))
        } else {
            "neu".to_string()
        };
        let plain = |t: String| Line {
            text: t,
            dim: false,
        };
        let sys = |t: String| Line { text: t, dim: true };
        let lines = vec![
            plain(format!(
                "terminal-clicker v{} (tty0)",
                env!("CARGO_PKG_VERSION")
            )),
            plain(String::new()),
            sys("[ 0.000] bios: 1 finger erkannt".into()),
            sys("[ 0.012] mem: 640 KB sollten reichen".into()),
            sys("[ 0.031] mount /dev/klick ............ ok".into()),
            sys(format!("[ 0.047] lade spielstand ............. {save}")),
            sys("[ 0.063] starte byte-daemon .......... ok".into()),
            sys(format!(
                "[ 0.071] upgrades installiert ........ {:03}",
                game.upgrades_bought()
            )),
            plain(String::new()),
            plain(format!(
                "bereit. [{}] oder klick zum starten ",
                keys::label(&[Msg::Confirm])
            )),
        ];
        Self {
            lines,
            started: Instant::now(),
        }
    }

    fn revealed(&self, animate: bool) -> (usize, usize) {
        let total: usize = self.lines.iter().map(|l| l.text.chars().count()).sum();
        let typed = (self.started.elapsed().as_secs_f64() * CHARS_PER_SEC) as usize;
        (if animate { typed.min(total) } else { total }, total)
    }
}

impl Page for Boot {
    fn hints(&self, _s: &Session) -> Vec<Hint> {
        vec![Hint::new(Msg::Confirm, "starten")]
    }

    /// Startet das Spiel: Event-Takt an, beim allerersten Mal zuerst das Intro.
    fn handle(&mut self, msg: Msg, s: &mut Session) -> Option<Screen> {
        if msg != Msg::Confirm {
            return None;
        }
        s.start();
        Some(if s.settings.intro_seen {
            Screen::Main
        } else {
            Screen::Help
        })
    }

    fn animating(&self, s: &Session, _now: Instant) -> bool {
        let (revealed, total) = self.revealed(s.settings.animations);
        revealed < total
    }

    fn fills(&self) -> bool {
        true
    }

    fn render(&self, s: &Session, v: &View) -> AnyElement {
        let (revealed, total) = self.revealed(s.settings.animations);
        let typing = revealed < total;

        let mut left = revealed;
        let mut shown = Vec::new();
        for line in &self.lines {
            let n = line.text.chars().count();
            let take = n.min(left);
            shown.push((line.text.chars().take(take).collect::<String>(), line.dim));
            left -= take;
            if take < n {
                break;
            }
        }
        let cursor = if typing || v.cursor_on { "_" } else { " " };
        let last = shown.len().saturating_sub(1);

        div()
            .flex_1()
            .p(px(28.))
            .flex()
            .flex_col()
            .bg(v.theme.bg)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, v.emit.on_press(Msg::Confirm))
            .children(shown.into_iter().enumerate().map(|(i, (text, dim))| {
                div()
                    .flex()
                    .whitespace_nowrap()
                    .when(dim, |d| d.text_color(v.theme.dim))
                    .child(text)
                    .when(i == last, |d| {
                        d.child(div().text_color(v.theme.fg).child(cursor))
                    })
            }))
            .into_any_element()
    }
}
