//! Typed static rendering for a native Boolean checkbox styled as a switch.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToggleInstance {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}
fn id(i: &ToggleInstance) -> &str {
    i.id.as_deref().unwrap_or(&i.name)
}
fn reserved_id(s: &str) -> bool {
    ["-hint", "-error", "-description"]
        .iter()
        .any(|suffix| s.ends_with(suffix))
}
impl ToggleInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !token(&self.name) || self.name.starts_with('_') {
            return Err("toggle name must be a non-reserved safe token".into());
        }
        if !token(id(self)) || reserved_id(id(self)) {
            return Err("toggle id must be a safe, non-reserved token".into());
        }
        if !text(&self.label) {
            return Err("toggle label must be nonblank plain text".into());
        }
        for (field, value) in [
            ("hint", self.hint.as_deref()),
            ("error", self.error.as_deref()),
        ] {
            if value.is_some_and(|v| !text(v)) {
                return Err(format!("{field} must be nonblank plain text"));
            }
        }
        // Both derived IDs are reserved even when the corresponding copy is absent.
        let mut ids = std::collections::BTreeSet::new();
        ids.insert(id(self).to_owned());
        ids.insert(format!("{}-hint", id(self)));
        ids.insert(format!("{}-error", id(self)));
        ids.insert(format!("{}-description", id(self)));
        if ids.len() != 4 {
            return Err("toggle contains ambiguous derived ids".into());
        }
        Ok(())
    }
}
pub fn render(i: &ToggleInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let id = id(i);
    let descriptions = i.hint.is_some() || i.error.is_some();
    let mut described = Vec::new();
    if i.hint.is_some() {
        described.push(format!("{id}-hint"));
    }
    if i.error.is_some() {
        described.push(format!("{id}-error"));
    }
    let attributes = format!(
        "class=\"cui-toggle{}\" data-cui-component=\"toggle\"",
        if i.error.is_some() {
            " cui-toggle--invalid"
        } else {
            ""
        }
    );
    let control = format!(
        "<input class=\"cui-toggle__control\" type=\"checkbox\" role=\"switch\" id=\"{}\" name=\"{}\" value=\"true\"{}{}{}{}{}>",
        escape(id),
        escape(&i.name),
        if described.is_empty() {
            String::new()
        } else {
            format!(" aria-describedby=\"{}\"", escape(&described.join(" ")))
        },
        if i.error.is_some() {
            " aria-invalid=\"true\""
        } else {
            ""
        },
        if i.checked { " checked" } else { "" },
        if i.required { " required" } else { "" },
        if i.disabled { " disabled" } else { "" }
    );
    let label = format!(
        "<label class=\"cui-toggle__label\" for=\"{}\">{}{}</label>",
        escape(id),
        control,
        escape(&i.label)
    );
    let description = if descriptions {
        format!(
            "<div class=\"cui-toggle__description\" id=\"{}-description\">{}{}</div>",
            escape(id),
            i.hint
                .as_ref()
                .map(|v| format!(
                    "<p class=\"cui-toggle__hint\" id=\"{}-hint\">{}</p>",
                    escape(id),
                    escape(v)
                ))
                .unwrap_or_default(),
            i.error
                .as_ref()
                .map(|v| format!(
                    "<p class=\"cui-toggle__error\" id=\"{}-error\">{}</p>",
                    escape(id),
                    escape(v)
                ))
                .unwrap_or_default()
        )
    } else {
        String::new()
    };
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label),
            ("[[description]]", &description),
        ],
    )
}
