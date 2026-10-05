use catalog_core::css_check::{check_css, ControlledProperty, PageFacts, RuleFacts, SelectorFacts};
use catalog_core::tokens::{
    resolve_tokens, CssDeclaration, CssRule, CssStylesheet, CssValue, SourceLocation,
    TokenDefinition, TokenMetadata, TokenRole, TokenSpec, ValuePart,
};
use catalog_core::{Component, PackageManifest};
use std::collections::{BTreeMap, BTreeSet};

fn def(name: &str) -> TokenDefinition {
    TokenDefinition {
        name: name.into(),
        role: TokenRole::Color,
        purpose: "Primary accent used by controls".into(),
        semantic: Some("accent".into()),
    }
}
fn loc(file: &str, line: u32) -> SourceLocation {
    SourceLocation {
        file: file.into(),
        line,
        column: 1,
    }
}
fn decl(name: &str, value: &str, expression: CssValue, line: u32) -> CssDeclaration {
    CssDeclaration {
        name: name.into(),
        value: value.into(),
        expression,
        important: false,
        location: loc("theme.css", line),
    }
}
fn rule(selector: &str, declarations: Vec<CssDeclaration>, line: u32) -> CssRule {
    CssRule {
        selector: selector.into(),
        conditions: vec![],
        declarations,
        location: loc("theme.css", line),
    }
}
fn spec(name: &str, owner: &str) -> TokenSpec {
    TokenSpec {
        definition: def(name),
        owner: owner.into(),
        status: "shared".into(),
    }
}
fn var(name: &str, fallback: Option<CssValue>) -> CssValue {
    CssValue {
        parts: vec![ValuePart::Var {
            name: name.into(),
            fallback,
        }],
    }
}

