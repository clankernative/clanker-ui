//! Pure package-side value guards conforming to Native's independent host ABI.
//!
//! The generic `ui_*` API is specified in `docs/ui-binding-abi-v2.md` and checked
//! by shared vectors; Native hosts do not depend on this component runtime.
use anyhow::{ensure, Context, Result};
use minijinja::Value;
use std::cmp::Ordering;
use unicode_segmentation::UnicodeSegmentation;

/// A persistent record key, never an expression or an HTML fragment.
pub fn token(value: &str) -> Result<String> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':')),
        "cui_token_requires_bounded_ascii_key"
    );
    Ok(value.to_owned())
}

/// Closed ordinal styles; never return caller-provided class or markup text.
pub fn button_variant(value: &Value) -> Result<&'static str> {
    ensure!(
        value.kind() == minijinja::value::ValueKind::Number
            && value.to_string().bytes().all(|byte| byte.is_ascii_digit()),
        "cui_button_variant_requires_integer"
    );
    match value.as_i64() {
        Some(0) => Ok("primary"),
        Some(1) => Ok("secondary"),
        Some(2) => Ok("quiet"),
        Some(3) => Ok("danger"),
        _ => anyhow::bail!("cui_button_variant_out_of_range"),
    }
}

pub fn button_size(value: &Value) -> Result<&'static str> {
    ensure!(
        value.kind() == minijinja::value::ValueKind::Number
            && value.to_string().bytes().all(|byte| byte.is_ascii_digit()),
        "cui_button_size_requires_integer"
    );
    match value.as_i64() {
        Some(0) => Ok("cui-button--compact"),
        Some(1) => Ok(""),
        _ => anyhow::bail!("cui_button_size_out_of_range"),
    }
}

#[derive(Debug)]
pub struct ImageSource(pub String);
impl minijinja::value::Object for ImageSource {}

pub fn text(value: &str) -> Result<String> {
    ensure!(!value.trim().is_empty(), "cui_text_requires_nonblank_text");
    ensure!(
        !value.chars().any(char::is_control),
        "cui_text_control_character"
    );
    Ok(value.to_owned())
}

pub fn plain(value: &str) -> Result<String> {
    ensure!(
        !value.chars().any(char::is_control),
        "cui_plain_control_character"
    );
    Ok(value.to_owned())
}

pub fn field_text(value: &str) -> Result<String> {
    ensure!(
        !value.trim().is_empty(),
        "cui_field_text_requires_nonblank_text"
    );
    ensure!(
        !value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t')),
        "cui_field_text_control_character"
    );
    Ok(value.to_owned())
}

pub fn initials(value: &str) -> Result<String> {
    text(value)?;
    let result = value.graphemes(true).take(4).collect::<Vec<_>>();
    ensure!(
        (1..=3).contains(&result.len()),
        "cui_initials_grapheme_count"
    );
    Ok(result.concat())
}

#[derive(Clone, Copy)]
enum Number {
    Integer(u128),
    Real(f64),
}

impl Number {
    fn parse(value: &str) -> Result<Self> {
        if let Ok(integer) = value.parse::<u128>() {
            return Ok(Self::Integer(integer));
        }
        let number: f64 = value.parse()?;
        ensure!(number.is_finite(), "cui_progress_finite_number_required");
        Ok(Self::Real(number))
    }

    fn from_value(value: &Value) -> Result<Self> {
        ensure!(
            matches!(
                value.kind(),
                minijinja::value::ValueKind::Number | minijinja::value::ValueKind::String
            ),
            "cui_progress_number_required"
        );
        Self::parse(value.as_str().unwrap_or(&value.to_string()))
    }

    fn compare(self, other: Self) -> Ordering {
        // Never round an integer field into f64: that could admit an over-limit
        // value or falsely report completion above 2^53.
        fn mixed(integer: u128, real: f64) -> Ordering {
            if real < 0.0 {
                return Ordering::Greater;
            }
            if real >= u128::MAX as f64 {
                return Ordering::Less;
            }
            let integral = real as u128;
            match integer.cmp(&integral) {
                Ordering::Equal if real.fract() > 0.0 => Ordering::Less,
                order => order,
            }
        }
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.cmp(&b),
            (Self::Integer(a), Self::Real(b)) => mixed(a, b),
            (Self::Real(a), Self::Integer(b)) => mixed(b, a).reverse(),
            (Self::Real(a), Self::Real(b)) => a.partial_cmp(&b).expect("validated finite numbers"),
        }
    }

    fn text(self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => value.to_string(),
        }
    }
}

