use catalog_core::authoring::{
    ContentPolicy, ElementForm, TemplateAttribute, TemplateAuthoring, TemplateElement,
    TemplateExample, ValueForm,
};
use catalog_core::Component;

fn valid_metadata() -> TemplateAuthoring {
    TemplateAuthoring {
        schema_version: 1,
        root_tag: "cui-example".into(),
        elements: vec![TemplateElement {
            tag: "cui-example".into(),
            form: ElementForm::Paired,
            allowed_parents: vec![],
            attributes: vec![TemplateAttribute {
                name: "label".into(),
                required: true,
                value_forms: vec![ValueForm::StringField, ValueForm::Literal],
                rules: vec!["Literal text may contain quotes, {braces}, and café.".into()],
            }],
            content: ContentPolicy::None,
            allowed_children: vec![],
            control_flow: vec![],
            rules: vec![],
        }],
        rules: vec!["Use the host's ordinary admission path.".into()],
        examples: vec![TemplateExample {
            name: "basic".into(),
            summary: "A literal with braces {x}, quotes \"ok\", and Unicode — café.".into(),
            template: "<cui-example label=\"{ \\\"quoted\\\" } — café\"></cui-example>".into(),
            requirements: vec!["Do not interpret this template as executable metadata.".into()],
        }],
    }
}

fn component_json(metadata: Option<serde_json::Value>) -> Vec<u8> {
    let mut component = serde_json::json!({
        "schemaVersion": 1,
        "name": "example",
        "status": "ready",
        "target": "native-html",
        "category": "content",
        "summary": "An example component",
        "keywords": [],
        "agent": {"useWhen": "Use it", "avoidWhen": "Avoid it", "composeWith": []},
        "contract": {"role": "content", "variants": ["default"], "requiredFields": [], "invariants": ["Static"]},
        "tokens": [],
        "dependencies": [],
        "assets": {"template": "components/example/template.html", "styles": "components/example/styles.css", "scripts": []},
        "fixtures": []
    });
    if let Some(metadata) = metadata {
        component["templateAuthoring"] = metadata;
    }
    serde_json::to_vec(&component).unwrap()
}

#[test]
fn authoring_roundtrips_and_component_parse_validates_it() {
    let metadata = valid_metadata();
    metadata.validate("example", "native-html").unwrap();
    let encoded = serde_json::to_vec(&metadata).unwrap();
    let decoded: TemplateAuthoring = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, metadata);
    assert!(decoded.examples[0]
        .template
        .contains("{ \\\"quoted\\\" } — café"));

    let parsed = Component::parse(&component_json(Some(
        serde_json::to_value(metadata).unwrap(),
    )))
    .unwrap();
    assert!(parsed.template_authoring.is_some());
}

#[test]
fn omitted_authoring_remains_compatible_and_is_not_serialized() {
    let parsed = Component::parse(&component_json(None)).unwrap();
    assert_eq!(parsed.template_authoring, None);
    let encoded = serde_json::to_value(parsed).unwrap();
    assert!(encoded.get("templateAuthoring").is_none());
}

#[test]
fn rejects_identity_content_and_closed_control_flow_drift() {
    let metadata = valid_metadata();
    assert!(metadata.validate("other", "native-html").is_err());
    assert!(metadata.validate("example", "react").is_err());
    let mut changed = metadata.clone();
    changed.schema_version = 2;
    assert!(changed.validate("example", "native-html").is_err());
    for flow in ["include", "macro", "set", "while", "import"] {
        let mut changed = metadata.clone();
        changed.elements[0].content = ContentPolicy::HostAdmitted;
        changed.elements[0].control_flow = vec![flow.into()];
        assert!(changed.validate("example", "native-html").is_err());
    }
    let mut changed = metadata.clone();
    changed.elements[0].form = ElementForm::SelfClosing;
    changed.elements[0].content = ContentPolicy::HostAdmitted;
    assert!(changed.validate("example", "native-html").is_err());
    let mut changed = metadata.clone();
    changed.examples[0].name = "not an identifier".into();
    assert!(changed.validate("example", "native-html").is_err());
    let mut changed = metadata.clone();
    changed.rules = vec!["a\nmultiline rule".into()];
    assert!(changed.validate("example", "native-html").is_err());
    for key in ["form", "content"] {
        let mut value = serde_json::to_value(&metadata).unwrap();
        value["elements"][0][key] = serde_json::json!("arbitrary");
        assert!(serde_json::from_value::<TemplateAuthoring>(value).is_err());
    }
}

