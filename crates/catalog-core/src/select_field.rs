//! Typed static rendering for a native, accessible select field.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectFieldInstance {
    pub id: String,
    pub name: String,
    pub label: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub selected: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Choice {
    pub value: String,
    pub label: String,
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
fn plain(s: &str) -> bool {
    !s.chars().any(char::is_control)
}
impl SelectFieldInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !token(&self.id) || !token(&self.name) || self.name.starts_with('_') {
            return Err("select field id/name must be safe non-reserved tokens".into());
        }
        if !text(&self.label) {
            return Err("select field label must be nonblank plain text".into());
        }
        if !(1..=100).contains(&self.choices.len()) {
            return Err("select field requires 1..=100 choices".into());
        }
        let mut values = std::collections::BTreeSet::new();
        for c in &self.choices {
            if !text(&c.value)
                || c.value.contains(['<', '>'])
                || !text(&c.label)
                || !plain(&c.value)
                || !values.insert(&c.value)
            {
                return Err("choice values must be unique nonblank plain text and labels nonblank plain text".into());
            }
        }
        if let Some(v) = &self.selected {
            if !self.choices.iter().any(|c| c.value == *v && !c.disabled) {
                return Err("selected value must match an enabled choice".into());
            }
        }
        for (name, value) in [
            ("placeholder", self.placeholder.as_deref()),
            ("hint", self.hint.as_deref()),
            ("error", self.error.as_deref()),
        ] {
            if value.is_some_and(|v| !plain(v) || ((name == "hint" || name == "error") && !text(v)))
            {
                return Err(format!("{name} must be plain text"));
            }
        }
        Ok(())
    }
}
pub fn render(i: &SelectFieldInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let descriptions = i.hint.is_some() || i.error.is_some();
    let mut options = String::new();
    if let Some(p) = &i.placeholder {
        options.push_str(&format!(
            "<option value=\"\" data-cui-placeholder disabled{}>{}</option>",
            if i.selected.is_none() {
                " selected"
            } else {
                ""
            },
            escape(p)
        ));
    }
    for c in &i.choices {
        options.push_str(&format!(
            "<option value=\"{}\"{}{}>{}</option>",
            escape(&c.value),
            if i.selected.as_deref() == Some(&c.value) {
                " selected"
            } else {
                ""
            },
            if c.disabled { " disabled" } else { "" },
            escape(&c.label)
        ));
    }
    let attr = format!(
        "class=\"cui-select-field{}\" data-cui-component=\"select-field\"",
        if i.error.is_some() {
            " cui-select-field--invalid"
        } else {
            ""
        }
    );
    let mut label = format!(
        "<label class=\"cui-select-field__label\" for=\"{}\">{}</label>",
        escape(&i.id),
        escape(&i.label)
    );
    if i.required {
        label = label.replace(
            "</label>",
            " <span class=\"cui-select-field__required\" aria-hidden=\"true\">*</span></label>",
        );
    }
    let mut control = format!(
        "<div class=\"cui-select-field__control-wrap\"><select class=\"cui-select-field__control\" id=\"{}\" name=\"{}\"",
        escape(&i.id),
        escape(&i.name)
    );
    if descriptions {
        control.push_str(&format!(
            " aria-describedby=\"{}-description\"",
            escape(&i.id)
        ));
    }
    if i.error.is_some() {
        control.push_str(" aria-invalid=\"true\"");
    }
    if i.required {
        control.push_str(" required");
    }
    if i.disabled {
        control.push_str(" disabled");
    }
    control.push('>');
    control.push_str(&options);
    control.push_str("</select></div>");
    let mut desc = String::new();
    if descriptions {
        desc.push_str(&format!(
            "<div class=\"cui-select-field__description\" id=\"{}-description\">",
            escape(&i.id)
        ));
        if let Some(e) = &i.error {
            desc.push_str(&format!(
                "<p class=\"cui-select-field__error\">{}</p>",
                escape(e)
            ));
        }
        if let Some(h) = &i.hint {
            desc.push_str(&format!(
                "<p class=\"cui-select-field__hint\">{}</p>",
                escape(h)
            ));
        }
        desc.push_str("</div>");
    }
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attr),
            ("[[label]]", &label),
            ("[[control]]", &control),
            ("[[description]]", &desc),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SelectFieldInstance {
        SelectFieldInstance {
            id: "region".into(),
            name: "region".into(),
            label: "Region <&>".into(),
            choices: vec![
                Choice {
                    value: "us".into(),
                    label: "United States".into(),
                    disabled: false,
                },
                Choice {
                    value: "xx".into(),
                    label: "Unavailable".into(),
                    disabled: true,
                },
            ],
            selected: Some("us".into()),
            placeholder: Some("Choose".into()),
            hint: Some("Pick one".into()),
            error: None,
            required: true,
            disabled: false,
        }
    }
    const F: &str = "<div [[attributes]]>[[label]][[control]][[description]]</div>";
    #[test]
    fn renders_escaped_native_select_and_descriptions() {
        let s = render(&fixture(), F).unwrap();
        assert!(s.contains("Region &lt;&amp;&gt;"));
        assert!(s.contains("value=\"us\" selected"));
        assert!(s.contains("value=\"xx\" disabled"));
        assert!(s.contains("aria-describedby=\"region-description\""));
    }
    #[test]
    fn rejects_unknown_disabled_selection_and_markup_value() {
        let mut x = fixture();
        x.selected = Some("xx".into());
        assert!(x.validate().is_err());
        x.selected = Some("missing".into());
        assert!(x.validate().is_err());
        x.selected = None;
        x.choices[0].value = "<b>".into();
        assert!(x.validate().is_err());
    }
    #[test]
    fn exact_slots_and_parse_unknown_fields() {
        assert!(render(&fixture(), "[[attributes]][[label]][[control]]").is_err());
        assert!(SelectFieldInstance::parse(
            r#"{"id":"x","name":"x","label":"X","choices":[],"oops":1}"#
        )
        .is_err());
    }
}
