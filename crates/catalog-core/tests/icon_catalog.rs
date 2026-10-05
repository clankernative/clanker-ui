use catalog_core::icon::IconCatalog;
use std::collections::BTreeMap;

fn geometries() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap()
}

fn metadata() -> IconCatalog {
    IconCatalog::parse(
        include_bytes!("../../../packages/vanilla/icon-catalog.json"),
        &geometries(),
    )
    .unwrap()
}

#[test]
fn package_metadata_matches_all_geometry_names_and_toolframe_categories() {
    let catalog = metadata();
    let geometries = geometries();
    let actual: Vec<_> = catalog
        .icons
        .iter()
        .map(|icon| icon.name.as_str())
        .collect();
    let expected: Vec<_> = geometries.keys().map(String::as_str).collect();
    assert_eq!(catalog.schema_version, 1);
    assert_eq!(actual, expected);
    assert_eq!(catalog.icons.len(), 100);
    assert_eq!(
        catalog
            .categories
            .iter()
            .map(|category| (category.name.as_str(), category.label.as_str()))
            .collect::<Vec<_>>(),
        [
            ("actions", "Actions"),
            ("communication", "Communication"),
            ("data", "Data"),
            ("navigation", "Navigation"),
            ("objects", "Objects"),
            ("status", "Status"),
            ("system-tools", "System"),
        ]
    );
    assert_eq!(catalog.lookup("cpu").unwrap().label, "Processor");
    assert_eq!(catalog.category_label("system-tools"), Some("System"));
}

#[test]
fn find_is_literal_case_insensitive_across_name_label_and_category() {
    let catalog = metadata();
    assert_eq!(catalog.find("ARROW").len(), 4);
    assert_eq!(catalog.find("processor")[0].name, "cpu");
    assert_eq!(catalog.find("COMMUNICATION").len(), 11);
    assert!(catalog.find("arrow*").is_empty());
}

#[test]
fn parse_rejects_unknown_schema_fields_names_and_invalid_metadata() {
    let geometry = BTreeMap::from([("alpha".to_string(), "<path/>".to_string())]);
    let valid = r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"actions"}]}"#;
    assert!(IconCatalog::parse(valid.as_bytes(), &geometry).is_ok());

    for invalid in [
        r#"{"schemaVersion":2,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"actions"}]}"#,
        r#"{"schemaVersion":1,"extra":true,"categories":[],"icons":[]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions","extra":true}],"icons":[]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":" ","category":"actions"}]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"missing"}]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"},{"name":"actions","label":"Duplicate"}],"icons":[{"name":"alpha","label":"Alpha","category":"actions"}]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"actions"},{"name":"alpha","label":"Duplicate","category":"actions"}]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"extra","label":"Extra","category":"actions"}]}"#,
        r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"actions"},{"name":"beta","label":"Beta","category":"actions"}]}"#,
    ] {
        assert!(
            IconCatalog::parse(invalid.as_bytes(), &geometry).is_err(),
            "{invalid}"
        );
    }
}