pub fn numeric_literal(value: &str) -> bool {
    Number::parse(value).is_ok()
}

fn progress(value: &Value, maximum: &Value) -> Result<(Number, Number)> {
    let value = Number::from_value(value)?;
    let maximum = Number::from_value(maximum)?;
    ensure!(
        maximum.compare(Number::Integer(0)) == Ordering::Greater
            && value.compare(Number::Integer(0)) != Ordering::Less
            && value.compare(maximum) != Ordering::Greater,
        "cui_progress_out_of_range"
    );
    Ok((value, maximum))
}

pub fn progress_value(value: &Value, maximum: &Value) -> Result<String> {
    let (value, _) = progress(value, maximum)?;
    Ok(value.text())
}

pub fn progress_maximum(value: &Value, maximum: &Value) -> Result<String> {
    let (_, maximum) = progress(value, maximum)?;
    Ok(maximum.text())
}

pub fn progress_complete(value: &Value, maximum: &Value) -> Result<bool> {
    let (value, maximum) = progress(value, maximum)?;
    Ok(value.compare(maximum) == Ordering::Equal)
}

pub fn validate_remote_image_source(value: &str) -> Result<()> {
    ensure!(
        value.len() <= 4096
            && value == value.trim()
            && !value.chars().any(|c| c.is_control() || c == '\\'),
        "cui_image_invalid_url"
    );
    let parsed = url::Url::parse(value)?;
    ensure!(
        parsed.scheme() == "https" && parsed.host_str().is_some_and(|host| !host.contains('*')),
        "cui_image_https_required"
    );
    ensure!(
        parsed.username().is_empty() && parsed.password().is_none(),
        "cui_image_credentials_forbidden"
    );
    Ok(())
}

pub fn image(value: &str) -> Result<ImageSource> {
    validate_remote_image_source(value)?;
    let parsed = url::Url::parse(value)?;
    Ok(ImageSource(parsed.to_string()))
}

fn template_error(error: impl std::fmt::Display) -> minijinja::Error {
    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, error.to_string())
}

