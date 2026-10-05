use catalog_core::tokens::{TokenCatalog, ValuePart};
use catalog_core::{Catalog, PackageManifest};
use clanker_ui::css_analysis::{inspect_ui, parse_stylesheet};
use clanker_ui::ports::LoadedPackage;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn parses_nested_conditions_and_preserves_rule_locations() {
    let css = "@media (min-width: 40rem) {\n @supports (display: grid) {\n .cui-card__body { color: var(--cui-text, rgb(2, 4, 6)); }\n }\n}";
    let sheet = parse_stylesheet("ui/theme.css", None, css).unwrap();
    assert_eq!(sheet.rules.len(), 1);
    let rule = &sheet.rules[0];
    assert_eq!(rule.selector, ".cui-card__body");
    assert_eq!(rule.conditions.len(), 2);
    assert_eq!(rule.declarations[0].location.file, "ui/theme.css");
    assert_eq!(rule.declarations[0].location.line, 3);
    assert!(
        matches!(rule.declarations[0].expression.parts.first(), Some(ValuePart::Var { name, fallback: Some(_) }) if name == "--cui-text")
    );
}

#[test]
fn parses_comments_strings_escapes_fallbacks_and_important_without_false_vars() {
    let css = r#".thing { /* ignored var(--comment) */
        CONTENT: "var(--not-a-token)";
        color: var(--cui-\74 ext, var(--cui-muted, rgb(1, 2, 3))) !important;
    }"#;
    let sheet = parse_stylesheet("app.css", None, css).unwrap();
    let declarations = &sheet.rules[0].declarations;
    assert_eq!(declarations[0].name, "content");
    assert!(declarations[0].expression.references().is_empty());
    assert_eq!(declarations[1].name, "color");
    assert!(declarations[1].important);
    assert_eq!(
        declarations[1].expression.references(),
        std::collections::BTreeSet::from(["--cui-text".to_owned(), "--cui-muted".to_owned()])
    );
}

#[test]
fn nested_functions_preserve_var_references_but_ignore_quoted_text() {
    let css = r#".nested {
        color: rgb(var(--cui-typo));
        width: calc(var(--cui-space) * 1px);
        content: fn("var(--cui-fake)", var(--cui-\74 ext));
    }"#;
    let sheet = parse_stylesheet("nested.css", None, css).unwrap();
    let declarations = &sheet.rules[0].declarations;
    assert_eq!(
        declarations[0].expression.references(),
        std::collections::BTreeSet::from(["--cui-typo".to_owned()])
    );
    assert_eq!(
        declarations[1].expression.references(),
        std::collections::BTreeSet::from(["--cui-space".to_owned()])
    );
    assert_eq!(
        declarations[2].expression.references(),
        std::collections::BTreeSet::from(["--cui-text".to_owned()])
    );
}

#[test]
fn deeply_nested_css_values_and_at_rules_are_rejected() {
    let nested_value = format!("{}var(--cui-token){}", "fn(".repeat(80), ")".repeat(80));
    let css = format!(".deep {{ color: {nested_value}; }}");
    assert!(parse_stylesheet("deep.css", None, &css)
        .unwrap_err()
        .contains("nesting limit"));

    let mut nested_rules = ".leaf { color: red; }".to_owned();
    for _ in 0..80 {
        nested_rules = format!("@media all {{ {nested_rules} }}");
    }
    assert!(parse_stylesheet("deep-rules.css", None, &nested_rules)
        .unwrap_err()
        .contains("nesting limit"));
}

#[test]
fn font_face_reads_are_kept_and_keyframe_steps_are_not_rules() {
    let css = "@font-face { font-family: demo; src: var(--cui-font); } @keyframes pulse { from { opacity: 0; } to { opacity: 1; } }";
    let sheet = parse_stylesheet("fonts.css", None, css).unwrap();
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(sheet.rules[0].selector, "@font-face");
    assert_eq!(
        sheet.rules[0].declarations[1].expression.references(),
        std::collections::BTreeSet::from(["--cui-font".to_owned()])
    );
}

fn package_assets() -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(base, &entry.path(), out);
            } else {
                let key = entry
                    .path()
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(key, fs::read(entry.path()).unwrap());
            }
        }
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let mut assets = BTreeMap::new();
    walk(&base, &base, &mut assets);
    assets
}

fn empty_catalog() -> Catalog {
    let components = package_assets()
        .iter()
        .filter(|(path, _)| path.starts_with("components/") && path.ends_with("/component.json"))
        .map(|(_, bytes)| catalog_core::Component::parse(bytes).unwrap())
        .collect();
    Catalog::new(
        PackageManifest {
            schema_version: 1,
            name: "@test/ui".into(),
            version: "1.0.0".into(),
            summary: "test package".into(),
            theme: "theme/default.css".into(),
            token_metadata: None,
            resources: vec![],
        },
        components,
    )
    .unwrap()
}

fn inspect_fixture(root: &Path) -> clanker_ui::css_analysis::UiInspection {
    let package = LoadedPackage {
        root: PathBuf::new(),
        catalog: empty_catalog(),
        digest: String::new(),
        assets: package_assets(),
    };
    inspect_ui(root, &package, &TokenCatalog { tokens: vec![] }, &[]).unwrap()
}

