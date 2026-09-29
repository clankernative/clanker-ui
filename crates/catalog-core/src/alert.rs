//! Typed selection and safe static rendering for contextual alert messages.

use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "check",
            Self::Warning | Self::Danger => "alert",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Announcement {
    Polite,
    Assertive,
}

impl Announcement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Polite => "polite",
            Self::Assertive => "assertive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlertInstance {
    pub title: String,
    pub body: String,
    pub tone: Tone,
    #[serde(default)]
    pub recovery_label: Option<String>,
    #[serde(default)]
    pub recovery_href: Option<String>,
    /// Omitted means a static region with no live announcement semantics.
    #[serde(default)]
    pub announcement: Option<Announcement>,
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

fn safe_href(value: &str) -> bool {
    crate::button::safe_href(value)
}

impl AlertInstance {
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_text(&self.title) || !valid_text(&self.body) {
            return Err("alert title and body must contain safe, nonblank text".into());
        }
        if self.recovery_label.is_some() != self.recovery_href.is_some() {
            return Err("alert recoveryLabel and recoveryHref must be supplied together".into());
        }
        if let Some(label) = &self.recovery_label {
            if !valid_text(label) {
                return Err("alert recoveryLabel must contain safe, nonblank text".into());
            }
        }
        if let Some(href) = &self.recovery_href {
            if !safe_href(href) {
                return Err("alert recoveryHref must be a safe destination".into());
            }
        }
        IconInstance {
            name: self.tone.icon().into(),
            size: IconSize::Medium,
            label: None,
        }
        .validate(icons)
    }
}

fn substitute(fragment: &str, values: &[(&str, String)]) -> Result<String, String> {
    let replacements: Vec<(String, &str)> = values
        .iter()
        .map(|(slot, value)| (format!("[[{slot}]]"), value.as_str()))
        .collect();
    let replacements: Vec<(&str, &str)> = replacements
        .iter()
        .map(|(slot, value)| (slot.as_str(), *value))
        .collect();
    crate::fragment::fill(fragment, &replacements)
}

/// Renders an alert fragment with exact adapter-owned slots and no runtime behavior.
pub fn render(
    instance: &AlertInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    let role = match instance.announcement {
        None => String::new(),
        Some(Announcement::Polite) => " role=\"status\" aria-live=\"polite\"".into(),
        Some(Announcement::Assertive) => " role=\"alert\" aria-live=\"assertive\"".into(),
    };
    let attributes = format!(
        "class=\"cui-alert cui-alert--{}\" data-cui-component=\"alert\" data-cui-tone=\"{}\"{}",
        instance.tone.as_str(),
        instance.tone.as_str(),
        role
    );
    let icon = icon::render(
        &IconInstance {
            name: instance.tone.icon().into(),
            size: IconSize::Medium,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )?;
    let recovery = match (&instance.recovery_label, &instance.recovery_href) {
        (Some(label), Some(href)) => format!(
            "<a class=\"cui-alert__recovery\" href=\"{}\">{}</a>",
            escape_html(href),
            escape_html(label)
        ),
        _ => String::new(),
    };
    substitute(
        fragment,
        &[
            ("attributes", attributes),
            (
                "icon",
                format!("<span class=\"cui-alert__icon\">{icon}</span>"),
            ),
            ("title", escape_html(&instance.title)),
            ("body", escape_html(&instance.body)),
            ("recovery", recovery),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "<section [[attributes]]><div class=\"cui-alert__layout\">[[icon]]<div class=\"cui-alert__content\"><h2 class=\"cui-alert__title\">[[title]]</h2><p class=\"cui-alert__body\">[[body]]</p>[[recovery]]</div></div></section>";
    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("info".into(), "<path d=\"M1 1\"></path>".into()),
            ("check".into(), "<path d=\"M2 2\"></path>".into()),
            ("alert".into(), "<path d=\"M3 3\"></path>".into()),
        ])
    }
    fn alert(tone: Tone) -> AlertInstance {
        AlertInstance {
            title: "Saved".into(),
            body: "Changes are available.".into(),
            tone,
            recovery_label: None,
            recovery_href: None,
            announcement: None,
        }
    }

    #[test]
    fn renders_each_semantic_tone_with_closed_decorative_icon_and_no_live_region_by_default() {
        for (tone, icon_name) in [
            (Tone::Info, "info"),
            (Tone::Success, "check"),
            (Tone::Warning, "alert"),
            (Tone::Danger, "alert"),
        ] {
            let html = render(&alert(tone), FRAGMENT, &icons()).unwrap();
            assert!(html.contains(&format!("data-cui-tone=\"{}\"", tone.as_str())));
            assert!(html.contains(&format!("data-cui-icon=\"{icon_name}\"")));
            assert!(html.contains("aria-hidden=\"true\""));
            assert!(!html.contains(" role="));
            assert!(!html.contains("aria-live"));
        }
    }

    #[test]
    fn escapes_plain_text_and_renders_paired_recovery_link_and_explicit_announcement() {
        let mut value = alert(Tone::Warning);
        value.title = "Check <input> & [[title]]".into();
        value.body = "Value \"bad\" & <retry>".into();
        value.recovery_label = Some("Review & retry".into());
        value.recovery_href = Some("?next=/review&safe=1".into());
        value.announcement = Some(Announcement::Polite);
        let html = render(&value, FRAGMENT, &icons()).unwrap();
        assert!(html.contains("Check &lt;input&gt; &amp; [[title]]"));
        assert!(html.contains("Value &quot;bad&quot; &amp; &lt;retry&gt;"));
        assert!(html.contains("href=\"?next=/review&amp;safe=1\""));
        assert!(html.contains("role=\"status\" aria-live=\"polite\""));
        value.announcement = Some(Announcement::Assertive);
        assert!(render(&value, FRAGMENT, &icons())
            .unwrap()
            .contains("role=\"alert\" aria-live=\"assertive\""));
    }

    #[test]
    fn rejects_invalid_content_urls_pairs_fragment_slots_and_unknown_properties() {
        for href in [
            "http:evil.test",
            "https://",
            "/has space",
            "https://user:secret@example.com",
        ] {
            let mut value = alert(Tone::Info);
            value.recovery_label = Some("Recover".into());
            value.recovery_href = Some(href.into());
            assert!(value.validate(&icons()).is_err(), "{href}");
        }
        let mut value = alert(Tone::Info);
        value.recovery_href = Some("javascript:alert(1)".into());
        assert!(value.validate(&icons()).is_err());
        value.recovery_href = Some("/recover".into());
        assert!(value.validate(&icons()).is_err());
        value.recovery_label = Some("Recover".into());
        value.recovery_href = Some("//outside.test".into());
        assert!(value.validate(&icons()).is_err());
        value.recovery_href = Some("/recover".into());
        value.body = "".into();
        assert!(value.validate(&icons()).is_err());
        value.body = "Body".into();
        assert!(render(
            &value,
            "<section [[attributes]]>[[icon]][[title]][[body]][[recovery]][[other]]</section>",
            &icons()
        )
        .is_err());
        assert!(render(&value, "[[attributes]][[icon]][[title]][[body]]", &icons()).is_err());
        assert!(serde_json::from_str::<AlertInstance>(
            r#"{"title":"T","body":"B","tone":"info","dismissible":true}"#
        )
        .is_err());
        assert!(serde_json::from_str::<AlertInstance>(
            r#"{"title":"T","body":"B","tone":"neutral"}"#
        )
        .is_err());
    }
}
