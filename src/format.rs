//! Zahlen- und Byte-Formatierung, umschaltbar in den Settings („zahlen“).
//!
//! - `voll`:    exakte Zahl, z. B. `10048514 B`
//! - `kb/mb`:   1000er-Schritte, z. B. `10.05 MB`
//! - `kib/mib`: 1024er-Schritte wie `ls -h`, z. B. `9.58 MiB`

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumberFormat {
    #[default]
    Full,
    Si,
    Binary,
}

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
            NumberFormat::Si => Some((1000., &["B", "KB", "MB", "GB", "TB", "PB", "EB"])),
            NumberFormat::Binary => Some((1024., &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"])),
        }
    }
}

/// 1.0 → "1", 1.5 → "1.5"
pub fn trim_float(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as u64)
    } else {
        format!("{v:.1}")
    }
}

/// Umgerechnet in die größte passende Einheit, oder `None` bei `voll`
/// bzw. Werten unter 1 KB.
fn scaled(v: f64, f: NumberFormat) -> Option<String> {
    let (base, units) = f.scale()?;
    if v.abs() < base {
        return None;
    }
    let mut x = v;
    let mut unit = 0;
    while x.abs() >= base && unit + 1 < units.len() {
        x /= base;
        unit += 1;
    }
    Some(format!("{x:.2} {}", units[unit]))
}

/// Byte-Menge, z. B. Kosten oder ein Paket: `451 B`, `10.05 MB`.
pub fn bytes(v: u64, f: NumberFormat) -> String {
    scaled(v as f64, f).unwrap_or_else(|| format!("{v} B"))
}

/// Wie [`bytes`], aber bei `voll` vierstellig mit Nullen (`0042 B`), wie im Plan.
pub fn bytes_padded(v: u64, f: NumberFormat) -> String {
    scaled(v as f64, f).unwrap_or_else(|| format!("{v:04} B"))
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
pub fn counter(v: u64, f: NumberFormat) -> String {
    match f {
        NumberFormat::Full => format!("{v:04}"),
        _ => bytes(v, f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NumberFormat::*;

    #[test]
    fn full_keeps_exact_numbers() {
        assert_eq!(bytes(10_048_514, Full), "10048514 B");
        assert_eq!(bytes_padded(42, Full), "0042 B");
        assert_eq!(counter(42, Full), "0042");
        assert_eq!(rate(1.5, Full), "1.5 B/s");
    }

    #[test]
    fn si_uses_powers_of_1000() {
        assert_eq!(bytes(999, Si), "999 B");
        assert_eq!(bytes(1_000, Si), "1.00 KB");
        assert_eq!(bytes(10_048_514, Si), "10.05 MB");
        assert_eq!(bytes(3_000_000_000, Si), "3.00 GB");
        assert_eq!(rate(1_200_000., Si), "1.20 MB/s");
        assert_eq!(counter(42, Si), "42 B");
    }

    #[test]
    fn binary_uses_powers_of_1024() {
        assert_eq!(bytes(1_023, Binary), "1023 B");
        assert_eq!(bytes(1_024, Binary), "1.00 KiB");
        assert_eq!(bytes(10_048_514, Binary), "9.58 MiB");
    }

    #[test]
    fn huge_values_stay_on_largest_unit() {
        assert!(bytes(u64::MAX, Si).ends_with(" EB"));
    }
}
