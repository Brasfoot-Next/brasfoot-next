//! Numbers, money and dates as the game shows them, in one place. The words come from the
//! catalog.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::clock::Clock;

/// Port of `best.C.c(long)`: "1 milhão 500 mil", "35 mil", "800".
pub fn money(t: &Translator, value: i64) -> String {
    if value < 0 {
        return format!("-{}", money(t, value.saturating_abs()));
    }
    let unit = |key: &str| t.text(key, None);
    let amount = |number: &str, key: &str| t.text("money-amount", Some(&args([("number", number.to_owned()), ("unit", unit(key))])));
    let digits = value.to_string();
    let d = |a: usize, b: usize| digits[a..b].to_owned();
    if value == 0 {
        return amount("0", "money-unit-thousand");
    }
    if value <= 999 {
        return digits;
    }
    if value <= 999_999 {
        let n = digits.len() - 3;
        return amount(&d(0, n), "money-unit-thousand");
    }
    if value > 99_999_999_999 {
        return compact_money(t, value);
    }
    let (big, rest) = match value {
        ..=1_999_999 => (amount(&d(0, 1), "money-unit-million-one"), d(1, 4)),
        ..=9_999_999 => (amount(&d(0, 1), "money-unit-million"), d(1, 4)),
        ..=99_999_999 => (amount(&d(0, 2), "money-unit-million"), d(2, 5)),
        ..=999_999_999 => (amount(&d(0, 3), "money-unit-million"), d(3, 6)),
        ..=1_999_999_999 => (amount(&d(0, 1), "money-unit-billion-one"), d(1, 4)),
        ..=9_999_999_999 => (amount(&d(0, 1), "money-unit-billion"), d(1, 4)),
        _ => (amount(&d(0, 2), "money-unit-billion"), d(2, 5)),
    };
    // the second part: millions below the billions, with the original's shortcuts for leading
    // zeros (which always say "mil", even after billions)
    let rest = if rest == "000" {
        String::new()
    } else if rest.starts_with("00") {
        amount(&rest[2..3], "money-unit-thousand")
    } else if rest.starts_with('0') {
        amount(&rest[1..3], "money-unit-thousand")
    } else {
        amount(&rest, if value <= 999_999_999 { "money-unit-thousand" } else { "money-unit-million" })
    };
    format!("{big} {rest}")
}

/// Port of `best.C.a(double, int)`: "850 mil", "1.2M", "35M".
pub fn compact_money(t: &Translator, value: i64) -> String {
    fn step(t: &Translator, value: f64, unit: usize) -> String {
        let v = ((value as i64) / 100) as f64 / 10.0;
        let whole = v * 10.0 % 10.0 == 0.0;
        if v < 1000.0 {
            let number = if !(v > 99.9) && !whole && (whole || !(v > 9.99)) { format!("{v}") } else { ((v as i64) * 10 / 10).to_string() };
            format!("{number}{}", t.text(&format!("money-compact-{unit}"), None))
        } else {
            step(t, v, unit + 1)
        }
    }
    step(t, value as f64, 0)
}

/// dd/MM/yyyy (`SimpleDateFormat("dd/MM/yyyy")`).
pub fn date(t: &Translator, clock: &Clock, millis: i64) -> String {
    let f = clock.fields(millis);
    t.text("date-short", Some(&args([("day", format!("{:02}", f.day)), ("month", format!("{:02}", f.month + 1)), ("year", f.year.to_string())])))
}

/// `DateFormat.getDateInstance()` in pt-BR: "23 de set. de 2022".
pub fn date_medium(t: &Translator, clock: &Clock, millis: i64) -> String {
    let f = clock.fields(millis);
    let month = t.text(&format!("month-short-{}", f.month), None);
    t.text("date-medium", Some(&args([("day", f.day.to_string()), ("month", month), ("year", f.year.to_string())])))
}

/// An ISO `yyyy-mm-dd` date (the native save's header) like `date_medium`; anything else as is.
pub fn iso_date_medium(t: &Translator, iso: &str) -> String {
    let mut parts = iso.splitn(3, '-').map(|p| p.parse::<u32>().ok());
    match (parts.next().flatten(), parts.next().flatten(), parts.next().flatten()) {
        (Some(year), Some(month @ 1..=12), Some(day)) => {
            let month = t.text(&format!("month-short-{}", month - 1), None);
            t.text("date-medium", Some(&args([("day", day.to_string()), ("month", month), ("year", year.to_string())])))
        }
        _ => iso.to_owned(),
    }
}

