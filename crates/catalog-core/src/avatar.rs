//! Typed initials-only identity avatar contract and safe static rendering.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Size {
    ExtraSmall,
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
}

impl Size {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExtraSmall => "extra-small",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::ExtraLarge => "extra-large",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Neutral,
    Brand,
    Success,
}

impl Tone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Brand => "brand",
            Self::Success => "success",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AvatarInstance {
    pub initials: String,
    pub label: String,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub tone: Tone,
}

impl AvatarInstance {
    pub fn validate(&self) -> Result<(), String> {
        validate_text(&self.initials, "avatar initials")?;
        if self.initials.graphemes(true).count() > 3 {
            return Err("avatar initials must contain at most three grapheme clusters".into());
        }
        validate_text(&self.label, "avatar label")
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

/// Renders a named, noninteractive initials avatar into the single `[[avatar]]` slot.
/// Image inputs are deliberately not admitted by this contract.
pub fn render(instance: &AvatarInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    if fragment.matches("[[avatar]]").count() != 1 {
        return Err("avatar fragment must contain exactly one [[avatar]] slot".into());
    }
    if fragment.matches("[[").count() != 1 || fragment.matches("]]").count() != 1 {
        return Err("avatar fragment contains an unsupported or malformed slot".into());
    }

    let markup = format!(
        "<span class=\"cui-avatar cui-avatar--{} cui-avatar--{}\" role=\"img\" aria-label=\"{}\" data-cui-component=\"avatar\"><span class=\"cui-avatar__initials\" aria-hidden=\"true\">{}</span></span>",
        instance.size.as_str(),
        instance.tone.as_str(),
        escape_html(&instance.label),
        escape_html(&instance.initials),
    );
    crate::fragment::fill(fragment, &[("[[avatar]]", &markup)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "[[avatar]]";

    fn instance() -> AvatarInstance {
        AvatarInstance {
            initials: "ET".into(),
            label: "Emilia Torres".into(),
            size: Size::Medium,
            tone: Tone::Neutral,
        }
    }

    #[test]
    fn renders_every_size_and_tone_as_a_named_image() {
        for (size, class) in [
            (Size::ExtraSmall, "extra-small"),
            (Size::Small, "small"),
            (Size::Medium, "medium"),
            (Size::Large, "large"),
            (Size::ExtraLarge, "extra-large"),
        ] {
            let html = render(&AvatarInstance { size, ..instance() }, FRAGMENT).unwrap();
            assert!(html.contains(&format!("cui-avatar--{class}")));
            assert!(html.contains("role=\"img\" aria-label=\"Emilia Torres\""));
            assert!(html.contains("aria-hidden=\"true\">ET</span>"));
        }
        for (tone, class) in [
            (Tone::Neutral, "neutral"),
            (Tone::Brand, "brand"),
            (Tone::Success, "success"),
        ] {
            let html = render(&AvatarInstance { tone, ..instance() }, FRAGMENT).unwrap();
            assert!(html.contains(&format!("cui-avatar--{class}")));
        }
    }

    #[test]
    fn defaults_and_strict_json_contract_are_enforced() {
        let parsed: AvatarInstance =
            serde_json::from_str(r#"{"initials":"ET","label":"Emilia Torres"}"#).unwrap();
        assert_eq!(parsed.size, Size::Medium);
        assert_eq!(parsed.tone, Tone::Neutral);
        for json in [
            r#"{"initials":"ET","label":"Name","src":"/person.png"}"#,
            r#"{"initials":"ET","label":"Name","image":"/person.png"}"#,
            r#"{"initials":"ET","label":"Name","imageSource":"/person.png"}"#,
            r#"{"initials":"ET","label":"Name","image_source":"/person.png"}"#,
            r#"{"initials":"ET","label":"Name","size":"giant"}"#,
        ] {
            assert!(
                serde_json::from_str::<AvatarInstance>(json).is_err(),
                "{json}"
            );
        }
    }

    #[test]
    fn counts_unicode_graphemes_and_rejects_blank_or_control_text() {
        for initials in ["A\u{301}B", "👩‍💻", "👨‍👩‍👧"] {
            assert!(render(
                &AvatarInstance {
                    initials: initials.into(),
                    ..instance()
                },
                FRAGMENT
            )
            .is_ok());
        }
        for initials in ["", "   ", "ABCD", "A\nB"] {
            assert!(render(
                &AvatarInstance {
                    initials: initials.into(),
                    ..instance()
                },
                FRAGMENT
            )
            .is_err());
        }
        for label in ["", " \t ", "Name\nOther"] {
            assert!(render(
                &AvatarInstance {
                    label: label.into(),
                    ..instance()
                },
                FRAGMENT
            )
            .is_err());
        }
    }

    #[test]
    fn escapes_text_and_does_not_reinterpret_slot_looking_content() {
        let html = render(
            &AvatarInstance {
                initials: "A&<".into(),
                label: "Person [[avatar]] \"<one>\" & 'two'".into(),
                ..instance()
            },
            FRAGMENT,
        )
        .unwrap();
        assert!(html.contains(
            "aria-label=\"Person [[avatar]] &quot;&lt;one&gt;&quot; &amp; &#39;two&#39;\""
        ));
        assert!(html.contains("A&amp;&lt;"));
        assert_eq!(html.matches("role=\"img\"").count(), 1);
        assert_eq!(html.matches("aria-label=").count(), 1);
    }

    #[test]
    fn rejects_missing_duplicate_unknown_and_malformed_fragment_slots() {
        for fragment in [
            "",
            "prefix",
            "[[avatar]][[avatar]]",
            "[[avatar]][[extra]]",
            "[[avatar]][[",
            "[[avatar]]]]",
        ] {
            assert!(render(&instance(), fragment).is_err(), "{fragment}");
        }
    }
}
