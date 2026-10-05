//! Typed, accessible static rendering for concise contextual help.
use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Placement {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}
impl Placement {
    fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TooltipInstance {
    pub id: String,
    pub trigger_label: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub arrow: bool,
}
fn default_icon() -> String {
    "help".into()
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('-')
        && !v.ends_with('-')
        && v.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn valid_text(v: &str) -> bool {
    !v.trim().is_empty() && !v.chars().any(char::is_control)
}
fn escape(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl TooltipInstance {
    /// Parse JSON and validate stable IDs, visible text, and caller-supplied locked icon geometry.
    pub fn parse(bytes: &[u8], icons: &BTreeMap<String, String>) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate(icons)?;
        Ok(value)
    }
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("tooltip id must be lowercase kebab-case".into());
        }
        for (name, value) in [
            ("triggerLabel", &self.trigger_label),
            ("title", &self.title),
        ] {
            if !valid_text(value) {
                return Err(format!("tooltip {name} must contain safe, nonblank text"));
            }
        }
        if self.description.as_ref().is_some_and(|v| !valid_text(v)) {
            return Err("tooltip description must contain safe, nonblank text".into());
        }
        icon::render(
            &IconInstance {
                name: self.icon.clone(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        Ok(())
    }
    /// Returns the stable bubble ID emitted by this tooltip.
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::from([self.id.clone()])
    }
}
/// Renders a noninteractive tooltip bubble and its keyboard-focusable trigger.
pub fn render(
    instance: &TooltipInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    let icon = icon::render(
        &IconInstance {
            name: instance.icon.clone(),
            size: IconSize::Small,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )?;
    let description = instance
        .description
        .as_ref()
        .map(|d| {
            format!(
                "<span class=\"cui-tooltip__description\">{}</span>",
                escape(d)
            )
        })
        .unwrap_or_default();
    let supporting = if instance.description.is_some() {
        " cui-tooltip__bubble--supporting"
    } else {
        ""
    };
    let arrow = if instance.arrow {
        "<span class=\"cui-tooltip__arrow\" aria-hidden=\"true\"></span>"
    } else {
        ""
    };
    let html = format!(
        "<span class=\"cui-tooltip\" data-cui-component=\"tooltip\" data-cui-tooltip-placement=\"{}\"><button class=\"cui-tooltip__trigger\" type=\"button\" aria-label=\"{}\" aria-describedby=\"{}\" data-cui-tooltip-trigger>{}</button><span class=\"cui-tooltip__bubble{}\" id=\"{}\" role=\"tooltip\" data-cui-tooltip-bubble><span class=\"cui-tooltip__title\">{}</span>{}{}</span></span>",
        instance.placement.as_str(),
        escape(&instance.trigger_label),
        escape(&instance.id),
        icon,
        supporting,
        escape(&instance.id),
        escape(&instance.title),
        description,
        arrow
    );
    crate::fragment::fill(fragment, &[("[[tooltip]]", &html)])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn icons() -> BTreeMap<String, String> {
        ["help", "info"]
            .into_iter()
            .map(|n| (n.into(), "<path d=\"M1 1\"></path>".into()))
            .collect()
    }
    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/vanilla/components/tooltip/fixtures/typical.json"
        ))
        .unwrap()
    }
    #[test]
    fn typical_fixture_matches_golden_and_reserves_id() {
        let icons = icons();
        let value = TooltipInstance::parse(&fixture(), &icons).unwrap();
        let html = render(
            &value,
            include_str!("../../../packages/vanilla/components/tooltip/fragment.html"),
            &icons,
        )
        .unwrap();
        assert_eq!(
            html,
            include_str!(
                "../../../packages/vanilla/components/tooltip/fixtures/typical.golden.html"
            )
        );
        assert_eq!(
            value.derived_ids(),
            BTreeSet::from(["retention-help".into()])
        );
    }
    #[test]
    fn rejects_unknown_fields_invalid_ids_enum_text_and_icons() {
        let icons = icons();
        assert!(TooltipInstance::parse(
            br#"{"id":"bad id","triggerLabel":"Help","title":"Text"}"#,
            &icons
        )
        .is_err());
        assert!(TooltipInstance::parse(
            br#"{"id":"help","triggerLabel":"Help","title":"Text","placement":"center"}"#,
            &icons
        )
        .is_err());
        assert!(TooltipInstance::parse(
            br#"{"id":"help","triggerLabel":"Help","title":"Text","unexpected":true}"#,
            &icons
        )
        .is_err());
        let mut value = TooltipInstance::parse(&fixture(), &icons).unwrap();
        value.icon = "not-in-catalog".into();
        assert!(value.validate(&icons).is_err());
    }
}
