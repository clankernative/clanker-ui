//! Typed, app-formatted number text and deterministic static markup.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    #[default]
    None,
    Roll,
}

impl Motion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Roll => "roll",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct NumberId(String);

impl NumberId {
    pub fn parse(value: String) -> Result<Self, String> {
        if value.is_empty()
            || value.len() > 128
            || value.starts_with('_')
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        {
            return Err("number id must be a safe non-reserved token".into());
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn validate(&self) -> Result<(), String> {
        Self::parse(self.0.clone()).map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Up,
    Down,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NumberInstance {
    pub value: String,
    #[serde(default)]
    pub id: Option<NumberId>,
    #[serde(default)]
    pub motion: Motion,
    #[serde(default)]
    pub direction: Direction,
    #[serde(default = "default_duration")]
    pub duration_ms: u16,
}

fn default_duration() -> u16 {
    320
}

impl NumberInstance {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(id) = &self.id {
            id.validate()?;
        }
        if self.value.trim().is_empty() || self.value.chars().any(char::is_control) {
            return Err("number value must contain safe, nonblank text".into());
        }
        if self.duration_ms > 1000 {
            return Err("number durationMs must be between 0 and 1000".into());
        }
        Ok(())
    }
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// `value` is already checked/rendered text (including any host binding helper).
/// `id` must already be a validated [`NumberId`]; it is still escaped here.
pub(crate) fn markup(
    value: &str,
    motion: Motion,
    duration_ms: u16,
    update_attribute: Option<&str>,
    id: Option<&NumberId>,
    direction: Option<Direction>,
) -> String {
    let id = id
        .map(|id| format!(" id=\"{}\"", escape_html(id.as_str())))
        .unwrap_or_default();
    let direction = direction
        .map(|direction| format!(" data-cui-number-direction=\"{}\"", direction.as_str()))
        .unwrap_or_default();
    let attr = update_attribute
        .map(|v| format!(" data-cui-number-value=\"{v}\""))
        .unwrap_or_default();
    format!("<span class=\"cui-number\" data-cui-component=\"number\"{id}{direction} data-cui-number-motion=\"{}\" data-cui-number-duration=\"{}\"{}><span class=\"cui-number__canonical\">{value}</span></span>", motion.as_str(), duration_ms, attr)
}

pub fn render(instance: &NumberInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    let value = escape_html(&instance.value);
    let element = markup(
        &value,
        instance.motion,
        instance.duration_ms,
        Some(&value),
        instance.id.as_ref(),
        Some(instance.direction),
    );
    crate::fragment::fill(fragment, &[("[[number]]", &element)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_renders_escaped_canonical_text() {
        let value: NumberInstance = serde_json::from_str(r#"{"value":"$1,200"}"#).unwrap();
        assert_eq!(value.motion, Motion::None);
        assert_eq!(value.duration_ms, 320);
        let html = render(
            &NumberInstance {
                value: "<b>&".into(),
                ..value
            },
            "[[number]]",
        )
        .unwrap();
        assert!(html.contains("&lt;b&gt;&amp;"));
        assert!(html.contains("data-cui-number-value=\"&lt;b&gt;&amp;\""));
    }

    #[test]
    fn package_fixtures_match_canonical_server_markup() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/vanilla/components/number/fixtures");
        for file in ["static.json", "roll.json"] {
            let fixture: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(base.join(file)).unwrap()).unwrap();
            let expected = fixture["expectedHtml"].as_str().unwrap();
            let mut instance: serde_json::Value = fixture.clone();
            instance.as_object_mut().unwrap().remove("expectedHtml");
            assert_eq!(
                render(
                    &serde_json::from_value(instance).unwrap(),
                    include_str!("../../../packages/vanilla/components/number/fragment.html"),
                )
                .unwrap(),
                expected,
                "{file}"
            );
        }
    }

    #[test]
    fn rejects_blank_controls_unknown_properties_and_out_of_range_duration() {
        assert!(serde_json::from_str::<NumberInstance>(r#"{"value":"x","wat":1}"#).is_err());
        assert!(
            serde_json::from_str::<NumberInstance>(r#"{"value":"x","id":"_private"}"#)
                .unwrap()
                .validate()
                .is_err()
        );
        assert!(
            serde_json::from_str::<NumberInstance>(r#"{"value":"x","id":"safe-id"}"#)
                .unwrap()
                .validate()
                .is_ok()
        );
        assert!(
            serde_json::from_str::<NumberInstance>(r#"{"value":"x","durationMs":1001}"#)
                .unwrap()
                .validate()
                .is_err()
        );
        assert!(NumberInstance {
            value: "\n".into(),
            id: None,
            motion: Motion::Roll,
            direction: Direction::Up,
            duration_ms: 0
        }
        .validate()
        .is_err());
    }
}
