//! Selectable literal values with progressive, presentation-only clipboard feedback.
use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyFieldInstance {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default = "idle_label")]
    pub idle_label: String,
    #[serde(default = "copied_label")]
    pub copied_label: String,
    #[serde(default = "failed_label")]
    pub failed_label: String,
}
fn idle_label() -> String {
    "Copy value".into()
}
fn copied_label() -> String {
    "Copied".into()
}
fn failed_label() -> String {
    "Copy failed; select the value to copy it.".into()
}
fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('_')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
}
fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
}
fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl CopyFieldInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let instance: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        instance.validate()?;
        Ok(instance)
    }
    pub fn control_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }
    /// These names must be reserved by the host alongside authored template IDs.
    pub fn derived_ids(&self) -> Vec<String> {
        let id = self.control_id();
        vec![id.into(), format!("{id}-hint"), format!("{id}-status")]
    }
    pub fn validate(&self) -> Result<(), String> {
        if !token(&self.name) || !token(self.control_id()) {
            return Err("copy field name/id must be safe non-reserved tokens".into());
        }
        if [
            &self.label,
            &self.value,
            &self.idle_label,
            &self.copied_label,
            &self.failed_label,
        ]
        .iter()
        .any(|value| !text(value))
            || self.hint.as_deref().is_some_and(|value| !text(value))
        {
            return Err(
                "copy field values and feedback must be nonblank plain text of at most 4096 bytes"
                    .into(),
            );
        }
        Ok(())
    }
}
pub fn render(
    instance: &CopyFieldInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate()?;
    let id = esc(instance.control_id());
    let icon = icon::render(
        &IconInstance {
            name: "copy".into(),
            size: IconSize::Small,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )?;
    let attributes = "class=\"cui-copy-field\" data-cui-component=\"copy-field\"";
    let described = if instance.hint.is_some() {
        format!(" aria-describedby=\"{id}-hint\"")
    } else {
        String::new()
    };
    let control = format!(
        "<label class=\"cui-copy-field__label\" for=\"{id}\">{}</label><div class=\"cui-copy-field__row\"><input class=\"cui-copy-field__control\" id=\"{id}\" name=\"{}\" type=\"text\" value=\"{}\" readonly spellcheck=\"false\"{described} data-cui-copy-source><button class=\"cui-copy-field__button\" type=\"button\" aria-label=\"{}\" data-cui-copy-trigger data-cui-copy-label-idle=\"{}\" data-cui-copy-label-copied=\"{}\" data-cui-copy-label-failed=\"{}\" hidden>{icon}</button></div>",
        esc(&instance.label),
        esc(&instance.name),
        esc(&instance.value),
        esc(&instance.idle_label),
        esc(&instance.idle_label),
        esc(&instance.copied_label),
        esc(&instance.failed_label)
    );
    let hint = instance
        .hint
        .as_deref()
        .map(|hint| {
            format!(
                "<p class=\"cui-copy-field__hint\" id=\"{id}-hint\">{}</p>",
                esc(hint)
            )
        })
        .unwrap_or_default();
    let description = format!(
        "{hint}<p class=\"cui-copy-field__status\" id=\"{id}-status\" aria-live=\"polite\" aria-atomic=\"true\" data-cui-copy-status></p>"
    );
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", attributes),
            ("[[control]]", &control),
            ("[[description]]", &description),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<div [[attributes]]>[[control]][[description]]</div>";
    fn instance() -> CopyFieldInstance {
        CopyFieldInstance::parse(
            r#"{"name":"ticket","label":"Ticket <&>","value":"TASK-42","hint":"Select & copy"}"#,
        )
        .unwrap()
    }
    #[test]
    fn renders_selectable_native_fallback_with_closed_icon_and_feedback() {
        let icons =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        let html = render(&instance(), F, &icons).unwrap();
        assert!(html.contains("readonly"));
        assert!(html.contains("Ticket &lt;&amp;&gt;"));
        assert!(html.contains("aria-describedby=\"ticket-hint\""));
        assert!(html.contains(" hidden>"));
        assert_eq!(
            instance().derived_ids(),
            ["ticket", "ticket-hint", "ticket-status"]
        );
    }
    #[test]
    fn rejects_unknown_fields_reserved_tokens_controls_and_missing_fragment_slots() {
        assert!(CopyFieldInstance::parse(
            r#"{"name":"x","label":"X","value":"v","html":"<b>no</b>"}"#
        )
        .is_err());
        for change in ["_private", "bad name", ""] {
            let mut i = instance();
            i.name = change.into();
            assert!(i.validate().is_err());
        }
        for change in ["", "\n", "v\u{7f}"] {
            let mut i = instance();
            i.value = change.into();
            assert!(i.validate().is_err());
        }
        let icons =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        assert!(render(&instance(), "[[attributes]][[control]]", &icons).is_err());
        assert!(render(&instance(), F, &BTreeMap::new()).is_err());
    }
}
