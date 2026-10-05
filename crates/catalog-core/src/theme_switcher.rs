//! Typed browser-local theme selection; the app owns theme tokens and their CSS effects.
use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Presentation {
    #[default]
    Text,
    Icons,
}
impl Presentation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Icons => "icons",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub icon: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeSwitcherInstance {
    pub label: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub presentation: Presentation,
    pub default_choice: String,
    pub system_light_theme: String,
    pub system_dark_theme: String,
    #[serde(default = "default_storage_key")]
    pub storage_key: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
}
fn default_storage_key() -> Option<String> {
    Some("clanker-theme".into())
}
fn valid_token(v: &str) -> bool {
    let mut bytes = v.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z'))
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
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
impl ThemeSwitcherInstance {
    /// Parse JSON and validate all configured choices and locked icon geometry.
    pub fn parse(bytes: &[u8], icons: &BTreeMap<String, String>) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate(icons)?;
        Ok(value)
    }
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_text(&self.label) {
            return Err("theme switcher label must contain safe, nonblank text".into());
        }
        if !(2..=8).contains(&self.choices.len()) {
            return Err("theme switcher requires between two and eight choices".into());
        }
        let mut values = BTreeSet::new();
        for choice in &self.choices {
            if !valid_token(&choice.value) {
                return Err("theme choice value must be a safe token".into());
            }
            if !values.insert(choice.value.as_str()) {
                return Err("theme choice values must be unique".into());
            }
            if !valid_text(&choice.label) {
                return Err("theme choice label must contain safe, nonblank text".into());
            }
            icon::render(
                &IconInstance {
                    name: choice.icon.clone(),
                    size: IconSize::Small,
                    label: None,
                },
                "<svg [[attributes]]>[[geometry]]</svg>",
                icons,
            )?;
        }
        for (name, value) in [
            ("defaultChoice", &self.default_choice),
            ("systemLightTheme", &self.system_light_theme),
            ("systemDarkTheme", &self.system_dark_theme),
        ] {
            if !valid_token(value) || !values.contains(value.as_str()) {
                return Err(format!(
                    "theme switcher {name} must match a configured choice"
                ));
            }
        }
        if self.system_light_theme == "system" || self.system_dark_theme == "system" {
            return Err(
                "systemLightTheme and systemDarkTheme must name concrete theme choices".into(),
            );
        }
        if self.storage_key.as_ref().is_some_and(|v| !valid_token(v)) {
            return Err("theme switcher storageKey must be a safe token".into());
        }
        if self.target_id.as_ref().is_some_and(|v| !valid_token(v)) {
            return Err("theme switcher targetId must be a safe id token".into());
        }
        Ok(())
    }
    /// The switcher emits no IDs; `targetId`, when present, refers to an app-owned element.
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }
}
/// Render controls as a progressive enhancement over meaningful no-JS fallback text.
pub fn render(
    instance: &ThemeSwitcherInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    let mut choices = String::new();
    for choice in &instance.choices {
        let icon = icon::render(
            &IconInstance {
                name: choice.icon.clone(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        choices.push_str(&format!("<button class=\"cui-theme-switcher__option\" type=\"button\" title=\"{}\" data-cui-theme-value=\"{}\" aria-pressed=\"false\" hidden><span class=\"cui-theme-switcher__icon\" aria-hidden=\"true\">{}</span><span class=\"cui-theme-switcher__label\">{}</span></button>", escape(&choice.label), escape(&choice.value), icon, escape(&choice.label)));
    }
    let target = instance
        .target_id
        .as_ref()
        .map(|v| format!(" data-cui-theme-target-id=\"{}\"", escape(v)))
        .unwrap_or_default();
    let html = format!(
        "<div class=\"cui-theme-switcher cui-theme-switcher--{}\" data-cui-component=\"theme-switcher\" data-cui-theme-default=\"{}\" data-cui-theme-system-light=\"{}\" data-cui-theme-system-dark=\"{}\" data-cui-theme-storage-key=\"{}\"{} role=\"group\" aria-label=\"{}\"><p class=\"cui-theme-switcher__fallback\">Theme: {} (interactive choices require JavaScript).</p><div class=\"cui-theme-switcher__choices\">{}</div></div>",
        instance.presentation.as_str(),
        escape(&instance.default_choice),
        escape(&instance.system_light_theme),
        escape(&instance.system_dark_theme),
        escape(instance.storage_key.as_deref().unwrap_or("")),
        target,
        escape(&instance.label),
        escape(
            &instance
                .choices
                .iter()
                .find(|c| c.value == instance.default_choice)
                .expect("validated default choice")
                .label
        ),
        choices
    );
    crate::fragment::fill(fragment, &[("[[themeSwitcher]]", &html)])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn icons() -> BTreeMap<String, String> {
        ["contrast", "sun", "moon", "waves", "leaf"]
            .into_iter()
            .map(|n| (n.into(), "<path d=\"M1 1\"></path>".into()))
            .collect()
    }
    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/vanilla/components/theme-switcher/fixtures/default.json"
        ))
        .unwrap()
    }
    #[test]
    fn default_fixture_matches_golden_and_application_values_remain_open() {
        let icons = icons();
        let value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        let html = render(
            &value,
            include_str!("../../../packages/vanilla/components/theme-switcher/fragment.html"),
            &icons,
        )
        .unwrap();
        assert_eq!(
            html,
            include_str!(
                "../../../packages/vanilla/components/theme-switcher/fixtures/default.golden.html"
            )
        );
        let custom = ThemeSwitcherInstance {
            label: "Appearance".into(),
            choices: vec![
                Choice {
                    value: "system".into(),
                    label: "Automatic".into(),
                    icon: "contrast".into(),
                },
                Choice {
                    value: "oceanic".into(),
                    label: "Oceanic".into(),
                    icon: "waves".into(),
                },
                Choice {
                    value: "forest".into(),
                    label: "Forest".into(),
                    icon: "leaf".into(),
                },
            ],
            presentation: Presentation::Icons,
            default_choice: "system".into(),
            system_light_theme: "oceanic".into(),
            system_dark_theme: "forest".into(),
            storage_key: None,
            target_id: Some("preview-root".into()),
        };
        assert!(custom.validate(&icons).is_ok());
        assert!(custom.derived_ids().is_empty());
    }
    #[test]
    fn rejects_unknown_fields_counts_duplicates_unsafe_ids_mismatched_themes_and_icons() {
        let icons = icons();
        assert!(ThemeSwitcherInstance::parse(br#"{"label":"Theme","extra":1}"#, &icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.choices.pop();
        assert!(value.validate(&icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.choices[1].value = "system".into();
        assert!(value.validate(&icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.target_id = Some("#root".into());
        assert!(value.validate(&icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.default_choice = "custom".into();
        assert!(value.validate(&icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.choices[0].icon = "script".into();
        assert!(value.validate(&icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.system_light_theme = "system".into();
        assert!(value.validate(&icons).is_err());
        let mut value = ThemeSwitcherInstance::parse(&fixture(), &icons).unwrap();
        value.system_dark_theme = "system".into();
        assert!(value.validate(&icons).is_err());
    }
}
