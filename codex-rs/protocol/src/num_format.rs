use std::sync::OnceLock;

use icu_decimal::DecimalFormatter;
use icu_decimal::input::Decimal;
use icu_decimal::options::DecimalFormatterOptions;
use icu_locale_core::Locale;

fn formatter() -> &'static Option<DecimalFormatter> {
    static FORMATTER: OnceLock<Option<DecimalFormatter>> = OnceLock::new();

    FORMATTER.get_or_init(|| {
        let loc: Locale = sys_locale::get_locale()
            .and_then(|loc| loc.parse().ok())
            .or_else(|| "en-US".parse().ok())?;
        DecimalFormatter::try_new(loc.into(), DecimalFormatterOptions::default()).ok()
    })
}

/// Format a u64 with locale-aware digit separators (e.g. "12345" -> "12,345"
/// for en-US).
pub fn format_with_separators(n: u64) -> String {
    match formatter() {
        Some(formatter) => formatter.format(&Decimal::from(n)).to_string(),
        None => n.to_string(),
    }
}

/// Format token counts to 3 significant figures, using base-10 SI suffixes.
///
/// Examples (en-US):
///   - 999 -> "999"
///   - 1200 -> "1.20K"
///   - 123456789 -> "123M"
pub fn format_si_suffix(n: u64) -> String {
    // Format `n / scale` with the requested number of fractional digits.
    fn format_scaled(n: u64, scale: u64, frac_digits: u32) -> String {
        let value = n as f64 / scale as f64;
        match formatter() {
            Some(fmt) => {
                let scaled: u64 = (value * 10f64.powi(frac_digits as i32)).round() as u64;
                let mut dec = Decimal::from(scaled);
                dec.multiply_pow10(-(frac_digits as i16));
                fmt.format(&dec).to_string()
            }
            None => match frac_digits {
                2 => format!("{:.02}", value),
                1 => format!("{:.01}", value),
                _ => format!("{:.00}", value),
            },
        }
    }

    const UNITS: &[(u64, &str)] = &[
        (1, ""),
        (1_000, "K"),
        (1_000_000, "M"),
        (1_000_000_000, "G"),
    ];

    let f = n as f64;
    for (scale, suffix) in UNITS {
        if (100.0 * f / *scale as f64).round() < 1000.0 {
            return format!("{}{}", format_scaled(n, *scale, 2), suffix);
        } else if (10.0 * f / *scale as f64).round() < 1000.0 {
            return format!("{}{}", format_scaled(n, *scale, 1), suffix);
        } else if (f / *scale as f64).round() < 1000.0 {
            return format!("{}{}", format_scaled(n, *scale, 0), suffix);
        }
    }

    // Above 1000G, keep whole‑G precision.
    format!(
        "{}G",
        format_with_separators(((n as f64) / 1e9).round() as u64)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kmg() {
        assert_eq!(format_si_suffix(0), "0");
        assert_eq!(format_si_suffix(999), "999");
        assert_eq!(format_si_suffix(1_000), "1.00K");
        assert_eq!(format_si_suffix(1_200), "1.20K");
        assert_eq!(format_si_suffix(10_000), "10.0K");
        assert_eq!(format_si_suffix(100_000), "100K");
        assert_eq!(format_si_suffix(999_500), "1.00M");
        assert_eq!(format_si_suffix(1_000_000), "1.00M");
        assert_eq!(format_si_suffix(1_234_000), "1.23M");
        assert_eq!(format_si_suffix(12_345_678), "12.3M");
        assert_eq!(format_si_suffix(999_950_000), "1.00G");
        assert_eq!(format_si_suffix(1_000_000_000), "1.00G");
        assert_eq!(format_si_suffix(1_234_000_000), "1.23G");
        // Above 1000G we keep whole‑G precision (no higher unit supported here).
        assert_eq!(format_si_suffix(1_234_000_000_000), "1,234G");
    }
}
