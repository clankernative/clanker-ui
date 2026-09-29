//! Typed, static rendering for accessible native form fields.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    #[default]
    Input,
    Textarea,
}

impl FieldKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Textarea => "textarea",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    #[default]
    Text,
    Email,
    Url,
    Number,
    Password,
    Search,
    Date,
    #[serde(rename = "tel")]
    Tel,
}

impl InputType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Email => "email",
            Self::Url => "url",
            Self::Number => "number",
            Self::Password => "password",
            Self::Search => "search",
            Self::Date => "date",
            Self::Tel => "tel",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormFieldInstance {
    pub id: String,
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub kind: FieldKind,
    #[serde(default)]
    pub input_type: Option<InputType>,
    #[serde(default)]
    pub rows: Option<u8>,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub autocomplete: Option<String>,
    #[serde(default)]
    pub max_length: Option<u32>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub readonly: bool,
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn valid_description(value: &str) -> bool {
    !value.trim().is_empty()
        && !value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

fn valid_value(value: &str, kind: FieldKind) -> bool {
    !value.chars().any(|character| {
        character.is_control()
            && !(kind == FieldKind::Textarea && matches!(character, '\n' | '\r' | '\t'))
    })
}

impl FormFieldInstance {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_token(&self.id) {
            return Err("form field id must be a non-empty safe token".into());
        }
        if !valid_token(&self.name) || self.name.starts_with('_') {
            return Err("form field name must be a non-reserved safe token".into());
        }
        if !valid_text(&self.label) {
            return Err("form field label must be nonblank safe text".into());
        }
        if self
            .placeholder
            .as_deref()
            .is_some_and(|text| text.chars().any(char::is_control))
        {
            return Err("form field placeholder must be plain text".into());
        }
        for (name, value) in [
            ("hint", self.hint.as_deref()),
            ("error", self.error.as_deref()),
        ] {
            if value.is_some_and(|text| !valid_description(text)) {
                return Err(format!("form field {name} must be nonblank plain text"));
            }
        }
        if !valid_value(&self.value, self.kind) {
            return Err("form field value contains unsupported control characters".into());
        }
        if let Some(autocomplete) = self.autocomplete.as_deref() {
            if !valid_token(autocomplete) {
                return Err("form field autocomplete must be a safe token".into());
            }
        }
        if self
            .max_length
            .is_some_and(|length| !(1..=65_535).contains(&length))
        {
            return Err("maxLength must be between 1 and 65535".into());
        }
        if self.required && self.readonly {
            return Err("readonly fields cannot be required".into());
        }
        match self.kind {
            FieldKind::Input if self.rows.is_some() => {
                return Err("rows are only valid for textarea fields".into());
            }
            FieldKind::Textarea if self.input_type.is_some() => {
                return Err("inputType is only valid for input fields".into());
            }
            FieldKind::Textarea if self.rows.is_some_and(|rows| !(2..=40).contains(&rows)) => {
                return Err("textarea rows must be between 2 and 40".into());
            }
            FieldKind::Input
                if self.max_length.is_some_and(|_| {
                    matches!(
                        self.input_type.unwrap_or_default(),
                        InputType::Number | InputType::Date
                    )
                }) =>
            {
                return Err("maxLength is only valid for textual input types".into());
            }
            _ => {}
        }
        Ok(())
    }
}

/// Render one field into the component-owned fragment. Slot expansion is exact,
/// single-pass, and never treats inserted content as another template.
pub fn render(instance: &FormFieldInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    const SLOTS: [&str; 4] = [
        "[[attributes]]",
        "[[label]]",
        "[[control]]",
        "[[description]]",
    ];
    for slot in SLOTS {
        if fragment.matches(slot).count() != 1 {
            return Err(format!(
                "form-field fragment must contain exactly one {slot} slot"
            ));
        }
    }

    let invalid = instance.error.is_some();
    let attributes = format!(
        "class=\"cui-form-field{}\" data-cui-component=\"form-field\" data-cui-kind=\"{}\"",
        if invalid {
            " cui-form-field--invalid"
        } else {
            ""
        },
        instance.kind.as_str()
    );
    let required_mark = if instance.required {
        " <span class=\"cui-form-field__required\" aria-hidden=\"true\">*</span>"
    } else {
        ""
    };
    let label = format!(
        "<label class=\"cui-form-field__label\" for=\"{}\">{}{}</label>",
        escape_html(&instance.id),
        escape_html(&instance.label),
        required_mark
    );
    let descriptions = [
        instance.hint.as_deref().map(|text| ("hint", text)),
        instance.error.as_deref().map(|text| ("error", text)),
    ];
    let has_description = descriptions.iter().any(Option::is_some);
    let described_by = if has_description {
        let ids = descriptions
            .iter()
            .flatten()
            .map(|(kind, _)| format!("{}-{kind}", instance.id))
            .collect::<Vec<_>>()
            .join(" ");
        format!(" aria-describedby=\"{}\"", escape_html(&ids))
    } else {
        String::new()
    };
    let invalid_attr = if invalid {
        " aria-invalid=\"true\""
    } else {
        ""
    };
    let required_attr = if instance.required { " required" } else { "" };
    let readonly_attr = if instance.readonly { " readonly" } else { "" };
    let max_length = instance
        .max_length
        .map(|length| format!(" maxlength=\"{length}\""))
        .unwrap_or_default();
    let placeholder = instance
        .placeholder
        .as_deref()
        .map(|text| format!(" placeholder=\"{}\"", escape_html(text)))
        .unwrap_or_default();
    let autocomplete = instance
        .autocomplete
        .as_deref()
        .map(|text| format!(" autocomplete=\"{}\"", escape_html(text)))
        .unwrap_or_default();
    let control = match instance.kind {
        FieldKind::Input => format!(
            "<input class=\"cui-form-field__control\" id=\"{}\" name=\"{}\" type=\"{}\" value=\"{}\"{}{}{}{}{}{}{}>",
            escape_html(&instance.id),
            escape_html(&instance.name),
            instance.input_type.unwrap_or_default().as_str(),
            escape_html(&instance.value),
            placeholder,
            autocomplete,
            max_length,
            described_by,
            invalid_attr,
            required_attr,
            readonly_attr
        ),
        FieldKind::Textarea => format!(
            "<textarea class=\"cui-form-field__control cui-form-field__control--textarea\" id=\"{}\" name=\"{}\" rows=\"{}\"{}{}{}{}{}{}{}>{}</textarea>",
            escape_html(&instance.id),
            escape_html(&instance.name),
            instance.rows.unwrap_or(3),
            placeholder,
            autocomplete,
            max_length,
            described_by,
            invalid_attr,
            required_attr,
            readonly_attr,
            escape_html(&instance.value)
        ),
    };
    let description = if has_description {
        let mut rendered = "<div class=\"cui-form-field__description\">".to_owned();
        for (kind, value) in descriptions.into_iter().flatten() {
            rendered.push_str(&format!(
                "<p class=\"cui-form-field__{kind}\" id=\"{}-{kind}\">{}</p>",
                escape_html(&instance.id),
                escape_html(value)
            ));
        }
        rendered.push_str("</div>");
        rendered
    } else {
        String::new()
    };
    let values = [attributes, label, control, description];
    let replacements = SLOTS
        .iter()
        .zip(values.iter())
        .map(|(slot, value)| (*slot, value.as_str()))
        .collect::<Vec<_>>();
    crate::fragment::fill(fragment, &replacements)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "<div [[attributes]]>[[label]][[control]][[description]]</div>";

    fn instance() -> FormFieldInstance {
        FormFieldInstance {
            id: "email".into(),
            name: "email".into(),
            label: "Email".into(),
            kind: FieldKind::Input,
            input_type: Some(InputType::Email),
            rows: None,
            value: String::new(),
            placeholder: None,
            hint: None,
            error: None,
            autocomplete: None,
            max_length: None,
            required: false,
            readonly: false,
        }
    }

    #[test]
    fn renders_each_supported_input_type_and_native_states() {
        for (kind, expected) in [
            (InputType::Text, "text"),
            (InputType::Email, "email"),
            (InputType::Url, "url"),
            (InputType::Number, "number"),
            (InputType::Password, "password"),
            (InputType::Search, "search"),
            (InputType::Date, "date"),
            (InputType::Tel, "tel"),
        ] {
            let mut field = instance();
            field.input_type = Some(kind);
            assert!(render(&field, FRAGMENT)
                .unwrap()
                .contains(&format!("type=\"{expected}\"")));
        }
        let mut field = instance();
        field.required = true;
        field.hint = Some("Work address".into());
        field.error = Some("Unavailable".into());
        let html = render(&field, FRAGMENT).unwrap();
        assert!(html.contains("aria-describedby=\"email-hint email-error\""));
        assert!(html.contains("aria-invalid=\"true\""));
        assert!(html.contains(" required"));
        assert!(html.contains("id=\"email-hint\""));
        assert!(html.contains("id=\"email-error\""));
        field.required = false;
        field.readonly = true;
        assert!(render(&field, FRAGMENT).unwrap().contains(" readonly"));
    }

    #[test]
    fn textarea_preserves_newlines_and_emits_autocomplete_and_maxlength() {
        let mut field = instance();
        field.kind = FieldKind::Textarea;
        field.input_type = None;
        field.rows = Some(5);
        field.value = "One <two>\n\t[[label]] & {{ value }}".into();
        field.hint = Some("Line one\nline two".into());
        field.autocomplete = Some("off".into());
        field.max_length = Some(255);
        let html = render(&field, FRAGMENT).unwrap();
        assert!(html.contains("rows=\"5\""));
        assert!(html.contains("One &lt;two&gt;\n\t[[label]] &amp; {{ value }}"));
        assert!(html.contains("aria-describedby=\"email-hint\""));
        assert!(html.contains("autocomplete=\"off\""));
        assert!(html.contains("maxlength=\"255\""));
    }

    #[test]
    fn escapes_password_values_and_emits_allowed_maxlength() {
        let mut field = instance();
        field.id = "field:alpha".into();
        field.name = "secret".into();
        field.label = "Secret <value> & \"quoted\"".into();
        field.input_type = Some(InputType::Password);
        field.value = "p<&\"'".into();
        field.placeholder = Some(String::new());
        field.max_length = Some(255);
        let html = render(&field, FRAGMENT).unwrap();
        assert!(html.contains("Secret &lt;value&gt; &amp; &quot;quoted&quot;"));
        assert!(html.contains("value=\"p&lt;&amp;&quot;&#39;\""));
        assert!(html.contains("placeholder=\"\""));
        assert!(html.contains("maxlength=\"255\""));
        assert!(!html.contains("type=\"text\""));
    }

    #[test]
    fn rejects_blank_descriptions_controls_and_invalid_maxlength() {
        let mut field = instance();
        field.hint = Some(" \n\t".into());
        assert!(render(&field, FRAGMENT).is_err());
        field.hint = None;
        field.error = Some("\t".into());
        assert!(render(&field, FRAGMENT).is_err());
        field.error = None;
        field.value = "bad\0value".into();
        assert!(render(&field, FRAGMENT).is_err());
        field.value = "line\nbreak".into();
        assert!(render(&field, FRAGMENT).is_err());
        field.value = String::new();
        field.max_length = Some(0);
        assert!(render(&field, FRAGMENT).is_err());
        field.max_length = Some(65_536);
        assert!(render(&field, FRAGMENT).is_err());
        field.max_length = Some(12);
        field.input_type = Some(InputType::Number);
        assert!(render(&field, FRAGMENT).is_err());
        field.input_type = Some(InputType::Date);
        assert!(render(&field, FRAGMENT).is_err());
        for kind in [
            InputType::Text,
            InputType::Email,
            InputType::Url,
            InputType::Password,
            InputType::Search,
            InputType::Tel,
        ] {
            field.input_type = Some(kind);
            assert!(render(&field, FRAGMENT)
                .unwrap()
                .contains("maxlength=\"12\""));
        }
    }

    #[test]
    fn rejects_unknown_props_invalid_tokens_states_and_fragment_slots() {
        assert!(serde_json::from_str::<FormFieldInstance>(
            r#"{"id":"x","name":"x","label":"X","disabled":true}"#
        )
        .is_err());
        assert!(serde_json::from_str::<FormFieldInstance>(
            r#"{"id":"x","name":"x","label":"X","rawAttributes":"x"}"#
        )
        .is_err());
        let mut field = instance();
        field.id = "bad id".into();
        assert!(render(&field, FRAGMENT).is_err());
        field = instance();
        field.name = "_ticket".into();
        assert!(render(&field, FRAGMENT).is_err());
        field = instance();
        field.kind = FieldKind::Textarea;
        field.rows = Some(41);
        assert!(render(&field, FRAGMENT).is_err());
        field = instance();
        field.kind = FieldKind::Textarea;
        field.input_type = Some(InputType::Date);
        assert!(render(&field, FRAGMENT).is_err());
        assert!(render(&instance(), "[[attributes]][[label]][[control]]").is_err());
        assert!(render(
            &instance(),
            "[[attributes]][[label]][[control]][[description]][[other]]"
        )
        .is_err());
        let mut field = instance();
        field.required = true;
        field.readonly = true;
        assert!(render(&field, FRAGMENT).is_err());
    }

    #[test]
    fn golden_fixtures_cover_every_declared_variant() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct StaticFixture<T> {
            #[serde(flatten)]
            instance: T,
            expected_html: String,
        }

        let fixtures = [
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-text.json"
                ),
                Some(InputType::Text),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-email.json"
                ),
                Some(InputType::Email),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-url.json"
                ),
                Some(InputType::Url),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-number.json"
                ),
                Some(InputType::Number),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-password.json"
                ),
                Some(InputType::Password),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-search.json"
                ),
                Some(InputType::Search),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-date.json"
                ),
                Some(InputType::Date),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/input-tel.json"
                ),
                Some(InputType::Tel),
            ),
            (
                include_str!(
                    "../../../packages/vanilla/components/form-field/fixtures/textarea.json"
                ),
                None,
            ),
        ];
        let fragment =
            include_str!("../../../packages/vanilla/components/form-field/fragment.html");
        let mut covered = std::collections::BTreeSet::new();
        for (source, input_type) in fixtures {
            let fixture: StaticFixture<FormFieldInstance> = serde_json::from_str(source).unwrap();
            assert_eq!(
                render(&fixture.instance, fragment).unwrap(),
                fixture.expected_html
            );
            match input_type {
                Some(expected) => {
                    assert_eq!(fixture.instance.kind, FieldKind::Input);
                    assert_eq!(fixture.instance.input_type, Some(expected));
                    covered.insert(expected.as_str());
                }
                None => {
                    assert_eq!(fixture.instance.kind, FieldKind::Textarea);
                    covered.insert("textarea");
                }
            }
        }
        assert_eq!(covered.len(), 9);
    }
}
