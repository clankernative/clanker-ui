//! Typed rendering for a form-confirmation dialog; the application still owns submission policy.
use crate::{
    fragment,
    icon::{self, IconInstance, IconSize},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Neutral,
    Danger,
}
impl Tone {
    fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Danger => "danger",
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmDialogInstance {
    pub id: String,
    pub form_id: String,
    pub title: String,
    pub body: String,
    pub trigger_label: String,
    pub confirm_label: String,
    #[serde(default = "default_cancel_label")]
    pub cancel_label: String,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub heading_level: HeadingLevel,
    #[serde(default)]
    pub submit_name: Option<String>,
    #[serde(default)]
    pub submit_value: Option<String>,
}
fn default_cancel_label() -> String {
    "Cancel".into()
}
impl ConfirmDialogInstance {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let x: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        x.validate()?;
        Ok(x)
    }
    pub fn validate(&self) -> Result<(), String> {
        for (v, n) in [(&self.id, "id"), (&self.form_id, "formId")] {
            if !valid_id(v) {
                return Err(format!("confirm dialog {n} must be lowercase kebab-case"));
            }
        }
        for (v, n) in [
            (&self.title, "title"),
            (&self.body, "body"),
            (&self.trigger_label, "triggerLabel"),
            (&self.confirm_label, "confirmLabel"),
            (&self.cancel_label, "cancelLabel"),
        ] {
            text(v, n)?;
        }
        match (&self.submit_name, &self.submit_value) {
            (Some(n), Some(v)) => {
                if !valid_field(n) {
                    return Err("submitName must be a safe form field token".into());
                }
                text(v, "submitValue")?;
            }
            (None, None) => {}
            _ => return Err("submitName and submitValue must be provided together".into()),
        }
        if self.derived_ids().contains(&self.form_id) {
            return Err("confirm dialog formId must not collide with a derived dialog id".into());
        }
        Ok(())
    }
    pub fn derived_ids(&self) -> BTreeSet<String> {
        BTreeSet::from([
            self.id.clone(),
            format!("{}-title", self.id),
            format!("{}-body", self.id),
        ])
    }
}
pub fn render(
    instance: &ConfirmDialogInstance,
    fragment_text: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate()?;
    let icon = icon::render(
        &IconInstance {
            name: "alert".into(),
            size: IconSize::Large,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )?;
    let level = instance.heading_level.as_str();
    let submit = instance
        .submit_name
        .as_ref()
        .map(|name| {
            format!(
                " name=\"{}\" value=\"{}\"",
                escape(name),
                escape(instance.submit_value.as_ref().unwrap())
            )
        })
        .unwrap_or_default();
    let html = format!(
        "<div class=\"cui-confirm-dialog cui-confirm-dialog--{}\" data-cui-component=\"confirm-dialog\"><button class=\"cui-button cui-button--{} cui-button--standard\" type=\"submit\" form=\"{}\" data-cui-confirm-trigger{submit}>{}</button><dialog class=\"cui-confirm-dialog__dialog\" id=\"{}\" aria-labelledby=\"{}-title\" aria-describedby=\"{}-body\" data-cui-confirm-dialog><div class=\"cui-confirm-dialog__surface\"><span class=\"cui-confirm-dialog__icon\" aria-hidden=\"true\">{icon}</span><div class=\"cui-confirm-dialog__content\"><{level} class=\"cui-confirm-dialog__title\" id=\"{}-title\">{}</{level}><p class=\"cui-confirm-dialog__body\" id=\"{}-body\">{}</p></div><div class=\"cui-confirm-dialog__actions\"><button class=\"cui-button cui-button--secondary cui-button--standard\" type=\"button\" data-cui-confirm-cancel>{}</button><button class=\"cui-button cui-button--{} cui-button--standard\" type=\"button\" data-cui-confirm-accept>{}</button></div></div></dialog></div>",
        instance.tone.as_str(),
        if instance.tone == Tone::Danger {
            "danger"
        } else {
            "secondary"
        },
        escape(&instance.form_id),
        escape(&instance.trigger_label),
        escape(&instance.id),
        escape(&instance.id),
        escape(&instance.id),
        escape(&instance.id),
        escape(&instance.title),
        escape(&instance.id),
        escape(&instance.body),
        escape(&instance.cancel_label),
        if instance.tone == Tone::Danger {
            "danger"
        } else {
            "primary"
        },
        escape(&instance.confirm_label)
    );
    fragment::fill(fragment_text, &[("[[confirm_dialog]]", &html)])
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('-')
        && !v.ends_with('-')
        && v.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn valid_field(v: &str) -> bool {
    let mut bytes = v.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z'))
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
}
fn text(v: &str, n: &str) -> Result<(), String> {
    if v.trim().is_empty() || v.chars().any(char::is_control) {
        Err(format!(
            "confirm dialog {n} must contain safe, nonblank text"
        ))
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
    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([("alert".into(), "<path d=\"M1 1\"></path>".into())])
    }
    fn sample() -> ConfirmDialogInstance {
        ConfirmDialogInstance {
            id: "remove-source".into(),
            form_id: "remove-source-form".into(),
            title: "Remove <source>?".into(),
            body: "Cannot undo".into(),
            trigger_label: "Remove".into(),
            confirm_label: "Yes, remove".into(),
            cancel_label: default_cancel_label(),
            tone: Tone::Danger,
            heading_level: HeadingLevel::H3,
            submit_name: Some("operation".into()),
            submit_value: Some("remove".into()),
        }
    }
    #[test]
    fn renders_all_semantic_variants_and_real_submit_fallback() {
        for tone in [Tone::Neutral, Tone::Danger] {
            for heading_level in [HeadingLevel::H2, HeadingLevel::H3, HeadingLevel::H4] {
                let mut x = sample();
                x.tone = tone;
                x.heading_level = heading_level;
                let h = render(&x, "[[confirm_dialog]]", &icons()).unwrap();
                for v in [
                    "type=\"submit\" form=\"remove-source-form\"",
                    "name=\"operation\" value=\"remove\"",
                    heading_level.as_str(),
                    "data-cui-confirm-accept",
                    "data-cui-confirm-cancel",
                    "Remove &lt;source&gt;?",
                ] {
                    assert!(h.contains(v), "{v}");
                }
            }
        }
    }
    #[test]
    fn declared_variant_goldens_match_exact_rendering() {
        let package_icons: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        for fixture in [
            include_str!(
                "../../../packages/vanilla/components/confirm-dialog/fixtures/neutral-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/confirm-dialog/fixtures/danger-golden.json"
            ),
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let instance: ConfirmDialogInstance =
                serde_json::from_value(value["options"].clone()).unwrap();
            assert_eq!(
                render(&instance, "[[confirm_dialog]]\n", &package_icons).unwrap(),
                value["expectedHtml"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn rejects_unknown_malformed_pair_and_fragment_errors() {
        assert!(ConfirmDialogInstance::parse(br#"{"id":"bad id"}"#).is_err());
        let mut x = sample();
        x.submit_name = None;
        assert!(x.validate().is_err());
        let mut x = sample();
        x.submit_name = Some("bad name".into());
        assert!(x.validate().is_err());
        let mut x = sample();
        x.form_id = "remove-source-title".into();
        assert!(x.validate().is_err());
        assert!(ConfirmDialogInstance::parse(br#"{"id":"x","formId":"f","title":"T","body":"B","triggerLabel":"Go","confirmLabel":"Yes","rawHtml":"<b>"}"#).is_err());
        for f in [
            "",
            "[[confirm_dialog]][[confirm_dialog]]",
            "[[confirm_dialog]][[other]]",
        ] {
            assert!(render(&sample(), f, &icons()).is_err());
        }
    }
}
