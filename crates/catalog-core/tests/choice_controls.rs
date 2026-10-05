#[path = "../src/checkbox_group.rs"]
mod checkbox_group;
#[path = "../src/radio_group.rs"]
mod radio_group;
#[path = "../src/toggle.rs"]
mod toggle;
mod fragment {
    pub use catalog_core::fragment::*;
}

const CHECKBOX: &str =
    include_str!("../../../packages/vanilla/components/checkbox-group/fragment.html");
const RADIO: &str = include_str!("../../../packages/vanilla/components/radio-group/fragment.html");
const TOGGLE: &str = include_str!("../../../packages/vanilla/components/toggle/fragment.html");

#[test]
fn fixture_states_render_deterministically_and_keep_native_semantics() {
    for fixture in [
        include_str!("../../../packages/vanilla/components/checkbox-group/fixtures/typical.json"),
        include_str!(
            "../../../packages/vanilla/components/checkbox-group/fixtures/validation.json"
        ),
        include_str!("../../../packages/vanilla/components/checkbox-group/fixtures/disabled.json"),
        include_str!("../../../packages/vanilla/components/checkbox-group/fixtures/inline.json"),
    ] {
        let value = checkbox_group::CheckboxGroupInstance::parse(fixture).unwrap();
        assert_eq!(
            checkbox_group::render(&value, CHECKBOX).unwrap(),
            checkbox_group::render(&value, CHECKBOX).unwrap()
        );
    }
    let typical = checkbox_group::CheckboxGroupInstance::parse(include_str!(
        "../../../packages/vanilla/components/checkbox-group/fixtures/typical.json"
    ))
    .unwrap();
    let html = checkbox_group::render(&typical, CHECKBOX).unwrap();
    assert!(html.contains("<fieldset id=\"notification-options\""));
    assert!(html.contains("name=\"notifications\" value=\"failures\" checked"));
    assert!(html.contains("notification-options-choice-1-hint"));
    assert!(html.contains("aria-describedby=\"notification-options-description\""));
    let invalid = checkbox_group::CheckboxGroupInstance::parse(include_str!(
        "../../../packages/vanilla/components/checkbox-group/fixtures/validation.json"
    ))
    .unwrap();
    let html = checkbox_group::render(&invalid, CHECKBOX).unwrap();
    assert!(html.contains("<fieldset id=\"scope\""));
    assert!(html.contains("id=\"scope\" class=\"cui-checkbox-group cui-checkbox-group--stacked cui-checkbox-group--invalid\" data-cui-component=\"checkbox-group\" aria-invalid=\"true\" aria-describedby=\"scope-description scope-error\""));
    assert!(html.contains("value=\"billing\" disabled"));
    assert!(html.contains("scope-error"));
    let disabled = checkbox_group::CheckboxGroupInstance::parse(include_str!(
        "../../../packages/vanilla/components/checkbox-group/fixtures/disabled.json"
    ))
    .unwrap();
    assert!(checkbox_group::render(&disabled, CHECKBOX)
        .unwrap()
        .contains(" disabled"));

    for fixture in [
        include_str!("../../../packages/vanilla/components/radio-group/fixtures/typical.json"),
        include_str!("../../../packages/vanilla/components/radio-group/fixtures/validation.json"),
        include_str!("../../../packages/vanilla/components/radio-group/fixtures/disabled.json"),
        include_str!("../../../packages/vanilla/components/radio-group/fixtures/inline.json"),
    ] {
        let value = radio_group::RadioGroupInstance::parse(fixture).unwrap();
        assert_eq!(
            radio_group::render(&value, RADIO).unwrap(),
            radio_group::render(&value, RADIO).unwrap()
        );
    }
    let typical = radio_group::RadioGroupInstance::parse(include_str!(
        "../../../packages/vanilla/components/radio-group/fixtures/typical.json"
    ))
    .unwrap();
    let html = radio_group::render(&typical, RADIO).unwrap();
    assert!(html.contains("type=\"radio\""));
    assert!(html.contains("name=\"retention\" value=\"90\" checked required"));
    assert!(html.contains("retention-period-choice-2-hint"));
    assert!(html.contains("id=\"retention-period\""));
    let validation = radio_group::RadioGroupInstance::parse(include_str!(
        "../../../packages/vanilla/components/radio-group/fixtures/validation.json"
    ))
    .unwrap();
    let html = radio_group::render(&validation, RADIO).unwrap();
    assert!(html.contains("release-mode-error"));
    assert!(html.contains("aria-invalid=\"true\""));
    assert!(html.contains("cui-radio-group--stacked"));
    let disabled = radio_group::RadioGroupInstance::parse(include_str!(
        "../../../packages/vanilla/components/radio-group/fixtures/disabled.json"
    ))
    .unwrap();
    assert!(radio_group::render(&disabled, RADIO)
        .unwrap()
        .contains(" disabled"));

    for fixture in [
        include_str!("../../../packages/vanilla/components/toggle/fixtures/typical.json"),
        include_str!("../../../packages/vanilla/components/toggle/fixtures/unchecked.json"),
        include_str!("../../../packages/vanilla/components/toggle/fixtures/validation.json"),
        include_str!("../../../packages/vanilla/components/toggle/fixtures/disabled.json"),
    ] {
        let value = toggle::ToggleInstance::parse(fixture).unwrap();
        assert_eq!(
            toggle::render(&value, TOGGLE).unwrap(),
            toggle::render(&value, TOGGLE).unwrap()
        );
    }
    let typical = toggle::ToggleInstance::parse(include_str!(
        "../../../packages/vanilla/components/toggle/fixtures/typical.json"
    ))
    .unwrap();
    let html = toggle::render(&typical, TOGGLE).unwrap();
    assert!(html.contains("role=\"switch\""));
    assert!(html.contains("value=\"true\""));
    assert!(html.contains("id=\"weekly-digest\""));
    assert!(html.contains("checked"));
    let validation = toggle::ToggleInstance::parse(include_str!(
        "../../../packages/vanilla/components/toggle/fixtures/validation.json"
    ))
    .unwrap();
    let html = toggle::render(&validation, TOGGLE).unwrap();
    assert!(html.contains("aria-describedby=\"retention-confirm-error\""));
    assert!(html.contains("id=\"retention-confirm-description\""));
    assert!(html.contains("required"));
}

