//! Typed static rendering for the page-heading component.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageHeaderInstance {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl PageHeaderInstance {
    pub fn validate(&self) -> Result<(), String> {
        validate_text(&self.title, "page header title")?;
        if let Some(description) = &self.description {
            validate_text(description, "page header description")?;
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

/// Renders a static page heading. The adapter fragment must contain exactly one
/// `[[page-header]]` slot; inserted text and markup-like slot strings are never re-parsed.
pub fn render(instance: &PageHeaderInstance, fragment: &str) -> Result<String, String> {
    render_with_actions(instance, None, fragment)
}

/// Compose actions admitted by the host, never a deserialized HTML string.
/// The app still owns all forms, commands, and destinations in this slot.
pub fn render_with_actions(
    instance: &PageHeaderInstance,
    actions: Option<&crate::layout::AdmittedChildren>,
    fragment: &str,
) -> Result<String, String> {
    instance.validate()?;
    let description = instance
        .description
        .as_deref()
        .map(|text| {
            format!(
                "<p class=\"cui-page-header__description\">{}</p>",
                escape_html(text)
            )
        })
        .unwrap_or_default();
    let actions = actions
        .map(|content| {
            if content.as_markup().trim().is_empty() {
                return Err("page header actions must not be blank".to_owned());
            }
            Ok(format!(
                "<div class=\"cui-page-header__actions\">{}</div>",
                content.as_markup()
            ))
        })
        .transpose()?
        .unwrap_or_default();
    let header = format!(
        "<header class=\"cui-page-header\" data-cui-component=\"page-header\"><div class=\"cui-page-header__content\"><h1 class=\"cui-page-header__title\">{}</h1>{description}</div>{actions}</header>",
        escape_html(&instance.title)
    );
    crate::fragment::fill(fragment, &[("[[page-header]]", &header)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "[[page-header]]";

    fn instance() -> PageHeaderInstance {
        PageHeaderInstance {
            title: "Deployments".into(),
            description: None,
        }
    }

    #[test]
    fn renders_title_only_and_described_page_headings() {
        assert_eq!(
            render(&instance(), FRAGMENT).unwrap(),
            "<header class=\"cui-page-header\" data-cui-component=\"page-header\"><div class=\"cui-page-header__content\"><h1 class=\"cui-page-header__title\">Deployments</h1></div></header>"
        );
        assert_eq!(
            render(
                &PageHeaderInstance {
                    description: Some("Review production changes.".into()),
                    ..instance()
                },
                FRAGMENT
            )
            .unwrap(),
            "<header class=\"cui-page-header\" data-cui-component=\"page-header\"><div class=\"cui-page-header__content\"><h1 class=\"cui-page-header__title\">Deployments</h1><p class=\"cui-page-header__description\">Review production changes.</p></div></header>"
        );
    }

    #[test]
    fn escapes_plain_text_without_reinterpreting_slot_looking_strings() {
        let html = render(
            &PageHeaderInstance {
                title: "Records <live> & [[page-header]] \"now\"".into(),
                description: Some("Use <strong>text</strong> & details".into()),
            },
            FRAGMENT,
        )
        .unwrap();
        assert!(html.contains("Records &lt;live&gt; &amp; [[page-header]] &quot;now&quot;"));
        assert!(html.contains("Use &lt;strong&gt;text&lt;/strong&gt; &amp; details"));
        assert_eq!(html.matches("<header ").count(), 1);
    }

    #[test]
    fn rejects_blank_control_text_and_invalid_or_extra_fragment_slots() {
        for title in ["", "  ", "line\nbreak", "tab\there"] {
            assert!(render(
                &PageHeaderInstance {
                    title: title.into(),
                    description: None,
                },
                FRAGMENT
            )
            .is_err());
        }
        for description in ["", " \t", "line\rbreak"] {
            assert!(render(
                &PageHeaderInstance {
                    description: Some(description.into()),
                    ..instance()
                },
                FRAGMENT
            )
            .is_err());
        }
        for fragment in [
            "",
            "<header>fixed</header>",
            "[[page-header]][[page-header]]",
            "[[page-header]][[other]]",
        ] {
            assert!(render(&instance(), fragment).is_err(), "{fragment}");
        }
    }

    #[test]
    fn composes_admitted_actions_but_rejects_blank_slots() {
        let actions = crate::layout::AdmittedChildren::from_host_admitted(
            "<a href=\"https://example.test\">Create</a>",
        );
        let html = render_with_actions(&instance(), Some(&actions), FRAGMENT).unwrap();
        assert!(html.contains("<div class=\"cui-page-header__actions\"><a href=\"https://example.test\">Create</a></div>"));
        let empty = crate::layout::AdmittedChildren::from_host_admitted(" ");
        assert!(render_with_actions(&instance(), Some(&empty), FRAGMENT).is_err());
    }

    #[test]
    fn serde_uses_exact_camel_case_api_and_rejects_actions_or_unknown_fields() {
        let parsed: PageHeaderInstance =
            serde_json::from_str(r#"{"title":"Overview","description":"Current state"}"#).unwrap();
        assert_eq!(parsed.description.as_deref(), Some("Current state"));
        assert!(serde_json::from_str::<PageHeaderInstance>(
            r#"{"title":"Overview","actions":"<button>"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<PageHeaderInstance>(
            r#"{"title":"Overview","rawHtml":"<b>"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<PageHeaderInstance>(
            r#"{"title":"Overview","Description":"wrong case"}"#
        )
        .is_err());
    }
}
