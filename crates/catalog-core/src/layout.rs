//! Typed static rendering for the six Toolframe layout primitives.
//!
//! `AdmittedChildren` is a trust-boundary marker, not an HTML sanitizer. Its
//! constructor is for a Native host only after ordinary template/HTML admission;
//! this crate deliberately does not repeat host admission or budget checks.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Already-admitted host markup. Do not construct from untrusted/user input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedChildren(String);

impl AdmittedChildren {
    /// Marks markup admitted by the host's normal HTML/template pipeline.
    ///
    /// Rust visibility is not a sandbox. Call only after the host has applied
    /// its normal admission policy and budgets; this type does not sanitize.
    pub fn from_host_admitted(markup: impl Into<String>) -> Self {
        Self(markup.into())
    }

    pub(crate) fn as_markup(&self) -> &str {
        &self.0
    }
}

/// Slot map for card. The body is required; actions are optional admitted markup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardSlots {
    pub body: AdmittedChildren,
    pub actions: Option<AdmittedChildren>,
}
impl CardSlots {
    /// Convert these named slots to the generic adapter slot map.
    pub fn into_slot_map(self) -> BTreeMap<String, AdmittedChildren> {
        let mut slots = BTreeMap::from([("body".to_owned(), self.body)]);
        if let Some(actions) = self.actions {
            slots.insert("actions".to_owned(), actions);
        }
        slots
    }
}
/// Named, ordered slots for split. DOM order is always start then end.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SplitSlots {
    pub start: AdmittedChildren,
    pub end: AdmittedChildren,
}
impl SplitSlots {
    /// Convert these named slots to the generic adapter slot map.
    pub fn into_slot_map(self) -> BTreeMap<String, AdmittedChildren> {
        BTreeMap::from([
            ("start".to_owned(), self.start),
            ("end".to_owned(), self.end),
        ])
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Gap {
    None,
    Compact,
    #[default]
    Standard,
    Spacious,
    Generous,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Alignment {
    #[default]
    Stretch,
    Start,
    Center,
    End,
    Baseline,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Justification {
    #[default]
    Start,
    Center,
    End,
    Between,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Breakpoint {
    Compact,
    #[default]
    Standard,
    Wide,
    Never,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum ContainerWidth {
    Reading,
    Compact,
    #[default]
    Standard,
    Wide,
    Full,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Gutter {
    None,
    Compact,
    #[default]
    Standard,
    Generous,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Wrapping {
    #[default]
    Wrap,
    Nowrap,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Columns {
    #[default]
    Responsive,
    Two,
    Three,
    Four,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Minimum {
    Narrow,
    #[default]
    Standard,
    Wide,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Ratio {
    #[default]
    Equal,
    StartWide,
    EndWide,
    StartDominant,
    EndDominant,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Padding {
    None,
    Compact,
    #[default]
    Standard,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
pub enum HeadingLevel {
    #[default]
    #[serde(rename = "h2")]
    H2,
    #[serde(rename = "h3")]
    H3,
    #[serde(rename = "h4")]
    H4,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[derive(Default)]
pub struct ContainerConfig {
    #[serde(default)]
    pub width: ContainerWidth,
    #[serde(default)]
    pub gutter: Gutter,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StackConfig {
    #[serde(default)]
    pub gap: Gap,
    #[serde(default)]
    pub alignment: Alignment,
}
impl Default for StackConfig {
    fn default() -> Self {
        Self {
            gap: Gap::Standard,
            alignment: Alignment::Stretch,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClusterConfig {
    #[serde(default = "compact_gap")]
    pub gap: Gap,
    #[serde(default = "center_alignment")]
    pub alignment: Alignment,
    #[serde(default)]
    pub justification: Justification,
    #[serde(default)]
    pub wrapping: Wrapping,
}
fn compact_gap() -> Gap {
    Gap::Compact
}
fn center_alignment() -> Alignment {
    Alignment::Center
}
impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            gap: Gap::Compact,
            alignment: Alignment::Center,
            justification: Justification::Start,
            wrapping: Wrapping::Wrap,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridConfig {
    #[serde(default)]
    pub columns: Columns,
    #[serde(default)]
    pub minimum: Minimum,
    #[serde(default)]
    pub gap: Gap,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default)]
    pub collapse_at: Breakpoint,
}
impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: Columns::Responsive,
            minimum: Minimum::Standard,
            gap: Gap::Standard,
            alignment: Alignment::Stretch,
            collapse_at: Breakpoint::Standard,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitConfig {
    #[serde(default)]
    pub ratio: Ratio,
    #[serde(default)]
    pub gap: Gap,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default)]
    pub collapse_at: Breakpoint,
}
impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            ratio: Ratio::Equal,
            gap: Gap::Standard,
            alignment: Alignment::Stretch,
            collapse_at: Breakpoint::Standard,
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CoverHeight {
    Compact,
    #[default]
    Standard,
    Fill,
    Viewport,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LayerPlacement {
    #[default]
    Center,
    TopStart,
    TopEnd,
    BottomStart,
    BottomEnd,
    Stretch,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PaneHeight {
    #[default]
    Content,
    Compact,
    Standard,
    Fill,
    Viewport,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReelItemWidth {
    Narrow,
    #[default]
    Standard,
    Wide,
    Content,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReelSnap {
    #[default]
    Free,
    Start,
    Center,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SidebarSide {
    Start,
    #[default]
    End,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RailWidth {
    Narrow,
    #[default]
    Standard,
    Wide,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SwitchSizing {
    #[default]
    Natural,
    Equal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoverConfig {
    #[serde(default)]
    pub height: CoverHeight,
    #[serde(default)]
    pub gap: Gap,
}
impl Default for CoverConfig {
    fn default() -> Self {
        Self {
            height: CoverHeight::Standard,
            gap: Gap::Standard,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerConfig {
    #[serde(default)]
    pub placement: LayerPlacement,
    #[serde(default)]
    pub inset: Gap,
}
impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            placement: LayerPlacement::Center,
            inset: Gap::Standard,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaneConfig {
    #[serde(default)]
    pub height: PaneHeight,
    #[serde(default)]
    pub body_scrolling: Option<bool>,
}
impl Default for PaneConfig {
    fn default() -> Self {
        Self {
            height: PaneHeight::Content,
            body_scrolling: None,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReelConfig {
    pub label: String,
    #[serde(default)]
    pub item_width: ReelItemWidth,
    #[serde(default)]
    pub snap: ReelSnap,
    #[serde(default)]
    pub gap: Gap,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidebarConfig {
    #[serde(default)]
    pub side: SidebarSide,
    #[serde(default)]
    pub width: RailWidth,
    #[serde(default)]
    pub gap: Gap,
    #[serde(default = "start_alignment")]
    pub alignment: Alignment,
    #[serde(default)]
    pub collapse_at: Breakpoint,
}
fn start_alignment() -> Alignment {
    Alignment::Start
}
impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            side: SidebarSide::End,
            width: RailWidth::Standard,
            gap: Gap::Standard,
            alignment: Alignment::Start,
            collapse_at: Breakpoint::Standard,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwitchConfig {
    #[serde(default)]
    pub sizing: SwitchSizing,
    #[serde(default)]
    pub gap: Gap,
    #[serde(default)]
    pub alignment: Alignment,
    #[serde(default)]
    pub justification: Justification,
    #[serde(default)]
    pub collapse_at: Breakpoint,
}
impl Default for SwitchConfig {
    fn default() -> Self {
        Self {
            sizing: SwitchSizing::Natural,
            gap: Gap::Standard,
            alignment: Alignment::Stretch,
            justification: Justification::Start,
            collapse_at: Breakpoint::Standard,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardConfig {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub padding: Padding,
    #[serde(default)]
    pub heading_level: HeadingLevel,
}
impl Default for CardConfig {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            padding: Padding::Standard,
            heading_level: HeadingLevel::H2,
        }
    }
}

fn text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        Err(format!("{field} must contain safe, nonblank text"))
    } else {
        Ok(())
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace(char::from(39), "&#39;")
}
fn exact_slots(slots: &BTreeMap<String, AdmittedChildren>, names: &[&str]) -> Result<(), String> {
    if slots.len() != names.len() || names.iter().any(|n| !slots.contains_key(*n)) {
        Err(format!(
            "expected exactly these admitted child slots: {}",
            names.join(", ")
        ))
    } else {
        Ok(())
    }
}
fn fill(fragment: &str, fills: &[(&str, &str)]) -> Result<String, String> {
    crate::fragment::fill(fragment, fills)
}

/// Render a named layout using typed JSON options and separate host-admitted slots.
/// Slot sets are component-specific and validated exactly before rendering.
pub fn render(
    name: &str,
    options: &Value,
    slots: &BTreeMap<String, AdmittedChildren>,
    fragment: &str,
) -> Result<String, String> {
    match name {
        "cover" => {
            let c: CoverConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            if !slots.contains_key("primary")
                || slots
                    .keys()
                    .any(|s| !["primary", "top", "bottom"].contains(&s.as_str()))
            {
                return Err(
                    "cover requires primary and accepts only optional top/bottom slots".into(),
                );
            }
            let class = format!(
                "cui-cover cui-cover--height-{} cui-cover--gap-{}",
                k(c.height),
                k(c.gap)
            );
            let top = slots.get("top").map(|v| v.as_markup()).unwrap_or("");
            let bottom = slots.get("bottom").map(|v| v.as_markup()).unwrap_or("");
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[top]]", top),
                    ("[[primary]]", slots["primary"].as_markup()),
                    ("[[bottom]]", bottom),
                ],
            )
        }
        "layer" => {
            let c: LayerConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["base", "foreground"])?;
            let class = format!(
                "cui-layer cui-layer--placement-{} cui-layer--inset-{}",
                k(c.placement),
                k(c.inset)
            );
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[base]]", slots["base"].as_markup()),
                    ("[[foreground]]", slots["foreground"].as_markup()),
                ],
            )
        }
        "pane" => {
            let c: PaneConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            if !slots.contains_key("body")
                || slots
                    .keys()
                    .any(|s| !["body", "header", "footer"].contains(&s.as_str()))
            {
                return Err(
                    "pane requires body and accepts only optional header/footer slots".into(),
                );
            }
            let scrolling = c.body_scrolling.unwrap_or(c.height != PaneHeight::Content);
            let mut classes = vec![
                "cui-pane".to_owned(),
                format!("cui-pane--height-{}", k(c.height)),
            ];
            if c.height != PaneHeight::Content {
                classes.push("cui-pane--bounded".into());
            }
            if scrolling {
                classes.push("cui-pane--scroll".into());
            }
            if slots.contains_key("header") {
                classes.push("cui-pane--has-header".into());
            }
            if slots.contains_key("footer") {
                classes.push("cui-pane--has-footer".into());
            }
            let class = classes.join(" ");
            let header = slots
                .get("header")
                .map(|v| format!("<div class=\"cui-pane__header\">{}</div>", v.as_markup()))
                .unwrap_or_default();
            let footer = slots
                .get("footer")
                .map(|v| format!("<div class=\"cui-pane__footer\">{}</div>", v.as_markup()))
                .unwrap_or_default();
            let body = if scrolling {
                format!(
                    "<div class=\"cui-pane__body\" tabindex=\"0\">{}</div>",
                    slots["body"].as_markup()
                )
            } else {
                format!(
                    "<div class=\"cui-pane__body\">{}</div>",
                    slots["body"].as_markup()
                )
            };
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[header]]", &header),
                    ("[[body]]", &body),
                    ("[[footer]]", &footer),
                ],
            )
        }
        "reel" => {
            let c: ReelConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            text(&c.label, "reel label")?;
            exact_slots(slots, &["body"])?;
            let class = format!(
                "cui-reel cui-reel--width-{} cui-reel--snap-{} cui-reel--gap-{}",
                k(c.item_width),
                k(c.snap),
                k(c.gap)
            );
            let label = escape(&c.label);
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[label]]", &label),
                    ("[[body]]", slots["body"].as_markup()),
                ],
            )
        }
        "sidebar" => {
            let c: SidebarConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["main", "aside"])?;
            let class = format!(
                "cui-sidebar cui-sidebar--side-{} cui-sidebar--width-{} cui-sidebar--gap-{} cui-sidebar--align-{} cui-sidebar--collapse-{}",
                k(c.side),
                k(c.width),
                k(c.gap),
                k(c.alignment),
                k(c.collapse_at)
            );
            let main = format!(
                "<div class=\"cui-sidebar__main\">{}</div>",
                slots["main"].as_markup()
            );
            let aside = format!(
                "<div class=\"cui-sidebar__aside\">{}</div>",
                slots["aside"].as_markup()
            );
            let (first, second) = if c.side == SidebarSide::Start {
                (&aside, &main)
            } else {
                (&main, &aside)
            };
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[regions]]", &format!("{first}{second}")),
                ],
            )
        }
        "switch" => {
            let c: SwitchConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["body"])?;
            let class = format!(
                "cui-switch cui-switch--sizing-{} cui-switch--gap-{} cui-switch--align-{} cui-switch--justify-{} cui-switch--collapse-{}",
                k(c.sizing),
                k(c.gap),
                k(c.alignment),
                k(c.justification),
                k(c.collapse_at)
            );
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[body]]", slots["body"].as_markup()),
                ],
            )
        }
        "container" => {
            let c: ContainerConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["body"])?;
            let class = format!(
                "cui-container cui-container--{} cui-container--gutter-{}",
                k(c.width),
                k(c.gutter)
            );
            fill(
                fragment,
                &[("[[class]]", &class), ("[[body]]", &slots["body"].0)],
            )
        }
        "stack" => {
            let c: StackConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["body"])?;
            let class = format!(
                "cui-stack cui-stack--gap-{} cui-stack--align-{}",
                k(c.gap),
                k(c.alignment)
            );
            fill(
                fragment,
                &[("[[class]]", &class), ("[[body]]", &slots["body"].0)],
            )
        }
        "cluster" => {
            let c: ClusterConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["body"])?;
            let class = format!(
                "cui-cluster cui-cluster--gap-{} cui-cluster--align-{} cui-cluster--justify-{} cui-cluster--{}",
                k(c.gap),
                k(c.alignment),
                k(c.justification),
                k(c.wrapping)
            );
            fill(
                fragment,
                &[("[[class]]", &class), ("[[body]]", &slots["body"].0)],
            )
        }
        "grid" => {
            let c: GridConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            exact_slots(slots, &["body"])?;
            let class = format!(
                "cui-grid cui-grid--columns-{} cui-grid--minimum-{} cui-grid--gap-{} cui-grid--align-{} cui-grid--collapse-{}",
                k(c.columns),
                k(c.minimum),
                k(c.gap),
                k(c.alignment),
                k(c.collapse_at)
            );
            fill(
                fragment,
                &[("[[class]]", &class), ("[[body]]", &slots["body"].0)],
            )
        }
        "split" => {
            let c: SplitConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            if slots.len() != 2 || !slots.contains_key("start") || !slots.contains_key("end") {
                return Err("split requires exactly start and end admitted slots".into());
            }
            let class = format!(
                "cui-split cui-split--ratio-{} cui-split--gap-{} cui-split--align-{} cui-split--collapse-{}",
                k(c.ratio),
                k(c.gap),
                k(c.alignment),
                k(c.collapse_at)
            );
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[start]]", &slots["start"].0),
                    ("[[end]]", &slots["end"].0),
                ],
            )
        }
        "card" => {
            let c: CardConfig =
                serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
            if let Some(t) = &c.title {
                text(t, "card title")?
            }
            if let Some(t) = &c.subtitle {
                text(t, "card subtitle")?
            }
            if !slots.contains_key("body") || slots.keys().any(|k| k != "body" && k != "actions") {
                return Err("card requires body and accepts only optional actions slot".into());
            }
            let title = c
                .title
                .as_deref()
                .map(|v| {
                    format!(
                        "<h{} class=\"cui-card__title\">{}</h{}>",
                        heading(c.heading_level),
                        escape(v),
                        heading(c.heading_level)
                    )
                })
                .unwrap_or_default();
            let subtitle = c
                .subtitle
                .as_deref()
                .map(|v| format!("<p class=\"cui-card__subtitle\">{}</p>", escape(v)))
                .unwrap_or_default();
            let header = if title.is_empty() && subtitle.is_empty() {
                String::new()
            } else {
                format!("<header class=\"cui-card__header\">{title}{subtitle}</header>")
            };
            let actions = slots
                .get("actions")
                .map(|a| format!("<footer class=\"cui-card__actions\">{}</footer>", a.0))
                .unwrap_or_default();
            let class = format!("cui-card cui-card--{}", k(c.padding));
            fill(
                fragment,
                &[
                    ("[[class]]", &class),
                    ("[[header]]", &header),
                    ("[[body]]", &slots["body"].0),
                    ("[[actions]]", &actions),
                ],
            )
        }
        _ => Err(format!("unknown layout component: {name}")),
    }
}
fn k<T: Serialize>(v: T) -> String {
    serde_json::to_value(v)
        .expect("layout option is serializable")
        .as_str()
        .expect("layout enum serializes as a string")
        .to_owned()
}
fn heading(h: HeadingLevel) -> u8 {
    match h {
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
    }
}
