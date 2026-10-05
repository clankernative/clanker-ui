//! Native no-JavaScript disclosure rendering; body markup is host-admitted only.
use crate::layout::AdmittedChildren;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Appearance {
    Plain,
    #[default]
    Contained,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisclosureInstance {
    pub summary: String,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub open: bool,
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl DisclosureInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.summary.trim().is_empty() || self.summary.chars().any(char::is_control) {
            return Err("disclosure summary must be nonblank plain text".into());
        }
        Ok(())
    }
}
/// Render an empty disclosure shell; host compositions should use `render_with_body`.
pub fn render(i: &DisclosureInstance, fragment: &str) -> Result<String, String> {
    render_inner(i, "", fragment)
}
/// Render with body admitted by the Native host. This marker does not sanitize markup.
pub fn render_with_body(
    i: &DisclosureInstance,
    body: &AdmittedChildren,
    fragment: &str,
) -> Result<String, String> {
    render_inner(i, body.as_markup(), fragment)
}
fn render_inner(i: &DisclosureInstance, body: &str, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let attrs = format!(
        "class=\"cui-disclosure cui-disclosure--{}\" data-cui-component=\"disclosure\"{}",
        if i.appearance == Appearance::Contained {
            "contained"
        } else {
            "plain"
        },
        if i.open { " open" } else { "" }
    );
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attrs),
            ("[[summary]]", &esc(&i.summary)),
            ("[[body]]", body),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str =
        "<details [[attributes]]><summary>[[summary]]</summary><div>[[body]]</div></details>";
    fn i() -> DisclosureInstance {
        DisclosureInstance {
            summary: "Advanced <settings>".into(),
            appearance: Appearance::Contained,
            open: false,
        }
    }
    #[test]
    fn native_open_and_admitted_body() {
        let h = render_with_body(
            &i(),
            &AdmittedChildren::from_host_admitted("<p>Trusted</p>"),
            F,
        )
        .unwrap();
        assert!(h.contains("<details class=\"cui-disclosure cui-disclosure--contained\""));
        assert!(h.contains("Advanced &lt;settings&gt;"));
        assert!(h.contains("<p>Trusted</p>"));
        let mut x = i();
        x.open = true;
        x.appearance = Appearance::Plain;
        assert!(render(&x, F)
            .unwrap()
            .contains("disclosure--plain\" data-cui-component=\"disclosure\" open"));
    }
    #[test]
    fn rejects_controls_unknown_enums_and_bad_fragment() {
        let mut x = i();
        x.summary = "\u{7f}".into();
        assert!(x.validate().is_err());
        assert!(DisclosureInstance::parse(r#"{"summary":"Hi","appearance":"popup"}"#).is_err());
        assert!(DisclosureInstance::parse(r#"{"summary":"Hi","unexpected":1}"#).is_err());
        assert!(render(&i(), "[[attributes]]").is_err());
    }
}
