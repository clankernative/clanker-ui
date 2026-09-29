//! Typed selection and safe rendering for the static semantic divider component.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Start,
    #[default]
    Center,
    End,
}

impl Alignment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DividerInstance {
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub alignment: Alignment,
}

impl DividerInstance {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(label) = &self.label {
            if label.trim().is_empty() || label.chars().any(char::is_control) {
                return Err("divider label must contain safe, nonblank text".into());
            }
            if self.orientation == Orientation::Vertical {
                return Err("vertical dividers cannot contain a label".into());
            }
        }
        Ok(())
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Renders a divider against the adapter-owned fragment. The fragment must contain
/// exactly one `[[attributes]]` slot and one `[[label]]` slot.
pub fn render(instance: &DividerInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    if fragment.matches("[[attributes]]").count() != 1 || fragment.matches("[[label]]").count() != 1
    {
        return Err("divider fragment must contain exactly one attributes and label slot".into());
    }
    if fragment.matches("[[").count() != 2 {
        return Err("divider fragment contains an unsupported slot".into());
    }

    let mut classes = vec!["cui-divider".to_owned()];
    classes.push(format!("cui-divider--{}", instance.orientation.as_str()));
    if instance.label.is_some() {
        classes.push("cui-divider--labelled".into());
        classes.push(format!("cui-divider--{}", instance.alignment.as_str()));
    }
    let attributes = format!(
        "class=\"{}\" data-cui-component=\"divider\" role=\"separator\" aria-orientation=\"{}\"{}",
        classes.join(" "),
        instance.orientation.as_str(),
        instance
            .label
            .as_deref()
            .map(|label| format!(" aria-label=\"{}\"", escape_html(label)))
            .unwrap_or_default(),
    );
    let label = instance
        .label
        .as_deref()
        .map(|value| {
            format!(
                "<span class=\"cui-divider__label\">{}</span>",
                escape_html(value)
            )
        })
        .unwrap_or_default();
    crate::fragment::fill(
        fragment,
        &[("[[attributes]]", &attributes), ("[[label]]", &label)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "<div [[attributes]]>[[label]]</div>";

    #[test]
    fn defaults_to_unlabelled_horizontal_separator() {
        let rendered = render(&DividerInstance::default(), FRAGMENT).unwrap();
        assert!(rendered.contains("cui-divider--horizontal"));
        assert!(rendered.contains("role=\"separator\" aria-orientation=\"horizontal\""));
        assert!(!rendered.contains("cui-divider--labelled"));
        assert!(!rendered.contains("cui-divider__label"));
        assert!(!rendered.contains("aria-label"));
    }

    #[test]
    fn encodes_aligned_label_in_text_and_accessible_name() {
        let instance = DividerInstance {
            label: Some("Recent <activity> & \"updates\"".into()),
            alignment: Alignment::Start,
            ..DividerInstance::default()
        };
        let rendered = render(&instance, FRAGMENT).unwrap();
        assert!(rendered.contains("cui-divider--start"));
        assert!(
            rendered.contains("aria-label=\"Recent &lt;activity&gt; &amp; &quot;updates&quot;\"")
        );
        assert!(rendered.contains("<span class=\"cui-divider__label\">Recent &lt;activity&gt; &amp; &quot;updates&quot;</span>"));
    }

    #[test]
    fn supports_vertical_rule_and_rejects_vertical_label() {
        let vertical = DividerInstance {
            orientation: Orientation::Vertical,
            ..DividerInstance::default()
        };
        let rendered = render(&vertical, FRAGMENT).unwrap();
        assert!(rendered.contains("cui-divider--vertical"));
        assert!(rendered.contains("aria-orientation=\"vertical\""));

        let invalid = DividerInstance {
            orientation: Orientation::Vertical,
            label: Some("Not allowed".into()),
            ..DividerInstance::default()
        };
        assert!(render(&invalid, FRAGMENT).is_err());
    }

    #[test]
    fn rejects_blank_or_control_character_labels_and_invalid_fragments() {
        for label in ["   ", "line\nbreak"] {
            let instance = DividerInstance {
                label: Some(label.into()),
                ..DividerInstance::default()
            };
            assert!(instance.validate().is_err());
        }
        assert!(render(&DividerInstance::default(), "<div [[attributes]]></div>").is_err());
        assert!(render(
            &DividerInstance::default(),
            "<div [[attributes]]>[[label]][[unexpected]]</div>"
        )
        .is_err());
    }

    #[test]
    fn deserializes_default_values_and_rejects_unknown_properties() {
        let instance: DividerInstance = serde_json::from_str("{}").unwrap();
        assert_eq!(instance.orientation, Orientation::Horizontal);
        assert_eq!(instance.alignment, Alignment::Center);
        assert!(instance.label.is_none());
        assert!(serde_json::from_str::<DividerInstance>(r#"{"size":"large"}"#).is_err());
        assert!(serde_json::from_str::<DividerInstance>(r#"{"orientation":"diagonal"}"#).is_err());
    }
}
