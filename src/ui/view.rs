//! Was jede Ansicht zum Zeichnen braucht, das nicht in der Sitzung steht.

use crate::theme::Theme;
use crate::ui::msg::Emit;
use gpui::{Pixels, Size, WindowAppearance};
use std::time::Instant;

pub struct View<'a> {
    pub theme: Theme,
    pub now: Instant,
    pub emit: &'a Emit,
    pub viewport: Size<Pixels>,
    /// Für die Vorschau des „system“-Themes.
    pub appearance: WindowAppearance,
    /// Blinkphase des Cursors `_` (Boot und Log blinken im selben Takt).
    pub cursor_on: bool,
}
