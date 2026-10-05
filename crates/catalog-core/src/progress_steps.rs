//! Static ordered workflow progress with optional icons rendered from locked package geometry.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}
impl Orientation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Detailed,
    Compact,
}
impl Appearance {
    fn as_str(self) -> &'static str {
        match self {
            Self::Detailed => "detailed",
            Self::Compact => "compact",
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StepState {
    Complete,
    Current,
    Error,
    Upcoming,
}
impl StepState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Current => "current",
            Self::Error => "error",
            Self::Upcoming => "upcoming",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgressStep {
    pub state: StepState,
    pub label: String,
    #[serde(default)]
    pub href: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateLabels {
    #[serde(default = "completed_label")]
    pub complete: String,
    #[serde(default = "current_label")]
    pub current: String,
    #[serde(default = "error_label")]
    pub error: String,
    #[serde(default = "upcoming_label")]
    pub upcoming: String,
}
fn completed_label() -> String {
    "Completed".into()
}
fn current_label() -> String {
    "Current".into()
}
fn error_label() -> String {
    "Needs attention".into()
}
fn upcoming_label() -> String {
    "Upcoming".into()
}
impl Default for StateLabels {
    fn default() -> Self {
        Self {
            complete: completed_label(),
            current: current_label(),
            error: error_label(),
            upcoming: upcoming_label(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProgressStepsInstance {
    #[serde(default = "default_label")]
    pub label: String,
    #[serde(default)]
    pub orientation: Orientation,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub state_labels: StateLabels,
    pub items: Vec<ProgressStep>,
}
fn default_label() -> String {
    "Progress".into()
}
fn safe_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
impl ProgressStepsInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let value: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !safe_text(&self.label) {
            return Err("progress steps label must be nonblank plain text".into());
        }
        if !(2..=10).contains(&self.items.len()) {
            return Err("progress steps require 2 to 10 items".into());
        }
        for value in [
            &self.state_labels.complete,
            &self.state_labels.current,
            &self.state_labels.error,
            &self.state_labels.upcoming,
        ] {
            if !safe_text(value) {
                return Err("state labels must be nonblank plain text".into());
            }
        }
        let active: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item.state, StepState::Current | StepState::Error))
            .map(|(n, _)| n)
            .collect();
        if active.len() != 1 {
            return Err("progress steps require exactly one current or error step".into());
        }
        let active = active[0];
        for (index, item) in self.items.iter().enumerate() {
            if !safe_text(&item.label) || item.detail.as_deref().is_some_and(|v| !safe_text(v)) {
                return Err("step labels and details must be nonblank plain text".into());
            }
            match item.state {
                StepState::Complete if index < active => {
                    let href = item.href.as_deref().ok_or("completed steps require href")?;
                    if !crate::button::safe_href(href) {
                        return Err("completed step href must be a safe destination".into());
                    }
                }
                StepState::Complete => {
                    return Err("completed steps must precede the current or error step".into());
                }
                StepState::Current | StepState::Error if index == active && item.href.is_none() => {
                }
                StepState::Current | StepState::Error if index == active => {
                    return Err("current and error steps forbid href".into());
                }
                StepState::Upcoming if index > active && item.href.is_none() => {}
                StepState::Upcoming if index > active => {
                    return Err("upcoming steps forbid href".into());
                }
                _ => {
                    return Err(
                        "steps must be ordered complete*, current-or-error, upcoming*".into(),
                    );
                }
            }
        }
        Ok(())
    }
}
fn state_label(labels: &StateLabels, state: StepState) -> &str {
    match state {
        StepState::Complete => &labels.complete,
        StepState::Current => &labels.current,
        StepState::Error => &labels.error,
        StepState::Upcoming => &labels.upcoming,
    }
}
/// Render without SVGs, using step numbers for all markers. Fragment slots: `[[attributes]]`, `[[label]]`, `[[items]]`.
pub fn render(instance: &ProgressStepsInstance, fragment: &str) -> Result<String, String> {
    render_inner(instance, fragment, None)
}
/// Optional validated icon seam: provide the locked geometry map and the package icon fragment
/// (`<svg [[attributes]]>[[geometry]]</svg>`). Complete/error markers use decorative `check`/`alert` SVGs;
/// the ordinary `render` API remains self-contained and uses numbered markers.
pub fn render_with_icons(
    instance: &ProgressStepsInstance,
    fragment: &str,
    geometries: &BTreeMap<String, String>,
    icon_fragment: &str,
) -> Result<String, String> {
    render_inner(instance, fragment, Some((geometries, icon_fragment)))
}
fn render_inner(
    instance: &ProgressStepsInstance,
    fragment: &str,
    icons: Option<(&BTreeMap<String, String>, &str)>,
) -> Result<String, String> {
    instance.validate()?;
    let mut items = String::new();
    for (index, item) in instance.items.iter().enumerate() {
        let marker = if let Some((geometry, icon_fragment)) = icons {
            let name = match item.state {
                StepState::Complete => Some("check"),
                StepState::Error => Some("alert"),
                _ => None,
            };
            if let Some(name) = name {
                let icon = crate::icon::IconInstance {
                    name: name.into(),
                    size: crate::icon::IconSize::Small,
                    label: None,
                };
                crate::icon::render(&icon, icon_fragment, geometry)?
            } else {
                (index + 1).to_string()
            }
        } else {
            (index + 1).to_string()
        };
        let content = if item.state == StepState::Complete {
            format!(
                "<a class=\"cui-progress-steps__target\" href=\"{}\">{}{}</a>",
                esc(item.href.as_deref().expect("validated href")),
                marker_html(&marker),
                step_copy(instance, item)
            )
        } else {
            let current = if matches!(item.state, StepState::Current | StepState::Error) {
                " aria-current=\"step\""
            } else {
                ""
            };
            format!(
                "<span class=\"cui-progress-steps__target\"{current}>{}{}{}</span>",
                marker_html(&marker),
                step_copy(instance, item),
                ""
            )
        };
        items.push_str(&format!(
            "<li class=\"cui-progress-steps__item cui-progress-steps__item--{}\">{content}</li>",
            item.state.as_str()
        ));
    }
    let attributes = format!(
        "class=\"cui-progress-steps cui-progress-steps--{} cui-progress-steps--{}\" data-cui-component=\"progress-steps\"",
        instance.orientation.as_str(),
        instance.appearance.as_str()
    );
    let label = esc(&instance.label);
    let list = format!("<ol class=\"cui-progress-steps__list\">{items}</ol>");
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attributes),
            ("[[label]]", &label),
            ("[[items]]", &list),
        ],
    )
}
fn marker_html(marker: &str) -> String {
    format!("<span class=\"cui-progress-steps__marker\" aria-hidden=\"true\">{marker}</span>")
}
fn step_copy(instance: &ProgressStepsInstance, item: &ProgressStep) -> String {
    let detail = item
        .detail
        .as_deref()
        .map(|v| {
            format!(
                "<span class=\"cui-progress-steps__detail\">{}</span>",
                esc(v)
            )
        })
        .unwrap_or_default();
    format!(
        "<span class=\"cui-progress-steps__content\"><span class=\"cui-progress-steps__sr-only\">{}: </span><span class=\"cui-progress-steps__label\">{}</span>{detail}</span>",
        esc(state_label(&instance.state_labels, item.state)),
        esc(&item.label)
    )
}
