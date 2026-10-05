//! Typed selection and safe static rendering for native progress.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgressInstance {
    pub label: String,
    pub state: ProgressState,
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub maximum: Option<f64>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub size: Size,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressState {
    Determinate,
    Indeterminate,
}

impl ProgressState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Determinate => "determinate",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
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
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Size {
    #[default]
    Regular,
    Compact,
}

impl Size {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Compact => "compact",
        }
    }
}

impl ProgressInstance {
    pub fn validate(&self) -> Result<(), String> {
        if !safe_text(&self.label) {
            return Err("progress label must contain safe, nonblank text".into());
        }
        if let Some(detail) = &self.detail {
            if !safe_text(detail) {
                return Err("progress detail must contain safe, nonblank text".into());
            }
        }
        if let Some(suffix) = &self.suffix {
            if !safe_text(suffix) {
                return Err("progress suffix must contain safe, nonblank text".into());
            }
        }

        match self.state {
            ProgressState::Determinate => {
                let (Some(value), Some(maximum)) = (self.value, self.maximum) else {
                    return Err("determinate progress requires both value and maximum".into());
                };
                if !value.is_finite() || !maximum.is_finite() {
                    return Err("progress value and maximum must be finite numbers".into());
                }
                if maximum <= 0.0 || value < 0.0 || value > maximum {
                    return Err("progress requires maximum > 0 and 0 <= value <= maximum".into());
                }
            }
            ProgressState::Indeterminate => {
                if self.value.is_some() || self.maximum.is_some() {
                    return Err("indeterminate progress cannot include value or maximum".into());
                }
                if self.suffix.is_some() {
                    return Err("indeterminate progress cannot include a value suffix".into());
                }
            }
        }
        Ok(())
    }
}

fn safe_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn number(value: f64) -> String {
    let value = value.to_string();
    value.strip_suffix(".0").unwrap_or(&value).to_owned()
}