fn rendered(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

pub fn ui_text(value: &Value, policy: &str, minimum: u64, maximum: u64) -> Result<String> {
    ensure!(
        matches!(policy, "nonblank" | "plain" | "multiline"),
        "ui_text_policy"
    );
    let text = rendered(value);
    ensure!(
        text.chars()
            .all(|c| !c.is_control() || (policy == "multiline" && matches!(c, '\t' | '\r' | '\n'))),
        "ui_text_control_character"
    );
    if policy != "plain" {
        ensure!(!text.trim().is_empty(), "ui_text_blank");
    }
    let count = text.graphemes(true).count() as u64;
    ensure!(count >= minimum, "ui_text_below_minimum_graphemes");
    ensure!(
        maximum == 0 || count <= maximum,
        "ui_text_above_maximum_graphemes"
    );
    Ok(text)
}

pub fn ui_key(value: &Value) -> Result<String> {
    let key = rendered(value);
    ensure!(
        !key.is_empty()
            && key.len() <= 128
            && key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')),
        "ui_key_invalid"
    );
    Ok(key)
}

// Bounded exact decimal representation for the generic binding ABI only.
// Input: <=256 bytes, exponent magnitude <=10_000. IEEE float expansion is
// separately bounded by its binary exponent; no arbitrary-precision dependency.
#[derive(Clone, Debug)]
struct Decimal {
    negative: bool,
    digits: String,
    scale: i32,
}

impl Decimal {
    fn parse(input: &str) -> Result<Self> {
        Self::parse_with_limit(input, 256)
    }
    fn parse_with_limit(input: &str, maximum_length: usize) -> Result<Self> {
        ensure!(
            !input.is_empty() && input.len() <= maximum_length,
            "ui_number_invalid"
        );
        let (mantissa, exponent) =
            if let Some((m, e)) = input.split_once('e').or_else(|| input.split_once('E')) {
                (m, e.parse::<i32>().context("ui_number_invalid")?)
            } else {
                (input, 0)
            };
        ensure!(exponent.unsigned_abs() <= 10_000, "ui_number_invalid");
        let (negative, body) = match input.as_bytes().first() {
            Some(b'-') => (true, &mantissa[1..]),
            Some(b'+') => (false, &mantissa[1..]),
            _ => (false, mantissa),
        };
        let mut split = body.split('.');
        let whole = split.next().unwrap_or_default();
        let fraction = split.next().unwrap_or_default();
        ensure!(
            split.next().is_none()
                && (!whole.is_empty() || !fraction.is_empty())
                && whole.bytes().all(|b| b.is_ascii_digit())
                && fraction.bytes().all(|b| b.is_ascii_digit()),
            "ui_number_invalid"
        );
        let mut digits = format!("{whole}{fraction}");
        let mut scale = i32::try_from(fraction.len())? - exponent;
        while digits.len() > 1 && digits.starts_with('0') {
            digits.remove(0);
        }
        while digits.len() > 1 && digits.ends_with('0') {
            digits.pop();
            scale -= 1;
        }
        if digits.bytes().all(|b| b == b'0') {
            return Ok(Self {
                negative: false,
                digits: "0".into(),
                scale: 0,
            });
        }
        Ok(Self {
            negative,
            digits,
            scale,
        })
    }
    fn canonical(&self) -> String {
        if self.digits == "0" {
            return "0".into();
        }
        let point = self.digits.len() as i64 - i64::from(self.scale);
        let value = if point <= 0 {
            format!("0.{}{}", "0".repeat((-point) as usize), self.digits)
        } else if point >= self.digits.len() as i64 {
            format!(
                "{}{}",
                self.digits,
                "0".repeat((point - self.digits.len() as i64) as usize)
            )
        } else {
            format!(
                "{}.{}",
                &self.digits[..point as usize],
                &self.digits[point as usize..]
            )
        };
        if self.negative {
            format!("-{value}")
        } else {
            value
        }
    }
    fn cmp_abs(&self, other: &Self) -> std::cmp::Ordering {
        if self.digits == "0" || other.digits == "0" {
            return match (self.digits == "0", other.digits == "0") {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (false, false) => unreachable!(),
            };
        }
        let a = self.digits.len() as i64 - i64::from(self.scale);
        let b = other.digits.len() as i64 - i64::from(other.scale);
        a.cmp(&b).then_with(|| {
            (0..self.digits.len().max(other.digits.len()))
                .map(|i| self.digits.as_bytes().get(i).copied().unwrap_or(b'0'))
                .cmp(
                    (0..self.digits.len().max(other.digits.len()))
                        .map(|i| other.digits.as_bytes().get(i).copied().unwrap_or(b'0')),
                )
        })
    }
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.negative != other.negative {
            return if self.negative {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        let cmp = self.cmp_abs(other);
        if self.negative {
            cmp.reverse()
        } else {
            cmp
        }
    }
}
pub fn valid_number_literal(value: &str) -> bool {
    Decimal::parse(value).is_ok()
}
pub fn compare_number_literals(left: &str, right: &str) -> Result<i8> {
    Ok(match Decimal::parse(left)?.cmp(&Decimal::parse(right)?) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}
fn decimal_times_small(digits: &str, factor: u8) -> String {
    let mut output = Vec::with_capacity(digits.len() + 4);
    let mut carry = 0u16;
    for digit in digits.bytes().rev() {
        let value = u16::from(digit - b'0') * u16::from(factor) + carry;
        output.push(b'0' + (value % 10) as u8);
        carry = value / 10;
    }
    while carry > 0 {
        output.push(b'0' + (carry % 10) as u8);
        carry /= 10;
    }
    output.reverse();
    String::from_utf8(output).expect("decimal digits are valid UTF-8")
}

// Exact finite decimal representation of the IEEE-754 value, used only for
// ordering. Unlike f64::to_string(), this retains the binary value exactly.
fn exact_float(value: f64) -> Result<Decimal> {
    ensure!(value.is_finite(), "ui_number_invalid");
    if value == 0.0 {
        return Decimal::parse("0");
    }
    let bits = value.abs().to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        ((1u64 << 52) | fraction, exponent_bits - 1023 - 52)
    };
    let (mut digits, mut scale) = if exponent >= 0 {
        let mut digits = mantissa.to_string();
        for _ in 0..exponent {
            digits = decimal_times_small(&digits, 2);
        }
        (digits, 0)
    } else {
        let mut digits = mantissa.to_string();
        for _ in 0..-exponent {
            digits = decimal_times_small(&digits, 5);
        }
        (digits, -exponent)
    };
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
        scale -= 1;
    }
    Ok(Decimal {
        negative: value.is_sign_negative(),
        digits,
        scale,
    })
}

#[derive(Clone, Debug)]
struct BindingNumber {
    order: Decimal,
    canonical: String,
}
fn binding_number(value: &Value) -> Result<BindingNumber> {
    let (order, canonical) = if let Some(text) = value.as_str() {
        let decimal = Decimal::parse(text)?;
        (decimal.clone(), decimal.canonical())
    } else {
        ensure!(
            value.kind() == minijinja::value::ValueKind::Number,
            "ui_number_invalid"
        );
        if value.is_integer() {
            let decimal = Decimal::parse(&value.to_string())?;
            (decimal.clone(), decimal.canonical())
        } else {
            let real = f64::try_from(value.clone()).context("ui_number_invalid")?;
            let order = exact_float(real)?;
            let canonical = Decimal::parse_with_limit(&real.to_string(), 1200)?.canonical();
            (order, canonical)
        }
    };
    Ok(BindingNumber { order, canonical })
}
/// Order numeric Values exactly; numeric strings are intentionally rejected.
pub fn ui_compare(left: &Value, right: &Value) -> Result<i64> {
    ensure!(
        left.kind() == minijinja::value::ValueKind::Number
            && right.kind() == minijinja::value::ValueKind::Number,
        "ui_compare_number_required"
    );
    Ok(
        match binding_number(left)?
            .order
            .cmp(&binding_number(right)?.order)
        {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
    )
}
/// Guard an integer-typed Value with exact, mathematically integral string bounds.
pub fn ui_integer(value: &Value, minimum: &str, maximum: &str) -> Result<String> {
    let raw = value.to_string();
    ensure!(
        value.kind() == minijinja::value::ValueKind::Number && value.is_integer(),
        "ui_integer_number_required"
    );
    let integer = raw;
    let n = Decimal::parse(&integer)?;
    let min = Decimal::parse(minimum)?;
    let max = Decimal::parse(maximum)?;
    ensure!(
        n.scale <= 0 && min.scale <= 0 && max.scale <= 0,
        "ui_integer_integral_required"
    );
    ensure!(
        n.cmp(&min) != std::cmp::Ordering::Less && n.cmp(&max) != std::cmp::Ordering::Greater,
        "ui_integer_out_of_bounds"
    );
    Ok(n.canonical())
}
/// Guard numbers or decimal strings; float ordering is exact, display is shortest.
pub fn ui_number(
    value: &Value,
    minimum: Option<&str>,
    maximum: Option<&str>,
    exclusive_minimum: bool,
) -> Result<String> {
    let n = binding_number(value)?;
    if let Some(min) = minimum {
        let cmp = n.order.cmp(&Decimal::parse(min)?);
        ensure!(
            if exclusive_minimum {
                cmp == std::cmp::Ordering::Greater
            } else {
                cmp != std::cmp::Ordering::Less
            },
            "ui_number_below_minimum"
        );
    }
    if let Some(max) = maximum {
        ensure!(
            n.order.cmp(&Decimal::parse(max)?) != std::cmp::Ordering::Greater,
            "ui_number_above_maximum"
        );
    }
    Ok(n.canonical)
}
/// Guard credential-free remote HTTPS while preserving the supplied source text.
pub fn ui_image(value: &Value) -> Result<ImageSource> {
    let source = rendered(value);
    validate_binding_image_source(&source)?;
    Ok(ImageSource(source))
}
fn validate_binding_image_source(value: &str) -> Result<()> {
    ensure!(value.len() <= 4096 && !value.is_empty(), "ui_image_length");
    ensure!(
        !value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
            && !value.contains('\\'),
        "ui_image_invalid_url"
    );
    let url = url::Url::parse(value).context("ui_image_invalid_url")?;
    ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "ui_image_https_required"
    );
    ensure!(
        url.host_str()
            .is_some_and(|host| !host.contains('*') && host.len() <= 253),
        "ui_image_host_invalid"
    );
    Ok(())
}
/// Register only the protocol's closed provider-neutral value capabilities.
pub fn install(environment: &mut minijinja::Environment<'_>) {
    environment.add_function(
        "ui_text",
        |value: Value, policy: String, minimum: u64, maximum: u64| {
            ui_text(&value, &policy, minimum, maximum).map_err(template_error)
        },
    );
    environment.add_function("ui_key", |value: Value| {
        ui_key(&value).map_err(template_error)
    });
    environment.add_function(
        "ui_integer",
        |value: Value, minimum: String, maximum: String| {
            ui_integer(&value, &minimum, &maximum).map_err(template_error)
        },
    );
    environment.add_function(
        "ui_number",
        |value: Value,
         minimum: Option<String>,
         maximum: Option<String>,
         exclusive_minimum: bool| {
            ui_number(
                &value,
                minimum.as_deref(),
                maximum.as_deref(),
                exclusive_minimum,
            )
            .map_err(template_error)
        },
    );
    environment.add_function("ui_compare", |left: Value, right: Value| {
        ui_compare(&left, &right).map_err(template_error)
    });
    environment.add_function("ui_image", |value: Value| {
        ui_image(&value)
            .map(minijinja::Value::from_object)
            .map_err(template_error)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_key_grammar_matches_literal_component_tokens() {
        assert_eq!(token("acct:42").unwrap(), "acct:42");
        for value in ["", "space key", "<markup>", "row\"", "{{expr}}", "line\n"] {
            assert!(token(value).is_err(), "{value:?}");
        }
        assert!(token(&"a".repeat(129)).is_err());
        for byte in 0_u8..=127 {
            let key = format!("key{}", char::from(byte));
            assert_eq!(
                token(&key).is_ok(),
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-:"
                    .contains(&byte),
                "ASCII byte {byte}"
            );
        }
        let mut environment = minijinja::Environment::empty();
        install(&mut environment);
        assert_eq!(
            environment
                .render_str(
                    "row-{{ ui_key(key) }}",
                    minijinja::context!(key => "account_42")
                )
                .unwrap(),
            "row-account_42"
        );
    }

    #[test]
    fn progress_preserves_integers_above_f64_precision() {
        let value = Value::from("9007199254740993");
        let maximum = Value::from("9007199254740994");
        assert_eq!(
            progress_value(&value, &maximum).unwrap(),
            "9007199254740993"
        );
        assert_eq!(
            progress_maximum(&value, &maximum).unwrap(),
            "9007199254740994"
        );
        assert!(!progress_complete(&value, &maximum).unwrap());
        assert!(progress(&Value::from(2), &Value::from("1.5")).is_err());
        assert!(progress(&Value::from("NaN"), &Value::from(1)).is_err());
        assert!(progress(
            &Value::from("9007199254740993"),
            &Value::from("9007199254740992.0")
        )
        .is_err());
    }

    #[test]
    fn enum_text_and_image_guards_remain_closed() {
        assert_eq!(button_variant(&Value::from(0)).unwrap(), "primary");
        assert!(button_variant(&Value::from(u64::MAX)).is_err());
        assert!(button_variant(&Value::from(1.0)).is_err());
        assert_eq!(text("hello").unwrap(), "hello");
        assert!(text(" \n").is_err());
        assert_eq!(plain("").unwrap(), "");
        assert!(plain("x\n").is_err());
        assert_eq!(field_text("a\nb").unwrap(), "a\nb");
        assert!(field_text(" \n\t").is_err());
        assert_eq!(
            image("https://cdn.example.test/a.png").unwrap().0,
            "https://cdn.example.test/a.png"
        );
        for invalid in [
            "http://cdn.example.test/a",
            "https://u:p@cdn.example.test/a",
            " https://cdn.example.test/a",
            "https://x.test/a\\b",
        ] {
            assert!(image(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn install_registers_generic_protocol_helpers() {
        let mut environment = minijinja::Environment::empty();
        install(&mut environment);
        assert_eq!(
            environment
                .render_str(
                    r#"{{ ui_text(value, "nonblank", 1, 3) }}"#,
                    minijinja::context!(value => "OK")
                )
                .unwrap(),
            "OK"
        );
        assert_eq!(
            environment
                .render_str("{{ ui_number(1, 0, 2, false) }}", ())
                .unwrap(),
            "1"
        );
        assert_eq!(
            environment
                .render_str("{{ ui_compare(1, 2) }}", ())
                .unwrap(),
            "-1"
        );
        assert!(environment
            .render_str("{{ ui_integer(4, 0, 3) }}", ())
            .is_err());
        assert_eq!(
            environment
                .render_str(r#"{% if ui_integer(2, 0, 3) == "0" %}primary{% elif ui_integer(2, 0, 3) == "1" %}secondary{% elif ui_integer(2, 0, 3) == "2" %}quiet{% else %}danger{% endif %}"#, ())
                .unwrap(),
            "quiet"
        );
        assert!(environment
            .render_str(
                r#"{% if ui_integer(4, 0, 3) == "0" %}primary{% else %}secondary{% endif %}"#,
                ()
            )
            .is_err());
        assert!(environment
            .render_str(r#"{{ cui_text("legacy") }}"#, ())
            .is_err());
        assert!(environment
            .render_str(r#"{{ ui_key("bad:key") }}"#, ())
            .is_err());
    }
}
