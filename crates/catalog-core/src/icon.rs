//! Typed selection and safe rendering for the package's closed SVG icon catalog.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl IconSize {
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
pub struct IconInstance {
    pub name: String,
    #[serde(default)]
    pub size: IconSize,
    #[serde(default)]
    pub label: Option<String>,
}

impl IconInstance {
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !icons.contains_key(&self.name) {
            return Err(format!("unknown icon: {}", self.name));
        }
        if self.name.is_empty()
            || !self
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err("icon name must be a closed catalog name".into());
        }
        if let Some(label) = &self.label {
            if label.trim().is_empty() || label.chars().any(char::is_control) {
                return Err("icon label must contain safe text".into());
            }
        }
        Ok(())
    }
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn safe_geometry(value: &str) -> bool {
    // Package geometry is a fragment, never a complete SVG or executable markup.
    let lowercase = value.to_ascii_lowercase();
    !value.is_empty()
        && !lowercase.contains("<script")
        && !lowercase.contains("<svg")
        && !lowercase.contains("foreignobject")
        && !lowercase.contains("javascript:")
        && !lowercase.contains("href")
        && !lowercase.contains("style")
        && !value.contains("{{")
        && !value.contains("[[")
        && !value.contains('&')
        && !value.split_whitespace().any(|part| {
            let part = part.to_ascii_lowercase();
            part.starts_with("on") && part.contains('=')
        })
        && value
            .chars()
            .all(|character| !character.is_control() || character == '\n' || character == '\t')
}

/// Renders one checked icon against the adapter-owned fragment template.
/// The fragment must contain exactly one `[[attributes]]` and `[[geometry]]` slot.
pub fn render(
    instance: &IconInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    if fragment.matches("[[attributes]]").count() != 1
        || fragment.matches("[[geometry]]").count() != 1
    {
        return Err("icon fragment must contain exactly one attributes and geometry slot".into());
    }
    if fragment.matches("[[").count() != 2 {
        return Err("icon fragment contains an unsupported slot".into());
    }
    let geometry = icons
        .get(&instance.name)
        .ok_or_else(|| format!("unknown icon: {}", instance.name))?;
    if !safe_geometry(geometry) {
        return Err(format!("unsafe SVG geometry for icon: {}", instance.name));
    }
    let attributes = format!(
        "class=\"cui-icon cui-icon--{}\" data-cui-component=\"icon\" data-cui-icon=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" focusable=\"false\"{}",
        instance.size.as_str(),
        instance.name,
        match &instance.label {
            Some(label) => format!(" role=\"img\" aria-label=\"{}\"", escape_attribute(label)),
            None => " aria-hidden=\"true\"".into(),
        }
    );
    let output = fragment
        .replace("[[attributes]]", &attributes)
        .replace("[[geometry]]", geometry);
    if output.contains("[[") {
        return Err("icon fragment contains an unsupported slot".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([("check".into(), "<path d=\"M1 1\"></path>".into())])
    }

    fn icon() -> IconInstance {
        IconInstance {
            name: "check".into(),
            size: IconSize::Medium,
            label: None,
        }
    }

    const FRAGMENT: &str = "<svg [[attributes]]>[[geometry]]</svg>";

    #[test]
    fn decorative_is_hidden_and_labeled_icon_is_an_image() {
        let decorative = render(&icon(), FRAGMENT, &icons()).unwrap();
        assert!(decorative.contains("aria-hidden=\"true\""));
        assert!(!decorative.contains("role=\"img\""));
        let labeled = IconInstance {
            label: Some("Status & <check>\"".into()),
            ..icon()
        };
        let labeled = render(&labeled, FRAGMENT, &icons()).unwrap();
        assert!(labeled.contains("role=\"img\""));
        assert!(labeled.contains("aria-label=\"Status &amp; &lt;check&gt;&quot;\""));
        assert!(!labeled.contains("aria-hidden"));
    }

    #[test]
    fn closed_names_and_unsafe_values_are_rejected() {
        for name in ["missing", "check\"><script>", "Check", "../check"] {
            let instance = IconInstance {
                name: name.into(),
                ..icon()
            };
            assert!(instance.validate(&icons()).is_err(), "{name}");
        }
        let instance = IconInstance {
            label: Some("  ".into()),
            ..icon()
        };
        assert!(instance.validate(&icons()).is_err());
        let mut malicious_icons = icons();
        malicious_icons.insert("check".into(), "<script>alert(1)</script>".into());
        assert!(render(&icon(), FRAGMENT, &malicious_icons).is_err());
    }

    #[test]
    fn sizes_and_output_are_deterministic_and_slots_are_exact() {
        for (size, class) in [
            (IconSize::Small, "cui-icon--small"),
            (IconSize::Medium, "cui-icon--medium"),
            (IconSize::Large, "cui-icon--large"),
        ] {
            let instance = IconInstance { size, ..icon() };
            let first = render(&instance, FRAGMENT, &icons()).unwrap();
            assert_eq!(first, render(&instance, FRAGMENT, &icons()).unwrap());
            assert!(first.contains(class));
        }
        assert!(render(
            &icon(),
            "<svg [[attributes]]>[[geometry]][[geometry]]</svg>",
            &icons()
        )
        .is_err());
    }
}
