//! Zahlen- und Byte-Formatierung, umschaltbar in den Settings („zahlen“).
//!
//! - `voll`:    exakte Zahl, z. B. `10048514 B` (ab 10^21 wissenschaftlich: `1.23e24 B`)
//! - `kb/mb`:   1000er-Schritte bis QB (10^30), z. B. `10.05 MB`
//! - `kib/mib`: 1024er-Schritte bis QiB, z. B. `9.58 MiB`
//!
//! Was über die größte Einheit hinausgeht, wird wissenschaftlich geschrieben
//! (`4.20e33 QB` wäre unlesbar, also `4.20e63 B`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumberFormat {
    #[default]
    Full,
    Si,
    Binary,
}

/// Ab hier schreibt auch `voll` wissenschaftlich – sonst wird die Zahl zu lang.
const FULL_SCI_FROM: f64 = 1e21;

impl NumberFormat {
    pub const ALL: [NumberFormat; 3] = [NumberFormat::Full, NumberFormat::Si, NumberFormat::Binary];

    pub fn label(self) -> &'static str {
        match self {
            NumberFormat::Full => "voll",
            NumberFormat::Si => "kb/mb",
            NumberFormat::Binary => "kib/mib",
        }
    }

    fn scale(self) -> Option<(f64, &'static [&'static str])> {
        match self {
            NumberFormat::Full => None,
            NumberFormat::Si => Some((
                1000.,
                &["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB", "RB", "QB"],
            )),
            NumberFormat::Binary => Some((
                1024.,
                &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "RiB", "QiB"],
            )),
        }
    }
}

/// 1.0 → "1", 1.5 → "1.5"
pub fn trim_float(v: f64) -> String {
    if v.abs() >= FULL_SCI_FROM {
        return sci(v);
    }
    if (v - v.round()).abs() < 1e-9 {
        format!("{:.0}", v.round())
    } else {
        format!("{v:.1}")
    }
}

/// Wissenschaftliche Schreibweise mit zwei Nachkommastellen: `1.23e24`.
pub fn sci(v: f64) -> String {
    format!("{v:.2e}")
}

/// Umgerechnet in die größte passende Einheit, oder `None` bei `voll`
/// bzw. Werten unter 1 KB.
fn scaled(v: f64, f: NumberFormat) -> Option<String> {
    let (base, units) = f.scale()?;
    if v.abs() < base {
        return None;
    }
    let top = base.powi(units.len() as i32 - 1);
    if v.abs() >= top * 1000. {
        // Jenseits von 1000 QB: lesbarer als „123456789.00 QB“.
        return Some(format!("{} B", sci(v)));
    }
    let mut x = v;
    let mut unit = 0;
    while x.abs() >= base && unit + 1 < units.len() {
        x /= base;
        unit += 1;
    }
    Some(format!("{x:.2} {}", units[unit]))
}

fn full(v: f64, width: usize) -> String {
    if v.abs() >= FULL_SCI_FROM {
        format!("{} B", sci(v))
    } else {
        format!("{:0width$.0} B", v.floor())
    }
}

/// Byte-Menge, z. B. Kosten oder ein Paket: `451 B`, `10.05 MB`.
pub fn bytes(v: f64, f: NumberFormat) -> String {
    scaled(v, f).unwrap_or_else(|| full(v, 0))
}

/// Wie [`bytes`], aber bei `voll` vierstellig mit Nullen (`0042 B`), wie im Plan.
pub fn bytes_padded(v: f64, f: NumberFormat) -> String {
    scaled(v, f).unwrap_or_else(|| full(v, 4))
}

/// Rate pro Sekunde (darf Nachkommastellen haben): `1.5 B/s`, `1.20 MB/s`.
pub fn rate(v: f64, f: NumberFormat) -> String {
    match scaled(v, f) {
        Some(s) => format!("{s}/s"),
        None => format!("{} B/s", trim_float(v)),
    }
}

/// Text für den großen Zähler: `voll` = `0042` (Einheit steht darüber),
/// sonst mit Einheit, z. B. `10.05 MB`.
pub fn counter(v: f64, f: NumberFormat) -> String {
    match f {
        NumberFormat::Full if v.abs() >= FULL_SCI_FROM => sci(v),
        NumberFormat::Full => format!("{:04.0}", v.floor()),
        _ => bytes(v, f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NumberFormat::*;

    #[test]
    fn full_keeps_exact_numbers() {
        assert_eq!(bytes(10_048_514., Full), "10048514 B");
        assert_eq!(bytes_padded(42., Full), "0042 B");
        assert_eq!(counter(42., Full), "0042");
        assert_eq!(rate(1.5, Full), "1.5 B/s");
    }

    #[test]
    fn full_switches_to_scientific_for_huge_values() {
        assert_eq!(bytes(1.8e19, Full), "18000000000000000000 B");
        assert_eq!(bytes(1.234e24, Full), "1.23e24 B");
        assert_eq!(counter(5e30, Full), "5.00e30");
    }

    #[test]
    fn si_uses_powers_of_1000() {
        assert_eq!(bytes(999., Si), "999 B");
        assert_eq!(bytes(1_000., Si), "1.00 KB");
        assert_eq!(bytes(10_048_514., Si), "10.05 MB");
        assert_eq!(bytes(3e9, Si), "3.00 GB");
        assert_eq!(rate(1.2e6, Si), "1.20 MB/s");
        assert_eq!(counter(42., Si), "42 B");
    }

    #[test]
    fn units_go_past_exabyte() {
        assert_eq!(bytes(1.845e19, Si), "18.45 EB");
        assert_eq!(bytes(2e21, Si), "2.00 ZB");
        assert_eq!(bytes(3e24, Si), "3.00 YB");
        assert_eq!(bytes(4e27, Si), "4.00 RB");
        assert_eq!(bytes(5e30, Si), "5.00 QB");
        assert_eq!(bytes(6e35, Si), "6.00e35 B");
    }

    #[test]
    fn binary_uses_powers_of_1024() {
        assert_eq!(bytes(1_023., Binary), "1023 B");
        assert_eq!(bytes(1_024., Binary), "1.00 KiB");
        assert_eq!(bytes(10_048_514., Binary), "9.58 MiB");
    }
}
