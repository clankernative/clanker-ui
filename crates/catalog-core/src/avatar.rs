//! Typed initials/image identity avatar contract and safe static rendering.

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
    #[serde(default)]
    pub image_source: Option<String>,
}

impl AvatarInstance {
    pub fn validate(&self) -> Result<(), String> {
        validate_text(&self.initials, "avatar initials")?;
        if self.initials.graphemes(true).count() > 3 {
            return Err("avatar initials must contain at most three grapheme clusters".into());
        }
        validate_text(&self.label, "avatar label")?;
        if let Some(source) = &self.image_source {
            if !safe_image_source(source) {
                return Err(
                    "avatar imageSource must be a safe HTTPS URL or app/instance asset path".into(),
                );
            }
        }
        Ok(())
    }
}

/// Admits absolute HTTPS URLs without credentials, or host-managed app/instance assets.
pub fn safe_image_source(value: &str) -> bool {
    if value.is_empty()
        || value != value.trim()
        || value.chars().any(char::is_control)
        || value.contains('\\')
    {
        return false;
    }

    if let Some(url) = value
        .strip_prefix("/assets/app/")
        .or_else(|| value.strip_prefix("/assets/instance/"))
    {
        return !url.is_empty()
            && !url.starts_with('/')
            && !url.split('/').any(|segment| {
                segment.is_empty()
                    || segment == "."
                    || segment == ".."
                    || segment.to_ascii_lowercase().contains("%2e")
                    || segment.to_ascii_lowercase().contains("%2f")
                    || segment.to_ascii_lowercase().contains("%5c")
            })
            && !value.contains(char::is_whitespace)
            && !value.contains('#');
    }

    if !value
        .get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
        || value.contains(char::is_whitespace)
    {
        return false;
    }
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
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

/// Renders a named, noninteractive avatar into the single `[[avatar]]` slot.
pub fn render(instance: &AvatarInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    if fragment.matches("[[avatar]]").count() != 1 {
        return Err("avatar fragment must contain exactly one [[avatar]] slot".into());
    }
    if fragment.matches("[[").count() != 1 || fragment.matches("]]").count() != 1 {
        return Err("avatar fragment contains an unsupported or malformed slot".into());
    }

    let wrapper = format!(
        "<span class=\"cui-avatar cui-avatar--{} cui-avatar--{}\" role=\"img\" aria-label=\"{}\" data-cui-component=\"avatar\">",
        instance.size.as_str(),
        instance.tone.as_str(),
        escape_html(&instance.label)
    );
    // Keep the no-image branch byte-for-byte compatible with the original static output.
    let markup = match &instance.image_source {
        None => format!(
            "{wrapper}<span class=\"cui-avatar__initials\" aria-hidden=\"true\">{}</span></span>",
            escape_html(&instance.initials)
        ),
        Some(source) => format!(
            "{wrapper}<span class=\"cui-avatar__initials\" aria-hidden=\"true\">{}</span><img class=\"cui-avatar__image\" src=\"{}\" alt=\"\" aria-hidden=\"true\" referrerpolicy=\"no-referrer\" loading=\"lazy\" decoding=\"async\"></span>",
            escape_html(&instance.initials),
            escape_html(source)
        ),
    };
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
            image_source: None,
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
    fn image_source_is_optional_strict_and_rendered_with_safe_fallback_attributes() {
        let parsed: AvatarInstance =
            serde_json::from_str(r#"{"initials":"ET","label":"Emilia Torres"}"#).unwrap();
        assert_eq!(parsed.image_source, None);
        assert_eq!(parsed.size, Size::Medium);
        assert_eq!(parsed.tone, Tone::Neutral);
        let mut value = instance();
        value.image_source = Some("https://images.example.test/a&b.png".into());
        let html = render(&value, FRAGMENT).unwrap();
        assert!(html.contains("src=\"https://images.example.test/a&amp;b.png\" alt=\"\" aria-hidden=\"true\" referrerpolicy=\"no-referrer\" loading=\"lazy\" decoding=\"async\""));
        assert!(html
            .contains("<span class=\"cui-avatar__initials\" aria-hidden=\"true\">ET</span><img"));
        for source in [
            "javascript:alert(1)",
            "http://example.test/a",
            "//example.test/a",
            "https://u:p@example.test/a",
            "https://bad host/a",
            "https://example.test\\@evil",
            "/assets/other/a.png",
            "/assets/app/../secret",
            "/assets/instance/%2e%2e/private",
        ] {
            value.image_source = Some(source.into());
            assert!(value.validate().is_err(), "{source}");
        }
        for source in [
            "https://example.test/a",
            "/assets/app/user/a.png",
            "/assets/instance/123/a.png",
        ] {
            assert!(safe_image_source(source), "{source}");
        }
        let invalid: AvatarInstance = serde_json::from_str(
            r#"{"initials":"ET","label":"Name","imageSource":"javascript:alert(1)"}"#,
        )
        .unwrap();
        assert!(invalid.validate().is_err());
        for json in [
            r#"{"initials":"ET","label":"Name","src":"/person.png"}"#,
            r#"{"initials":"ET","label":"Name","image":"/person.png"}"#,
            r#"{"initials":"ET","label":"Name","srcURL":"https://example.test/a"}"#,
            r#"{"initials":"ET","label":"Name","image_source":"/assets/app/a"}"#,
            r#"{"initials":"ET","label":"Name","size":"giant"}"#,
        ] {
            assert!(serde_json::from_str::<AvatarInstance>(json).is_err());
        }
    }

    #[test]
    fn package_fixtures_match_the_declared_fragment() {
        let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/vanilla/components/avatar");
        let fragment = std::fs::read_to_string(package.join("fragment.html")).unwrap();
        for file in [
            "extra-small-neutral.json",
            "small-brand.json",
            "medium-success.json",
            "large-neutral.json",
            "extra-large-brand.json",
            "medium-neutral.json",
            "small-success.json",
            "large-brand.json",
            "image-https.json",
            "image-host-asset.json",
        ] {
            let mut fixture: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(package.join("fixtures").join(file)).unwrap(),
            )
            .unwrap();
            let expected = fixture["expectedHtml"].as_str().unwrap().to_owned();
            fixture.as_object_mut().unwrap().remove("expectedHtml");
            let value: AvatarInstance = serde_json::from_value(fixture).unwrap();
            assert_eq!(render(&value, &fragment).unwrap(), expected, "{file}");
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
    fn escapes_text_and_rejects_invalid_fragment_slots() {
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
