use std::collections::BTreeMap;

use catalog_core::expansion::{self, Binding, Package};

fn package_assets() -> BTreeMap<String, Vec<u8>> {
    fn collect(
        root: &std::path::Path,
        base: &std::path::Path,
        assets: &mut BTreeMap<String, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                collect(&entry.path(), base, assets);
            } else {
                let key = entry
                    .path()
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                assets.insert(key, std::fs::read(entry.path()).unwrap());
            }
        }
    }
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let mut assets = BTreeMap::new();
    collect(&base, &base, &mut assets);
    assets
}

fn bindings(input: &str) -> Vec<Binding> {
    expansion::expand(input, &Package::from_assets(&package_assets()).unwrap())
        .unwrap()
        .bindings
}

fn binding(field_path: &str, expected_kind: &str, component: &str, attribute: &str) -> Binding {
    Binding {
        field_path: field_path.into(),
        expected_kind: expected_kind.into(),
        component: component.into(),
        attribute: attribute.into(),
    }
}

#[test]
fn records_whole_page_header_title_and_integer_button_variant() {
    let package = Package::from_assets(&package_assets()).unwrap();
    let expanded = expansion::expand(
        r#"<cui-button label="{{ page.label }}" variant="{{ page.button_variant }}" size="{{ page.button_size }}"/>"#,
        &package,
    )
    .unwrap();
    assert!(expanded
        .html
        .contains(r#"ui_integer(page.button_variant, 0, 3) == "0""#));
    assert!(expanded
        .html
        .contains(r#"ui_integer(page.button_size, 0, 1) == "0""#));
    assert!(expanded
        .html
        .contains(r#"ui_text(page.label, "nonblank", 1, 0)"#));
    assert!(!expanded.html.contains("cui_"));
    assert_eq!(
        bindings(
            r#"<cui-page-header title="{{ page.title }}"/><cui-button label="Save" variant="{{ page.button_variant }}"/>"#
        ),
        vec![
            binding("page.title", "string", "page-header", "title"),
            binding("page.button_variant", "integer", "button", "variant"),
        ]
    );
}

#[test]
fn records_nested_button_bindings_inside_filter_bar_and_table_cell_once() {
    let input = r#"<cui-filter-bar label="Filters"><cui-slot name="controls"><cui-button label="{{ filters.apply_label }}"/></cui-slot></cui-filter-bar><cui-data-table caption="Rows" columns='["Action"]'><cui-table-row id="row-1"><cui-table-cell><cui-button label="{{ rows.action_label }}"/></cui-table-cell></cui-table-row></cui-data-table>"#;
    assert_eq!(
        bindings(input),
        vec![
            binding("filters.apply_label", "string", "button", "label"),
            binding("rows.action_label", "string", "button", "label"),
        ]
    );
}

#[test]
fn records_validated_select_crumb_and_page_item_fields_once() {
    let input = r#"<cui-select-field id="status" name="status" label="Status"><cui-option value="{{ options.value }}" label="{{ options.label }}"/></cui-select-field><cui-breadcrumbs label="Path"><cui-crumb label="{{ crumbs.parent }}" href="/home"/><cui-crumb label="Here" current="true"/></cui-breadcrumbs><cui-pagination label="Pages"><cui-page kind="page" label="{{ pages.label }}" href="/page/1"/><cui-page kind="current" label="Current" current="true"/></cui-pagination>"#;
    let actual = bindings(input);
    assert_eq!(
        actual,
        vec![
            binding("options.label", "string", "option", "label"),
            binding("options.value", "string", "option", "value"),
            binding("crumbs.parent", "string", "crumb", "label"),
            binding("pages.label", "string", "page", "label"),
        ]
    );
    for expected in &actual {
        assert_eq!(
            actual.iter().filter(|actual| *actual == expected).count(),
            1
        );
    }
}

#[test]
fn progress_and_choice_bindings_use_only_generic_abi2_capabilities() {
    let package = Package::from_assets(&package_assets()).unwrap();
    let progress = expansion::expand(
        r#"<cui-progress label="Load" state="determinate" value="{{ page.value }}" maximum="{{ page.maximum }}"/>"#,
        &package,
    )
    .unwrap();
    assert!(progress
        .html
        .contains(r#"ui_number(page.value, "0", page.maximum, false)"#));
    assert!(progress
        .html
        .contains(r#"ui_number(page.maximum, "0", none, true)"#));
    assert!(progress
        .html
        .contains("ui_compare(page.value, page.maximum) == 0"));
    assert!(!progress.html.contains("cui_progress"));
    let large_literal = expansion::expand(
        r#"<cui-progress label="Load" state="determinate" value="18446744073709551615" maximum="18446744073709551615"/>"#,
        &package,
    )
    .unwrap();
    assert!(large_literal
        .html
        .contains(r#"ui_number("18446744073709551615", "0", "18446744073709551615", false)"#));
    assert!(large_literal
        .html
        .contains(r#"ui_compare("18446744073709551615", "18446744073709551615") == 0"#));

    let controls = expansion::expand(
        r#"<cui-select-field id="state" name="state" label="State" selected="{{ page.selected }}"><cui-choice value="done" label="Done"/></cui-select-field><cui-breadcrumbs label="Path"><cui-crumb label="Home" href="/"/><cui-crumb label="Current" current="true"/></cui-breadcrumbs>"#,
        &package,
    )
    .unwrap();
    assert!(controls.html.contains(r#"data-ui-choice-set="true""#));
    assert!(controls
        .html
        .contains(r#"data-ui-choice-value="{{ ui_text(page.selected, "plain", 0, 0) }}""#));
    assert!(controls.html.contains(r#"data-ui-navigation="true""#));
    assert!(controls
        .html
        .contains(r#"data-ui-navigation-minimum-items="2""#));
    assert!(controls
        .html
        .contains(r#"data-ui-navigation-current-last="true""#));
    assert!(controls
        .html
        .contains(r#"data-ui-navigation-ancestor-links="true""#));
    assert!(!controls.html.contains("data-cui-navigation"));
}

#[test]
fn malformed_or_unknown_attributes_with_bindings_are_rejected() {
    let package = Package::from_assets(&package_assets()).unwrap();
    for input in [
        r#"<cui-button label="Save" unknown="{{ page.value }}"/>"#,
        r#"<cui-button label="{{ page.title }"/>"#,
        r#"<cui-page-header title="prefix {{ page.title }}"/>"#,
        r#"<cui-breadcrumbs label="Path"><cui-crumb label="Home" href="{{ routes.home }}" bad="{{ page.bad }}"/><cui-crumb label="Here" current="true"/></cui-breadcrumbs>"#,
    ] {
        assert!(
            expansion::expand(input, &package).is_err(),
            "accepted {input}"
        );
    }
}
