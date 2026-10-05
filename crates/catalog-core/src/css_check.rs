use crate::tokens::{CssRule, SourceLocation, TokenCatalog};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorFacts {
    pub selector: String,
    pub internal: bool,
    pub unscoped_elements: Vec<String>,
    pub components: Vec<String>,
    pub non_component: bool,
    pub target_known: bool,
    pub global_token_scope: bool,
    pub controlled_properties: Vec<ControlledProperty>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlledProperty {
    pub component: String,
    pub property: String,
    pub tokens: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleFacts {
    pub rule: CssRule,
    pub selectors: Vec<SelectorFacts>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageFacts {
    pub component_elements: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CssDiagnostic {
    pub code: String,
    pub severity: String,
    pub rule: String,
    pub message: String,
    pub location: SourceLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

fn is_token(name: &str) -> bool {
    name.starts_with("--cui-")
}
fn color_property(name: &str) -> bool {
    let p = name.to_ascii_lowercase();
    p == "color"
        || p == "background"
        || p == "background-color"
        || p == "border"
        || p.starts_with("border-")
            && (p.ends_with("-color")
                || [
                    "border-top",
                    "border-right",
                    "border-bottom",
                    "border-left",
                    "border-block",
                    "border-inline",
                ]
                .contains(&p.as_str()))
}
// Keep diagnostic construction centralized; these fields mirror the serialized diagnostic contract.
#[allow(clippy::too_many_arguments)]
fn push(
    out: &mut Vec<CssDiagnostic>,
    code: &str,
    severity: &str,
    rule: &str,
    message: String,
    location: &SourceLocation,
    token: Option<String>,
    component: Option<String>,
    selector: Option<String>,
) {
    out.push(CssDiagnostic {
        code: code.into(),
        severity: severity.into(),
        rule: rule.into(),
        message,
        location: location.clone(),
        token,
        component,
        selector,
    });
}

pub fn check_css(
    catalog: &TokenCatalog,
    rules: &[RuleFacts],
    pages: &PageFacts,
) -> Vec<CssDiagnostic> {
    let known: BTreeSet<&str> = catalog.tokens.iter().map(|t| t.name.as_str()).collect();
    let mut out = Vec::new();
    for facts in rules {
        let rule = &facts.rule;
        for selector in &facts.selectors {
            if selector.target_known && selector.internal {
                push(
                    &mut out,
                    "CUI001",
                    "warning",
                    "internal-selector",
                    format!(
                        "Selector targets internal implementation markup: {}",
                        selector.selector
                    ),
                    &rule.location,
                    None,
                    None,
                    Some(selector.selector.clone()),
                );
            }
            if selector.target_known {
                for element in &selector.unscoped_elements {
                    if [
                        "table", "th", "td", "caption", "h1", "h2", "h3", "h4", "h5", "h6",
                        "button", "input",
                    ]
                    .contains(&element.as_str())
                        && pages
                            .component_elements
                            .values()
                            .any(|elements| elements.contains(element))
                    {
                        push(
                            &mut out,
                            "CUI001",
                            "warning",
                            "unscoped-element",
                            format!("Bare {element} selector can affect component-owned markup"),
                            &rule.location,
                            None,
                            None,
                            Some(selector.selector.clone()),
                        );
                    }
                }
            }
        }
        for declaration in &rule.declarations {
            let refs = declaration.expression.references();
            let set_token = is_token(&declaration.name);
            for token in refs
                .iter()
                .filter(|name| is_token(name))
                .chain(set_token.then_some(&declaration.name))
            {
                if !known.contains(token.as_str()) {
                    push(
                        &mut out,
                        "CUI001",
                        "error",
                        "unknown-token",
                        format!("Unknown design token {token}"),
                        &declaration.location,
                        Some(token.clone()),
                        None,
                        Some(rule.selector.clone()),
                    );
                }
            }
            let custom = is_token(&declaration.name);
            if !custom {
                for selector in &facts.selectors {
                    if !selector.target_known {
                        continue;
                    }
                    if selector.non_component && !selector.global_token_scope {
                        for token in &refs {
                            if is_token(token) {
                                if let Some(details) = catalog.get(token) {
                                    if details.owner != "theme" {
                                        push(
                                            &mut out,
                                            "CUI001",
                                            "warning",
                                            "foreign-token-consumption",
                                            format!(
                                                "Component token owned by {} is consumed on non-component markup",
                                                details.owner
                                            ),
                                            &declaration.location,
                                            Some(token.clone()),
                                            Some(details.owner.clone()),
                                            Some(selector.selector.clone()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    for component in &selector.components {
                        if color_property(&declaration.name) && refs.is_empty() {
                            for controlled in selector.controlled_properties.iter().filter(|p| {
                                p.component == *component
                                    && p.property.eq_ignore_ascii_case(&declaration.name)
                            }) {
                                if let Some(token) = controlled.tokens.first() {
                                    push(
                                        &mut out,
                                        "CUI001",
                                        "warning",
                                        "raw-color-competes-with-token",
                                        format!(
                                            "Raw color declaration competes with token-controlled {}",
                                            declaration.name
                                        ),
                                        &declaration.location,
                                        Some(token.clone()),
                                        Some(component.clone()),
                                        Some(selector.selector.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            if declaration.important {
                let component_override = custom
                    && facts.selectors.iter().any(|s| {
                        s.target_known
                            && s.components.iter().any(|component| {
                                s.controlled_properties.iter().any(|p| {
                                    p.component == *component
                                        && p.tokens.contains(&declaration.name)
                                })
                            })
                    });
                let suspicious = component_override
                    || refs.iter().any(|name| is_token(name))
                    || facts.selectors.iter().any(|s| {
                        s.target_known
                            && (s.internal
                                || s.unscoped_elements.iter().any(|e| {
                                    [
                                        "table", "th", "td", "caption", "h1", "h2", "h3", "h4",
                                        "h5", "h6", "button", "input",
                                    ]
                                    .contains(&e.as_str())
                                })
                                || (color_property(&declaration.name)
                                    && !s.controlled_properties.is_empty()))
                    });
                if suspicious {
                    push(
                        &mut out,
                        "CUI001",
                        "warning",
                        "important-suspicious",
                        format!(
                            "!important on potentially conflicting declaration {}",
                            declaration.name
                        ),
                        &declaration.location,
                        refs.iter().find(|name| is_token(name)).cloned(),
                        None,
                        Some(rule.selector.clone()),
                    );
                }
            }
        }
    }
    // Stable, duplicate-free diagnostics; selector facts may legitimately overlap.
    out.sort_by(|a, b| {
        (
            &a.location.file,
            a.location.line,
            a.location.column,
            &a.rule,
            &a.selector,
            &a.token,
            &a.component,
            &a.code,
        )
            .cmp(&(
                &b.location.file,
                b.location.line,
                b.location.column,
                &b.rule,
                &b.selector,
                &b.token,
                &b.component,
                &b.code,
            ))
    });
    out.dedup_by(|a, b| {
        a.code == b.code
            && a.rule == b.rule
            && a.location == b.location
            && a.token == b.token
            && a.component == b.component
            && a.selector == b.selector
    });
    out
}