#[test]
fn inspection_resolves_local_includes_matches_component_dom_and_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("page.html"), "{% include 'partial.html' %}<section class=panel><button>Host</button></section><div style=\"color:var(--cui-text)\"></div><style>.inline { color:var(--cui-text); }</style>").unwrap();
    fs::write(
        dir.path().join("partial.html"),
        "<cui-button label=\"Save\" />",
    )
    .unwrap();
    fs::write(
        dir.path().join("app.css"),
        ".cui-button { color: red; } button { color: blue; }",
    )
    .unwrap();
    let first = inspect_fixture(dir.path());
    let second = inspect_fixture(dir.path());
    assert_eq!(first.files, second.files);
    assert_eq!(first.rules, second.rules);
    assert!(
        first.pages.component_elements.contains_key("button"),
        "{:?}",
        first.limitations
    );
    assert_eq!(
        first
            .rules
            .iter()
            .filter(|r| r.rule.selector == ".cui-button")
            .count(),
        1
    );
    assert!(first
        .rules
        .iter()
        .any(|r| r.rule.selector == "*" && !r.selectors[0].target_known));
    assert!(first.rules.iter().any(|r| r.rule.selector == ".inline"));
    assert!(first
        .limitations
        .iter()
        .any(|item| item.contains("source line/column")));
}

#[test]
fn inspection_bounds_ui_root_and_directory_tree() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    let mut deep = root.clone();
    for _ in 0..66 {
        deep.push("nested");
        fs::create_dir(&deep).unwrap();
    }
    let package = LoadedPackage {
        root: PathBuf::new(),
        catalog: empty_catalog(),
        digest: String::new(),
        assets: package_assets(),
    };
    assert!(
        inspect_ui(&root, &package, &TokenCatalog { tokens: vec![] }, &[])
            .unwrap_err()
            .contains("directory nesting limit")
    );

    #[cfg(unix)]
    {
        let link = dir.path().join("root-link");
        std::os::unix::fs::symlink(&root, &link).unwrap();
        assert!(
            inspect_ui(&link, &package, &TokenCatalog { tokens: vec![] }, &[])
                .unwrap_err()
                .contains("symlink UI root")
        );
    }
}

#[test]
fn inspection_bounds_expanded_include_growth() {
    let dir = tempfile::tempdir().unwrap();
    for index in 0..16 {
        let contents = if index == 15 {
            "x".repeat(2_048)
        } else {
            format!(
                "{{% include 'part-{}.html' %}}{{% include 'part-{}.html' %}}",
                index + 1,
                index + 1
            )
        };
        fs::write(dir.path().join(format!("part-{index}.html")), contents).unwrap();
    }
    fs::write(dir.path().join("page.html"), "{% include 'part-0.html' %}").unwrap();
    let package = LoadedPackage {
        root: PathBuf::new(),
        catalog: empty_catalog(),
        digest: String::new(),
        assets: package_assets(),
    };
    let error =
        inspect_ui(dir.path(), &package, &TokenCatalog { tokens: vec![] }, &[]).unwrap_err();
    assert!(error.contains("expanded HTML exceeds"), "{error}");
}

#[test]
fn inspection_distinguishes_component_roots_from_internal_markup() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("theme.css"),
        "[data-cui-component='data-table'] { --cui-data-table-head-text: green; } \
         .cui-data-table { --cui-data-table-head-text: green; } \
         .cui-data-table th { color: green; } \
         .cui-data-table__head { color: green; }",
    )
    .unwrap();
    let inspected = inspect_fixture(dir.path());
    let internal_for = |selector: &str| {
        inspected
            .rules
            .iter()
            .find(|fact| fact.rule.selector.trim() == selector)
            .unwrap_or_else(|| panic!("missing selector {selector:?}"))
            .selectors[0]
            .internal
    };
    let attribute_root = inspected
        .rules
        .iter()
        .find(|fact| fact.rule.selector.contains("data-cui-component"))
        .unwrap_or_else(|| {
            panic!(
                "missing component attribute root: {:?}",
                inspected
                    .rules
                    .iter()
                    .map(|fact| &fact.rule.selector)
                    .collect::<Vec<_>>()
            )
        })
        .selectors[0]
        .internal;

    assert!(!attribute_root);
    assert!(!internal_for(".cui-data-table"));
    assert!(internal_for(".cui-data-table th"));
    assert!(internal_for(".cui-data-table__head"));

    let diagnostics = catalog_core::css_check::check_css(
        &TokenCatalog { tokens: vec![] },
        &inspected.rules,
        &inspected.pages,
    );
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.rule == "internal-selector"
            && diagnostic.selector.as_deref() == Some("[data-cui-component=\"data-table\"]")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.rule == "internal-selector"
            && diagnostic.selector.as_deref() == Some(".cui-data-table th")
    }));
}

#[test]
fn inspection_rejects_symlinks_and_file_budget_overflows() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("page.html"), "<p>ok</p>").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path().join("page.html"), dir.path().join("linked.html"))
            .unwrap();
        let package = LoadedPackage {
            root: PathBuf::new(),
            catalog: empty_catalog(),
            digest: String::new(),
            assets: package_assets(),
        };
        assert!(inspect_ui(dir.path(), &package, &TokenCatalog { tokens: vec![] }, &[]).is_err());
        fs::remove_file(dir.path().join("linked.html")).unwrap();
    }
    fs::write(dir.path().join("large.html"), vec![b'a'; 1_048_577]).unwrap();
    let package = LoadedPackage {
        root: PathBuf::new(),
        catalog: empty_catalog(),
        digest: String::new(),
        assets: package_assets(),
    };
    assert!(inspect_ui(dir.path(), &package, &TokenCatalog { tokens: vec![] }, &[]).is_err());
}

#[test]
fn malformed_or_unclosed_css_is_rejected() {
    assert!(parse_stylesheet("bad.css", None, ".card { color: red;").is_err());
    assert!(parse_stylesheet("bad.css", None, ".card color red;").is_err());
}
