//! Target-independent button selection and validation.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ButtonInstance {
    pub id: String,
    pub component: String,
    pub label: TextValue,
    pub destination: Destination,
    #[serde(default)]
    pub variant: Variant,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub leading_icon: Option<String>,
    #[serde(default)]
    pub edge_aligned: bool,
    #[serde(default)]
    pub disabled: BoolValue,
    #[serde(default)]
    pub busy: BoolValue,
    #[serde(default)]
    pub busy_label: Option<TextValue>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum TextValue {
    Literal { value: String },
    Field { path: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum BoolValue {
    Literal(bool),
    Field { path: String },
}

impl Default for BoolValue {
    fn default() -> Self {
        Self::Literal(false)
    }
}

impl BoolValue {
    pub fn literal(&self) -> Option<bool> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Field { .. } => None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Literal(_) => Ok(()),
            Self::Field { path } if valid_path(path) => Ok(()),
            _ => Err("state field must be a checked page path".into()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Destination {
    Action,
    Submit,
    Link { href: HrefValue },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum HrefValue {
    Literal { value: String },
    Route { name: String },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Variant {
    Primary,
    #[default]
    Secondary,
    Danger,
    Quiet,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Size {
    Compact,
    #[default]
    Standard,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn valid_path(path: &str) -> bool {
    !path.is_empty() && path.split('.').all(identifier) && !path.contains('-')
}

impl TextValue {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Literal { value } if !value.trim().is_empty() => Ok(()),
            Self::Field { path } if valid_path(path) => Ok(()),
            _ => Err("label must contain text or a checked field path".into()),
        }
    }
}

impl ButtonInstance {
    pub fn validate(&self, package_name: &str) -> Result<(), String> {
        if !identifier(&self.id) || self.id.contains('_') {
            return Err("button instance id must be lowercase kebab-case".into());
        }
        if self.component != format!("{package_name}/button") {
            return Err(format!("{} is not this package's button", self.component));
        }
        self.label.validate()?;
        self.disabled.validate()?;
        self.busy.validate()?;
        if self.busy.literal() != Some(false) {
            self.busy_label
                .as_ref()
                .ok_or("busy button requires a replacement label")?
                .validate()?;
        } else if self.busy_label.is_some() {
            return Err("busyLabel is only valid when busy can be true".into());
        }
        if let Some(icon) = &self.leading_icon {
            if !identifier(icon) || icon.contains('_') {
                return Err("leadingIcon must be a catalog icon name".into());
            }
        }
        if let Destination::Link { href } = &self.destination {
            match href {
                HrefValue::Route { name } if identifier(name) && !name.contains('-') => {}
                HrefValue::Literal { value } if safe_href(value) => {}
                _ => {
                    return Err(
                        "link requires a safe HTTP(S), relative, or named route target".into(),
                    )
                }
            }
        }
        Ok(())
    }
}

pub fn safe_href(value: &str) -> bool {
    if value.is_empty()
        || value != value.trim()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return false;
    }
    if value.starts_with("//") {
        return false;
    }
    if let Some((scheme, _)) = value.split_once(':') {
        if !scheme.contains('/') && !scheme.contains('?') && !scheme.contains('#') {
            return matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
                && value[scheme.len()..].starts_with("://")
                && !value.contains(char::is_whitespace)
                && url::Url::parse(value).is_ok_and(|url| {
                    url.host_str().is_some()
                        && url.username().is_empty()
                        && url.password().is_none()
                });
        }
    }
    !value.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> ButtonInstance {
        ButtonInstance {
            id: "save-link".into(),
            component: "@clanker/vanilla/button".into(),
            label: TextValue::Literal {
                value: "Save".into(),
            },
            destination: Destination::Submit,
            variant: Variant::Primary,
            size: Size::Standard,
            leading_icon: None,
            edge_aligned: false,
            disabled: BoolValue::Literal(false),
            busy: BoolValue::Literal(false),
            busy_label: None,
        }
    }

    #[test]
    fn rejects_unsafe_links_and_empty_labels() {
        assert!(instance().validate("@clanker/vanilla").is_ok());
        for href in [
            "javascript:alert(1)",
            "data:text/html,x",
            "//example.com",
            "",
            "https://",
            "http:evil.test",
            " https://example.test/",
            "/path\nwith-break",
            "https://example.test/with space",
            "https://user:secret@example.test/",
            "https://user@example.test/",
        ] {
            let mut button = instance();
            button.destination = Destination::Link {
                href: HrefValue::Literal { value: href.into() },
            };
            assert!(button.validate("@clanker/vanilla").is_err(), "{href}");
        }
        let mut button = instance();
        button.label = TextValue::Literal {
            value: "   ".into(),
        };
        assert!(button.validate("@clanker/vanilla").is_err());
    }

    #[test]
    fn busy_requires_a_replacement_label() {
        let mut button = instance();
        button.busy = BoolValue::Literal(true);
        assert!(button.validate("@clanker/vanilla").is_err());
        button.busy_label = Some(TextValue::Field {
            path: "links.saving_label".into(),
        });
        assert!(button.validate("@clanker/vanilla").is_ok());
        button.busy = BoolValue::Field {
            path: "is_busy".into(),
        };
        assert!(button.validate("@clanker/vanilla").is_ok());
        button.disabled = BoolValue::Field {
            path: "not valid".into(),
        };
        assert!(button.validate("@clanker/vanilla").is_err());
        assert!(serde_json::from_str::<BoolValue>(r#"{"path":"valid","extra":true}"#).is_err());
    }
}