#[test]
fn metadata_is_closed_bounded_and_accepts_legacy_component_omission() {
    let bytes = br#"{"schemaVersion":1,"tokens":[{"name":"--cui-accent","role":"color","purpose":"Accent color","semantic":"accent"}]}"#;
    assert_eq!(TokenMetadata::parse(bytes).unwrap().tokens.len(), 1);
    assert!(TokenMetadata::parse(br#"{"schemaVersion":2,"tokens":[]}"#).is_err());
    assert!(TokenMetadata::parse(br#"{"schemaVersion":1,"tokens":[{"name":"--cui-accent","role":"color","purpose":"Accent","extra":true}]}"#).is_err());
    assert!(TokenMetadata::parse(
        br#"{"schemaVersion":1,"tokens":[{"name":"accent","role":"color","purpose":"Accent"}]}"#
    )
    .is_err());
    let component: Component = serde_json::from_str("{\"schemaVersion\":1,\"name\":\"example\",\"status\":\"ready\",\"target\":\"native-html\",\"category\":\"content\",\"summary\":\"Example\",\"keywords\":[],\"agent\":{\"useWhen\":\"Use it\",\"avoidWhen\":\"Avoid it\",\"composeWith\":[]},\"contract\":{\"role\":\"content\",\"variants\":[\"default\"],\"requiredFields\":[],\"invariants\":[\"Static\"]},\"tokens\":[],\"dependencies\":[],\"assets\":{\"template\":\"components/example/template.html\",\"styles\":\"components/example/styles.css\",\"scripts\":[]},\"fixtures\":[]}").unwrap();
    assert!(component.token_descriptions.is_empty());

    let package = PackageManifest::parse(br#"{"schemaVersion":1,"name":"@clanker/vanilla","version":"0.1.0","summary":"UI","theme":"theme/default.css","resources":["theme/token-catalog.json"],"tokenMetadata":"theme/token-catalog.json"}"#).unwrap();
    assert_eq!(
        package.token_metadata.as_deref(),
        Some("theme/token-catalog.json")
    );
    assert!(PackageManifest::parse(br#"{"schemaVersion":1,"name":"@clanker/vanilla","version":"0.1.0","summary":"UI","theme":"theme/default.css","tokenMetadata":"../token.json"}"#).is_err());
    assert!(PackageManifest::parse(br#"{"schemaVersion":1,"name":"@clanker/vanilla","version":"0.1.0","summary":"UI","theme":"theme/default.css","resources":["theme/token-catalog.json"],"tokenMetadata":"theme/tokens.css"}"#).is_err());
}

#[test]
fn resolves_alias_chain_and_use_fallback_without_inventing_initial_value() {
    let theme = CssStylesheet {
        path: "theme.css".into(),
        owner: None,
        rules: vec![rule(
            ":root",
            vec![
                decl("--cui-base", "#123", CssValue::literal("#123"), 1),
                decl(
                    "--cui-accent",
                    "var(--cui-base)",
                    var("--cui-base", None),
                    2,
                ),
            ],
            1,
        )],
    };
    let component = CssStylesheet {
        path: "button.css".into(),
        owner: Some("button".into()),
        rules: vec![rule(
            ".button",
            vec![
                decl("color", "var(--cui-accent)", var("--cui-accent", None), 1),
                decl(
                    "border-color",
                    "var(--cui-border, #abc)",
                    var("--cui-border", Some(CssValue::literal("#abc"))),
                    2,
                ),
            ],
            1,
        )],
    };
    let catalog = resolve_tokens(
        &[
            spec("--cui-base", "theme"),
            spec("--cui-accent", "theme"),
            spec("--cui-border", "button"),
        ],
        &[theme, component],
    )
    .unwrap();
    assert_eq!(
        catalog
            .get("--cui-accent")
            .unwrap()
            .default_value
            .as_deref(),
        Some("#123")
    );
    assert_eq!(
        catalog
            .get("--cui-border")
            .unwrap()
            .default_value
            .as_deref(),
        Some("#abc")
    );
    assert_eq!(
        catalog.get("--cui-border").unwrap().resolution,
        "resolved-fallback"
    );
    assert_eq!(
        catalog.get("--cui-accent").unwrap().readers,
        vec!["button".to_string()]
    );
    assert_eq!(
        catalog.get("--cui-base").unwrap().readers,
        vec!["button".to_string()]
    );
    assert_eq!(
        catalog.get("--cui-border").unwrap().sources[0].resolution,
        "fallback"
    );
}

#[test]
fn duplicate_unknown_and_alias_cycles_fail_and_conditional_values_stay_contextual() {
    let specs = [spec("--cui-a", "theme"), spec("--cui-b", "theme")];
    let cyc = CssStylesheet {
        path: "theme.css".into(),
        owner: None,
        rules: vec![rule(
            ":root",
            vec![
                decl("--cui-a", "var(--cui-b)", var("--cui-b", None), 1),
                decl("--cui-b", "var(--cui-a)", var("--cui-a", None), 2),
            ],
            1,
        )],
    };
    assert!(resolve_tokens(&specs, &[cyc])
        .unwrap_err()
        .contains("cycle"));
    assert!(resolve_tokens(&[spec("--cui-a", "theme"), spec("--cui-a", "button")], &[]).is_err());
    let unknown = CssStylesheet {
        path: "x.css".into(),
        owner: Some("button".into()),
        rules: vec![rule(
            ".x",
            vec![decl(
                "color",
                "var(--cui-missing)",
                var("--cui-missing", None),
                1,
            )],
            1,
        )],
    };
    assert!(resolve_tokens(&specs, &[unknown])
        .unwrap_err()
        .contains("unknown token"));
    let conditional = CssStylesheet {
        path: "theme.css".into(),
        owner: None,
        rules: vec![CssRule {
            conditions: vec!["dark".into()],
            ..rule(
                ":root",
                vec![decl("--cui-a", "black", CssValue::literal("black"), 1)],
                1,
            )
        }],
    };
    let catalog = resolve_tokens(&specs, &[conditional]).unwrap();
    assert_eq!(catalog.get("--cui-a").unwrap().default_value, None);
    assert_eq!(catalog.get("--cui-a").unwrap().resolution, "contextual");
}

#[test]
fn css_check_reports_six_rules_but_allows_inherited_custom_property_override() {
    let catalog = resolve_tokens(&[spec("--cui-accent", "button")], &[]).unwrap();
    let mk_decl = |name: &str, expression: CssValue, important: bool| CssDeclaration {
        name: name.into(),
        value: "x".into(),
        expression,
        important,
        location: loc("component.css", 7),
    };
    let css_rule = CssRule {
        selector: "button.internal".into(),
        conditions: vec![],
        declarations: vec![
            mk_decl("color", var("--cui-accent", None), true),
            mk_decl("background-color", CssValue::literal("red"), false),
            mk_decl("--cui-missing", CssValue::literal("red"), false),
        ],
        location: loc("component.css", 6),
    };
    let facts = RuleFacts {
        rule: css_rule,
        selectors: vec![
            SelectorFacts {
                selector: "button.internal".into(),
                internal: true,
                unscoped_elements: vec!["button".into()],
                components: vec!["button".into()],
                non_component: true,
                target_known: true,
                global_token_scope: false,
                controlled_properties: vec![ControlledProperty {
                    component: "button".into(),
                    property: "background-color".into(),
                    tokens: vec!["--cui-accent".into()],
                }],
            },
            SelectorFacts {
                selector: "button.internal".into(),
                internal: false,
                unscoped_elements: vec![],
                components: vec![],
                non_component: true,
                target_known: true,
                global_token_scope: false,
                controlled_properties: vec![],
            },
        ],
    };
    let pages = PageFacts {
        component_elements: BTreeMap::from([("button".into(), BTreeSet::from(["button".into()]))]),
    };
    let diagnostics = check_css(&catalog, &[facts], &pages);
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code == "CUI001"));
    let rules: BTreeSet<_> = diagnostics.iter().map(|d| d.rule.as_str()).collect();
    assert_eq!(
        rules,
        BTreeSet::from([
            "foreign-token-consumption",
            "important-suspicious",
            "internal-selector",
            "raw-color-competes-with-token",
            "unscoped-element",
            "unknown-token",
        ])
    );
    assert!(diagnostics.windows(2).all(|pair| {
        (
            &pair[0].location.file,
            pair[0].location.line,
            pair[0].location.column,
            &pair[0].rule,
            &pair[0].selector,
            &pair[0].token,
        ) <= (
            &pair[1].location.file,
            pair[1].location.line,
            pair[1].location.column,
            &pair[1].rule,
            &pair[1].selector,
            &pair[1].token,
        )
    }));
}

#[test]
fn diagnostics_are_stably_sorted_and_global_inherited_overrides_are_clean() {
    let catalog = resolve_tokens(&[spec("--cui-accent", "theme")], &[]).unwrap();
    let declaration = CssDeclaration {
        name: "--cui-accent".into(),
        value: "blue".into(),
        expression: CssValue::literal("blue"),
        important: false,
        location: loc("theme.css", 1),
    };
    let facts = RuleFacts {
        rule: CssRule {
            selector: ":root".into(),
            conditions: vec![],
            declarations: vec![declaration],
            location: loc("theme.css", 1),
        },
        selectors: vec![],
    };
    assert!(check_css(
        &catalog,
        std::slice::from_ref(&facts),
        &PageFacts::default()
    )
    .is_empty());
    let a = check_css(
        &catalog,
        &[facts.clone(), facts.clone()],
        &PageFacts::default(),
    );
    let b = check_css(&catalog, &[facts.clone(), facts], &PageFacts::default());
    assert_eq!(a, b);
}

#[test]
fn initial_uses_consistent_local_fallbacks_but_variant_defaults_are_not_guessed() {
    let specs = [spec("--cui-heading", "theme"), spec("--cui-title", "card")];
    let root = CssStylesheet {
        path: "theme.css".into(),
        owner: None,
        rules: vec![rule(
            ":root",
            vec![
                decl("--cui-heading", "#18203b", CssValue::literal("#18203b"), 1),
                decl("--cui-title", "initial", CssValue::literal("initial"), 2),
            ],
            1,
        )],
    };
    let mut component = CssStylesheet {
        path: "card.css".into(),
        owner: Some("card".into()),
        rules: vec![rule(
            ".card",
            vec![decl(
                "color",
                "",
                var("--cui-title", Some(var("--cui-heading", None))),
                1,
            )],
            1,
        )],
    };
    let catalog = resolve_tokens(&specs, &[root.clone(), component.clone()]).unwrap();
    assert_eq!(
        catalog.get("--cui-title").unwrap().default_value.as_deref(),
        Some("#18203b")
    );
    assert_eq!(
        catalog.get("--cui-title").unwrap().resolution,
        "resolved-fallback"
    );
    component.rules.push(rule(
        ".card.other",
        vec![decl(
            "color",
            "",
            var("--cui-title", Some(CssValue::literal("red"))),
            2,
        )],
        2,
    ));
    let catalog = resolve_tokens(&specs, &[root, component]).unwrap();
    assert_eq!(catalog.get("--cui-title").unwrap().default_value, None);
    assert_eq!(catalog.get("--cui-title").unwrap().resolution, "contextual");
}

#[test]
fn component_local_aliases_have_transitive_readers_and_fallback_cycles_are_rejected() {
    let specs = [spec("--cui-base", "theme"), spec("--cui-local", "card")];
    let root = CssStylesheet {
        path: "theme.css".into(),
        owner: None,
        rules: vec![rule(
            ":root",
            vec![decl("--cui-base", "red", CssValue::literal("red"), 1)],
            1,
        )],
    };
    let local = CssStylesheet {
        path: "card.css".into(),
        owner: Some("card".into()),
        rules: vec![rule(
            ".card",
            vec![
                decl("--cui-local", "", var("--cui-base", None), 1),
                decl("color", "", var("--cui-local", None), 2),
            ],
            1,
        )],
    };
    let catalog = resolve_tokens(&specs, &[root.clone(), local]).unwrap();
    assert_eq!(catalog.get("--cui-base").unwrap().readers, vec!["card"]);
    let mut cyclic = root;
    cyclic.rules[0].declarations[0].expression =
        var("--app-missing", Some(var("--cui-base", None)));
    assert!(resolve_tokens(&specs, &[cyclic])
        .unwrap_err()
        .contains("cycle"));
}