#[test]
fn grammar_edges_agree_and_helpers_are_reachable() {
    let mut metadata = valid_metadata();
    let mut helper = metadata.elements[0].clone();
    helper.tag = "cui-child".into();
    helper.allowed_parents = vec!["cui-example".into()];
    metadata.elements.push(helper);
    assert!(metadata.validate("example", "native-html").is_err());
    metadata.elements[0].content = ContentPolicy::Helpers;
    metadata.elements[0].allowed_children = vec!["cui-child".into()];
    metadata.validate("example", "native-html").unwrap();
    metadata.elements[1].allowed_parents.clear();
    assert!(metadata.validate("example", "native-html").is_err());
    metadata.elements[1].allowed_parents = vec!["cui-example".into()];
    let mut orphan = metadata.elements[1].clone();
    orphan.tag = "cui-orphan".into();
    orphan.content = ContentPolicy::Helpers;
    orphan.allowed_parents = vec!["cui-orphan".into()];
    orphan.allowed_children = vec!["cui-orphan".into()];
    metadata.elements.push(orphan);
    assert!(metadata.validate("example", "native-html").is_err());
}

#[test]
fn templates_roundtrip_across_generated_unicode_and_json_metacharacters() {
    // Bounded exhaustive generation needs no additional test dependency.
    for character in (0..=127)
        .filter_map(char::from_u32)
        .chain(['é', '—', '🦀', '\u{2028}'])
    {
        let mut metadata = valid_metadata();
        metadata.examples[0].template = format!(
            "<cui-example>{character} {{% if ok %}} {{{{ value }}}} {{% endif %}}</cui-example>"
        );
        metadata.validate("example", "native-html").unwrap();
        let encoded = serde_json::to_vec(&metadata).unwrap();
        assert_eq!(
            serde_json::from_slice::<TemplateAuthoring>(&encoded).unwrap(),
            metadata
        );
    }
}

#[test]
fn rejects_empty_duplicate_and_oversized_example_sets() {
    let mut metadata = valid_metadata();
    metadata.examples.clear();
    assert!(metadata.validate("example", "native-html").is_err());
    let mut metadata = valid_metadata();
    metadata.examples.push(metadata.examples[0].clone());
    assert!(metadata.validate("example", "native-html").is_err());
    let mut metadata = valid_metadata();
    metadata.rules = vec!["rule".into(); 33];
    assert!(metadata.validate("example", "native-html").is_err());
    let mut metadata = valid_metadata();
    let example = metadata.examples[0].clone();
    metadata.examples = (0..16)
        .map(|index| {
            let mut example = example.clone();
            example.name = format!("recipe-{index}");
            example.template = "x".repeat(8192);
            example
        })
        .collect();
    let error = metadata.validate("example", "native-html").unwrap_err();
    assert!(error.contains("128 KiB"), "{error}");
}

#[test]
fn rejects_invalid_shapes_references_duplicates_and_budgets() {
    let mut metadata = valid_metadata();
    metadata.elements[0]
        .allowed_children
        .push("cui-missing".into());
    assert!(metadata.validate("example", "native-html").is_err());

    let mut metadata = valid_metadata();
    metadata.elements.push(metadata.elements[0].clone());
    assert!(metadata.validate("example", "native-html").is_err());

    let mut metadata = valid_metadata();
    metadata.elements[0].attributes[0]
        .value_forms
        .push(ValueForm::Literal);
    assert!(metadata.validate("example", "native-html").is_err());

    let mut metadata = valid_metadata();
    metadata.examples[0].template = "x".repeat(32 * 1024 + 1);
    assert!(metadata.validate("example", "native-html").is_err());

    let mut metadata = valid_metadata();
    metadata.examples[0].template = "x".repeat(130 * 1024);
    assert!(metadata.validate("example", "native-html").is_err());

    let mut metadata = valid_metadata();
    metadata.elements[0].tag = "cui-Upper".into();
    assert!(metadata.validate("example", "native-html").is_err());

    assert!(Component::parse(&component_json(Some(serde_json::json!({
        "schemaVersion": 1,
        "rootTag": "cui-example",
        "elements": [],
        "rules": [],
        "examples": [],
        "unrecognized": true
    }))))
    .is_err());
}
