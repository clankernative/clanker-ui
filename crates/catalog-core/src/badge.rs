//! Typed selection and safe static rendering for semantic badges.

use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeInstance {
    pub label: String,
    pub tone: Tone,
    /// Overrides the tone's semantic default with a closed catalog icon name.
    #[serde(default)]
    pub icon: Option<String>,
    /// Removes the icon entirely, including a semantic default or override.
    #[serde(default = "default_show_icon")]
    pub show_icon: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
    Running,
}

impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::Running => "running",
        }
    }

    fn default_icon(self) -> Option<&'static str> {
        match self {
            Self::Neutral => None,
            Self::Info => Some("info"),
            Self::Success => Some("check"),
            Self::Warning | Self::Danger => Some("alert"),
            Self::Running => Some("restore"),
        }
    }
}

fn default_show_icon() -> bool {
    true
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

impl BadgeInstance {
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if self.label.trim().is_empty() || self.label.chars().any(char::is_control) {
            return Err("badge label must contain safe text".into());
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

/// Renders a static badge fragment. The adapter owns the fragment and must
/// validate its three unique slots before calling this function.
pub fn render(
    instance: &BadgeInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    if fragment.matches("[[attributes]]").count() != 1
        || fragment.matches("[[icon]]").count() != 1
        || fragment.matches("[[label]]").count() != 1
    {
        return Err(
            "badge fragment must contain exactly one attributes, icon, and label slot".into(),
        );
    }
    if fragment.matches("[[").count() != 3 {
        return Err("badge fragment contains an unsupported slot".into());
    }
    instance.validate(icons)?;
    let icon_name = if instance.show_icon {
        instance
            .icon
            .as_deref()
            .or_else(|| instance.tone.default_icon())
    } else {
        None
    };
    let icon_markup = match icon_name {
        Some(name) => {
            let rendered = icon::render(
                &IconInstance {
                    name: name.into(),
                    size: IconSize::Small,
                    label: None,
                },
                "<svg [[attributes]]>[[geometry]]</svg>",
                icons,
            )?;
            format!("<span class=\"cui-badge__icon\">{rendered}</span>")
        }
        None => String::new(),
    };
    let attributes = format!(
        "class=\"cui-badge cui-badge--{}\" data-cui-component=\"badge\" data-cui-tone=\"{}\"",
        instance.tone.as_str(),
        instance.tone.as_str()
    );
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[icon]]", &icon_markup),
            ("[[label]]", &escape(&instance.label)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("alert".into(), "<path d=\"M1 1\"></path>".into()),
            ("check".into(), "<path d=\"M2 2\"></path>".into()),
            ("info".into(), "<path d=\"M3 3\"></path>".into()),
            ("restore".into(), "<path d=\"M4 4\"></path>".into()),
        ])
    }

    fn instance(tone: Tone) -> BadgeInstance {
        BadgeInstance {
            label: "Ready".into(),
            tone,
            icon: None,
            show_icon: true,
        }
    }

    const FRAGMENT: &str =
        "<span [[attributes]]>[[icon]]<span class=\"cui-badge__label\">[[label]]</span></span>";

    #[test]
    fn renders_tone_specific_semantic_default_icons_and_neutral_text_only() {
        for (tone, expected_icon) in [
            (Tone::Neutral, None),
            (Tone::Info, Some("info")),
            (Tone::Success, Some("check")),
            (Tone::Warning, Some("alert")),
            (Tone::Danger, Some("alert")),
            (Tone::Running, Some("restore")),
        ] {
            let html = render(&instance(tone), FRAGMENT, &icons()).unwrap();
            assert!(html.contains(&format!("cui-badge--{}", tone.as_str())));
            assert!(html.contains(&format!("data-cui-tone=\"{}\"", tone.as_str())));
            assert_eq!(
                html.contains("data-cui-component=\"icon\""),
                expected_icon.is_some()
            );
            if let Some(name) = expected_icon {
                assert!(html.contains(&format!("data-cui-icon=\"{name}\"")));
                assert!(html.contains("aria-hidden=\"true\""));
            }
        }
    }

    #[test]
    fn escapes_label_and_supports_checked_custom_or_disabled_icon() {
        let mut badge = instance(Tone::Success);
        badge.label = "Passed <check> & \"verified\"".into();
        let html = render(&badge, FRAGMENT, &icons()).unwrap();
        assert!(html.contains("Passed &lt;check&gt; &amp; &quot;verified&quot;"));
        badge.icon = Some("info".into());
        assert!(render(&badge, FRAGMENT, &icons())
            .unwrap()
            .contains("data-cui-icon=\"info\""));
        badge.show_icon = false;
        assert!(!render(&badge, FRAGMENT, &icons())
            .unwrap()
            .contains("cui-badge__icon"));
    }

    #[test]
    fn rejects_invalid_label_icon_and_fragment_slots() {
        let mut badge = instance(Tone::Neutral);
        assert!(badge.validate(&icons()).is_ok());
        badge.label = "  ".into();
        assert!(badge.validate(&icons()).is_err());
        badge = instance(Tone::Neutral);
        badge.icon = Some("missing".into());
        assert!(badge.validate(&icons()).is_err());
        badge.icon = Some("check\"><script>".into());
        assert!(badge.validate(&icons()).is_err());
        badge.show_icon = false;
        assert!(render(&badge, FRAGMENT, &icons()).is_err());
        assert!(render(
            &instance(Tone::Neutral),
            "<span>[[attributes]][[icon]]</span>",
            &icons()
        )
        .is_err());
        assert!(render(
            &instance(Tone::Neutral),
            "[[attributes]][[icon]][[label]][[extra]]",
            &icons()
        )
        .is_err());
    }
}
