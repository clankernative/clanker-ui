//! Typed static rendering for an edge-attached native drawer.
use crate::{
    icon::{self, IconInstance, IconSize},
    layout::AdmittedChildren,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Placement {
    Start,
    #[default]
    End,
}
impl Placement {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Size {
    Narrow,
    #[default]
    Standard,
    Wide,
}
impl Size {
    fn as_str(self) -> &'static str {
        match self {
            Self::Narrow => "narrow",
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
    "Close drawer".into()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawerInstance {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub trigger_label: String,
    pub fallback_href: String,
    #[serde(default = "default_close_label")]
    pub close_label: String,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub heading_level: HeadingLevel,
}
impl DrawerInstance {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("drawer id must be lowercase kebab-case".into());
        }
        for (v, n) in [
            (&self.title, "title"),
            (&self.trigger_label, "triggerLabel"),
            (&self.close_label, "closeLabel"),
        ] {
            text(v, n)?
        }
        if let Some(v) = &self.description {
            text(v, "description")?
        }
        if !crate::button::safe_href(&self.fallback_href) {
            return Err("drawer fallbackHref must be a safe destination".into());
        }
        Ok(())
    }
    /// IDs emitted by this instance, reserved by page-level collision checkers.
    pub fn derived_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::from([self.id.clone(), format!("{}-title", self.id)]);
        if self.description.is_some() {
            ids.insert(format!("{}-description", self.id));
        }
        ids
    }
}
/// Render with required host-admitted `body` and optional `actions`; slots are never deserialized options.
pub fn render(
    instance: &DrawerInstance,
    slots: &BTreeMap<String, AdmittedChildren>,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate()?;
    if !slots.contains_key("body") || slots.keys().any(|k| k != "body" && k != "actions") {
        return Err("drawer requires body and accepts only optional actions admitted slots".into());
    }
    nonblank(&slots["body"], "drawer body")?;
    if let Some(a) = slots.get("actions") {
        nonblank(a, "drawer actions")?
    }
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
    let description = instance
        .description
        .as_ref()
        .map(|v| {
            format!(
                "<p class=\"cui-drawer__description\" id=\"{}-description\">{}</p>",
                id,
                escape(v)
            )
        })
        .unwrap_or_default();
    let described = if instance.description.is_some() {
        format!(" aria-describedby=\"{id}-description\"")
    } else {
        String::new()
    };
    let actions = slots
        .get("actions")
        .map(|a| {
            format!(
                "<footer class=\"cui-drawer__actions\">{}</footer>",
                a.as_markup()
            )
        })
        .unwrap_or_default();
    let html = format!(
        "<div class=\"cui-drawer\" data-cui-component=\"drawer\"><a class=\"cui-button cui-button--secondary cui-button--standard\" href=\"{}\" aria-haspopup=\"dialog\" aria-controls=\"{id}\" data-cui-modal-trigger>{}</a><dialog class=\"cui-drawer__dialog cui-drawer__dialog--{} cui-drawer__dialog--{}\" id=\"{id}\" aria-labelledby=\"{title_id}\"{described} data-cui-modal-dialog><div class=\"cui-drawer__surface\"><header class=\"cui-drawer__header\"><div class=\"cui-drawer__heading\"><{level} class=\"cui-drawer__title\" id=\"{title_id}\">{}</{level}>{description}</div><button class=\"cui-drawer__close\" type=\"button\" aria-label=\"{}\" data-cui-modal-close>{close_icon}</button></header><div class=\"cui-drawer__body\">{}</div>{actions}</div></dialog></div>",
        escape(&instance.fallback_href),
        escape(&instance.trigger_label),
        instance.placement.as_str(),
        instance.size.as_str(),
        escape(&instance.title),
        escape(&instance.close_label),
        slots["body"].as_markup()
    );
    crate::fragment::fill(fragment, &[("[[drawer]]", &html)])
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('-')
        && !v.ends_with('-')
        && v.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn text(v: &str, n: &str) -> Result<(), String> {
    if v.trim().is_empty() || v.chars().any(char::is_control) {
        Err(format!("drawer {n} must contain safe, nonblank text"))
    } else {
        Ok(())
    }
}
fn nonblank(v: &AdmittedChildren, n: &str) -> Result<(), String> {
    if v.as_markup().trim().is_empty() {
        Err(format!("{n} must not be blank"))
    } else {
        Ok(())
    }
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
    const F: &str = "[[drawer]]";
    fn i() -> DrawerInstance {
        DrawerInstance {
            id: "connection-drawer".into(),
            title: "Edit <connection>".into(),
            description: None,
            trigger_label: "Open inspector".into(),
            fallback_href: "/connections/42".into(),
            close_label: default_close_label(),
            placement: Placement::End,
            size: Size::Standard,
            heading_level: HeadingLevel::H2,
        }
    }
    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([(
            "close".into(),
            "<path d=\"m18 6-12 12M6 6l12 12\"></path>".into(),
        )])
    }
    fn s() -> BTreeMap<String, AdmittedChildren> {
        BTreeMap::from([(
            "body".into(),
            AdmittedChildren::from_host_admitted("<p>admitted</p>"),
        )])
    }
    #[test]
    fn renders_all_placement_size_heading_variants_description_actions_and_fallback() {
        for placement in [Placement::Start, Placement::End] {
            for size in [Size::Narrow, Size::Standard, Size::Wide] {
                for heading_level in [HeadingLevel::H2, HeadingLevel::H3, HeadingLevel::H4] {
                    let mut x = i();
                    x.placement = placement;
                    x.size = size;
                    x.heading_level = heading_level;
                    let html = render(&x, &s(), F, &icons()).unwrap();
                    assert!(html.contains("href=\"/connections/42\""));
                    assert!(html.contains(&format!("cui-drawer__dialog--{}", placement.as_str())));
                    assert!(html.contains(&format!("cui-drawer__dialog--{}", size.as_str())));
                    assert!(html.contains(&format!(
                        "<{} class=\"cui-drawer__title\"",
                        heading_level.as_str()
                    )));
                }
            }
        }
        let mut x = i();
        x.description = Some("Update <ownership>".into());
        let mut slots = s();
        slots.insert(
            "actions".into(),
            AdmittedChildren::from_host_admitted("<button>Save</button>"),
        );
        let html = render(&x, &slots, F, &icons()).unwrap();
        assert!(html.contains("aria-describedby=\"connection-drawer-description\""));
        assert!(html.contains("Update &lt;ownership&gt;"));
        assert!(html.contains("cui-drawer__actions"));
    }
    #[test]
    fn rejects_unsafe_config_unknown_fields_and_slots() {
        for href in ["javascript:alert(1)", "//evil.test"] {
            let mut x = i();
            x.fallback_href = href.into();
            assert!(render(&x, &s(), F, &icons()).is_err())
        }
        assert!(DrawerInstance::parse(
            br#"{"id":"x","title":"T","triggerLabel":"Open","fallbackHref":"/x","rawHtml":"bad"}"#
        )
        .is_err());
        assert!(DrawerInstance::parse(br#"{"id":"x","title":"T","triggerLabel":"Open","fallbackHref":"/x","placement":"middle"}"#).is_err());
        let mut slots = s();
        slots.remove("body");
        assert!(render(&i(), &slots, F, &icons()).is_err());
        slots = s();
        slots.insert("other".into(), AdmittedChildren::from_host_admitted("x"));
        assert!(render(&i(), &slots, F, &icons()).is_err());
        assert!(render(&i(), &s(), "[[drawer]][[other]]", &icons()).is_err());
    }
    #[test]
    fn declared_variant_goldens_match_exact_rendering() {
        for fixture in [
            include_str!(
                "../../../packages/vanilla/components/drawer/fixtures/start-narrow-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/drawer/fixtures/end-standard-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/drawer/fixtures/start-wide-golden.json"
            ),
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let instance: DrawerInstance =
                serde_json::from_value(value["options"].clone()).unwrap();
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
                render(&instance, &slots, "[[drawer]]\n", &icons()).unwrap(),
                value["expectedHtml"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn id_set_accounts_for_optional_description() {
        assert_eq!(
            i().derived_ids(),
            BTreeSet::from(["connection-drawer".into(), "connection-drawer-title".into()])
        );
        let mut x = i();
        x.description = Some("Description".into());
        assert_eq!(x.derived_ids().len(), 3);
    }
}
