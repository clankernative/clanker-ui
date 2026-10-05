#[path = "../src/theme.rs"]
mod theme;

use theme::{font_css, reference_css, Theme};

const RESOURCE: &str = include_str!("../../../packages/vanilla/theme/reference.css");

#[test]
fn reference_theme_names_are_closed_and_ordered() {
    let names = Theme::ALL.map(Theme::as_str);
    assert_eq!(names, ["light", "dark", "ocean", "forest", "plum"]);
    for name in names {
        assert_eq!(Theme::try_from(name).unwrap().as_str(), name);
    }
    for name in ["", "Light", "system", "custom", "plum "] {
        assert!(Theme::try_from(name).is_err(), "accepted {name:?}");
    }
}

#[test]
fn generated_reference_resource_is_fresh_and_deterministic() {
    let generated = reference_css();
    assert_eq!(generated, reference_css());
    assert_eq!(generated, RESOURCE);
}

#[test]
fn every_theme_contains_complete_semantic_roles_and_legacy_aliases() {
    let css = reference_css();
    let roles = [
        "surface-canvas",
        "surface-raised",
        "surface-overlay",
        "surface-sunken",
        "surface-hover",
        "surface-selected",
        "text-primary",
        "text-secondary",
        "text-muted",
        "text-inverse",
        "border-subtle",
        "border-strong",
        "accent",
        "accent-hover",
        "accent-active",
        "accent-subtle",
        "accent-text",
        "link",
        "link-hover",
        "focus-ring",
        "selection-background",
        "selection-text",
        "info-surface",
        "info-text",
        "info-border",
        "success-surface",
        "success-text",
        "success-border",
        "warning-surface",
        "warning-text",
        "warning-border",
        "danger-surface",
        "danger-text",
        "danger-border",
        "shadow-control",
        "shadow-overlay",
        "shadow-raised",
    ];
    for name in Theme::ALL.map(Theme::as_str) {
        let start = css
            .find(&format!(":root[data-cui-theme=\"{name}\"] {{"))
            .unwrap();
        let end = css[start..].find("}\n\n").unwrap() + start;
        let block = &css[start..end];
        assert!(block.contains(&format!(
            "color-scheme: {};",
            if name == "dark" { "dark" } else { "light" }
        )));
        for role in roles {
            assert!(
                block.contains(&format!("--cui-theme-{role}:")),
                "{name} missing {role}"
            );
        }
        for alias in ["surface", "text", "muted", "border", "focus", "danger"] {
            assert!(
                block.contains(&format!("--cui-{alias}: var(")),
                "{name} missing alias {alias}"
            );
        }
    }
}

#[test]
fn reference_css_is_opt_in_and_fonts_are_separate_relative_resource() {
    let css = reference_css();
    assert!(!css.contains(":root {"));
    assert!(!css.contains(":root:not([data-cui-theme])"));
    assert!(!css.contains("prefers-color-scheme"));
    assert!(!css.contains("@font-face"));
    assert!(css.contains("@media (prefers-reduced-motion: reduce)"));
    assert!(css.contains(":root[data-cui-theme] {"));

    let fonts = font_css();
    assert_eq!(
        fonts,
        include_str!("../../../packages/vanilla/theme/fonts.css")
    );
    assert!(fonts.contains("../fonts/geist-sans-variable.woff2"));
    assert!(fonts.contains("../fonts/geist-mono-variable.woff2"));
    assert!(!fonts.contains("url(\"/"));
}
