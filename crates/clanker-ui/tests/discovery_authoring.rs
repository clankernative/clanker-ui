use catalog_core::{properties::describe_component_properties, Catalog};
use clanker_ui::{local::LocalPackage, native::NativeAdapter, ports::PackageSource};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn invoke(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_clanker-ui"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn discovery_forwards_component_owned_grammar_without_editable_html_properties() {
    let lock = root().join("examples/button-app/ui.lock.json");
    let lock = lock.to_str().unwrap();
    let exported = invoke(&["properties", "--lock", lock]);
    for name in [
        "data-table",
        "filter-bar",
        "select-field",
        "breadcrumbs",
        "pagination",
    ] {
        let manifest: Value = serde_json::from_slice(
            &fs::read(root().join(format!("packages/vanilla/components/{name}/component.json")))
                .unwrap(),
        )
        .unwrap();
        let expected = &manifest["templateAuthoring"];
        for command in ["describe", "context"] {
            let response = invoke(&[command, name, "--lock", lock]);
            assert_eq!(
                &response["data"]["component"]["templateAuthoring"],
                expected
            );
            assert_eq!(
                &response["data"]["properties"]["templateAuthoring"],
                expected
            );
            if name == "data-table" {
                assert_eq!(response["data"]["properties"]["slots"], json!([]));
                if command == "context" {
                    assert!(response["data"]["example"]["input"]["rows"][0]["cells"][0].is_string());
                }
            }
        }
        let entry = exported["data"]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap();
        assert_eq!(&entry["templateAuthoring"], expected);
        assert!(entry["fields"]
            .as_array()
            .unwrap()
            .iter()
            .all(|field| field["name"] != "templateAuthoring"));
    }
    let badge = invoke(&["describe", "badge", "--lock", lock]);
    assert!(badge["data"]["component"]
        .get("templateAuthoring")
        .is_none());
    assert!(badge["data"]["properties"]
        .get("templateAuthoring")
        .is_none());
}

#[test]
fn properties_projection_uses_metadata_rather_than_component_name_switches() {
    let package = LocalPackage.load(&root().join("packages/vanilla")).unwrap();
    let mut table = package.catalog.get("data-table").unwrap().clone();
    table
        .template_authoring
        .as_mut()
        .unwrap()
        .rules
        .push("App-specific authoring note for this test.".into());
    let descriptor = describe_component_properties(&table).unwrap();
    assert_eq!(descriptor.template_authoring, table.template_authoring);
    table.template_authoring = None;
    assert!(describe_component_properties(&table)
        .unwrap()
        .template_authoring
        .is_none());
    // A different component carries exactly the metadata the caller supplies;
    // this projection is not a five-component dispatch table.
    let mut button = package.catalog.get("button").unwrap().clone();
    let mut authoring = descriptor.template_authoring.unwrap();
    authoring.root_tag = "cui-button".into();
    authoring.elements = vec![catalog_core::authoring::TemplateElement {
        tag: "cui-button".into(),
        form: catalog_core::authoring::ElementForm::SelfClosing,
        allowed_parents: vec![],
        attributes: vec![catalog_core::authoring::TemplateAttribute {
            name: "label".into(),
            required: true,
            value_forms: vec![catalog_core::authoring::ValueForm::Literal],
            rules: vec![],
        }],
        content: catalog_core::authoring::ContentPolicy::None,
        allowed_children: vec![],
        control_flow: vec![],
        rules: vec![],
    }];
    authoring.examples[0].template = "<cui-button label=\"Continue\" />".into();
    authoring.validate("button", "native-html").unwrap();
    button.template_authoring = Some(authoring);
    assert_eq!(
        describe_component_properties(&button)
            .unwrap()
            .template_authoring,
        button.template_authoring
    );
}

#[test]
fn verify_rejects_an_authoring_example_with_invalid_row_shape() {
    let mut package = LocalPackage.load(&root().join("packages/vanilla")).unwrap();
    let mut components = package.catalog.components().cloned().collect::<Vec<_>>();
    let table = components
        .iter_mut()
        .find(|component| component.name == "data-table")
        .unwrap();
    table.template_authoring.as_mut().unwrap().examples[0].template =
        "<cui-data-table caption=\"Broken\" columns='[\"Name\",\"Status\"]'><cui-table-row id=\"one\"><cui-table-cell>Only one cell</cui-table-cell></cui-table-row></cui-data-table>".into();
    package.assets.insert(
        "components/data-table/component.json".into(),
        serde_json::to_vec(table).unwrap(),
    );
    package.catalog = Catalog::new(package.catalog.package.clone(), components).unwrap();
    let icons = serde_json::from_slice(&package.assets["icons.json"]).unwrap();
    let properties = NativeAdapter::property_catalog_with_icons(&package, &icons).unwrap();
    package.assets.insert(
        "property-catalog.json".into(),
        serde_json::to_vec(&properties).unwrap(),
    );
    let error = NativeAdapter::verify(&LocalPackage, &package).unwrap_err();
    assert!(
        error.contains("data-table authoring example rich-customers"),
        "{error}"
    );
    assert!(error.contains("cell count"), "{error}");
}

#[test]
fn rich_customer_example_renders_empty_and_composed_cells_with_escaped_app_data() {
    let package = LocalPackage.load(&root().join("packages/vanilla")).unwrap();
    let example = &package
        .catalog
        .get("data-table")
        .unwrap()
        .template_authoring
        .as_ref()
        .unwrap()
        .examples[0];
    let temp = tempfile::tempdir().unwrap();
    let ui = temp.path().join("ui");
    fs::create_dir_all(ui.join("pages")).unwrap();
    fs::write(ui.join("app.css"), "").unwrap();
    fs::write(ui.join("pages/index.html"), &example.template).unwrap();
    let scene = temp.path().join("scene.json");
    let lock = root().join("examples/button-app/ui.lock.json");
    for customers in [
        json!([]),
        json!([
            {"id":"customer-1", "name":"<script>danger</script>", "email":"one&two@example.test", "active":true, "has_scheme":true},
            {"id":"customer-2", "name":"Inactive", "email":"two@example.test", "active":false, "has_scheme":false}
        ]),
    ] {
        fs::write(&scene, serde_json::to_vec(&json!({"page":"pages/index.html", "data":{"customers":customers}, "routes":{"customer_show":"/customers/detail"}})).unwrap()).unwrap();
        let preview = clanker_ui::preview::render(&lock, &ui, &scene, None).unwrap();
        let html = &preview.html;
        assert!(!html.contains("<cui-"));
        if customers.as_array().unwrap().is_empty() {
            assert!(html.contains("id=\"customers-empty\""));
            assert!(html.contains("No customers"));
            assert_eq!(html.matches("<td ").count(), 3);
        } else {
            assert!(html.contains("id=\"customer-customer-1\""));
            assert!(html.contains("&lt;script&gt;danger&lt;/script&gt;"));
            assert!(html.contains("one&amp;two@example.test"));
            // Preview navigation is intentionally fake; Native owns route binding.
            assert!(html.contains("href=\"/customers/detail?id=customer-1\""));
            assert!(html.contains("cui-badge--success"));
            assert!(html.contains("cui-badge--neutral"));
            assert!(html.contains("Scheme active"));
            assert!(html.contains("No scheme"));
            assert!(!html.contains("id=\"customers-empty\""));
            assert_eq!(html.matches("<td ").count(), 6);
        }
    }
}
