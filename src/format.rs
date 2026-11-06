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
                &[
                    "B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB", "RB", "QB",
                ],
            )),
            NumberFormat::Binary => Some((
                1024.,
                &[
                    "B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "RiB", "QiB",
                ],
            )),
        }
    }
}

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

fn sci(v: f64) -> String {
    format!("{v:.2e}")
}

/// Ganze Zahl mit Tausenderpunkten (`1.234.567`), ab 10^21 wissenschaftlich.
pub fn grouped(v: f64) -> String {
    if v.abs() >= FULL_SCI_FROM {
        return sci(v);
    }
    let digits = format!("{:.0}", v.floor());
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

pub fn bar(share: f64, width: usize) -> String {
    let filled = ((share.clamp(0., 1.) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

pub fn duration(seconds: f64) -> String {
    let s = seconds.max(0.) as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else {
        format!("{m}m {s:02}s")
    }
}

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

pub fn bytes(v: f64, f: NumberFormat) -> String {
    scaled(v, f).unwrap_or_else(|| full(v, 0))
}

pub fn bytes_padded(v: f64, f: NumberFormat) -> String {
    scaled(v, f).unwrap_or_else(|| full(v, 4))
}

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
    fn grouped_and_duration() {
        assert_eq!(grouped(999.), "999");
        assert_eq!(grouped(1_234_567.), "1.234.567");
        assert_eq!(grouped(2e25), "2.00e25");
        assert_eq!(duration(65.), "1m 05s");
        assert_eq!(duration(3725.), "1h 02m 05s");
        assert_eq!(bar(0.5, 4), "██░░");
        assert_eq!(bar(2.0, 3), "███");
    }

    #[test]
    fn binary_uses_powers_of_1024() {
        assert_eq!(bytes(1_023., Binary), "1023 B");
        assert_eq!(bytes(1_024., Binary), "1.00 KiB");
        assert_eq!(bytes(10_048_514., Binary), "9.58 MiB");
    }
}