/// Renders a progress fragment after validating the adapter-owned slots.
/// The fragment must contain exactly one attributes, label, progress, suffix,
/// and detail slot. Substitutions are single-pass, so slot-shaped user text is
/// escaped as text rather than interpreted as another template slot.
pub fn render(instance: &ProgressInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;

    let mut classes = vec![
        "cui-progress".to_owned(),
        format!("cui-progress--{}", instance.tone.as_str()),
        format!("cui-progress--{}", instance.size.as_str()),
        format!("cui-progress--{}", instance.state.as_str()),
    ];
    if instance.state == ProgressState::Determinate && instance.value == instance.maximum {
        classes.push("cui-progress--complete".into());
    }
    let attributes = format!(
        "class=\"{}\" data-cui-component=\"progress\" data-cui-tone=\"{}\" data-cui-size=\"{}\" data-cui-state=\"{}\"",
        classes.join(" "),
        instance.tone.as_str(),
        instance.size.as_str(),
        instance.state.as_str()
    );
    let progress = match (instance.value, instance.maximum) {
        (Some(value), Some(maximum)) => format!(
            "<progress class=\"cui-progress__track\" value=\"{}\" max=\"{}\" aria-label=\"{}\"></progress>",
            number(value),
            number(maximum),
            escape_html(&instance.label)
        ),
        _ => format!(
            "<progress class=\"cui-progress__track\" aria-label=\"{}\"></progress>",
            escape_html(&instance.label)
        ),
    };
    let label = escape_html(&instance.label);
    let suffix = instance
        .suffix
        .as_deref()
        .map(|text| {
            format!(
                "<span class=\"cui-progress__suffix\">{}</span>",
                escape_html(text)
            )
        })
        .unwrap_or_default();
    let detail = instance
        .detail
        .as_deref()
        .map(|text| {
            format!(
                "<p class=\"cui-progress__detail\">{}</p>",
                escape_html(text)
            )
        })
        .unwrap_or_default();
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label),
            ("[[progress]]", &progress),
            ("[[suffix]]", &suffix),
            ("[[detail]]", &detail),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str =
        "<div [[attributes]]><span>[[label]]</span>[[suffix]][[progress]][[detail]]</div>";

    fn determinate(value: f64, maximum: f64) -> ProgressInstance {
        ProgressInstance {
            label: "Importing records".into(),
            state: ProgressState::Determinate,
            value: Some(value),
            maximum: Some(maximum),
            suffix: Some("42 of 100 records".into()),
            detail: Some("Validating fields".into()),
            tone: Tone::Info,
            size: Size::Regular,
        }
    }

    fn indeterminate() -> ProgressInstance {
        ProgressInstance {
            label: "Preparing release".into(),
            state: ProgressState::Indeterminate,
            value: None,
            maximum: None,
            suffix: None,
            detail: None,
            tone: Tone::Warning,
            size: Size::Compact,
        }
    }

    #[test]
    fn renders_all_tones_and_sizes_and_native_progress_states() {
        for tone in [Tone::Info, Tone::Success, Tone::Warning, Tone::Danger] {
            for size in [Size::Regular, Size::Compact] {
                let mut instance = determinate(42.0, 100.0);
                instance.tone = tone;
                instance.size = size;
                let html = render(&instance, FRAGMENT).unwrap();
                assert!(html.contains(&format!("cui-progress--{}", tone.as_str())));
                assert!(html.contains(&format!("cui-progress--{}", size.as_str())));
                assert!(html.contains("value=\"42\" max=\"100\""));
                assert!(!html.contains("role=\"")); // Native progress supplies its implicit role.
            }
        }
        let html = render(&indeterminate(), FRAGMENT).unwrap();
        assert!(html.contains("cui-progress--indeterminate"));
        assert!(!html.contains(" value=\""));
        assert!(!html.contains(" max=\""));
    }

    #[test]
    fn renders_boundaries_and_complete_state_without_clamping() {
        for (value, maximum) in [(0.0, 1.0), (1.0, 1.0), (0.0, 100.0)] {
            assert!(render(&determinate(value, maximum), FRAGMENT).is_ok());
        }
        let html = render(&determinate(100.0, 100.0), FRAGMENT).unwrap();
        assert!(html.contains("cui-progress--complete"));
        assert!(render(&determinate(101.0, 100.0), FRAGMENT).is_err());
    }

    #[test]
    fn escapes_plain_text_and_does_not_reinterpret_slot_shaped_text() {
        let mut instance = determinate(2.5, 10.0);
        instance.label = "Task [[detail]] <done> & \"quoted\"".into();
        instance.suffix = Some("2.5 & counting".into());
        let html = render(&instance, FRAGMENT).unwrap();
        assert!(html.contains("Task [[detail]] &lt;done&gt; &amp; &quot;quoted&quot;"));
        assert!(
            html.contains("aria-label=\"Task [[detail]] &lt;done&gt; &amp; &quot;quoted&quot;\"")
        );
        assert!(html.contains("2.5 &amp; counting"));
        assert_eq!(html.matches("[[detail]]").count(), 2);
    }

    #[test]
    fn rejects_missing_or_inconsistent_state_and_invalid_numbers() {
        for instance in [
            determinate(-1.0, 10.0),
            determinate(11.0, 10.0),
            determinate(1.0, 0.0),
            determinate(f64::NAN, 10.0),
            determinate(f64::INFINITY, 10.0),
            determinate(1.0, f64::INFINITY),
        ] {
            assert!(instance.validate().is_err());
        }
        let mut instance = determinate(1.0, 10.0);
        instance.maximum = None;
        assert!(instance.validate().is_err());
        let mut instance = indeterminate();
        instance.value = Some(0.0);
        assert!(instance.validate().is_err());
        instance.value = None;
        instance.suffix = Some("done".into());
        assert!(instance.validate().is_err());
    }

    #[test]
    fn rejects_bad_text_unknown_json_fields_and_bad_slots() {
        let mut instance = determinate(1.0, 2.0);
        instance.label = "  ".into();
        assert!(instance.validate().is_err());
        instance = determinate(1.0, 2.0);
        instance.detail = Some("line\nbreak".into());
        assert!(instance.validate().is_err());
        assert!(serde_json::from_str::<ProgressInstance>(
            r#"{"label":"x","state":"determinate","value":1,"maximum":2,"other":true}"#
        )
        .is_err());
        assert!(
            serde_json::from_str::<ProgressInstance>(r#"{"label":"x","state":"other"}"#).is_err()
        );
        assert!(render(
            &determinate(1.0, 2.0),
            "<div [[attributes]]>[[label]][[progress]][[suffix]]</div>"
        )
        .is_err());
        assert!(render(
            &determinate(1.0, 2.0),
            "[[attributes]][[label]][[progress]][[suffix]][[detail]][[extra]]"
        )
        .is_err());
        assert!(render(
            &determinate(1.0, 2.0),
            "[[attributes]][[label]][[progress]][[suffix][[detail]]"
        )
        .is_err());
    }

    #[test]
    fn deserializes_explicit_state_and_defaults_presentation_options() {
        let parsed: ProgressInstance =
            serde_json::from_str(r#"{"label":"Waiting","state":"indeterminate"}"#).unwrap();
        assert_eq!(parsed.state, ProgressState::Indeterminate);
        assert_eq!(parsed.tone, Tone::Info);
        assert_eq!(parsed.size, Size::Regular);
        assert!(parsed.validate().is_ok());
        assert!(serde_json::from_str::<ProgressInstance>(r#"{"label":"Waiting"}"#).is_err());
    }
}