/// `String.format("%.2f")` in pt-BR (`X.cE()`).
pub fn rating_two(t: &Translator, value: f64) -> String {
    if value == 10.0 {
        return "10".into();
    }
    format!("{value:.2}").replace('.', &t.text("decimal-separator", None))
}

/// A rating with one decimal, the decimal separator from the catalog.
pub fn rating(t: &Translator, value: f64) -> String {
    let text = format!("{value:.1}");
    text.replace('.', &t.text("decimal-separator", None))
}

/// `String.format("%,d")` in pt-BR: "176.200".
pub fn thousands(t: &Translator, value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let separator = t.text("thousands-separator", None);
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push_str(&separator);
        }
        out.push(c);
    }
    if value < 0 { format!("-{out}") } else { out }
}

/// A decimal with `places` digits (`String.format("%.Nf")` in pt-BR), half up like Java.
pub fn decimal(t: &Translator, value: f64, places: usize) -> String {
    let scale = 10f64.powi(places as i32);
    let rounded = (value.abs() * scale + 0.5).floor() / scale * value.signum();
    format!("{rounded:.places$}").replace('.', &t.text("decimal-separator", None))
}

/// Port of `Patrocinio.mi(long)`: "12,3 mi".
pub fn mi(t: &Translator, value: i64) -> String {
    t.text("money-mi", Some(&args([("value", decimal(t, value as f64 / 1e6, 1))])))
}

/// Port of `CaixaDialog.dinheiro(long)`: "1,25 bi", "12,3 mi", "350 mil", "800".
pub fn money_short(t: &Translator, value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let a = (value as f64).abs();
    let (number, unit) = if a >= 1e9 {
        (decimal(t, a / 1e9, 2), "money-short-bi")
    } else if a >= 1e6 {
        (decimal(t, a / 1e6, 1), "money-short-mi")
    } else if a >= 1e3 {
        (decimal(t, a / 1e3, 0), "money-short-thousand")
    } else {
        return format!("{sign}{}", a as i64);
    };
    t.text(unit, Some(&args([("sign", sign.to_owned()), ("value", number)])))
}

/// Port of `Economia.aproximado(long)`: "~ 15 mi", "~ 2,3 mi", "~ 1,2 bi".
pub fn approximate(t: &Translator, value: i64) -> String {
    const MI: f64 = 1_000_000.0;
    let sign = if value < 0 { "-" } else { "" };
    let v = (value as f64).abs();
    if v >= 1000.0 * MI {
        return t.text("money-approx-bi", Some(&args([("sign", sign.to_owned()), ("value", decimal(t, v / (1000.0 * MI), 1))])));
    }
    let mi = v / MI;
    let step = if mi >= 100.0 { 10.0 } else if mi >= 10.0 { 1.0 } else { 0.1 };
    let rounded = (mi / step + 0.5).floor() * step;
    let number = if step < 1.0 { decimal(t, rounded, 1) } else { decimal(t, rounded, 0) };
    t.text("money-approx-mi", Some(&args([("sign", sign.to_owned()), ("value", number)])))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values worked out by hand from `best.C.c(long)` and `best.C.a(double, int)`.
    #[test]
    fn money_reads_like_the_game() {
        let t = Translator::new(brasfoot_i18n::DEFAULT_LOCALE);
        assert_eq!(money(&t, 0), "0 mil");
        assert_eq!(money(&t, 850), "850");
        assert_eq!(money(&t, 35_400), "35 mil");
        assert_eq!(money(&t, 1_500_000), "1 milhão 500 mil");
        assert_eq!(money(&t, 2_000_000), "2 milhões ");
        assert_eq!(money(&t, 15_050_000), "15 milhões 50 mil");
        assert_eq!(money(&t, 3_007_000), "3 milhões 7 mil");
        assert_eq!(money(&t, -1_500_000), "-1 milhão 500 mil");
        assert_eq!(compact_money(&t, 850_000), "850 mil");
        assert_eq!(compact_money(&t, 1_234_567), "1.2M");
        assert_eq!(compact_money(&t, 35_000_000), "35M");
        assert_eq!(mi(&t, 12_345_678), "12,3 mi");
        assert_eq!(approximate(&t, 15_400_000), "~ 15 mi");
        assert_eq!(approximate(&t, 2_340_000), "~ 2,3 mi");
        assert_eq!(approximate(&t, 1_234_000_000), "~ 1,2 bi");
        assert_eq!(thousands(&t, 176_200), "176.200");
    }
}
