//! Typed selection and safe static rendering for metadata and filter tags.

use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Neutral,
    Brand,
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Brand => "brand",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TagInstance {
    pub label: String,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub count: Option<String>,
    #[serde(default)]
    pub count_label: Option<String>,
    #[serde(default)]
    pub remove_href: Option<String>,
    #[serde(default)]
    pub remove_label: Option<String>,
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

/// Accept application-relative destinations and explicit HTTP(S) URLs only.
fn safe_href(value: &str) -> bool {
    crate::button::safe_href(value)
}

impl TagInstance {
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_text(&self.label) {
            return Err("tag label must contain safe, nonblank text".into());
        }
        for (name, value) in [("href", &self.href), ("removeHref", &self.remove_href)] {
            if let Some(value) = value {
                if !safe_href(value) {
                    return Err(format!("tag {name} must be a safe destination"));
                }
            }
        }
        if self.href.is_some() && self.remove_href.is_some() {
            return Err("a linked tag cannot contain a second removal link".into());
        }
        if self.count.is_some() != self.count_label.is_some() {
            return Err("tag count and countLabel must be supplied together".into());
        }
        for (name, value) in [
            ("count", &self.count),
            ("countLabel", &self.count_label),
            ("removeLabel", &self.remove_label),
        ] {
            if let Some(value) = value {
                if !valid_text(value) {
                    return Err(format!("tag {name} must contain safe, nonblank text"));
                }
            }
        }
        if self.remove_href.is_some() != self.remove_label.is_some() {
            return Err("tag removeHref and removeLabel must be supplied together".into());
        }
        if let Some(name) = &self.icon {
            IconInstance {
                name: name.clone(),
                size: IconSize::Small,
                label: None,
            }
            .validate(icons)?;
        }
        Ok(())
    }
}

fn substitute(fragment: &str, values: &[(&str, String)]) -> Result<String, String> {
    let replacements: Vec<(&str, &str)> = values
        .iter()
        .map(|(_, value)| ("[[tag]]", value.as_str()))
        .collect();
    crate::fragment::fill(fragment, &replacements)
}

