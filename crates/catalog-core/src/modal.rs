//! Typed static rendering for a native modal with an honest no-JavaScript destination.
use crate::{
    icon::{self, IconInstance, IconSize},
    layout::AdmittedChildren,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Size {
    Small,
    #[default]
    Standard,
    Wide,
}
impl Size {
    fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Standard => "standard",
            Self::Wide => "wide",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HeadingLevel {
    #[default]
    H2,
    H3,
    H4,
}
impl HeadingLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
        }
    }
}

fn default_close_label() -> String {
    "Close dialog".into()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModalInstance {
    pub id: String,
    pub title: String,
    pub trigger_label: String,
    pub fallback_href: String,
    #[serde(default = "default_close_label")]
    pub close_label: String,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub heading_level: HeadingLevel,
}
impl ModalInstance {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("modal id must be lowercase kebab-case".into());
        }
        for (value, name) in [
            (&self.title, "title"),
            (&self.trigger_label, "triggerLabel"),
            (&self.close_label, "closeLabel"),
        ] {
            text(value, name)?;
        }
        if !crate::button::safe_href(&self.fallback_href) {
            return Err("modal fallbackHref must be a safe destination".into());
        }
        Ok(())
    }
    /// IDs emitted by this instance; callers should reserve all of them against page collisions.
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::from([self.id.clone(), format!("{}-title", self.id)])
    }
}
/// Render with required host-admitted `body` and optional `actions`; configuration never carries markup.
pub fn render(
    instance: &ModalInstance,
    slots: &BTreeMap<String, AdmittedChildren>,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate()?;
    exact_slots(slots, &["body", "actions"])?;
    nonblank(&slots["body"], "modal body")?;
    if let Some(actions) = slots.get("actions") {
        nonblank(actions, "modal actions")?;
    }
    let actions = slots
        .get("actions")
        .map(|a| {
            format!(
                "<footer class=\"cui-modal__actions\">{}</footer>",
                a.as_markup()
            )
        })
        .unwrap_or_default();
    let close_icon = icon::render(
        &IconInstance {
            name: "close".into(),
            size: IconSize::Medium,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )?;
    let id = escape(&instance.id);
    let title_id = escape(&format!("{}-title", instance.id));
    let level = instance.heading_level.as_str();
    let html = format!(
        "<div class=\"cui-modal\" data-cui-component=\"modal\"><a class=\"cui-button cui-button--secondary cui-button--standard\" href=\"{}\" aria-haspopup=\"dialog\" aria-controls=\"{id}\" data-cui-modal-trigger>{}</a><dialog class=\"cui-modal__dialog cui-modal__dialog--{}\" id=\"{id}\" aria-labelledby=\"{title_id}\" data-cui-modal-dialog><div class=\"cui-modal__surface\"><header class=\"cui-modal__header\"><{level} class=\"cui-modal__title\" id=\"{title_id}\">{}</{level}><button class=\"cui-modal__close\" type=\"button\" aria-label=\"{}\" data-cui-modal-close>{close_icon}</button></header><div class=\"cui-modal__body\">{}</div>{actions}</div></dialog></div>",
        escape(&instance.fallback_href),
        escape(&instance.trigger_label),
        instance.size.as_str(),
        escape(&instance.title),
        escape(&instance.close_label),
        slots["body"].as_markup()
    );
    crate::fragment::fill(fragment, &[("[[modal]]", &html)])
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn text(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        Err(format!("modal {name} must contain safe, nonblank text"))
    } else {
        Ok(())
    }
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn exact_slots(slots: &BTreeMap<String, AdmittedChildren>, allowed: &[&str]) -> Result<(), String> {
    if !slots.contains_key("body") || slots.keys().any(|key| !allowed.contains(&key.as_str())) {
        Err("modal requires body and accepts only optional actions admitted slots".into())
    } else {
        Ok(())
    }
}
fn nonblank(value: &AdmittedChildren, name: &str) -> Result<(), String> {
    if value.as_markup().trim().is_empty() {
        Err(format!("{name} must not be blank"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FRAGMENT: &str = "<main>[[modal]]</main>";
    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([(
            "close".into(),
            "<path d=\"m18 6-12 12M6 6l12 12\"></path>".into(),
        )])
    }
    fn instance() -> ModalInstance {
        ModalInstance {
            id: "review-access".into(),
            title: "Review <access>".into(),
            trigger_label: "Open review".into(),
            fallback_href: "/access/review".into(),
            close_label: default_close_label(),
            size: Size::Standard,
            heading_level: HeadingLevel::H2,
        }
    }
    fn slots() -> BTreeMap<String, AdmittedChildren> {
        BTreeMap::from([(
            "body".into(),
            AdmittedChildren::from_host_admitted("<p>Admitted body</p>"),
        )])
    }
    #[test]
    fn renders_variants_fallback_actions_and_escaped_values() {
        for size in [Size::Small, Size::Standard, Size::Wide] {
            for heading_level in [HeadingLevel::H2, HeadingLevel::H3, HeadingLevel::H4] {
                let mut i = instance();
                i.size = size;
                i.heading_level = heading_level;
                let html = render(&i, &slots(), FRAGMENT, &icons()).unwrap();
                assert!(html.contains("href=\"/access/review\""));
                assert!(html.contains(&format!("cui-modal__dialog--{}", size.as_str())));
                assert!(html.contains(&format!(
                    "<{} class=\"cui-modal__title\" id=",
                    heading_level.as_str()
                )));
                assert!(html.contains("Review &lt;access&gt;"));
            }
        }
        let mut s = slots();
        s.insert(
            "actions".into(),
            AdmittedChildren::from_host_admitted("<button>Save</button>"),
        );
        assert!(render(&instance(), &s, FRAGMENT, &icons())
            .unwrap()
            .contains("cui-modal__actions"));
    }
    #[test]
    fn rejects_invalid_config_unsafe_href_unknown_options_and_bad_fragments() {
        assert!(ModalInstance::parse(
            br#"{"id":"bad id","title":"T","triggerLabel":"Open","fallbackHref":"/go"}"#
        )
        .is_err());
        for href in ["javascript:alert(1)", "//evil.test", " /space"] {
            let mut i = instance();
            i.fallback_href = href.into();
            assert!(render(&i, &slots(), FRAGMENT, &icons()).is_err());
        }
        assert!(ModalInstance::parse(
            br#"{"id":"x","title":"T","triggerLabel":"Open","fallbackHref":"/x","rawHtml":"<b>"}"#
        )
        .is_err());
        assert!(ModalInstance::parse(
            br#"{"id":"x","title":"T","triggerLabel":"Open","fallbackHref":"/x","size":"huge"}"#
        )
        .is_err());
        for f in ["", "[[modal]][[modal]]", "[[modal]][[other]]"] {
            assert!(render(&instance(), &slots(), f, &icons()).is_err());
        }
        let mut s = slots();
        s.insert(
            "rawHtml".into(),
            AdmittedChildren::from_host_admitted("<b>"),
        );
        assert!(render(&instance(), &s, FRAGMENT, &icons()).is_err());
    }
    #[test]
    fn declared_variant_goldens_match_exact_rendering() {
        for fixture in [
            include_str!("../../../packages/vanilla/components/modal/fixtures/small-golden.json"),
            include_str!(
                "../../../packages/vanilla/components/modal/fixtures/standard-golden.json"
            ),
            include_str!("../../../packages/vanilla/components/modal/fixtures/wide-golden.json"),
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let instance: ModalInstance = serde_json::from_value(value["options"].clone()).unwrap();
            let slots = value["children"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, markup)| {
                    (
                        name.clone(),
                        AdmittedChildren::from_host_admitted(markup.as_str().unwrap()),
                    )
                })
                .collect();
            assert_eq!(
                render(&instance, &slots, "[[modal]]\n", &icons()).unwrap(),
                value["expectedHtml"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn exposes_complete_id_set_and_default_label() {
        let parsed = ModalInstance::parse(
            br#"{"id":"access","title":"Review","triggerLabel":"Open","fallbackHref":"/access"}"#,
        )
        .unwrap();
        assert_eq!(parsed.close_label, "Close dialog");
        assert_eq!(
            parsed.derived_ids(),
            BTreeSet::from(["access".into(), "access-title".into()])
        );
    }
}