#[test]
fn validates_bounds_unique_choices_native_selected_state_and_reserved_ids() {
    let mut checks = checkbox_group::CheckboxGroupInstance::parse(
        r#"{"name":"scope","legend":"Scope","choices":[{"value":"a","label":"A"}]}"#,
    )
    .unwrap();
    checks.choices.push(checks.choices[0].clone());
    assert!(checks.validate().is_err());
    checks.choices.clear();
    assert!(checks.validate().is_err());
    checks.choices = (0..21)
        .map(|n| checkbox_group::Choice {
            value: n.to_string(),
            label: format!("Choice {n}"),
            hint: None,
            checked: false,
            disabled: false,
        })
        .collect();
    assert!(checks.validate().is_err());
    checks.choices.truncate(1);
    checks.id = Some("scope-choice-1".into());
    assert!(checks.validate().is_err());
    checks.id = Some("scope-error".into());
    assert!(checks.validate().is_err());

    let mut radios = radio_group::RadioGroupInstance::parse(r#"{"name":"mode","legend":"Mode","choices":[{"value":"a","label":"A"},{"value":"b","label":"B","disabled":true}]}"#).unwrap();
    radios.selected = Some("b".into());
    assert!(radios.validate().is_err());
    radios.selected = Some("missing".into());
    assert!(radios.validate().is_err());
    radios.selected = None;
    radios.choices.pop();
    assert!(radios.validate().is_err());
    radios.choices.push(radio_group::Choice {
        value: "a".into(),
        label: "Again".into(),
        hint: None,
        disabled: false,
    });
    assert!(radios.validate().is_err());
    radios.choices[1].value = "b".into();
    radios.id = Some("mode-description".into());
    assert!(radios.validate().is_err());
}

#[test]
fn rejects_unknown_props_bad_tokens_control_text_and_markup_fragment_slots() {
    for json in [
        r#"{"name":"x","legend":"X","choices":[{"value":"a","label":"A"}],"unknown":true}"#,
        r#"{"name":"x","legend":"X","choices":[{"value":"a","label":"A","bogus":true}]}"#,
        r#"{"name":"x","legend":"X","layout":"columns","choices":[{"value":"a","label":"A"}]}"#,
        r#"{"name":"_x","legend":"X","choices":[{"value":"a","label":"A"}]}"#,
        r#"{"name":"x","legend":"Bad\u0000text","choices":[{"value":"a","label":"A"}]}"#,
    ] {
        assert!(
            checkbox_group::CheckboxGroupInstance::parse(json).is_err(),
            "{json}"
        );
    }
    for json in [
        r#"{"name":"x","legend":"X","choices":[{"value":"a","label":"A"},{"value":"b","label":"B"}],"extra":1}"#,
        r#"{"name":"x","legend":"X","choices":[{"value":"a","label":"A","selected":true},{"value":"b","label":"B"}]}"#,
        r#"{"name":"x","legend":"X","layout":"grid","choices":[{"value":"a","label":"A"},{"value":"b","label":"B"}]}"#,
        r#"{"name":"x","legend":"X\nY","choices":[{"value":"a","label":"A"},{"value":"b","label":"B"}]}"#,
    ] {
        assert!(
            radio_group::RadioGroupInstance::parse(json).is_err(),
            "{json}"
        );
    }
    for json in [
        r#"{"name":"x","label":"X","bogus":false}"#,
        r#"{"name":"x","label":"X","hint":" "}"#,
        r#"{"name":"_x","label":"X"}"#,
        r#"{"name":"x","id":"x-hint","label":"X"}"#,
        r#"{"name":"x","label":"bad\u0000label"}"#,
        r#"{"name":"x","label":"X","required":"yes"}"#,
    ] {
        assert!(toggle::ToggleInstance::parse(json).is_err(), "{json}");
    }
    let toggle = toggle::ToggleInstance::parse(r#"{"name":"x","label":"X"}"#).unwrap();
    assert!(toggle::render(&toggle, "[[attributes]][[label]]").is_err());
    assert!(checkbox_group::render(
        &checkbox_group::CheckboxGroupInstance::parse(
            r#"{"name":"x","legend":"X","choices":[{"value":"a","label":"A"}]}"#
        )
        .unwrap(),
        "[[attributes]][[legend]][[choices]][[description]][[bad]]"
    )
    .is_err());
}

#[test]
fn json_html_goldens_cover_every_declared_state() {
    fn expected(json: &str) -> String {
        serde_json::from_str::<serde_json::Value>(json).unwrap()["expectedHtml"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    for (source, golden) in [
        (
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/typical.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/goldens/typical.json"
            ),
        ),
        (
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/validation.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/goldens/validation.json"
            ),
        ),
        (
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/disabled.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/goldens/disabled.json"
            ),
        ),
        (
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/inline.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/checkbox-group/fixtures/goldens/inline.json"
            ),
        ),
    ] {
        let i = checkbox_group::CheckboxGroupInstance::parse(source).unwrap();
        assert_eq!(
            checkbox_group::render(&i, CHECKBOX).unwrap(),
            expected(golden)
        );
    }
    for (source, golden) in [
        (
            include_str!("../../../packages/vanilla/components/radio-group/fixtures/typical.json"),
            include_str!(
                "../../../packages/vanilla/components/radio-group/fixtures/goldens/typical.json"
            ),
        ),
        (
            include_str!(
                "../../../packages/vanilla/components/radio-group/fixtures/validation.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/radio-group/fixtures/goldens/validation.json"
            ),
        ),
        (
            include_str!("../../../packages/vanilla/components/radio-group/fixtures/disabled.json"),
            include_str!(
                "../../../packages/vanilla/components/radio-group/fixtures/goldens/disabled.json"
            ),
        ),
        (
            include_str!("../../../packages/vanilla/components/radio-group/fixtures/inline.json"),
            include_str!(
                "../../../packages/vanilla/components/radio-group/fixtures/goldens/inline.json"
            ),
        ),
    ] {
        let i = radio_group::RadioGroupInstance::parse(source).unwrap();
        assert_eq!(radio_group::render(&i, RADIO).unwrap(), expected(golden));
    }
    for (source, golden) in [
        (
            include_str!("../../../packages/vanilla/components/toggle/fixtures/typical.json"),
            include_str!(
                "../../../packages/vanilla/components/toggle/fixtures/goldens/typical.json"
            ),
        ),
        (
            include_str!("../../../packages/vanilla/components/toggle/fixtures/unchecked.json"),
            include_str!(
                "../../../packages/vanilla/components/toggle/fixtures/goldens/unchecked.json"
            ),
        ),
        (
            include_str!("../../../packages/vanilla/components/toggle/fixtures/validation.json"),
            include_str!(
                "../../../packages/vanilla/components/toggle/fixtures/goldens/validation.json"
            ),
        ),
        (
            include_str!("../../../packages/vanilla/components/toggle/fixtures/disabled.json"),
            include_str!(
                "../../../packages/vanilla/components/toggle/fixtures/goldens/disabled.json"
            ),
        ),
    ] {
        let i = toggle::ToggleInstance::parse(source).unwrap();
        assert_eq!(toggle::render(&i, TOGGLE).unwrap(), expected(golden));
    }
}

#[test]
fn escapes_text_and_does_not_emit_transport_or_script() {
    let checks = checkbox_group::CheckboxGroupInstance::parse(
        r#"{"name":"prefs","legend":"A < B","choices":[{"value":"a&b\"","label":"A & <B>"}]}"#,
    )
    .unwrap();
    let html = checkbox_group::render(&checks, CHECKBOX).unwrap();
    assert!(html.contains("A &lt; B"));
    assert!(html.contains("A &amp; &lt;B&gt;"));
    assert!(html.contains("value=\"a&amp;b&quot;\""));
    let toggle =
        toggle::ToggleInstance::parse(r#"{"name":"x","label":"X","checked":false}"#).unwrap();
    let html = toggle::render(&toggle, TOGGLE).unwrap();
    assert!(!html.contains("checked"));
    assert!(!html.contains("hidden"));
    assert!(!html.contains("<script"));
}
