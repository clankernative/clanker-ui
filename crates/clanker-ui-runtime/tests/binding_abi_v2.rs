//! Runs the same versioned, typed vectors consumed independently by Native.
use clanker_ui_runtime::{install, ui_compare, ui_image, ui_integer, ui_key, ui_number, ui_text};
use minijinja::Value;
use serde::Deserialize;
use serde_json::Value as Json;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vectors {
    abi: String,
    version: u32,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    helper: String,
    args: Vec<Argument>,
    expect: Json,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Argument {
    #[serde(rename = "type")]
    kind: String,
    value: Option<String>,
}
impl Argument {
    fn value(&self) -> Value {
        let text = || self.value.as_deref().expect("typed argument payload");
        match self.kind.as_str() {
            "string" => Value::from(text()),
            "i64" => Value::from(text().parse::<i64>().unwrap()),
            "u64" => Value::from(text().parse::<u64>().unwrap()),
            "i128" => Value::from(text().parse::<i128>().unwrap()),
            "u128" => Value::from(text().parse::<u128>().unwrap()),
            "f64" => Value::from(text().parse::<f64>().unwrap()),
            "f64_bits" => {
                assert_eq!(text().len(), 16);
                Value::from(f64::from_bits(u64::from_str_radix(text(), 16).unwrap()))
            }
            "bool" => Value::from(text().parse::<bool>().unwrap()),
            "none" => {
                assert!(self.value.is_none());
                Value::from(())
            }
            other => panic!("unknown argument type {other}"),
        }
    }
    fn text(&self) -> &str {
        assert_eq!(self.kind, "string");
        self.value.as_deref().unwrap()
    }
    fn optional_text(&self) -> Option<&str> {
        if self.kind == "none" {
            assert!(self.value.is_none());
            None
        } else {
            Some(self.text())
        }
    }
}
fn evaluate(case: &Case) -> anyhow::Result<Json> {
    let a = &case.args;
    match case.helper.as_str() {
        "ui_text" => {
            assert_eq!(a.len(), 4);
            Ok(Json::from(ui_text(
                &a[0].value(),
                a[1].text(),
                a[2].value().to_string().parse()?,
                a[3].value().to_string().parse()?,
            )?))
        }
        "ui_key" => {
            assert_eq!(a.len(), 1);
            Ok(Json::from(ui_key(&a[0].value())?))
        }
        "ui_integer" => {
            assert_eq!(a.len(), 3);
            Ok(Json::from(ui_integer(
                &a[0].value(),
                a[1].text(),
                a[2].text(),
            )?))
        }
        "ui_number" => {
            assert_eq!(a.len(), 4);
            Ok(Json::from(ui_number(
                &a[0].value(),
                a[1].optional_text(),
                a[2].optional_text(),
                a[3].value().to_string().parse()?,
            )?))
        }
        "ui_compare" => {
            assert_eq!(a.len(), 2);
            Ok(Json::from(ui_compare(&a[0].value(), &a[1].value())?))
        }
        "ui_image" => {
            assert_eq!(a.len(), 1);
            Ok(Json::from(ui_image(&a[0].value())?.0))
        }
        other => panic!("unknown helper {other}"),
    }
}

#[test]
fn shared_binding_abi_v2_vectors() {
    let source = include_str!("../../../tests/protocol/ui-binding-abi-v2.json");
    assert!(source.len() <= 128 * 1024, "bounded corpus");
    let vectors: Vectors = serde_json::from_str(source).unwrap();
    assert_eq!(vectors.abi, "ui-binding");
    assert_eq!(vectors.version, 2);
    assert!(!vectors.cases.is_empty() && vectors.cases.len() <= 256);
    let mut ids = BTreeSet::new();
    for case in vectors.cases {
        assert!(ids.insert(case.id.clone()), "duplicate id {}", case.id);
        let expected = case.expect.as_object().unwrap();
        assert_eq!(expected.len(), 1, "one result discriminator");
        let actual = evaluate(&case);
        if let Some(ok) = expected.get("ok") {
            assert_eq!(actual.unwrap(), *ok, "{}", case.id);
        } else {
            let error = expected.get("error").unwrap().as_str().unwrap();
            assert_eq!(actual.unwrap_err().to_string(), error, "{}", case.id);
        }
    }
}

#[test]
fn installed_helpers_match_public_abi() {
    let mut environment = minijinja::Environment::empty();
    install(&mut environment);
    for (template, expected) in [
        (
            r#"{{ ui_integer(value, "-9007199254740994", "-9007199254740992") }}"#,
            "-9007199254740993",
        ),
        (
            r#"{{ ui_number(value, "-9007199254740994", none, false) }}"#,
            "-9007199254740993",
        ),
        (r#"{{ ui_compare(value, real) }}"#, "-1"),
        (r#"{{ ui_key(42) }}"#, "42"),
        (
            r#"{{ ui_text("x", "plain", 0, 18446744073709551615) }}"#,
            "x",
        ),
    ] {
        assert_eq!(
            environment.render_str(template, minijinja::context!(value => -9_007_199_254_740_993i64, real => -9_007_199_254_740_992f64)).unwrap(),
            expected,
            "{template}"
        );
    }
    for template in [
        r#"{{ ui_compare("1", 2) }}"#,
        r#"{{ ui_integer(1.0, "0", "2") }}"#,
        r#"{{ ui_number(0.1, none, "0.1", false) }}"#,
        r#"{{ ui_image("https://example.test/a b") }}"#,
    ] {
        assert!(environment.render_str(template, ()).is_err(), "{template}");
    }
    // The image guard preserves caller text instead of URL canonicalization.
    let image = environment
        .compile_expression(r#"ui_image("HTTPS://CDN.example.test:443/a")"#)
        .unwrap()
        .eval(())
        .unwrap();
    assert_eq!(
        image
            .downcast_object_ref::<clanker_ui_runtime::ImageSource>()
            .unwrap()
            .0,
        "HTTPS://CDN.example.test:443/a"
    );
}

#[test]
fn numeric_work_and_output_are_bounded() {
    for input in ["1e10000", "1e-10000", "-1e-10000"] {
        let canonical = ui_number(&Value::from(input), None, None, false).unwrap();
        assert!(canonical.len() <= 10_003);
    }
    assert!(ui_number(&Value::from("1e10001"), None, None, false).is_err());
    assert!(ui_number(&Value::from("1".repeat(257)), None, None, false).is_err());
    for real in [f64::MAX, f64::MIN, f64::MIN_POSITIVE, f64::from_bits(1)] {
        let canonical = ui_number(&Value::from(real), None, None, false).unwrap();
        assert!(canonical.len() <= 1200);
        assert_eq!(canonical.parse::<f64>().unwrap().to_bits(), real.to_bits());
        assert_eq!(
            ui_compare(&Value::from(real), &Value::from(real)).unwrap(),
            0
        );
    }
}

#[test]
fn exact_binary_fraction_bounds_and_integer_order() {
    for exponent in 0..=20u32 {
        for numerator in [-1_048_576i32, -127, -1, 0, 1, 127, 1_048_576] {
            let real = f64::from(numerator) / f64::from(1u32 << exponent);
            let coefficient = i128::from(numerator) * 5i128.pow(exponent);
            let decimal = format!("{coefficient}e-{exponent}");
            assert!(ui_number(&Value::from(real), Some(&decimal), Some(&decimal), false).is_ok());
        }
    }
    for integer in [
        i64::MIN,
        -9_007_199_254_740_993,
        -1,
        0,
        1,
        9_007_199_254_740_993,
        i64::MAX,
    ] {
        for other in [
            i64::MIN,
            -9_007_199_254_740_993,
            0,
            9_007_199_254_740_993,
            i64::MAX,
        ] {
            let expected = match integer.cmp(&other) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            assert_eq!(
                ui_compare(&Value::from(integer), &Value::from(other)).unwrap(),
                expected
            );
        }
    }
}
