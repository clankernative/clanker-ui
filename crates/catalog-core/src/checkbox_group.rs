//! Typed static rendering for native checkbox groups.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    #[default]
    Stacked,
    Inline,
}
impl Layout {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stacked => "stacked",
            Self::Inline => "inline",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choice {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckboxGroupInstance {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    pub legend: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub layout: Layout,
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
fn base_id(i: &CheckboxGroupInstance) -> &str {
    i.id.as_deref().unwrap_or(&i.name)
}
fn reserved_base(s: &str) -> bool {
    ["-hint", "-error", "-description"]
        .iter()
        .any(|suffix| s.ends_with(suffix))
        || s.rsplit_once("-choice-")
            .is_some_and(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}
impl CheckboxGroupInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !token(&self.name) || self.name.starts_with('_') {
            return Err("checkbox group name must be a non-reserved safe token".into());
        }
        if !token(base_id(self)) || reserved_base(base_id(self)) {
            return Err("checkbox group id must be a safe, non-reserved token".into());
        }
        if !text(&self.legend) {
            return Err("checkbox group legend must be nonblank plain text".into());
        }
        if !(1..=20).contains(&self.choices.len()) {
            return Err("checkbox group requires 1..=20 choices".into());
        }
        let mut values = std::collections::BTreeSet::new();
        for c in &self.choices {
            if !text(&c.value) || !text(&c.label) || !values.insert(&c.value) {
                return Err("checkbox choices require unique nonblank values and labels".into());
            }
            if c.hint.as_deref().is_some_and(|h| !text(h)) {
                return Err("choice hint must be nonblank plain text".into());
            }
        }
        for (field, value) in [
            ("description", self.description.as_deref()),
            ("error", self.error.as_deref()),
        ] {
            if value.is_some_and(|v| !text(v)) {
                return Err(format!("{field} must be nonblank plain text"));
            }
        }
        // IDs are namespaced by the explicit/default base and a stable role/index suffix.
        let mut ids = std::collections::BTreeSet::new();
        ids.insert(base_id(self).to_owned());
        ids.insert(format!("{}-description", base_id(self)));
        ids.insert(format!("{}-error", base_id(self)));
        for n in 1..=self.choices.len() {
            ids.insert(format!("{}-choice-{n}", base_id(self)));
            ids.insert(format!("{}-choice-{n}-hint", base_id(self)));
        }
        if ids.len() != 3 + self.choices.len() * 2 {
            return Err("checkbox group contains ambiguous derived ids".into());
        }
        Ok(())
    }
}
pub fn render(i: &CheckboxGroupInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let base = base_id(i);
    let descriptions = i.description.is_some() || i.error.is_some();
    let mut choices = String::new();
    for (index, c) in i.choices.iter().enumerate() {
        let id = format!("{base}-choice-{}", index + 1);
        let hint = c
            .hint
            .as_ref()
            .map(|v| {
                format!(
                    "<span class=\"cui-checkbox-group__choice-hint\" id=\"{id}-hint\">{}</span>",
                    escape(v)
                )
            })
            .unwrap_or_default();
        choices.push_str(&format!("<label class=\"cui-checkbox-group__choice\" for=\"{}\"><input class=\"cui-checkbox-group__control\" type=\"checkbox\" id=\"{}\" name=\"{}\" value=\"{}\"{}{}{}><span class=\"cui-checkbox-group__text\"><span class=\"cui-checkbox-group__label\">{}</span>{}</span></label>", escape(&id), escape(&id), escape(&i.name), escape(&c.value), if c.checked { " checked" } else { "" }, if i.disabled || c.disabled { " disabled" } else { "" }, c.hint.as_ref().map(|_| format!(" aria-describedby=\"{id}-hint\"")).unwrap_or_default(), escape(&c.label), hint));
    }
    let mut described = Vec::new();
    if descriptions {
        described.push(format!("{base}-description"));
    }
    if i.error.is_some() {
        described.push(format!("{base}-error"));
    }
    let description = if descriptions {
        format!(
            "<div class=\"cui-checkbox-group__description\" id=\"{}-description\">{}{}</div>",
            escape(base),
            i.error
                .as_ref()
                .map(|e| format!(
                    "<p class=\"cui-checkbox-group__error\" id=\"{}-error\">{}</p>",
                    escape(base),
                    escape(e)
                ))
                .unwrap_or_default(),
            i.description
                .as_ref()
                .map(|d| format!("<p class=\"cui-checkbox-group__hint\">{}</p>", escape(d)))
                .unwrap_or_default()
        )
    } else {
        String::new()
    };
    let attributes = format!(
        "id=\"{}\" class=\"cui-checkbox-group cui-checkbox-group--{}{}\" data-cui-component=\"checkbox-group\"{}{}{}",
        escape(base),
        i.layout.as_str(),
        if i.error.is_some() {
            " cui-checkbox-group--invalid"
        } else {
            ""
        },
        if i.disabled { " disabled" } else { "" },
        if i.error.is_some() {
            " aria-invalid=\"true\""
        } else {
            ""
        },
        if described.is_empty() {
            String::new()
        } else {
            format!(" aria-describedby=\"{}\"", escape(&described.join(" ")))
        }
    );
    let legend = format!(
        "<legend class=\"cui-checkbox-group__legend\">{}</legend>",
        escape(&i.legend)
    );
    let controls = format!("<div class=\"cui-checkbox-group__choices\">{choices}</div>");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[legend]]", &legend),
            ("[[choices]]", &controls),
            ("[[description]]", &description),
        ],
    )
}
