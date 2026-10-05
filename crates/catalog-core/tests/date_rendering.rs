use catalog_core::{
    date::Date,
    date_calendar::{CalendarInstance, Selection},
    date_picker::{PickerInstance, Presentation, Value},
};
use std::collections::{BTreeMap, BTreeSet};

fn calendar() -> CalendarInstance {
    serde_json::from_str(include_str!(
        "../../../packages/vanilla/components/date-calendar/fixtures/typical.json"
    ))
    .unwrap()
}
fn picker() -> PickerInstance {
    serde_json::from_str(include_str!(
        "../../../packages/vanilla/components/date-picker/fixtures/single.json"
    ))
    .unwrap()
}
fn emitted_ids(html: &str) -> BTreeSet<String> {
    html.split(" id=\"")
        .skip(1)
        .map(|part| part.split('"').next().unwrap().to_owned())
        .collect()
}
fn render_picker(value: &PickerInstance) -> String {
    value.render("[[picker]]", "[[calendar]]").unwrap()
}

#[test]
fn calendar_keeps_closed_json_label_arrays_and_exposes_constraints() {
    let mut value = calendar();
    value.month_labels[0] = "Jan|\"<>&".into();
    let html = value.render("[[calendar]]").unwrap();
    let attribute = html
        .split("data-cui-date-month-labels=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let decoded = attribute
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let labels: Vec<String> = serde_json::from_str(&decoded).unwrap();
    assert_eq!(labels, value.month_labels);
    assert!(html.contains("data-cui-date-minimum=\"2026-08-01\""));
    assert!(html.contains("data-cui-date-maximum=\"2026-09-30\""));
    assert_eq!(emitted_ids(&html), value.derived_ids());
}

#[test]
fn calendar_domain_edges_have_blank_cells_not_fabricated_dates() {
    for month in ["0001-01-01", "9999-12-01"] {
        let mut value = calendar();
        value.month = Date::parse(month).unwrap();
        value.today = value.month;
        value.minimum = None;
        value.maximum = None;
        value.unavailable_dates.clear();
        value.selection = Selection::None;
        value.first_day = 0;
        let html = value.render("[[calendar]]").unwrap();
        assert_eq!(html.matches("<td ").count(), 42);
        assert!(html.contains("aria-hidden=\"true\"></td>"));
        assert!(!html.contains("1970-") && !html.contains("0000-") && !html.contains("10000-"));
        assert_eq!(html.matches("tabindex=\"0\"").count(), 1);
        assert_eq!(html.matches("aria-current=\"date\"").count(), 1);
        let direction = if month.starts_with("0001") {
            "previous"
        } else {
            "next"
        };
        assert!(html.contains(&format!(
            "data-cui-date-{direction} aria-label=\"{}\" disabled",
            if direction == "previous" {
                &value.previous_label
            } else {
                &value.next_label
            }
        )));
    }
}

#[test]
fn calendar_has_an_enabled_roving_stop_even_without_visible_today_or_selection() {
    let mut value = calendar();
    value.today = Date::parse("2000-01-01").unwrap();
    value.selection = Selection::None;
    assert_eq!(
        value
            .render("[[calendar]]")
            .unwrap()
            .matches("tabindex=\"0\"")
            .count(),
        1
    );
    value.disabled = true;
    assert_eq!(
        value
            .render("[[calendar]]")
            .unwrap()
            .matches("tabindex=\"0\"")
            .count(),
        0
    );
}

#[test]
fn date_navigation_uses_only_locked_closed_icons_when_a_context_is_supplied() {
    let value = calendar();
    let icons: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
    let html = value.render_with_icons("[[calendar]]", &icons).unwrap();
    assert_eq!(html.matches("<svg ").count(), 2);
    assert!(html.contains(&icons["chevron-left"]));
    assert!(html.contains(&icons["chevron-right"]));
    assert!(!html.contains('‹') && !html.contains('›'));
    assert!(value
        .render_with_icons("[[calendar]]", &BTreeMap::new())
        .is_err());
}

#[test]
fn picker_reserves_every_emitted_id_and_marks_native_error_inputs() {
    for presentation in [Presentation::Inline, Presentation::Dropdown] {
        for range in [false, true] {
            let mut value = picker();
            value.presentation = presentation;
            if range {
                value.name = None;
                value.start_name = Some("start".into());
                value.end_name = Some("end".into());
                value.value = Value::Range {
                    start: None,
                    end: None,
                };
            } else {
                value.name = Some("date".into());
                value.start_name = None;
                value.end_name = None;
                value.value = Value::Single { date: None };
            }
            value.error = Some("Choose an available date".into());
            value.hint = Some("App validation remains authoritative".into());
            let html = render_picker(&value);
            assert_eq!(emitted_ids(&html), value.derived_ids());
            assert_eq!(
                html.matches("aria-invalid=\"true\"").count(),
                if range { 2 } else { 1 }
            );
            assert_eq!(
                html.matches("type=\"date\"").count(),
                if range { 2 } else { 1 }
            );
            value.error = None;
            value.hint = None;
            let html = render_picker(&value);
            assert_eq!(emitted_ids(&html), value.derived_ids());
            assert!(!html.contains("aria-invalid=\"true\""));
        }
    }
}

#[test]
fn failed_programmatic_dates_cannot_enter_rendering() {
    assert!(Date::new(0, 1, 1).is_err());
    assert!(Date::new(10_000, 1, 1).is_err());
    assert!(Date::new(2026, 2, 29).is_err());
}
