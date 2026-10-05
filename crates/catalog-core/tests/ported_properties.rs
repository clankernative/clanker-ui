use catalog_core::properties::{
    describe_properties, example_properties, example_properties_with_icons, validate_properties,
    validate_properties_with_icons,
};
use serde_json::json;
use std::collections::BTreeMap;

fn icons() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap()
}

#[test]
fn six_ports_have_typed_metadata_and_locked_icon_examples() {
    let icons = icons();
    for name in [
        "modal",
        "drawer",
        "popover",
        "tooltip",
        "toast",
        "theme-switcher",
    ] {
        assert_eq!(describe_properties(name).unwrap().name, name);
        let example = example_properties_with_icons(name, &icons)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(example.is_object());
    }
    for name in ["popover", "tooltip", "toast", "theme-switcher"] {
        assert!(
            example_properties(name).is_err(),
            "{name} requires caller-locked geometry"
        );
        assert!(validate_properties(name, json!({})).is_err());
    }
    assert!(example_properties("modal").is_ok());
    assert!(example_properties("drawer").is_ok());
}

#[test]
fn fixed_editor_ids_and_typed_values_are_strict() {
    let icons = icons();
    let modal = json!({"title":"Review","triggerLabel":"Open review","fallbackHref":"/review"});
    let normalized = validate_properties("modal", modal.clone()).unwrap();
    assert_eq!(normalized["id"], "editor-modal");
    let mut supplied_id = modal;
    supplied_id["id"] = json!("caller-id");
    assert!(validate_properties("modal", supplied_id).is_err());

    let popover = json!({"label":"More","leadingIcon":"not-in-the-catalog"});
    assert!(validate_properties_with_icons("popover", popover, &icons).is_err());
    let tooltip = json!({"triggerLabel":"Help","title":"More","placement":"center"});
    assert!(validate_properties_with_icons("tooltip", tooltip, &icons).is_err());

    let mut toast = json!({"label":"Updates","messages":[{"title":"Saved","body":"Done","tone":"success","actionHref":"/saved"}]});
    assert!(validate_properties_with_icons("toast", toast.clone(), &icons).is_err());
    toast["messages"][0]["actionLabel"] = json!("View");
    toast["messages"][0]["timeout"] = json!(999);
    assert!(validate_properties_with_icons("toast", toast, &icons).is_err());

    let mut theme = json!({"label":"Theme","choices":[{"value":"light","label":"Light","icon":"sun"},{"value":"dark","label":"Dark","icon":"moon"}],"defaultChoice":"contrast","systemLightTheme":"light","systemDarkTheme":"dark"});
    assert!(validate_properties_with_icons("theme-switcher", theme.clone(), &icons).is_err());
    theme["defaultChoice"] = json!("light");
    assert!(validate_properties_with_icons("theme-switcher", theme, &icons).is_ok());
}

#[test]
fn descriptors_keep_named_slots_outside_component_options() {
    for (name, slots) in [
        ("modal", vec![("body", true), ("actions", false)]),
        ("drawer", vec![("body", true), ("actions", false)]),
        ("popover", vec![("body", true)]),
    ] {
        let descriptor = describe_properties(name).unwrap();
        let actual: Vec<_> = descriptor
            .slots
            .iter()
            .map(|slot| (slot.name.as_str(), slot.required))
            .collect();
        assert_eq!(actual, slots);
    }
}
