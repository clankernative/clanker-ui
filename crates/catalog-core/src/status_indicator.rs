//! Typed selection and safe rendering for the static status-indicator component.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    #[default]
    Small,
    Large,
}

impl Size {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Large => "large",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusIndicatorInstance {
    pub label: String,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub pulse: bool,
}

impl StatusIndicatorInstance {
    pub fn validate(&self) -> Result<(), String> {
        validate_text(&self.label, "status label")?;
        if let Some(detail) = &self.detail {
            validate_text(detail, "status detail")?;
        }
        Ok(())
    }
}

fn validate_text(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(format!("{name} must contain safe, nonblank text"));
    }
    Ok(())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Renders a status indicator against its adapter-owned fragment. The fragment must
/// contain exactly one `[[attributes]]`, `[[label]]`, and `[[detail]]` slot.
pub fn render(instance: &StatusIndicatorInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    if fragment.matches("[[attributes]]").count() != 1
        || fragment.matches("[[label]]").count() != 1
        || fragment.matches("[[detail]]").count() != 1
    {
        return Err(
            "status-indicator fragment must contain exactly one attributes, label, and detail slot"
                .into(),
        );
    }
    if fragment.matches("[[").count() != 3 {
        return Err("status-indicator fragment contains an unsupported slot".into());
    }

    let mut classes = vec![
        "cui-status-indicator".to_owned(),
        format!("cui-status-indicator--{}", instance.tone.as_str()),
        format!("cui-status-indicator--{}", instance.size.as_str()),
    ];
    if instance.pulse {
        classes.push("cui-status-indicator--pulse".into());
    }
    let attributes = format!(
        "class=\"{}\" data-cui-component=\"status-indicator\"",
        classes.join(" ")
    );
    let detail = instance
        .detail
        .as_deref()
        .map(|text| {
            format!(
                "<span class=\"cui-status-indicator__detail\">{}</span>",
                escape_html(text)
            )
        })
        .unwrap_or_default();
    let output = fragment
        .replace("[[attributes]]", &attributes)
        .replace("[[label]]", &escape_html(&instance.label))
        .replace("[[detail]]", &detail);
    if output.contains("[[") {
        return Err("status-indicator fragment contains an unsupported slot".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "<span [[attributes]]><span class=\"cui-status-indicator__dot\" aria-hidden=\"true\"></span><span class=\"cui-status-indicator__label\">[[label]]</span>[[detail]]</span>";

    fn instance() -> StatusIndicatorInstance {
        StatusIndicatorInstance {
            label: "Operational".into(),
            tone: Tone::Success,
            detail: None,
            size: Size::Small,
            pulse: false,
        }
    }

    #[test]
    fn renders_each_semantic_tone_and_size_with_defaults() {
        for (tone, tone_class) in [
            (Tone::Neutral, "neutral"),
            (Tone::Info, "info"),
            (Tone::Success, "success"),
            (Tone::Warning, "warning"),
            (Tone::Danger, "danger"),
        ] {
            let rendered =
                render(&StatusIndicatorInstance { tone, ..instance() }, FRAGMENT).unwrap();
            assert!(rendered.contains(&format!("cui-status-indicator--{tone_class}")));
            assert!(rendered.contains("cui-status-indicator--small"));
            assert!(rendered.contains("aria-hidden=\"true\""));
            assert!(rendered.contains(">Operational</span>"));
            assert!(!rendered.contains("role=\"status\""));
        }
        let large = render(
            &StatusIndicatorInstance {
                size: Size::Large,
                ..instance()
            },
            FRAGMENT,
        )
        .unwrap();
        assert!(large.contains("cui-status-indicator--large"));
    }

    #[test]
    fn renders_optional_detail_and_pulse_without_losing_text_semantics() {
        let rendered = render(
            &StatusIndicatorInstance {
                detail: Some("All regions responding".into()),
                pulse: true,
                ..instance()
            },
            FRAGMENT,
        )
        .unwrap();
        assert!(rendered.contains("cui-status-indicator--pulse"));
        assert!(rendered.contains(
            "<span class=\"cui-status-indicator__detail\">All regions responding</span>"
        ));
        assert!(rendered.contains("Operational"));
    }

    #[test]
    fn escapes_label_and_detail_and_rejects_invalid_text_or_slots() {
        let rendered = render(
            &StatusIndicatorInstance {
                label: "Delayed <queue> & \"workers\"".into(),
                detail: Some("Waiting & retrying <now>".into()),
                ..instance()
            },
            FRAGMENT,
        )
        .unwrap();
        assert!(rendered.contains("Delayed &lt;queue&gt; &amp; &quot;workers&quot;"));
        assert!(rendered.contains("Waiting &amp; retrying &lt;now&gt;"));
        for label in ["   ", "line\nbreak"] {
            assert!(render(
                &StatusIndicatorInstance {
                    label: label.into(),
                    ..instance()
                },
                FRAGMENT
            )
            .is_err());
        }
        assert!(render(&instance(), "<span [[attributes]]>[[label]]</span>").is_err());
        assert!(render(
            &instance(),
            "<span [[attributes]]>[[label]][[detail]][[unexpected]]</span>"
        )
        .is_err());
    }

    #[test]
    fn serde_defaults_options_and_rejects_unknown_values() {
        let parsed: StatusIndicatorInstance = serde_json::from_str(r#"{"label":"Ready"}"#).unwrap();
        assert_eq!(parsed.tone, Tone::Neutral);
        assert_eq!(parsed.size, Size::Small);
        assert!(!parsed.pulse);
        assert!(parsed.detail.is_none());
        assert!(serde_json::from_str::<StatusIndicatorInstance>(
            r#"{"label":"Ready","tone":"purple"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<StatusIndicatorInstance>(
            r#"{"label":"Ready","unexpected":true}"#
        )
        .is_err());
    }
}
