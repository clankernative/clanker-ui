//! Typed static rendering for bounded toast notifications.
use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    Info,
    Success,
    Warning,
    Danger,
    Progress,
}
impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::Progress => "progress",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "check",
            Self::Warning | Self::Danger => "alert",
            Self::Progress => "restore",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    Inline,
    TopCenter,
    TopEnd,
    BottomCenter,
    #[default]
    BottomEnd,
}
impl Position {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::TopCenter => "top-center",
            Self::TopEnd => "top-end",
            Self::BottomCenter => "bottom-center",
            Self::BottomEnd => "bottom-end",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryPolicy {
    #[default]
    Reset,
    Preserve,
    Remove,
}
impl HistoryPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reset => "reset",
            Self::Preserve => "preserve",
            Self::Remove => "remove",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Message {
    pub title: String,
    pub body: String,
    pub tone: Tone,
    #[serde(default)]
    pub action_href: Option<String>,
    #[serde(default)]
    pub action_label: Option<String>,
    #[serde(default = "default_dismiss_label")]
    pub dismiss_label: Option<String>,
    #[serde(default)]
    pub timeout: Option<u32>,
}
fn default_dismiss_label() -> Option<String> {
    Some("Dismiss notification".into())
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToastInstance {
    pub label: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub history_policy: HistoryPolicy,
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
impl ToastInstance {
    /// Parse JSON and validate all text, links, limits, and closed icon geometry.
    pub fn parse(bytes: &[u8], icons: &BTreeMap<String, String>) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate(icons)?;
        Ok(value)
    }
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_text(&self.label) {
            return Err("toast label must contain safe, nonblank text".into());
        }
        if self.messages.is_empty() || self.messages.len() > 8 {
            return Err("toast region requires between one and eight messages".into());
        }
        for message in &self.messages {
            if !valid_text(&message.title) || !valid_text(&message.body) {
                return Err("toast title and body must contain safe, nonblank text".into());
            }
            if message.action_href.is_some() != message.action_label.is_some() {
                return Err("toast actionHref and actionLabel must be supplied together".into());
            }
            if let Some(href) = &message.action_href {
                if !crate::button::safe_href(href) {
                    return Err("toast actionHref must be a safe destination".into());
                }
            }
            if let Some(label) = &message.action_label {
                if !valid_text(label) {
                    return Err("toast actionLabel must contain safe, nonblank text".into());
                }
            }
            if let Some(label) = &message.dismiss_label {
                if !valid_text(label) {
                    return Err("toast dismissLabel must contain safe, nonblank text".into());
                }
            }
            if message
                .timeout
                .is_some_and(|n| !(1000..=60000).contains(&n))
            {
                return Err(
                    "toast timeout must be an integer between 1000 and 60000 milliseconds".into(),
                );
            }
            icon::render(
                &IconInstance {
                    name: message.tone.icon().into(),
                    size: IconSize::Medium,
                    label: None,
                },
                "<svg [[attributes]]>[[geometry]]</svg>",
                icons,
            )?;
        }
        icon::render(
            &IconInstance {
                name: "close".into(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        Ok(())
    }

    /// Toast markup emits no DOM IDs, so there are no derived IDs to reserve.
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }
}
/// Renders a toast region. App-owned message generation and replacement remain outside this contract.
pub fn render(
    instance: &ToastInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    let mut messages = String::new();
    for message in &instance.messages {
        let icon = icon::render(
            &IconInstance {
                name: message.tone.icon().into(),
                size: IconSize::Medium,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        let dismiss = match &message.dismiss_label {
            Some(label) => format!(
                "<button class=\"cui-toast__dismiss\" type=\"button\" aria-label=\"{}\" data-cui-toast-dismiss>{}</button>",
                escape(label),
                icon::render(
                    &IconInstance {
                        name: "close".into(),
                        size: IconSize::Small,
                        label: None
                    },
                    "<svg [[attributes]]>[[geometry]]</svg>",
                    icons
                )?
            ),
            None => String::new(),
        };
        let action = match (&message.action_href, &message.action_label) {
            (Some(href), Some(label)) => format!(
                "<a class=\"cui-toast__action\" href=\"{}\">{}</a>",
                escape(href),
                escape(label)
            ),
            _ => String::new(),
        };
        let timeout = message
            .timeout
            .map(|n| format!(" data-cui-toast-timeout=\"{n}\"", n = n))
            .unwrap_or_default();
        let timer = if message.timeout.is_some() {
            "<span class=\"cui-toast__timer\" aria-hidden=\"true\"></span>"
        } else {
            ""
        };
        let role = if message.tone == Tone::Danger {
            "alert"
        } else {
            "status"
        };
        messages.push_str(&format!("<article class=\"cui-toast cui-toast--{}\" role=\"{}\" data-cui-toast{}><span class=\"cui-toast__icon\" aria-hidden=\"true\">{}</span><div class=\"cui-toast__content\"><p class=\"cui-toast__title\">{}</p><p class=\"cui-toast__body\">{}</p></div>{}{}{}</article>", message.tone.as_str(), role, timeout, icon, escape(&message.title), escape(&message.body), dismiss, action, timer));
    }
    let region = format!(
        "<div class=\"cui-toast-region cui-toast-region--{}\" role=\"region\" aria-label=\"{}\" data-cui-component=\"toast\" data-cui-history-policy=\"{}\">{}</div>",
        instance.position.as_str(),
        escape(&instance.label),
        instance.history_policy.as_str(),
        messages
    );
    crate::fragment::fill(fragment, &[("[[toastRegion]]", &region)])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn icons() -> BTreeMap<String, String> {
        ["info", "check", "alert", "restore", "close"]
            .into_iter()
            .map(|n| (n.into(), "<path d=\"M1 1\"></path>".into()))
            .collect()
    }
    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/vanilla/components/toast/fixtures/default.json"
        ))
        .unwrap()
    }
    #[test]
    fn default_fixture_matches_golden_and_validates_typed_content() {
        let icons = icons();
        let value = ToastInstance::parse(&fixture(), &icons).unwrap();
        let html = render(
            &value,
            include_str!("../../../packages/vanilla/components/toast/fragment.html"),
            &icons,
        )
        .unwrap();
        assert_eq!(
            html,
            include_str!("../../../packages/vanilla/components/toast/fixtures/default.golden.html")
        );
    }
    #[test]
    fn rejects_unknown_fields_bad_tones_count_actions_urls_timeout_and_localized_labels() {
        let icons = icons();
        assert!(
            ToastInstance::parse(br#"{"label":"n","messages":[],"extra":true}"#, &icons).is_err()
        );
        assert!(ToastInstance::parse(
            br#"{"label":"n","messages":[{"title":"x","body":"y","tone":"neutral"}]}"#,
            &icons
        )
        .is_err());
        let mut x = ToastInstance::parse(&fixture(), &icons).unwrap();
        x.messages[0].action_href = Some("javascript:alert(1)".into());
        x.messages[0].action_label = Some("Open".into());
        assert!(x.validate(&icons).is_err());
        let mut x = ToastInstance::parse(&fixture(), &icons).unwrap();
        x.messages[0].timeout = Some(999);
        assert!(x.validate(&icons).is_err());
        let mut x = ToastInstance::parse(&fixture(), &icons).unwrap();
        x.messages[0].dismiss_label = Some("\n".into());
        assert!(x.validate(&icons).is_err());
        let mut x = ToastInstance::parse(&fixture(), &icons).unwrap();
        x.messages = vec![x.messages[0].clone(); 9];
        assert!(x.validate(&icons).is_err());
    }
}
