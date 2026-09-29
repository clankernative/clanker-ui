//! Typed selection and safe static rendering for empty-state content.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    #[default]
    Start,
    Center,
}

impl Alignment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HeadingLevel {
    #[default]
    H2,
    H3,
    H4,
}

impl HeadingLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyStateInstance {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default)]
    pub heading_level: HeadingLevel,
    #[serde(default)]
    pub action_href: Option<String>,
    #[serde(default)]
    pub action_label: Option<String>,
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

impl EmptyStateInstance {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_text(&self.title) || !valid_text(&self.body) {
            return Err("empty-state title and body must contain safe, nonblank text".into());
        }
        if self.action_href.is_some() != self.action_label.is_some() {
            return Err("empty-state actionHref and actionLabel must be supplied together".into());
        }
        if let Some(label) = &self.action_label {
            if !valid_text(label) {
                return Err("empty-state actionLabel must contain safe, nonblank text".into());
            }
        }
        if let Some(href) = &self.action_href {
            if !crate::button::safe_href(href) {
                return Err("empty-state actionHref must be a safe destination".into());
            }
        }
        Ok(())
    }
}

/// Renders a semantic, static empty state through the fragment's exact single slot.
pub fn render(instance: &EmptyStateInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    let action = match (&instance.action_href, &instance.action_label) {
        (Some(href), Some(label)) => format!(
            "<a class=\"cui-empty-state__action\" href=\"{}\">{}</a>",
            escape_html(href),
            escape_html(label)
        ),
        _ => String::new(),
    };
    let action_class = if instance.action_href.is_some() {
        " cui-empty-state--with-action"
    } else {
        ""
    };
    let markup = format!(
        "<section class=\"cui-empty-state cui-empty-state--{}{}\" data-cui-component=\"empty-state\"><div class=\"cui-empty-state__layout\"><div class=\"cui-empty-state__content\"><{} class=\"cui-empty-state__title\">{}</{}><p class=\"cui-empty-state__body\">{}</p></div>{}</div></section>",
        instance.alignment.as_str(),
        action_class,
        instance.heading_level.as_str(),
        escape_html(&instance.title),
        instance.heading_level.as_str(),
        escape_html(&instance.body),
        action
    );
    crate::fragment::fill(fragment, &[("[[empty-state]]", &markup)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "[[empty-state]]";

    fn instance() -> EmptyStateInstance {
        EmptyStateInstance {
            title: "No incidents found".into(),
            body: "This environment has no open incidents.".into(),
            alignment: Alignment::Start,
            heading_level: HeadingLevel::H2,
            action_href: None,
            action_label: None,
        }
    }

    #[test]
    fn renders_each_alignment_and_heading_with_static_semantic_markup() {
        for alignment in [Alignment::Start, Alignment::Center] {
            for heading in [HeadingLevel::H2, HeadingLevel::H3, HeadingLevel::H4] {
                let mut value = instance();
                value.alignment = alignment;
                value.heading_level = heading;
                let html = render(&value, FRAGMENT).unwrap();
                assert!(html.contains(&format!("cui-empty-state--{}", alignment.as_str())));
                assert!(html.contains(&format!(
                    "<{} class=\"cui-empty-state__title\">",
                    heading.as_str()
                )));
                assert!(!html.contains("aria-live"));
                assert!(!html.contains("<button"));
            }
        }
    }

    #[test]
    fn escapes_text_and_attributes_and_renders_native_navigation_links() {
        let mut value = instance();
        value.title = "Check <input> & [[empty-state]]".into();
        value.body = "Value \"bad\" & <retry>".into();
        value.action_href = Some("?next=/review&safe=1".into());
        value.action_label = Some("Review & retry".into());
        let html = render(&value, FRAGMENT).unwrap();
        assert!(html.contains("Check &lt;input&gt; &amp; [[empty-state]]"));
        assert!(html.contains("Value &quot;bad&quot; &amp; &lt;retry&gt;"));
        assert!(html.contains("href=\"?next=/review&amp;safe=1\""));
        assert!(html.contains(">Review &amp; retry</a>"));
        assert_eq!(render(&value, FRAGMENT).unwrap(), html);
    }

    #[test]
    fn rejects_invalid_content_urls_pairs_and_fragment_slots() {
        let mut value = instance();
        for text in ["", "  ", "line\nbreak"] {
            value.title = text.into();
            assert!(value.validate().is_err(), "title: {text:?}");
        }
        value = instance();
        value.action_label = Some("Go".into());
        assert!(value.validate().is_err());
        value.action_label = None;
        value.action_href = Some("/go".into());
        assert!(value.validate().is_err());
        value.action_label = Some("Go".into());
        for href in [
            "javascript:alert(1)",
            "data:text/html,unsafe",
            "//example.test",
            "http:evil.test",
            "https://",
            "https://user:secret@example.test/",
            "https://user@example.test/",
            "/path with space",
        ] {
            value.action_href = Some(href.into());
            assert!(value.validate().is_err(), "href: {href}");
        }
        value.action_href = Some("/safe/path".into());
        for fragment in [
            "no slot",
            "[[empty-state]][[empty-state]]",
            "[[other]]",
            "[[empty-state",
        ] {
            assert!(render(&value, fragment).is_err(), "fragment: {fragment}");
        }
    }

    #[test]
    fn strict_camel_case_deserialization_and_enum_serialization() {
        let parsed: EmptyStateInstance = serde_json::from_str(
            r#"{"title":"Title","body":"Body","alignment":"center","headingLevel":"h4"}"#,
        )
        .unwrap();
        assert_eq!(parsed.alignment, Alignment::Center);
        assert_eq!(parsed.heading_level, HeadingLevel::H4);
        let defaults: EmptyStateInstance =
            serde_json::from_str(r#"{"title":"Title","body":"Body"}"#).unwrap();
        assert_eq!(defaults.alignment, Alignment::Start);
        assert_eq!(defaults.heading_level, HeadingLevel::H2);
        assert!(serde_json::from_str::<EmptyStateInstance>(
            r#"{"title":"T","body":"B","action_href":"/x"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<EmptyStateInstance>(
            r#"{"title":"T","body":"B","extra":true}"#
        )
        .is_err());
        assert!(serde_json::from_str::<EmptyStateInstance>(
            r#"{"title":"T","body":"B","headingLevel":"h1"}"#
        )
        .is_err());
    }
}
