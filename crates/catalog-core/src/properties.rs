//! Serializable property metadata for the generic editor. This is metadata only:
//! values are still decoded by the component's typed Rust contract.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PropertyKind {
    Text,
    Multiline,
    Boolean,
    Enum,
    Number,
    List,
    Object,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityCondition {
    pub field: String,
    pub equals: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    /// Serde/config key, not a Native template binding path.
    pub name: String,
    /// Native declaration attribute; empty for composition-only values.
    pub attribute: String,
    pub label: String,
    pub kind: PropertyKind,
    pub required: bool,
    pub default: Value,
    pub choices: Vec<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    #[serde(default)]
    pub integer: bool,
    pub help: String,
    pub visible_when: Option<VisibilityCondition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Property>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_kind: Option<PropertyKind>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SlotDescriptor {
    pub name: String,
    pub required: bool,
    pub help: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComponentProperties {
    pub name: String,
    pub fields: Vec<Property>,
    pub slots: Vec<SlotDescriptor>,
    /// Template guidance is a separate discovery read model, never editable props.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_authoring: Option<crate::authoring::TemplateAuthoring>,
}

/// Project typed instance properties together with component-owned authoring guidance.
pub fn describe_component_properties(
    component: &crate::Component,
) -> Result<ComponentProperties, String> {
    let mut descriptor = describe_properties(&component.name)?;
    descriptor.template_authoring = component.template_authoring.clone();
    Ok(descriptor)
}

fn p(
    name: &str,
    kind: PropertyKind,
    required: bool,
    default: Value,
    choices: &[&str],
    help: &str,
) -> Property {
    let integer =
        matches!(kind, PropertyKind::Number) && matches!(name, "count" | "rows" | "maxLength");
    Property {
        name: name.into(),
        attribute: native_attribute(name),
        label: name.to_owned(),
        kind,
        required,
        default,
        choices: choices.iter().map(|s| (*s).into()).collect(),
        minimum: None,
        maximum: None,
        integer,
        help: help.into(),
        visible_when: None,
        fields: vec![],
        item_kind: None,
    }
}
fn native_attribute(name: &str) -> String {
    match name {
        "leadingIcon" => "icon".into(),
        "resultSummary" => "summary".into(),
        "inputType" => "input-type".into(),
        "maxLength" => "maxlength".into(),
        _ => {
            let mut out = String::new();
            for c in name.chars() {
                if c.is_ascii_uppercase() {
                    out.push('-');
                    out.push(c.to_ascii_lowercase());
                } else {
                    out.push(c);
                }
            }
            out
        }
    }
}
fn text(name: &str, required: bool) -> Property {
    p(
        name,
        PropertyKind::Text,
        required,
        Value::Null,
        &[],
        "Plain text; validated by the component contract.",
    )
}
fn en(name: &str, default: &str, choices: &[&str]) -> Property {
    p(
        name,
        PropertyKind::Enum,
        false,
        json!(default),
        choices,
        "Choose a value supported by the typed component contract.",
    )
}
fn flag(name: &str, default: bool) -> Property {
    p(name, PropertyKind::Boolean, false, json!(default), &[], "")
}
fn number(name: &str, required: bool, default: Value, minimum: f64, maximum: f64) -> Property {
    let mut value = p(
        name,
        PropertyKind::Number,
        required,
        default,
        &[],
        "Validated bounded integer.",
    );
    value.minimum = Some(minimum);
    value.maximum = Some(maximum);
    value.integer = true;
    value
}
fn opttext(name: &str) -> Property {
    p(
        name,
        PropertyKind::Text,
        false,
        Value::Null,
        &[],
        "Optional plain text.",
    )
}
fn literal_text(name: &str, required: bool, help: &str) -> Property {
    let mut value = p("value", PropertyKind::Text, true, Value::Null, &[], help);
    let mut kind = p(
        "kind",
        PropertyKind::Enum,
        true,
        json!("literal"),
        &["literal"],
        "Only literal text is editor-owned; checked page-field bindings stay host-owned.",
    );
    kind.attribute.clear();
    value.attribute.clear();
    let mut result = p(name, PropertyKind::Object, required, Value::Null, &[], help);
    result.fields = vec![kind, value];
    result
}
fn button_destination() -> Property {
    let mut kind = p(
        "kind",
        PropertyKind::Enum,
        true,
        json!("action"),
        &["action", "submit", "link"],
        "Select an existing operation type; the editor does not create commands or routes.",
    );
    kind.attribute.clear();
    let href_kind = p(
        "kind",
        PropertyKind::Enum,
        true,
        json!("literal"),
        &["literal"],
        "Only literal safe destinations are editor-owned.",
    );
    let href_value = p(
        "value",
        PropertyKind::Text,
        true,
        Value::Null,
        &[],
        "Safe HTTP(S) or relative href; checked by the typed button contract.",
    );
    let mut href = p(
        "href",
        PropertyKind::Object,
        false,
        Value::Null,
        &[],
        "Only required for kind=link.",
    );
    href.visible_when = Some(VisibilityCondition {
        field: "kind".into(),
        equals: json!("link"),
    });
    href.fields = vec![href_kind, href_value];
    let mut result = p(
        "destination",
        PropertyKind::Object,
        true,
        Value::Null,
        &[],
        "Typed destination; application owns command and route behavior.",
    );
    result.fields = vec![kind, href];
    result
}
fn date_draft(name: &str, tag: &str, optional: bool) -> Property {
    let choices = if optional {
        vec!["none", "single", "range"]
    } else {
        vec!["single", "range"]
    };
    let mut value = p(
        name,
        PropertyKind::Object,
        !optional,
        if optional {
            json!({"kind":"none"})
        } else {
            Value::Null
        },
        &[],
        "Tagged ISO date draft; the Rust contract validates dates and ordering.",
    );
    let mut discriminator = en(tag, if optional { "none" } else { "single" }, &choices);
    discriminator.required = true;
    let mut date = text("date", optional);
    date.visible_when = Some(VisibilityCondition {
        field: tag.into(),
        equals: json!("single"),
    });
    let mut start = text("start", optional);
    start.visible_when = Some(VisibilityCondition {
        field: tag.into(),
        equals: json!("range"),
    });
    let mut end = opttext("end");
    end.visible_when = start.visible_when.clone();
    value.fields = vec![discriminator, date, start, end];
    value
}
fn scalar_list(name: &str, required: bool, item_kind: PropertyKind, help: &str) -> Property {
    let mut x = p(name, PropertyKind::List, required, json!([]), &[], help);
    x.item_kind = Some(item_kind);
    x
}
fn bounded_scalar_list(
    name: &str,
    required: bool,
    item_kind: PropertyKind,
    help: &str,
    min: f64,
    max: f64,
) -> Property {
    let mut value = scalar_list(name, required, item_kind, help);
    value.minimum = Some(min);
    value.maximum = Some(max);
    value
}
fn list(name: &str, required: bool, fields: Vec<Property>, help: &str) -> Property {
    let mut x = p(name, PropertyKind::List, required, json!([]), &[], help);
    x.fields = fields;
    x
}
fn bounded_list(
    name: &str,
    required: bool,
    fields: Vec<Property>,
    help: &str,
    min: f64,
    max: f64,
) -> Property {
    let mut value = list(name, required, fields, help);
    value.minimum = Some(min);
    value.maximum = Some(max);
    value
}
fn slots(required: &[&str], optional: &[&str]) -> Vec<SlotDescriptor> {
    required
        .iter()
        .map(|n| SlotDescriptor {
            name: (*n).into(),
            required: true,
            help: "Host-admitted named child content; never raw HTML input.".into(),
        })
        .chain(optional.iter().map(|n| SlotDescriptor {
            name: (*n).into(),
            required: false,
            help: "Host-admitted named child content; never raw HTML input.".into(),
        }))
        .collect()
}
fn component(
    name: &str,
    fields: Vec<Property>,
    slotlist: Vec<SlotDescriptor>,
) -> ComponentProperties {
    ComponentProperties {
        name: name.into(),
        fields,
        slots: slotlist,
        template_authoring: None,
    }
}

fn serde_choices<T: Serialize>(variants: &[T]) -> Vec<String> {
    variants
        .iter()
        .map(|value| {
            serde_json::to_value(value)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

/// Serialize explicit typed enum variants; no variant is inferred from a naming convention.
fn typed_choices(component: &str, field: &str) -> Option<Vec<String>> {
    use crate::{
        alert as a, avatar as av, badge as b, button as bt, divider as dv, empty_state as es,
        form_field as ff, icon as ic, layout as l, metric as m, pagination as pg, progress as pr,
        skeleton as sk, status_indicator as si, tag as tg,
    };
    let values = match (component, field) {
        ("container", "width") => serde_choices(&[
            l::ContainerWidth::Reading,
            l::ContainerWidth::Compact,
            l::ContainerWidth::Standard,
            l::ContainerWidth::Wide,
            l::ContainerWidth::Full,
        ]),
        ("container", "gutter") => serde_choices(&[
            l::Gutter::None,
            l::Gutter::Compact,
            l::Gutter::Standard,
            l::Gutter::Generous,
        ]),
        (
            "stack" | "cluster" | "grid" | "split" | "cover" | "sidebar" | "switch" | "reel",
            "gap",
        ) => serde_choices(&[
            l::Gap::None,
            l::Gap::Compact,
            l::Gap::Standard,
            l::Gap::Spacious,
            l::Gap::Generous,
        ]),
        ("stack" | "cluster" | "grid" | "split" | "sidebar" | "switch", "alignment") => {
            serde_choices(&[
                l::Alignment::Stretch,
                l::Alignment::Start,
                l::Alignment::Center,
                l::Alignment::End,
                l::Alignment::Baseline,
            ])
        }
        ("cluster" | "switch", "justification") => serde_choices(&[
            l::Justification::Start,
            l::Justification::Center,
            l::Justification::End,
            l::Justification::Between,
        ]),
        ("cluster", "wrapping") => serde_choices(&[l::Wrapping::Wrap, l::Wrapping::Nowrap]),
        ("grid", "columns") => serde_choices(&[
            l::Columns::Responsive,
            l::Columns::Two,
            l::Columns::Three,
            l::Columns::Four,
        ]),
        ("grid", "minimum") => {
            serde_choices(&[l::Minimum::Narrow, l::Minimum::Standard, l::Minimum::Wide])
        }
        ("grid" | "split" | "sidebar" | "switch", "collapseAt") => serde_choices(&[
            l::Breakpoint::Compact,
            l::Breakpoint::Standard,
            l::Breakpoint::Wide,
            l::Breakpoint::Never,
        ]),
        ("split", "ratio") => serde_choices(&[
            l::Ratio::Equal,
            l::Ratio::StartWide,
            l::Ratio::EndWide,
            l::Ratio::StartDominant,
            l::Ratio::EndDominant,
        ]),
        ("cover", "height") => serde_choices(&[
            l::CoverHeight::Compact,
            l::CoverHeight::Standard,
            l::CoverHeight::Fill,
            l::CoverHeight::Viewport,
        ]),
        ("layer", "placement") => serde_choices(&[
            l::LayerPlacement::Center,
            l::LayerPlacement::TopStart,
            l::LayerPlacement::TopEnd,
            l::LayerPlacement::BottomStart,
            l::LayerPlacement::BottomEnd,
            l::LayerPlacement::Stretch,
        ]),
        ("layer", "inset") => serde_choices(&[
            l::Gap::None,
            l::Gap::Compact,
            l::Gap::Standard,
            l::Gap::Spacious,
            l::Gap::Generous,
        ]),
        ("pane", "height") => serde_choices(&[
            l::PaneHeight::Content,
            l::PaneHeight::Compact,
            l::PaneHeight::Standard,
            l::PaneHeight::Fill,
            l::PaneHeight::Viewport,
        ]),
        ("reel", "itemWidth") => serde_choices(&[
            l::ReelItemWidth::Narrow,
            l::ReelItemWidth::Standard,
            l::ReelItemWidth::Wide,
            l::ReelItemWidth::Content,
        ]),
        ("reel", "snap") => {
            serde_choices(&[l::ReelSnap::Free, l::ReelSnap::Start, l::ReelSnap::Center])
        }
        ("sidebar", "side") => serde_choices(&[l::SidebarSide::Start, l::SidebarSide::End]),
        ("sidebar", "width") => serde_choices(&[
            l::RailWidth::Narrow,
            l::RailWidth::Standard,
            l::RailWidth::Wide,
        ]),
        ("switch", "sizing") => serde_choices(&[l::SwitchSizing::Natural, l::SwitchSizing::Equal]),
        ("card", "padding") => {
            serde_choices(&[l::Padding::None, l::Padding::Compact, l::Padding::Standard])
        }
        ("card" | "alert", "headingLevel") => serde_choices(&[
            l::HeadingLevel::H2,
            l::HeadingLevel::H3,
            l::HeadingLevel::H4,
        ]),
        ("alert", "tone") => serde_choices(&[
            a::Tone::Info,
            a::Tone::Success,
            a::Tone::Warning,
            a::Tone::Danger,
        ]),
        ("alert", "appearance") => serde_choices(&[
            a::Appearance::Soft,
            a::Appearance::Outlined,
            a::Appearance::Accent,
        ]),
        ("alert", "announcement") => {
            serde_choices(&[a::Announcement::Polite, a::Announcement::Assertive])
        }
        ("avatar", "size") => serde_choices(&[
            av::Size::ExtraSmall,
            av::Size::Small,
            av::Size::Medium,
            av::Size::Large,
            av::Size::ExtraLarge,
        ]),
        ("avatar", "tone") => {
            serde_choices(&[av::Tone::Neutral, av::Tone::Brand, av::Tone::Success])
        }
        ("badge", "tone") => serde_choices(&[
            b::Tone::Neutral,
            b::Tone::Info,
            b::Tone::Success,
            b::Tone::Warning,
            b::Tone::Danger,
            b::Tone::Running,
        ]),
        ("button", "variant") => serde_choices(&[
            bt::Variant::Primary,
            bt::Variant::Secondary,
            bt::Variant::Danger,
            bt::Variant::Quiet,
        ]),
        ("button", "size") => serde_choices(&[bt::Size::Compact, bt::Size::Standard]),
        ("divider", "orientation") => {
            serde_choices(&[dv::Orientation::Horizontal, dv::Orientation::Vertical])
        }
        ("divider", "alignment") => serde_choices(&[
            dv::Alignment::Start,
            dv::Alignment::Center,
            dv::Alignment::End,
        ]),
        ("empty-state", "alignment") => {
            serde_choices(&[es::Alignment::Start, es::Alignment::Center])
        }
        ("empty-state", "headingLevel") => serde_choices(&[
            es::HeadingLevel::H2,
            es::HeadingLevel::H3,
            es::HeadingLevel::H4,
        ]),
        ("form-field", "kind") => serde_choices(&[ff::FieldKind::Input, ff::FieldKind::Textarea]),
        ("form-field", "inputType") => serde_choices(&[
            ff::InputType::Text,
            ff::InputType::Email,
            ff::InputType::Url,
            ff::InputType::Number,
            ff::InputType::Password,
            ff::InputType::Search,
            ff::InputType::Date,
            ff::InputType::Tel,
        ]),
        ("icon", "size") => serde_choices(&[
            ic::IconSize::Small,
            ic::IconSize::Medium,
            ic::IconSize::Large,
        ]),
        ("metric", "trend") => serde_choices(&[m::Trend::Up, m::Trend::Down, m::Trend::Flat]),
        ("metric", "trendTone") => serde_choices(&[
            m::TrendTone::Positive,
            m::TrendTone::Negative,
            m::TrendTone::Neutral,
        ]),
        ("metric", "appearance") => {
            serde_choices(&[m::Appearance::Plain, m::Appearance::Contained])
        }
        ("pagination", "variant") => serde_choices(&[
            pg::Variant::Standard,
            pg::Variant::Outlined,
            pg::Variant::Compact,
        ]),
        ("pagination", "kind") => serde_choices(&[
            pg::ItemKind::Page,
            pg::ItemKind::Current,
            pg::ItemKind::Previous,
            pg::ItemKind::Next,
            pg::ItemKind::Gap,
        ]),
        ("progress", "state") => serde_choices(&[
            pr::ProgressState::Determinate,
            pr::ProgressState::Indeterminate,
        ]),
        ("progress", "tone") => serde_choices(&[
            pr::Tone::Info,
            pr::Tone::Success,
            pr::Tone::Warning,
            pr::Tone::Danger,
        ]),
        ("progress", "size") => serde_choices(&[pr::Size::Regular, pr::Size::Compact]),
        ("skeleton", "shape") => {
            serde_choices(&[sk::Shape::Text, sk::Shape::Rectangle, sk::Shape::Circle])
        }
        ("skeleton", "size") => {
            serde_choices(&[sk::Size::Small, sk::Size::Medium, sk::Size::Large])
        }
        ("skeleton", "width") => {
            serde_choices(&[sk::Width::Short, sk::Width::Medium, sk::Width::Full])
        }
        ("status-indicator", "tone") => serde_choices(&[
            si::Tone::Neutral,
            si::Tone::Info,
            si::Tone::Success,
            si::Tone::Warning,
            si::Tone::Danger,
        ]),
        ("status-indicator", "size") => serde_choices(&[si::Size::Small, si::Size::Large]),
        ("tag", "tone") => serde_choices(&[
            tg::Tone::Neutral,
            tg::Tone::Brand,
            tg::Tone::Info,
            tg::Tone::Success,
            tg::Tone::Warning,
            tg::Tone::Danger,
        ]),
        ("tag", "size") => serde_choices(&[tg::Size::Small, tg::Size::Medium, tg::Size::Large]),
        _ => return None,
    };
    Some(values)
}

fn enrich_fields(component: &str, fields: &mut [Property]) {
    for field in fields {
        if field.kind == PropertyKind::Enum {
            if let Some(choices) = typed_choices(component, &field.name) {
                field.choices = choices;
            }
        }
        enrich_fields(component, &mut field.fields);
    }
}

fn serde_default<T: Default + Serialize>() -> Option<Value> {
    serde_json::to_value(T::default()).ok()
}

fn typed_enum_default(component: &str, field: &str) -> Option<Value> {
    use crate::{
        alert as a, avatar as av, badge as b, button as bt, divider as dv, empty_state as es,
        icon as ic, metric as m, pagination as pg, progress as pr, skeleton as sk,
        status_indicator as si, tag as tg,
    };
    match (component, field) {
        ("avatar", "size") => serde_default::<av::Size>(),
        ("avatar", "tone") => serde_default::<av::Tone>(),
        ("badge", "tone") => serde_default::<b::Tone>(),
        ("button", "variant") => serde_default::<bt::Variant>(),
        ("button", "size") => serde_default::<bt::Size>(),
        ("divider", "orientation") => serde_default::<dv::Orientation>(),
        ("divider", "alignment") => serde_default::<dv::Alignment>(),
        ("empty-state", "alignment") => serde_default::<es::Alignment>(),
        ("empty-state", "headingLevel") => serde_default::<es::HeadingLevel>(),
        ("icon", "size") => serde_default::<ic::IconSize>(),
        ("alert", "appearance") => serde_default::<a::Appearance>(),
        ("alert", "headingLevel") => serde_default::<a::HeadingLevel>(),
        ("metric", "appearance") => serde_default::<m::Appearance>(),
        ("pagination", "variant") => serde_default::<pg::Variant>(),
        ("progress", "tone") => serde_default::<pr::Tone>(),
        ("progress", "size") => serde_default::<pr::Size>(),
        ("skeleton", "size") => serde_default::<sk::Size>(),
        ("skeleton", "width") => serde_default::<sk::Width>(),
        ("status-indicator", "tone") => serde_default::<si::Tone>(),
        ("status-indicator", "size") => serde_default::<si::Size>(),
        ("tag", "tone") => serde_default::<tg::Tone>(),
        ("tag", "size") => serde_default::<tg::Size>(),
        _ => None,
    }
}

fn layout_defaults(name: &str) -> Option<Value> {
    use crate::layout as l;
    macro_rules! value {
        ($ty:ty) => {
            serde_json::to_value(<$ty>::default()).ok()
        };
    }
    match name {
        "container" => value!(l::ContainerConfig),
        "stack" => value!(l::StackConfig),
        "cluster" => value!(l::ClusterConfig),
        "grid" => value!(l::GridConfig),
        "split" => value!(l::SplitConfig),
        "cover" => value!(l::CoverConfig),
        "layer" => value!(l::LayerConfig),
        "pane" => value!(l::PaneConfig),
        "sidebar" => value!(l::SidebarConfig),
        "switch" => value!(l::SwitchConfig),
        "card" => value!(l::CardConfig),
        _ => None,
    }
}

/// Describe editor properties from the in-crate contracts. No package or filesystem access.
pub fn describe_properties(name: &str) -> Result<ComponentProperties, String> {
    use PropertyKind::{Multiline as M, Number as N};
    let mut d = match name {
        "alert" => component(
            name,
            vec![
                text("title", true),
                p("body", M, true, Value::Null, &[], ""),
                p(
                    "tone",
                    PropertyKind::Enum,
                    true,
                    Value::Null,
                    &["info", "success", "warning", "danger"],
                    "",
                ),
                en("appearance", "soft", &["soft", "outlined", "accent"]),
                en("headingLevel", "h2", &["h2", "h3", "h4"]),
                opttext("recoveryLabel"),
                opttext("recoveryHref"),
                p(
                    "announcement",
                    PropertyKind::Enum,
                    false,
                    Value::Null,
                    &["polite", "assertive"],
                    "Optional explicit live-announcement mode; omitted is static.",
                ),
            ],
            vec![],
        ),
        "avatar" => component(
            name,
            vec![
                text("initials", true),
                text("label", true),
                en(
                    "size",
                    "medium",
                    &["extra-small", "small", "medium", "large", "extra-large"],
                ),
                en("tone", "neutral", &["neutral", "brand", "success"]),
                opttext("imageSource"),
            ],
            vec![],
        ),
        "badge" => component(
            name,
            vec![
                text("label", true),
                en(
                    "tone",
                    "neutral",
                    &["neutral", "info", "success", "warning", "danger", "running"],
                ),
                opttext("icon"),
                flag("showIcon", true),
            ],
            vec![],
        ),
        "breadcrumbs" => component(
            name,
            vec![
                p(
                    "label",
                    PropertyKind::Text,
                    false,
                    json!("Breadcrumbs"),
                    &[],
                    "Accessible navigation label.",
                ),
                list(
                    "items",
                    true,
                    vec![text("label", true), opttext("href")],
                    "At least two breadcrumb entries; final entry is current.",
                ),
            ],
            vec![],
        ),
        "button" => component(
            name,
            vec![
                literal_text(
                    "label",
                    true,
                    "Button label; use a literal TextValue. Field bindings are host-owned.",
                ),
                en(
                    "variant",
                    "secondary",
                    &["primary", "secondary", "danger", "quiet"],
                ),
                en("size", "standard", &["compact", "standard"]),
                opttext("leadingIcon"),
                flag("edgeAligned", false),
                flag("disabled", false),
                flag("busy", false),
                literal_text(
                    "busyLabel",
                    false,
                    "Optional literal TextValue shown when busy.",
                ),
                button_destination(),
            ],
            vec![],
        ),
        "card" => component(
            name,
            vec![
                opttext("title"),
                opttext("subtitle"),
                en("padding", "standard", &["none", "compact", "standard"]),
                en("headingLevel", "h2", &["h2", "h3", "h4"]),
            ],
            slots(&["body"], &["actions"]),
        ),
        "cluster" => component(
            name,
            vec![
                en(
                    "gap",
                    "compact",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
                en(
                    "alignment",
                    "center",
                    &["stretch", "start", "center", "end", "baseline"],
                ),
                en(
                    "justification",
                    "start",
                    &["start", "center", "end", "between"],
                ),
                en("wrapping", "wrap", &["wrap", "nowrap"]),
            ],
            slots(&[], &["body"]),
        ),
        "container" => component(
            name,
            vec![
                en(
                    "width",
                    "standard",
                    &["reading", "compact", "standard", "wide", "full"],
                ),
                en(
                    "gutter",
                    "standard",
                    &["none", "compact", "standard", "generous"],
                ),
            ],
            slots(&[], &["body"]),
        ),
        "cover" => component(
            name,
            vec![
                en(
                    "height",
                    "standard",
                    &["compact", "standard", "fill", "viewport"],
                ),
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
            ],
            slots(&["primary"], &["top", "bottom"]),
        ),
        "data-table" => component(
            name,
            vec![
                text("caption", true),
                flag("captionVisible", false),
                scalar_list(
                    "columns",
                    true,
                    PropertyKind::Text,
                    "1 to 16 plain-text column labels.",
                ),
                list(
                    "rows",
                    false,
                    vec![
                        opttext("id"),
                        scalar_list(
                            "cells",
                            true,
                            PropertyKind::Text,
                            "Plain text cells; count must match columns.",
                        ),
                    ],
                    "At most 100 plain-text rows.",
                ),
            ],
            vec![],
        ),
        "divider" => component(
            name,
            vec![
                en("orientation", "horizontal", &["horizontal", "vertical"]),
                opttext("label"),
                en("alignment", "center", &["start", "center", "end"]),
            ],
            vec![],
        ),
        "activity-feed" => component(
            name,
            vec![
                text("label", true),
                list(
                    "entries",
                    true,
                    vec![
                        text("title", true),
                        opttext("detail"),
                        opttext("actor"),
                        text("occurredAt", true),
                        text("timeLabel", true),
                        en(
                            "tone",
                            "neutral",
                            &["neutral", "info", "success", "warning", "danger"],
                        ),
                        opttext("href"),
                    ],
                    "Chronological activity entries.",
                ),
                flag("compact", false),
            ],
            vec![],
        ),
        "definition-list" => component(
            name,
            vec![
                list(
                    "items",
                    true,
                    vec![
                        text("term", true),
                        p(
                            "value",
                            M,
                            true,
                            Value::Null,
                            &[],
                            "Plain text value; rich content must be host-admitted.",
                        ),
                    ],
                    "Term/value pairs.",
                ),
                en("density", "standard", &["standard", "compact"]),
                flag("dividers", true),
            ],
            vec![],
        ),
        "disclosure" => component(
            name,
            vec![
                text("summary", true),
                en("appearance", "contained", &["plain", "contained"]),
                flag("open", false),
            ],
            slots(&["body"], &[]),
        ),
        "button-group" => component(
            name,
            vec![
                text("label", true),
                en("appearance", "joined", &["joined", "spaced"]),
                flag("fullWidth", false),
            ],
            slots(&["body"], &[]),
        ),
        "checkbox-group" => component(
            name,
            vec![
                text("name", true),
                text("legend", true),
                list(
                    "choices",
                    true,
                    vec![
                        text("value", true),
                        text("label", true),
                        opttext("hint"),
                        flag("checked", false),
                        flag("disabled", false),
                    ],
                    "Checkbox choices.",
                ),
                opttext("description"),
                opttext("error"),
                en("layout", "stacked", &["stacked", "inline"]),
                flag("disabled", false),
            ],
            vec![],
        ),
        "radio-group" => component(
            name,
            vec![
                text("name", true),
                text("legend", true),
                list(
                    "choices",
                    true,
                    vec![
                        text("value", true),
                        text("label", true),
                        opttext("hint"),
                        flag("disabled", false),
                    ],
                    "Radio choices.",
                ),
                opttext("selected"),
                opttext("description"),
                opttext("error"),
                flag("required", false),
                en("layout", "stacked", &["stacked", "inline"]),
                flag("disabled", false),
            ],
            vec![],
        ),
        "toggle" => component(
            name,
            vec![
                text("name", true),
                text("label", true),
                opttext("hint"),
                opttext("error"),
                flag("checked", false),
                flag("required", false),
                flag("disabled", false),
            ],
            vec![],
        ),
        "tabs" => component(
            name,
            vec![
                text("label", true),
                list(
                    "items",
                    true,
                    vec![
                        text("label", true),
                        text("href", true),
                        flag("active", false),
                    ],
                    "Peer-page navigation destinations.",
                ),
            ],
            vec![],
        ),
        "segmented-control" => component(
            name,
            vec![
                text("label", true),
                flag("equalWidth", false),
                list(
                    "items",
                    true,
                    vec![
                        en("kind", "link", &["link", "current", "disabled"]),
                        text("label", true),
                        opttext("href"),
                    ],
                    "Mutually exclusive destinations.",
                ),
            ],
            vec![],
        ),
        "progress-steps" => {
            let mut state_labels = p(
                "stateLabels",
                PropertyKind::Object,
                false,
                json!({"complete":"Completed","current":"Current","error":"Needs attention","upcoming":"Upcoming"}),
                &[],
                "Localized complete/current/error/upcoming text.",
            );
            state_labels.fields = vec![
                p(
                    "complete",
                    PropertyKind::Text,
                    false,
                    json!("Completed"),
                    &[],
                    "Localized complete state.",
                ),
                p(
                    "current",
                    PropertyKind::Text,
                    false,
                    json!("Current"),
                    &[],
                    "Localized current state.",
                ),
                p(
                    "error",
                    PropertyKind::Text,
                    false,
                    json!("Needs attention"),
                    &[],
                    "Localized error state.",
                ),
                p(
                    "upcoming",
                    PropertyKind::Text,
                    false,
                    json!("Upcoming"),
                    &[],
                    "Localized upcoming state.",
                ),
            ];
            component(
                name,
                vec![
                    text("label", true),
                    en("orientation", "horizontal", &["horizontal", "vertical"]),
                    en("appearance", "detailed", &["detailed", "compact"]),
                    list(
                        "items",
                        true,
                        vec![
                            en(
                                "state",
                                "upcoming",
                                &["complete", "current", "error", "upcoming"],
                            ),
                            text("label", true),
                            opttext("href"),
                            opttext("detail"),
                        ],
                        "Ordered workflow steps.",
                    ),
                    state_labels,
                ],
                vec![],
            )
        }
        "copy-field" => component(
            name,
            vec![
                text("name", true),
                text("label", true),
                p(
                    "value",
                    M,
                    true,
                    Value::Null,
                    &[],
                    "Literal selectable value.",
                ),
                opttext("hint"),
                p(
                    "idleLabel",
                    PropertyKind::Text,
                    false,
                    json!("Copy value"),
                    &[],
                    "Idle copy-button label.",
                ),
                p(
                    "copiedLabel",
                    PropertyKind::Text,
                    false,
                    json!("Copied"),
                    &[],
                    "Success label.",
                ),
                p(
                    "failedLabel",
                    PropertyKind::Text,
                    false,
                    json!("Copy failed; select the value to copy it."),
                    &[],
                    "Failure guidance.",
                ),
            ],
            vec![],
        ),
        "empty-state" => component(
            name,
            vec![
                text("title", true),
                p("body", M, true, Value::Null, &[], ""),
                en("alignment", "center", &["start", "center"]),
                en("headingLevel", "h2", &["h2", "h3", "h4"]),
                opttext("actionHref"),
                opttext("actionLabel"),
            ],
            vec![],
        ),
        "filter-bar" => component(
            name,
            vec![text("label", true), opttext("resultSummary")],
            slots(&["controls"], &["actions", "applied"]),
        ),
        "form-field" => component(
            name,
            vec![
                text("name", true),
                text("label", true),
                en("kind", "input", &["input", "textarea"]),
                p(
                    "inputType",
                    PropertyKind::Enum,
                    false,
                    Value::Null,
                    &[
                        "text", "email", "url", "number", "password", "search", "date", "tel",
                    ],
                    "Only applies to input fields.",
                ),
                p("rows", N, false, Value::Null, &[], "Textarea row count."),
                p("value", M, false, json!(""), &[], "Browser draft value."),
                opttext("placeholder"),
                opttext("hint"),
                opttext("error"),
                opttext("autocomplete"),
                p(
                    "maxLength",
                    N,
                    false,
                    Value::Null,
                    &[],
                    "Maximum input length.",
                ),
                flag("required", false),
                flag("readonly", false),
                flag("disabled", false),
            ],
            vec![],
        ),
        "grid" => component(
            name,
            vec![
                en(
                    "columns",
                    "responsive",
                    &["responsive", "two", "three", "four"],
                ),
                en("minimum", "standard", &["narrow", "standard", "wide"]),
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
                en(
                    "alignment",
                    "stretch",
                    &["stretch", "start", "center", "end", "baseline"],
                ),
                en(
                    "collapseAt",
                    "standard",
                    &["compact", "standard", "wide", "never"],
                ),
            ],
            slots(&[], &["body"]),
        ),
        "icon" => component(
            name,
            vec![
                text("name", true),
                en("size", "medium", &["small", "medium", "large"]),
                opttext("label"),
            ],
            vec![],
        ),
        "layer" => component(
            name,
            vec![
                en(
                    "placement",
                    "center",
                    &[
                        "center",
                        "top-start",
                        "top-end",
                        "bottom-start",
                        "bottom-end",
                        "stretch",
                    ],
                ),
                en(
                    "inset",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
            ],
            slots(&["base", "foreground"], &[]),
        ),
        "modal" => component(
            name,
            vec![
                text("title", true),
                text("triggerLabel", true),
                text("fallbackHref", true),
                p(
                    "closeLabel",
                    PropertyKind::Text,
                    false,
                    json!("Close dialog"),
                    &[],
                    "Close control label.",
                ),
                en("size", "standard", &["small", "standard", "wide"]),
                en("headingLevel", "h2", &["h2", "h3", "h4"]),
            ],
            slots(&["body"], &["actions"]),
        ),
        "drawer" => component(
            name,
            vec![
                text("title", true),
                opttext("description"),
                text("triggerLabel", true),
                text("fallbackHref", true),
                p(
                    "closeLabel",
                    PropertyKind::Text,
                    false,
                    json!("Close drawer"),
                    &[],
                    "Close control label.",
                ),
                en("placement", "end", &["start", "end"]),
                en("size", "standard", &["narrow", "standard", "wide"]),
                en("headingLevel", "h2", &["h2", "h3", "h4"]),
            ],
            slots(&["body"], &["actions"]),
        ),
        "popover" => component(
            name,
            vec![
                text("label", true),
                en("alignment", "start", &["start", "end"]),
                en("width", "standard", &["narrow", "standard", "wide"]),
                opttext("leadingIcon"),
            ],
            slots(&["body"], &[]),
        ),
        "tooltip" => component(
            name,
            vec![
                text("triggerLabel", true),
                text("title", true),
                opttext("description"),
                p(
                    "icon",
                    PropertyKind::Text,
                    false,
                    json!("help"),
                    &[],
                    "Closed icon name from the locked package catalog.",
                ),
                en("placement", "top", &["top", "right", "bottom", "left"]),
                flag("arrow", false),
            ],
            vec![],
        ),
        "toast" => component(
            name,
            vec![
                text("label", true),
                bounded_list(
                    "messages",
                    true,
                    vec![
                        text("title", true),
                        p(
                            "body",
                            M,
                            true,
                            Value::Null,
                            &[],
                            "Plain text message body.",
                        ),
                        p(
                            "tone",
                            PropertyKind::Enum,
                            true,
                            Value::Null,
                            &["info", "success", "warning", "danger", "progress"],
                            "Closed tone and corresponding locked icon.",
                        ),
                        opttext("actionHref"),
                        opttext("actionLabel"),
                        p(
                            "dismissLabel",
                            PropertyKind::Text,
                            false,
                            json!("Dismiss notification"),
                            &[],
                            "Optional dismiss control label; null omits the control.",
                        ),
                        {
                            let mut timeout = p(
                                "timeout",
                                N,
                                false,
                                Value::Null,
                                &[],
                                "Optional timeout from 1000 to 60000 milliseconds.",
                            );
                            timeout.integer = true;
                            timeout.minimum = Some(1000.0);
                            timeout.maximum = Some(60000.0);
                            timeout
                        },
                    ],
                    "One to eight messages; action fields are paired.",
                    1.0,
                    8.0,
                ),
                en(
                    "position",
                    "bottom-end",
                    &[
                        "inline",
                        "top-center",
                        "top-end",
                        "bottom-center",
                        "bottom-end",
                    ],
                ),
                en("historyPolicy", "reset", &["reset", "preserve", "remove"]),
            ],
            vec![],
        ),
        "theme-switcher" => component(
            name,
            vec![
                text("label", true),
                bounded_list(
                    "choices",
                    true,
                    vec![text("value", true), text("label", true), text("icon", true)],
                    "Two to eight choices using locked closed icon names.",
                    2.0,
                    8.0,
                ),
                en("presentation", "text", &["text", "icons"]),
                text("defaultChoice", true),
                text("systemLightTheme", true),
                text("systemDarkTheme", true),
                p(
                    "storageKey",
                    PropertyKind::Text,
                    false,
                    json!("clanker-theme"),
                    &[],
                    "Optional app-owned storage key; null disables persistence.",
                ),
                opttext("targetId"),
            ],
            vec![],
        ),
        "metric" => component(
            name,
            vec![
                text("label", true),
                text("value", true),
                opttext("detail"),
                p(
                    "trend",
                    PropertyKind::Enum,
                    false,
                    Value::Null,
                    &["up", "down", "flat"],
                    "Trend fields must be supplied together.",
                ),
                p(
                    "trendTone",
                    PropertyKind::Enum,
                    false,
                    Value::Null,
                    &["positive", "negative", "neutral"],
                    "Trend fields must be supplied together.",
                ),
                opttext("trendLabel"),
                opttext("trendAnnouncement"),
                opttext("icon"),
                en("appearance", "plain", &["plain", "contained"]),
            ],
            vec![],
        ),
        "page-header" => component(
            name,
            vec![text("title", true), opttext("description")],
            slots(&[], &["actions"]),
        ),
        "pagination" => component(
            name,
            vec![
                p(
                    "label",
                    PropertyKind::Text,
                    false,
                    json!("Pagination"),
                    &[],
                    "Accessible navigation label.",
                ),
                en("variant", "standard", &["standard", "outlined", "compact"]),
                list(
                    "items",
                    true,
                    vec![
                        en(
                            "kind",
                            "page",
                            &["page", "current", "previous", "next", "gap"],
                        ),
                        p("label", PropertyKind::Text, true, Value::Null, &[], ""),
                        opttext("href"),
                    ],
                    "Exactly one current page is required.",
                ),
            ],
            vec![],
        ),
        "pane" => component(
            name,
            vec![
                en(
                    "height",
                    "content",
                    &["content", "compact", "standard", "fill", "viewport"],
                ),
                p(
                    "bodyScrolling",
                    PropertyKind::Boolean,
                    false,
                    Value::Null,
                    &[],
                    "If omitted, bounded heights scroll and content height does not.",
                ),
            ],
            slots(&["body"], &["header", "footer"]),
        ),
        "progress" => {
            let mut value = p(
                "value",
                N,
                false,
                Value::Null,
                &[],
                "Required with maximum when state is determinate; 0 <= value <= maximum.",
            );
            value.minimum = Some(0.0);
            value.visible_when = Some(VisibilityCondition {
                field: "state".into(),
                equals: json!("determinate"),
            });
            let mut maximum = p(
                "maximum",
                N,
                false,
                Value::Null,
                &[],
                "Must be greater than zero for determinate progress.",
            );
            maximum.minimum = Some(f64::MIN_POSITIVE);
            maximum.visible_when = value.visible_when.clone();
            component(
                name,
                vec![
                    text("label", true),
                    en("state", "indeterminate", &["determinate", "indeterminate"]),
                    value,
                    maximum,
                    opttext("suffix"),
                    opttext("detail"),
                    en("tone", "info", &["info", "success", "warning", "danger"]),
                    en("size", "regular", &["regular", "compact"]),
                ],
                vec![],
            )
        }
        "reel" => component(
            name,
            vec![
                text("label", true),
                en(
                    "itemWidth",
                    "standard",
                    &["narrow", "standard", "wide", "content"],
                ),
                en("snap", "free", &["free", "start", "center"]),
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
            ],
            slots(&[], &["body"]),
        ),
        "select-field" => component(
            name,
            vec![
                text("name", true),
                text("label", true),
                list(
                    "choices",
                    true,
                    vec![
                        text("value", true),
                        text("label", true),
                        flag("disabled", false),
                    ],
                    "Each choice has a unique value.",
                ),
                opttext("selected"),
                opttext("placeholder"),
                opttext("hint"),
                opttext("error"),
                flag("required", false),
                flag("disabled", false),
            ],
            vec![],
        ),
        "sidebar" => component(
            name,
            vec![
                en("side", "end", &["start", "end"]),
                en("width", "standard", &["narrow", "standard", "wide"]),
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
                en(
                    "alignment",
                    "start",
                    &["stretch", "start", "center", "end", "baseline"],
                ),
                en(
                    "collapseAt",
                    "standard",
                    &["compact", "standard", "wide", "never"],
                ),
            ],
            slots(&["main", "aside"], &[]),
        ),
        "skeleton" => component(
            name,
            vec![
                p(
                    "shape",
                    PropertyKind::Enum,
                    true,
                    Value::Null,
                    &["text", "rectangle", "circle"],
                    "",
                ),
                en("size", "medium", &["small", "medium", "large"]),
                en("width", "full", &["short", "medium", "full"]),
                flag("animated", true),
            ],
            vec![],
        ),
        "split" => component(
            name,
            vec![
                en(
                    "ratio",
                    "equal",
                    &[
                        "equal",
                        "start-wide",
                        "end-wide",
                        "start-dominant",
                        "end-dominant",
                    ],
                ),
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
                en(
                    "alignment",
                    "stretch",
                    &["stretch", "start", "center", "end", "baseline"],
                ),
                en(
                    "collapseAt",
                    "standard",
                    &["compact", "standard", "wide", "never"],
                ),
            ],
            slots(&["start", "end"], &[]),
        ),
        "stack" => component(
            name,
            vec![
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
                en(
                    "alignment",
                    "stretch",
                    &["stretch", "start", "center", "end", "baseline"],
                ),
            ],
            slots(&[], &["body"]),
        ),
        "status-indicator" => component(
            name,
            vec![
                text("label", true),
                en(
                    "tone",
                    "neutral",
                    &["neutral", "info", "success", "warning", "danger"],
                ),
                opttext("detail"),
                en("size", "small", &["small", "large"]),
                flag("pulse", false),
            ],
            vec![],
        ),
        "switch" => component(
            name,
            vec![
                en("sizing", "natural", &["natural", "equal"]),
                en(
                    "gap",
                    "standard",
                    &["none", "compact", "standard", "spacious", "generous"],
                ),
                en(
                    "alignment",
                    "stretch",
                    &["stretch", "start", "center", "end", "baseline"],
                ),
                en(
                    "justification",
                    "start",
                    &["start", "center", "end", "between"],
                ),
                en(
                    "collapseAt",
                    "standard",
                    &["compact", "standard", "wide", "never"],
                ),
            ],
            slots(&[], &["body"]),
        ),
        "tag" => component(
            name,
            vec![
                text("label", true),
                en(
                    "tone",
                    "neutral",
                    &["neutral", "brand", "info", "success", "warning", "danger"],
                ),
                en("size", "medium", &["small", "medium", "large"]),
                opttext("href"),
                opttext("icon"),
                opttext("count"),
                opttext("countLabel"),
                opttext("removeHref"),
                opttext("removeLabel"),
            ],
            vec![],
        ),
        "date-calendar" => component(
            name,
            vec![
                text("month", true),
                text("today", true),
                text("label", true),
                scalar_list(
                    "monthLabels",
                    true,
                    PropertyKind::Text,
                    "Exactly twelve explicit localized month labels.",
                ),
                scalar_list(
                    "weekdayShortLabels",
                    true,
                    PropertyKind::Text,
                    "Seven explicit localized short weekday labels.",
                ),
                scalar_list(
                    "weekdayFullLabels",
                    true,
                    PropertyKind::Text,
                    "Seven explicit localized full weekday labels.",
                ),
                text("previousLabel", true),
                text("nextLabel", true),
                number("firstDay", true, json!(0), 0.0, 6.0),
                opttext("minimum"),
                opttext("maximum"),
                scalar_list(
                    "unavailableDates",
                    false,
                    PropertyKind::Text,
                    "Unique strict ISO dates.",
                ),
                en("selectionMode", "single", &["single", "range"]),
                date_draft("selection", "kind", true),
                flag("interactive", false),
                flag("disabled", false),
            ],
            vec![],
        ),
        "date-picker" => component(
            name,
            vec![
                text("label", true),
                date_draft("value", "mode", false),
                text("month", true),
                text("today", true),
                scalar_list(
                    "monthLabels",
                    true,
                    PropertyKind::Text,
                    "Exactly twelve explicit localized month labels.",
                ),
                scalar_list(
                    "weekdayShortLabels",
                    true,
                    PropertyKind::Text,
                    "Seven explicit localized short weekday labels.",
                ),
                scalar_list(
                    "weekdayFullLabels",
                    true,
                    PropertyKind::Text,
                    "Seven explicit localized full weekday labels.",
                ),
                number("firstDay", true, json!(0), 0.0, 6.0),
                text("previousLabel", true),
                text("nextLabel", true),
                text("startLabel", true),
                text("endLabel", true),
                text("calendarLabel", true),
                text("openCalendarLabel", true),
                opttext("minimum"),
                opttext("maximum"),
                scalar_list(
                    "unavailableDates",
                    false,
                    PropertyKind::Text,
                    "Unavailable strict ISO dates.",
                ),
                en("presentation", "inline", &["inline", "dropdown"]),
                opttext("hint"),
                opttext("error"),
                flag("required", false),
                flag("disabled", false),
                opttext("name"),
                opttext("startName"),
                opttext("endName"),
            ],
            vec![],
        ),
        "command-menu" => component(
            name,
            vec![
                text("title", true),
                text("triggerLabel", true),
                text("fallbackHref", true),
                bounded_list(
                    "groups",
                    true,
                    vec![
                        text("id", true),
                        text("label", true),
                        bounded_list(
                            "items",
                            true,
                            vec![
                                text("id", true),
                                text("href", true),
                                text("label", true),
                                opttext("description"),
                                opttext("shortcut"),
                                opttext("icon"),
                                bounded_scalar_list(
                                    "keywords",
                                    false,
                                    PropertyKind::Text,
                                    "At most twelve plain keywords.",
                                    0.0,
                                    12.0,
                                ),
                            ],
                            "Groups contain 1..=20 items; ids are unique safe data keys.",
                            1.0,
                            20.0,
                        ),
                    ],
                    "One to eight groups with safe unique data keys.",
                    1.0,
                    8.0,
                ),
                text("searchLabel", true),
                text("searchPlaceholder", true),
                text("emptyText", true),
                text("instructions", true),
                text("closeLabel", true),
                {
                    let mut shortcut = p(
                        "shortcut",
                        PropertyKind::Object,
                        false,
                        Value::Null,
                        &[],
                        "Optional one-character global shortcut.",
                    );
                    shortcut.fields = vec![text("key", true), text("label", true)];
                    shortcut
                },
            ],
            vec![],
        ),
        "confirm-dialog" => component(
            name,
            vec![
                text("formId", true),
                text("title", true),
                text("body", true),
                text("triggerLabel", true),
                text("confirmLabel", true),
                text("cancelLabel", false),
                en("tone", "neutral", &["neutral", "danger"]),
                en("headingLevel", "h2", &["h2", "h3", "h4"]),
                opttext("submitName"),
                opttext("submitValue"),
            ],
            vec![],
        ),
        "file-upload" => component(
            name,
            vec![
                text("name", true),
                text("label", true),
                opttext("prompt"),
                opttext("dropLabel"),
                opttext("fileListLabel"),
                opttext("hint"),
                opttext("accept"),
                flag("multiple", false),
                flag("required", false),
                flag("disabled", false),
                opttext("error"),
                bounded_list(
                    "files",
                    false,
                    vec![
                        text("id", true),
                        text("name", true),
                        opttext("meta"),
                        {
                            let mut status = p(
                                "status",
                                PropertyKind::Object,
                                true,
                                Value::Null,
                                &[],
                                "Tagged ready/complete/failed or uploading with progress 0..=100.",
                            );
                            status.fields = vec![
                                en(
                                    "state",
                                    "ready",
                                    &["ready", "uploading", "complete", "failed"],
                                ),
                                number("progress", false, Value::Null, 0.0, 100.0),
                            ];
                            status
                        },
                        text("statusLabel", true),
                        p(
                            "action",
                            PropertyKind::Object,
                            false,
                            Value::Null,
                            &[],
                            "Optional paired label and safe href.",
                        ),
                    ],
                    "At most 25 server-owned display rows; no upload transport is implied.",
                    0.0,
                    25.0,
                ),
            ],
            vec![],
        ),
        "data-viewport" => component(
            name,
            vec![
                text("label", true),
                en("height", "standard", &["compact", "standard", "viewport"]),
                en("mode", "paged", &["paged", "incremental", "windowed"]),
                {
                    let mut window = p(
                        "window",
                        PropertyKind::Object,
                        false,
                        Value::Null,
                        &[],
                        "Windowed-only bounds and optional rendered anchor.",
                    );
                    window.fields = vec![
                        number("start", true, Value::Null, 0.0, 9_999_999.0),
                        number("total", true, Value::Null, 1.0, 10_000_000.0),
                        number("itemSize", true, Value::Null, 16.0, 4096.0),
                        number("overscan", true, Value::Null, 1.0, 100.0),
                        opttext("anchor"),
                    ];
                    window
                },
                bounded_list(
                    "items",
                    false,
                    vec![text("id", true), text("content", true)],
                    "Plain-text list items; 1..=500.",
                    1.0,
                    500.0,
                ),
                scalar_list(
                    "columns",
                    false,
                    PropertyKind::Text,
                    "One to twelve plain-text table headers.",
                ),
                bounded_list(
                    "rows",
                    false,
                    vec![
                        text("id", true),
                        bounded_scalar_list(
                            "cells",
                            true,
                            PropertyKind::Text,
                            "Plain-text cells matching columns.",
                            0.0,
                            12.0,
                        ),
                        flag("highlighted", false),
                    ],
                    "Plain-text table rows; 1..=500.",
                    1.0,
                    500.0,
                ),
                text("pagination", true),
            ],
            vec![],
        ),
        _ => return Err(format!("unknown ready component: {name}")),
    };
    if let Some(defaults) = layout_defaults(name) {
        for field in &mut d.fields {
            if let Some(value) = defaults.get(&field.name) {
                field.default = value.clone();
            }
        }
    }
    enrich_fields(name, &mut d.fields);
    for field in &mut d.fields {
        if let Some(default) = typed_enum_default(name, &field.name) {
            field.default = default;
        }
    }
    Ok(d)
}

/// Return a valid, representative typed instance for editor previews and contract tests.
/// Layout samples are generated from their Rust `Default` implementations.
pub fn example_properties(name: &str) -> Result<Value, String> {
    example_properties_context(name, None)
}

/// Return a representative instance validated against the caller's locked icon geometry.
pub fn example_properties_with_icons(
    name: &str,
    icons: &std::collections::BTreeMap<String, String>,
) -> Result<Value, String> {
    example_properties_context(name, Some(icons))
}

fn example_properties_context(
    name: &str,
    icons: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<Value, String> {
    let sample = match name {
        "modal" => {
            json!({"title":"Confirm changes","triggerLabel":"Review changes","fallbackHref":"/changes",})
        }
        "drawer" => {
            json!({"title":"Filters","triggerLabel":"Open filters","fallbackHref":"/filters",})
        }
        "popover" => json!({"label":"More options",}),
        "tooltip" => json!({"triggerLabel":"Help","title":"Helpful context."}),
        "toast" => {
            json!({"label":"Notifications","messages":[{"title":"Saved","body":"Your changes were saved.","tone":"success"}]})
        }
        "theme-switcher" => {
            json!({"label":"Theme","choices":[{"value":"light","label":"Light","icon":"sun"},{"value":"dark","label":"Dark","icon":"moon"}],"defaultChoice":"light","systemLightTheme":"light","systemDarkTheme":"dark"})
        }
        "activity-feed" => {
            json!({"label":"Recent activity","entries":[{"title":"Deployment complete","detail":"Release 2.4 is live.","actor":"Jordan","occurredAt":"2024-01-15T12:00:00Z","timeLabel":"Today","tone":"success","href":"/releases/2.4"}]})
        }
        "definition-list" => json!({"items":[{"term":"Owner","value":"Platform"}]}),
        "disclosure" => json!({"summary":"More details"}),
        "button-group" => json!({"label":"Actions"}),
        "checkbox-group" => {
            json!({"name":"topics","legend":"Topics","choices":[{"value":"rust","label":"Rust","checked":true}]})
        }
        "radio-group" => {
            json!({"name":"region","legend":"Region","choices":[{"value":"west","label":"West"},{"value":"east","label":"East"}]})
        }
        "toggle" => json!({"name":"notifications","label":"Notifications"}),
        "tabs" => {
            json!({"label":"Sections","items":[{"label":"Overview","href":"/overview","active":true},{"label":"Activity","href":"/activity"}]})
        }
        "segmented-control" => {
            json!({"label":"Views","items":[{"kind":"current","label":"List"},{"kind":"link","label":"Board","href":"/board"}]})
        }
        "progress-steps" => {
            json!({"items":[{"state":"complete","label":"Started","href":"/started"},{"state":"current","label":"Review"},{"state":"upcoming","label":"Publish"}]})
        }
        "copy-field" => json!({"name":"reference","label":"Reference","value":"REF-42"}),
        "date-calendar" => {
            json!({"month":"2026-08-01","today":"2026-08-25","label":"Release calendar","monthLabels":["January","February","March","April","May","June","July","August","September","October","November","December"],"weekdayShortLabels":["Su","Mo","Tu","We","Th","Fr","Sa"],"weekdayFullLabels":["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"],"previousLabel":"Previous month","nextLabel":"Next month","firstDay":0})
        }
        "date-picker" => {
            json!({"label":"Release date","value":{"mode":"single","date":"2026-08-25"},"month":"2026-08-01","today":"2026-08-25","monthLabels":["January","February","March","April","May","June","July","August","September","October","November","December"],"weekdayShortLabels":["Su","Mo","Tu","We","Th","Fr","Sa"],"weekdayFullLabels":["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"],"firstDay":0,"previousLabel":"Previous month","nextLabel":"Next month","startLabel":"Date","endLabel":"End date","calendarLabel":"Choose a release date","openCalendarLabel":"Open calendar","presentation":"dropdown","required":true,"name":"releaseDate"})
        }
        "command-menu" => {
            json!({"title":"Application commands","triggerLabel":"Open commands","fallbackHref":"/search","groups":[{"id":"actions","label":"Actions","items":[{"id":"new-report","href":"/reports/new","label":"Create report","icon":"plus","keywords":["build","analytics"]}]}]})
        }
        "confirm-dialog" => {
            json!({"formId":"archive-project-form","title":"Archive project?","body":"Members will no longer be able to edit this project.","triggerLabel":"Archive project","confirmLabel":"Archive"})
        }
        "file-upload" => {
            json!({"name":"attachments","label":"Attachments","accept":".pdf,.png","multiple":true,"files":[{"id":"requirements","name":"requirements.pdf","status":{"state":"uploading","progress":48},"statusLabel":"Uploading 48%"}]})
        }
        "data-viewport" => {
            json!({"label":"Recent activity","mode":"paged","pagination":"Previous · Page 1 · Next","columns":["Event","Owner","Status"],"rows":[{"id":"activity-1","cells":["Quarterly review","Mina Chen","Open"]}]})
        }
        "alert" => json!({"title":"Notice","body":"A representative message.","tone":"info"}),
        "avatar" => json!({"initials":"JD","label":"Jordan Doe"}),
        "badge" => json!({"label":"Ready","tone":"success"}),
        "breadcrumbs" => json!({"items":[{"label":"Home","href":"/"},{"label":"Current"}]}),
        "button" => {
            json!({"label":{"kind":"literal","value":"Continue"},"destination":{"kind":"action"}})
        }
        "card" | "container" | "stack" | "cluster" | "grid" | "split" | "cover" | "layer"
        | "pane" | "sidebar" | "switch" => json!({}),
        "data-table" => {
            json!({"caption":"Example data","columns":["Name","State"],"rows":[{"cells":["Sample task","Ready"]}]})
        }
        "divider" => json!({}),
        "empty-state" => json!({"title":"Nothing here yet","body":"Try again later."}),
        "filter-bar" => json!({"label":"Filters"}),
        "form-field" => json!({"name":"search","label":"Search"}),
        "icon" => json!({"name":"check"}),
        "metric" => json!({"label":"Active users","value":"42"}),
        "page-header" => json!({"title":"Overview"}),
        "pagination" => json!({"items":[{"kind":"current","label":"1"}]}),
        "progress" => json!({"label":"Upload","state":"determinate","value":1,"maximum":2}),
        "reel" => json!({"label":"Featured items"}),
        "select-field" => {
            json!({"name":"region","label":"Region","choices":[{"value":"west","label":"West"}]})
        }
        "skeleton" => json!({"shape":"text"}),
        "status-indicator" => json!({"label":"Online"}),
        "tag" => json!({"label":"Active"}),
        _ => return Err(format!("unknown ready component: {name}")),
    };
    validate_properties_context(name, sample, icons)
}

/// Validate presentation-only JSON through the actual Rust serde contract and its available core validator.
/// `id`/`component` are fixed for editor previews and must not be supplied. The returned normalized
/// instance may include those injected values. Icon membership and rendering/host-admission checks
/// still require adapter/package context and must be repeated before rendering.
pub fn validate_properties(name: &str, input: Value) -> Result<Value, String> {
    validate_properties_context(name, input, None)
}

/// Validate properties using the exact geometry catalog locked by the caller's package.
pub fn validate_properties_with_icons(
    name: &str,
    input: Value,
    icons: &std::collections::BTreeMap<String, String>,
) -> Result<Value, String> {
    validate_properties_context(name, input, Some(icons))
}

fn validate_properties_context(
    name: &str,
    mut input: Value,
    icons: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<Value, String> {
    let _ = describe_properties(name)?;
    // Browser-owned properties never supply host infrastructure identifiers.
    match name {
        "button" => {
            let object = input
                .as_object_mut()
                .ok_or("button properties must be an object")?;
            if object.contains_key("id") || object.contains_key("component") {
                return Err(
                    "button id/component are fixed host infrastructure, not editor properties"
                        .into(),
                );
            }
            for key in ["label", "busyLabel"] {
                if let Some(value) = object.get(key) {
                    if value.get("kind").and_then(Value::as_str) != Some("literal") {
                        return Err(format!(
                            "{key} must use a literal TextValue; page bindings are host-owned"
                        ));
                    }
                }
            }
            if let Some(destination) = object.get("destination") {
                let kind = destination.get("kind").and_then(Value::as_str);
                if !matches!(kind, Some("action" | "submit" | "link")) {
                    return Err("destination must be action, submit, or link".into());
                }
                if kind == Some("link")
                    && destination
                        .get("href")
                        .and_then(|v| v.get("kind"))
                        .and_then(Value::as_str)
                        != Some("literal")
                {
                    return Err(
                        "editor links must use a literal href; route bindings are host-owned"
                            .into(),
                    );
                }
            }
            object.insert("id".into(), json!("editor-preview"));
            object.insert("component".into(), json!("@clanker/vanilla/button"));
        }
        "modal" | "drawer" | "popover" | "tooltip" => {
            let object = input
                .as_object_mut()
                .ok_or("component properties must be an object")?;
            if object.contains_key("id") {
                return Err(
                    "id is a fixed editor preview identifier, not a caller property".into(),
                );
            }
            object.insert("id".into(), json!(format!("editor-{name}")));
        }
        "checkbox-group" | "radio-group" | "toggle" | "copy-field" | "date-calendar"
        | "date-picker" | "command-menu" | "confirm-dialog" | "data-viewport" => {
            let object = input
                .as_object_mut()
                .ok_or("component properties must be an object")?;
            if object.contains_key("id") {
                return Err("id is fixed host infrastructure, not an editor property".into());
            }
            object.insert("id".into(), json!(format!("editor-{name}")));
        }
        "file-upload" => {
            let object = input
                .as_object_mut()
                .ok_or("component properties must be an object")?;
            if object.contains_key("id") {
                return Err("id is fixed host infrastructure, not an editor property".into());
            }
            object.insert("id".into(), json!("editor-file-upload"));
        }
        "form-field" | "select-field" => {
            let object = input
                .as_object_mut()
                .ok_or("component properties must be an object")?;
            if object.contains_key("id") {
                return Err("id is fixed host infrastructure, not an editor property".into());
            }
            object.insert("id".into(), json!(format!("editor-{name}")));
        }
        _ => {}
    }
    macro_rules! check { ($ty:ty $(, $validate:expr)?) => {{ let v: $ty=serde_json::from_value(input).map_err(|e|e.to_string())?; $(($validate)(&v)?;)? serde_json::to_value(v).map_err(|e|e.to_string()) }} }
    use crate as c;
    match name {
        "modal" => check!(c::modal::ModalInstance, |v: &c::modal::ModalInstance| v
            .validate()),
        "drawer" => check!(
            c::drawer::DrawerInstance,
            |v: &c::drawer::DrawerInstance| v.validate()
        ),
        "popover" => {
            let icons = icons.ok_or("popover validation requires locked icon geometry")?;
            check!(
                c::popover::PopoverInstance,
                |v: &c::popover::PopoverInstance| v.validate(icons)
            )
        }
        "tooltip" => {
            let icons = icons.ok_or("tooltip validation requires locked icon geometry")?;
            check!(
                c::tooltip::TooltipInstance,
                |v: &c::tooltip::TooltipInstance| v.validate(icons)
            )
        }
        "toast" => {
            let icons = icons.ok_or("toast validation requires locked icon geometry")?;
            check!(c::toast::ToastInstance, |v: &c::toast::ToastInstance| v
                .validate(icons))
        }
        "theme-switcher" => {
            let icons = icons.ok_or("theme-switcher validation requires locked icon geometry")?;
            check!(
                c::theme_switcher::ThemeSwitcherInstance,
                |v: &c::theme_switcher::ThemeSwitcherInstance| v.validate(icons)
            )
        }
        "activity-feed" => check!(
            c::activity_feed::ActivityFeedInstance,
            |v: &c::activity_feed::ActivityFeedInstance| v.validate()
        ),
        "definition-list" => check!(
            c::definition_list::DefinitionListInstance,
            |v: &c::definition_list::DefinitionListInstance| v.validate()
        ),
        "disclosure" => check!(
            c::disclosure::DisclosureInstance,
            |v: &c::disclosure::DisclosureInstance| v.validate()
        ),
        "button-group" => check!(
            c::button_group::ButtonGroupInstance,
            |v: &c::button_group::ButtonGroupInstance| v.validate()
        ),
        "checkbox-group" => check!(
            c::checkbox_group::CheckboxGroupInstance,
            |v: &c::checkbox_group::CheckboxGroupInstance| v.validate()
        ),
        "radio-group" => check!(
            c::radio_group::RadioGroupInstance,
            |v: &c::radio_group::RadioGroupInstance| v.validate()
        ),
        "toggle" => check!(
            c::toggle::ToggleInstance,
            |v: &c::toggle::ToggleInstance| v.validate()
        ),
        "tabs" => check!(c::tabs::TabsInstance, |v: &c::tabs::TabsInstance| v
            .validate()),
        "segmented-control" => check!(
            c::segmented_control::SegmentedControlInstance,
            |v: &c::segmented_control::SegmentedControlInstance| v.validate()
        ),
        "progress-steps" => check!(
            c::progress_steps::ProgressStepsInstance,
            |v: &c::progress_steps::ProgressStepsInstance| v.validate()
        ),
        "copy-field" => check!(
            c::copy_field::CopyFieldInstance,
            |v: &c::copy_field::CopyFieldInstance| v.validate()
        ),
        "date-calendar" => check!(
            c::date_calendar::CalendarInstance,
            |v: &c::date_calendar::CalendarInstance| v.validate()
        ),
        "date-picker" => check!(
            c::date_picker::PickerInstance,
            |v: &c::date_picker::PickerInstance| v.validate()
        ),
        "command-menu" => {
            let icons = icons.ok_or("command-menu validation requires locked icon geometry")?;
            check!(
                c::command_menu::CommandMenuInstance,
                |v: &c::command_menu::CommandMenuInstance| v.validate(icons)
            )
        }
        "confirm-dialog" => check!(
            c::confirm_dialog::ConfirmDialogInstance,
            |v: &c::confirm_dialog::ConfirmDialogInstance| v.validate()
        ),
        "file-upload" => check!(
            c::file_upload::FileUploadInstance,
            |v: &c::file_upload::FileUploadInstance| v.validate()
        ),
        "data-viewport" => check!(
            c::data_viewport::ViewportInstance,
            |v: &c::data_viewport::ViewportInstance| v.validate()
        ),
        "alert" => check!(c::alert::AlertInstance),
        "avatar" => check!(
            c::avatar::AvatarInstance,
            |v: &c::avatar::AvatarInstance| v.validate()
        ),
        "badge" => check!(c::badge::BadgeInstance),
        "breadcrumbs" => check!(
            c::breadcrumbs::BreadcrumbsInstance,
            |v: &c::breadcrumbs::BreadcrumbsInstance| v.validate()
        ),
        "button" => check!(
            c::button::ButtonInstance,
            |v: &c::button::ButtonInstance| v.validate("@clanker/vanilla")
        ),
        "data-table" => check!(
            c::data_table::DataTableInstance,
            |v: &c::data_table::DataTableInstance| v.validate()
        ),
        "divider" => check!(
            c::divider::DividerInstance,
            |v: &c::divider::DividerInstance| v.validate()
        ),
        "empty-state" => check!(
            c::empty_state::EmptyStateInstance,
            |v: &c::empty_state::EmptyStateInstance| v.validate()
        ),
        "filter-bar" => check!(
            c::filter_bar::FilterBarInstance,
            |v: &c::filter_bar::FilterBarInstance| v.validate()
        ),
        "form-field" => check!(c::form_field::FormFieldInstance),
        "metric" => check!(c::metric::MetricInstance),
        "page-header" => check!(
            c::page_header::PageHeaderInstance,
            |v: &c::page_header::PageHeaderInstance| v.validate()
        ),
        "pagination" => check!(
            c::pagination::PaginationInstance,
            |v: &c::pagination::PaginationInstance| v.validate()
        ),
        "progress" => check!(
            c::progress::ProgressInstance,
            |v: &c::progress::ProgressInstance| v.validate()
        ),
        "select-field" => check!(
            c::select_field::SelectFieldInstance,
            |v: &c::select_field::SelectFieldInstance| v.validate()
        ),
        "skeleton" => check!(c::skeleton::SkeletonInstance),
        "status-indicator" => check!(
            c::status_indicator::StatusIndicatorInstance,
            |v: &c::status_indicator::StatusIndicatorInstance| v.validate()
        ),
        "tag" => check!(c::tag::TagInstance),
        "container" => check!(c::layout::ContainerConfig),
        "stack" => check!(c::layout::StackConfig),
        "cluster" => check!(c::layout::ClusterConfig),
        "grid" => check!(c::layout::GridConfig),
        "split" => check!(c::layout::SplitConfig),
        "cover" => check!(c::layout::CoverConfig),
        "layer" => check!(c::layout::LayerConfig),
        "pane" => check!(c::layout::PaneConfig),
        "reel" => check!(c::layout::ReelConfig),
        "sidebar" => check!(c::layout::SidebarConfig),
        "switch" => check!(c::layout::SwitchConfig),
        "card" => check!(c::layout::CardConfig),
        // These types have no context-free validator; serde still rejects unknown/ill-typed keys.
        "icon" => check!(c::icon::IconInstance),
        _ => Err(format!("validation is not implemented for {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const NAMES: &[&str] = &[
        "activity-feed",
        "alert",
        "avatar",
        "button-group",
        "checkbox-group",
        "copy-field",
        "definition-list",
        "disclosure",
        "badge",
        "breadcrumbs",
        "button",
        "card",
        "cluster",
        "container",
        "cover",
        "data-table",
        "data-viewport",
        "date-calendar",
        "date-picker",
        "command-menu",
        "confirm-dialog",
        "file-upload",
        "divider",
        "empty-state",
        "filter-bar",
        "form-field",
        "grid",
        "icon",
        "layer",
        "metric",
        "modal",
        "drawer",
        "popover",
        "tooltip",
        "toast",
        "theme-switcher",
        "page-header",
        "pagination",
        "pane",
        "progress",
        "progress-steps",
        "radio-group",
        "reel",
        "segmented-control",
        "select-field",
        "sidebar",
        "skeleton",
        "split",
        "stack",
        "status-indicator",
        "switch",
        "tabs",
        "tag",
        "toggle",
    ];
    #[test]
    fn describes_all_ready_contracts() {
        for n in NAMES {
            assert_eq!(describe_properties(n).unwrap().name, *n);
        }
    }
    #[test]
    fn rejects_unknown_and_unknown_fields() {
        assert!(describe_properties("nope").is_err());
        assert!(validate_properties(
            "progress",
            json!({"label":"x","state":"determinate","value":3,"maximum":2})
        )
        .is_err());
        assert!(validate_properties(
            "progress",
            json!({"label":"x","state":"determinate","value":1,"maximum":2,"wat":1})
        )
        .is_err());
    }
    #[test]
    fn every_component_has_a_typed_valid_example() {
        let icons: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        for name in NAMES {
            let sample = if matches!(
                *name,
                "popover" | "tooltip" | "toast" | "theme-switcher" | "command-menu"
            ) {
                example_properties_with_icons(name, &icons)
            } else {
                example_properties(name)
            }
            .unwrap_or_else(|e| panic!("{name}: {e}"));
            let described: std::collections::BTreeSet<_> = describe_properties(name)
                .unwrap()
                .fields
                .into_iter()
                .map(|field| field.name)
                .collect();
            let actual: std::collections::BTreeSet<_> = sample
                .as_object()
                .unwrap()
                .keys()
                .filter(|key| key.as_str() != "id" && key.as_str() != "component")
                .cloned()
                .collect();
            assert_eq!(
                described, actual,
                "{name}: descriptors must cover the existing serialized contract, not a parallel field list"
            );
        }
    }
    #[test]
    fn draft_port_metadata_uses_typed_context_and_fixed_editor_ids() {
        let icons: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        let command = json!({"title":"Commands","triggerLabel":"Open commands","fallbackHref":"/search","groups":[{"id":"main","label":"Main","items":[{"id":"home","href":"/","label":"Home"}]}]});
        assert!(validate_properties("command-menu", command.clone()).is_err());
        assert!(validate_properties_with_icons("command-menu", command, &icons).is_ok());
        for name in [
            "date-calendar",
            "date-picker",
            "command-menu",
            "confirm-dialog",
            "data-viewport",
            "file-upload",
        ] {
            let mut sample = example_properties_with_icons(name, &icons).unwrap();
            sample["id"] = json!("caller-id");
            assert!(
                validate_properties_with_icons(name, sample, &icons).is_err(),
                "{name}"
            );
        }
        let mut calendar = example_properties("date-calendar").unwrap();
        calendar["firstDay"] = json!(7);
        assert!(validate_properties("date-calendar", calendar).is_err());
        let mut confirm = example_properties("confirm-dialog").unwrap();
        confirm["submitName"] = json!("intent");
        assert!(validate_properties("confirm-dialog", confirm).is_err());
    }
    #[test]
    fn descriptor_enum_choices_and_attributes_follow_contracts() {
        let button = describe_properties("button").unwrap();
        let label = button.fields.iter().find(|f| f.name == "label").unwrap();
        assert_eq!(label.kind, PropertyKind::Object);
        assert_eq!(
            label
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["kind", "value"]
        );
        assert_eq!(
            button
                .fields
                .iter()
                .find(|f| f.name == "leadingIcon")
                .unwrap()
                .attribute,
            "icon"
        );
        assert_eq!(
            describe_properties("filter-bar")
                .unwrap()
                .fields
                .iter()
                .find(|f| f.name == "resultSummary")
                .unwrap()
                .attribute,
            "summary"
        );
        let input = describe_properties("form-field")
            .unwrap()
            .fields
            .into_iter()
            .find(|f| f.name == "inputType")
            .unwrap();
        assert_eq!(
            input.choices,
            ["text", "email", "url", "number", "password", "search", "date", "tel"]
        );
        let tag = describe_properties("tag").unwrap();
        assert!(tag
            .fields
            .iter()
            .find(|f| f.name == "tone")
            .unwrap()
            .choices
            .contains(&"brand".into()));
        assert!(tag
            .fields
            .iter()
            .find(|f| f.name == "size")
            .unwrap()
            .choices
            .contains(&"medium".into()));
    }
    #[test]
    fn editor_validation_blocks_binding_paths_and_fixed_infrastructure() {
        assert!(validate_properties(
            "button",
            json!({"label":{"kind":"field","path":"user.name"},"destination":{"kind":"action"}})
        )
        .is_err());
        assert!(validate_properties("button", json!({"label":{"kind":"literal","value":"Go"},"destination":{"kind":"link","href":{"kind":"route","name":"admin"}}})).is_err());
        assert!(validate_properties("button", json!({"id":"caller-id","label":{"kind":"literal","value":"Go"},"destination":{"kind":"action"}})).is_err());
        assert!(validate_properties(
            "form-field",
            json!({"id":"caller-id","name":"x","label":"X"})
        )
        .is_err());
        assert!(validate_properties("tag", json!({"label":"Ready","tone":"running"})).is_err());
    }
    #[test]
    fn native_control_ids_are_injected_and_never_user_supplied() {
        for name in ["checkbox-group", "radio-group", "toggle", "copy-field"] {
            let sample = example_properties(name).unwrap();
            assert_eq!(sample["id"], format!("editor-{name}"));
            let mut supplied = sample.clone();
            supplied.as_object_mut().unwrap().remove("id");
            supplied["id"] = json!("caller-owned");
            assert!(validate_properties(name, supplied).is_err(), "{name}");
        }
    }

    #[test]
    fn progress_bounds_and_visibility_are_described() {
        let d = describe_properties("progress").unwrap();
        let v = d.fields.iter().find(|f| f.name == "value").unwrap();
        assert_eq!(v.minimum, Some(0.0));
        assert_eq!(
            v.visible_when.as_ref().unwrap().equals,
            json!("determinate")
        );
    }
    #[test]
    fn descriptor_enum_defaults_match_serde_defaults() {
        for name in NAMES {
            for field in describe_properties(name).unwrap().fields {
                if let Some(default) = typed_enum_default(name, &field.name) {
                    assert_eq!(field.default, default, "{name}.{}", field.name);
                }
            }
        }
    }
    #[test]
    fn descriptor_defaults_match_typed_layout_configs() {
        for name in [
            "container",
            "stack",
            "cluster",
            "grid",
            "split",
            "cover",
            "layer",
            "pane",
            "sidebar",
            "switch",
            "card",
        ] {
            let normalized = validate_properties(name, json!({})).unwrap();
            for field in describe_properties(name).unwrap().fields {
                if !field.required && !field.default.is_null() {
                    assert_eq!(
                        normalized.get(&field.name),
                        Some(&field.default),
                        "{name}.{}",
                        field.name
                    );
                }
            }
        }
    }
}