/// Renders a deterministic tag fragment. The fragment has exactly one `[[tag]]` slot.
pub fn render(
    instance: &TagInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    let mut classes = vec![
        "cui-tag".to_owned(),
        format!("cui-tag--{}", instance.tone.as_str()),
        format!("cui-tag--{}", instance.size.as_str()),
    ];
    let mut content = String::new();
    if let Some(name) = &instance.icon {
        let icon = icon::render(
            &IconInstance {
                name: name.clone(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        content.push_str(&format!("<span class=\"cui-tag__icon\">{icon}</span>"));
    }
    content.push_str(&format!(
        "<span class=\"cui-tag__label\">{}</span>",
        escape_html(&instance.label)
    ));
    if let (Some(count), Some(label)) = (&instance.count, &instance.count_label) {
        content.push_str(&format!(
            "<span class=\"cui-tag__count\" aria-hidden=\"true\">{}</span><span class=\"cui-tag__count-label\">{}</span>",
            escape_html(count),
            escape_html(label)
        ));
    }
    let element = if let Some(href) = &instance.href {
        classes.push("cui-tag--linked".into());
        format!(
            "<a class=\"{}\" href=\"{}\" data-cui-component=\"tag\" data-cui-tone=\"{}\">{content}</a>",
            classes.join(" "),
            escape_html(href),
            instance.tone.as_str()
        )
    } else if let (Some(href), Some(label)) = (&instance.remove_href, &instance.remove_label) {
        let close_icon = icon::render(
            &IconInstance {
                name: "close".into(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        format!(
            "<span class=\"{}\" data-cui-component=\"tag\" data-cui-tone=\"{}\">{content}<a class=\"cui-tag__remove\" href=\"{}\" aria-label=\"{}\">{close_icon}</a></span>",
            classes.join(" "),
            instance.tone.as_str(),
            escape_html(href),
            escape_html(label)
        )
    } else {
        format!(
            "<span class=\"{}\" data-cui-component=\"tag\" data-cui-tone=\"{}\">{content}</span>",
            classes.join(" "),
            instance.tone.as_str()
        )
    };
    substitute(fragment, &[("tag", element)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "[[tag]]";
    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("tag".into(), "<path d=\"M1 1\"></path>".into()),
            ("check".into(), "<path d=\"M2 2\"></path>".into()),
            ("close".into(), "<path d=\"M3 3\"></path>".into()),
        ])
    }
    fn tag() -> TagInstance {
        TagInstance {
            label: "Production".into(),
            tone: Tone::Neutral,
            size: Size::Medium,
            href: None,
            icon: None,
            count: None,
            count_label: None,
            remove_href: None,
            remove_label: None,
        }
    }

    #[test]
    fn renders_all_toolframe_tones_and_sizes_as_closed_metadata() {
        for tone in [
            Tone::Neutral,
            Tone::Brand,
            Tone::Info,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
        ] {
            let mut value = tag();
            value.tone = tone;
            let html = render(&value, FRAGMENT, &icons()).unwrap();
            assert!(html.contains(&format!("cui-tag--{}", tone.as_str())));
            assert!(html.contains("data-cui-component=\"tag\""));
        }
        for size in [Size::Small, Size::Medium, Size::Large] {
            let mut value = tag();
            value.size = size;
            assert!(render(&value, FRAGMENT, &icons())
                .unwrap()
                .contains(&format!("cui-tag--{}", size.as_str())));
        }
    }

    #[test]
    fn renders_link_count_icon_and_remove_link_with_escaped_values() {
        let mut value = tag();
        value.label = "Needs <review> & [[tag]]".into();
        value.href = Some("/records?owner=me&state=open".into());
        value.count = Some("12".into());
        value.count_label = Some("12 records".into());
        value.icon = Some("tag".into());
        let linked = render(&value, FRAGMENT, &icons()).unwrap();
        assert!(linked.contains("href=\"/records?owner=me&amp;state=open\""));
        assert!(linked.contains("Needs &lt;review&gt; &amp; [[tag]]"));
        assert!(linked.contains("class=\"cui-tag__count-label\">12 records</span>"));
        assert!(linked.contains("class=\"cui-tag__count\" aria-hidden=\"true\">12</span>"));
        assert!(linked.contains("data-cui-icon=\"tag\""));

        value.href = None;
        value.remove_href = Some("?clear=1&amp;keep=2".replace("&amp;", "&"));
        value.remove_label = Some("Remove <filter>".into());
        let removable = render(&value, FRAGMENT, &icons()).unwrap();
        assert!(removable.contains("aria-label=\"Remove &lt;filter&gt;\""));
        assert!(removable.contains("class=\"cui-tag__remove\""));
        assert!(!removable.starts_with("<a "));
    }

    #[test]
    fn rejects_unsafe_destinations_incomplete_pairs_bad_text_and_unknown_icons() {
        for href in [
            "javascript:alert(1)",
            "data:text/html,bad",
            "//evil.test",
            "https://bad.test\\\\x",
            " /records",
            "/records with spaces",
            "http:evil.test",
            "https://",
            "https://user:secret@example.com",
        ] {
            let mut value = tag();
            value.href = Some(href.into());
            assert!(value.validate(&icons()).is_err(), "{href}");
        }
        for (count, label) in [(Some("1".into()), None), (None, Some("one".into()))] {
            let mut value = tag();
            value.count = count;
            value.count_label = label;
            assert!(value.validate(&icons()).is_err());
        }
        let mut value = tag();
        value.href = Some("/items".into());
        value.remove_href = Some("/clear".into());
        value.remove_label = Some("Remove".into());
        assert!(value.validate(&icons()).is_err());
        value.href = None;
        value.remove_href = None;
        value.remove_label = None;
        value.icon = Some("missing".into());
        assert!(value.validate(&icons()).is_err());
        value.icon = None;
        value.label = "  ".into();
        assert!(value.validate(&icons()).is_err());
    }

    #[test]
    fn fragment_slots_are_exact_and_user_slot_text_is_not_reinterpreted() {
        let mut value = tag();
        value.label = "[[tag]]".into();
        assert!(render(&value, "<span>[[tag]][[tag]]</span>", &icons()).is_err());
        assert!(render(&value, "[[tag]][[unknown]]", &icons()).is_err());
        assert!(render(&value, FRAGMENT, &icons())
            .unwrap()
            .contains("[[tag]]"));
        assert!(serde_json::from_str::<TagInstance>(r#"{"label":"x","unknown":true}"#).is_err());
        assert!(serde_json::from_str::<TagInstance>(r#"{"label":"x","tone":"purple"}"#).is_err());
    }
}
