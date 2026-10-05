//! Typed static rendering for a native details/summary contextual popover.
use crate::{
    icon::{self, IconInstance, IconSize},
    layout::AdmittedChildren,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    #[default]
    Start,
    End,
}
impl Alignment {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Width {
    Narrow,
    #[default]
    Standard,
    Wide,
}
impl Width {
    fn as_str(self) -> &'static str {
        match self {
            Self::Narrow => "narrow",
            Self::Standard => "standard",
            Self::Wide => "wide",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PopoverInstance {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default)]
    pub width: Width,
    #[serde(default)]
    pub leading_icon: Option<String>,
}
impl PopoverInstance {
    pub fn parse(bytes: &[u8], icons: &BTreeMap<String, String>) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate(icons)?;
        Ok(value)
    }
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("popover id must be lowercase kebab-case".into());
        }
        if self.label.trim().is_empty() || self.label.chars().any(char::is_control) {
            return Err("popover label must contain safe, nonblank text".into());
        }
        if let Some(name) = &self.leading_icon {
            icon::render(
                &IconInstance {
                    name: name.clone(),
                    size: IconSize::Small,
                    label: None,
                },
                "<svg [[attributes]]>[[geometry]]</svg>",
                icons,
            )?;
        }
        icon::render(
            &IconInstance {
                name: "chevron-down".into(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        Ok(())
    }
    /// IDs emitted by this instance, reserved by page-level collision checkers.
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::from([self.id.clone()])
    }
}
/// Compose separately admitted body content; no markup is accepted from configuration.
pub fn render(
    instance: &PopoverInstance,
    content: &AdmittedChildren,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    if content.as_markup().trim().is_empty() {
        return Err("popover content must not be blank".into());
    }
    let leading = instance
        .leading_icon
        .as_ref()
        .map(|name| {
            icon::render(
                &IconInstance {
                    name: name.clone(),
                    size: IconSize::Small,
                    label: None,
                },
                "<svg [[attributes]]>[[geometry]]</svg>",
                icons,
            )
            .map(|svg| {
                format!(
                    "<span class=\"cui-popover__leading-icon\" aria-hidden=\"true\">{svg}</span>"
                )
            })
        })
        .transpose()?
        .unwrap_or_default();
    let chevron = icon::render(
        &IconInstance {
            name: "chevron-down".into(),
            size: IconSize::Small,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )?;
    let html = format!(
        "<details class=\"cui-popover cui-popover--{}\" data-cui-component=\"popover\"><summary class=\"cui-popover__trigger\" aria-controls=\"{}\">{leading}<span class=\"cui-popover__label\">{}</span><span class=\"cui-popover__chevron\" aria-hidden=\"true\">{chevron}</span></summary><div class=\"cui-popover__panel cui-popover__panel--{}\" id=\"{}\">{}</div></details>",
        instance.alignment.as_str(),
        escape(&instance.id),
        escape(&instance.label),
        instance.width.as_str(),
        escape(&instance.id),
        content.as_markup()
    );
    crate::fragment::fill(fragment, &[("[[popover]]", &html)])
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('-')
        && !v.ends_with('-')
        && v.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn escape(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "[[popover]]";
    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "chevron-down".into(),
                "<path d=\"m6 9 6 6 6-6\"></path>".into(),
            ),
            (
                "help".into(),
                "<circle cx=\"12\" cy=\"12\" r=\"9\"></circle>".into(),
            ),
            (
                "info".into(),
                "<circle cx=\"12\" cy=\"12\" r=\"9\"></circle>\n<path d=\"M12 11v5M12 8h.01\"></path>".into(),
            ),
        ])
    }
    fn i() -> PopoverInstance {
        PopoverInstance {
            id: "permission-help".into(),
            label: "Who can edit? <details>".into(),
            alignment: Alignment::Start,
            width: Width::Standard,
            leading_icon: None,
        }
    }
    fn content() -> AdmittedChildren {
        AdmittedChildren::from_host_admitted("<p>Trusted <strong>content</strong></p>")
    }
    #[test]
    fn renders_native_fallback_and_every_layout_variant() {
        for alignment in [Alignment::Start, Alignment::End] {
            for width in [Width::Narrow, Width::Standard, Width::Wide] {
                let mut x = i();
                x.alignment = alignment;
                x.width = width;
                let html = render(&x, &content(), F, &icons()).unwrap();
                assert!(html.contains("<details class=\"cui-popover"));
                assert!(html.contains("<summary class=\"cui-popover__trigger\""));
                assert!(html.contains(&format!("cui-popover--{}", alignment.as_str())));
                assert!(html.contains(&format!("cui-popover__panel--{}", width.as_str())));
                assert!(html.contains("Who can edit? &lt;details&gt;"));
                assert!(html.contains("<strong>content</strong>"));
            }
        }
        let mut x = i();
        x.leading_icon = Some("help".into());
        assert!(render(&x, &content(), F, &icons())
            .unwrap()
            .contains("data-cui-icon=\"help\""));
    }
    #[test]
    fn rejects_unknown_props_invalid_names_icons_fragment_and_blank_slot() {
        assert!(PopoverInstance::parse(br#"{"id":"bad id","label":"More"}"#, &icons()).is_err());
        assert!(PopoverInstance::parse(
            br#"{"id":"help","label":"More","unknown":true}"#,
            &icons()
        )
        .is_err());
        assert!(PopoverInstance::parse(
            br#"{"id":"help","label":"More","alignment":"center"}"#,
            &icons()
        )
        .is_err());
        let mut x = i();
        x.leading_icon = Some("script".into());
        assert!(render(&x, &content(), F, &icons()).is_err());
        assert!(render(&i(), &content(), "[[popover]][[x]]", &icons()).is_err());
        assert!(render(
            &i(),
            &AdmittedChildren::from_host_admitted("  "),
            F,
            &icons()
        )
        .is_err());
    }
    #[test]
    fn declared_variant_goldens_match_exact_rendering() {
        for fixture in [
            include_str!(
                "../../../packages/vanilla/components/popover/fixtures/start-narrow-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/popover/fixtures/end-standard-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/popover/fixtures/start-standard-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/popover/fixtures/end-wide-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/popover/fixtures/start-wide-golden.json"
            ),
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let instance: PopoverInstance =
                serde_json::from_value(value["options"].clone()).unwrap();
            let content =
                AdmittedChildren::from_host_admitted(value["children"]["body"].as_str().unwrap());
            assert_eq!(
                render(&instance, &content, "[[popover]]\n", &icons()).unwrap(),
                value["expectedHtml"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn parse_and_ids_are_explicit() {
        let x = PopoverInstance::parse(br#"{"id":"help","label":"Help"}"#, &icons()).unwrap();
        assert_eq!(x.derived_ids(), BTreeSet::from(["help".into()]));
    }
}
