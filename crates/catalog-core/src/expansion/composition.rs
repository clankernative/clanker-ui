//! Bounded, build-time child composition. Child markup is app-authored source,
//! not page data; ordinary template/form/resource admission still runs afterwards.
use super::*;

const MAX_DEPTH: usize = 16;
pub(super) const NAMES: [&str; 45] = [
    "button",
    "icon",
    "badge",
    "divider",
    "status-indicator",
    "form-field",
    "tag",
    "alert",
    "progress",
    "avatar",
    "empty-state",
    "metric",
    "skeleton",
    "page-header",
    "container",
    "stack",
    "cluster",
    "grid",
    "split",
    "card",
    "cover",
    "layer",
    "pane",
    "reel",
    "sidebar",
    "switch",
    "select-field",
    "filter-bar",
    "data-table",
    "breadcrumbs",
    "pagination",
    "activity-feed",
    "definition-list",
    "disclosure",
    "button-group",
    "tabs",
    "segmented-control",
    "progress-steps",
    "checkbox-group",
    "radio-group",
    "toggle",
    "copy-field",
    "theme-switcher",
    "tooltip",
    "toast",
];
const VOID: &[&str] = &["area", "br", "col", "hr", "img", "input", "wbr"];

#[derive(Default)]
pub(super) struct Children {
    pub(super) body: String,
    pub(super) slots: BTreeMap<String, String>,
    pub(super) definitions: Vec<String>,
}

pub(super) fn expand<const N: usize>(
    input: &str,
    package: Option<&Package>,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
) -> Result<String> {
    if !input.to_ascii_lowercase().contains("<cui-")
        && !input.to_ascii_lowercase().contains("</cui-")
    {
        return Ok(input.to_owned());
    }
    let mut cursor = 0;
    let result = region(
        input,
        &mut cursor,
        None,
        false,
        0,
        package,
        counts,
        bindings,
    )?;
    ensure!(
        result.slots.is_empty(),
        "cui-slot requires a composition parent"
    );
    Ok(result.body)
}

pub(super) fn tag_end(input: &str, start: usize) -> Result<usize> {
    let mut quote = None;
    for (offset, byte) in input.as_bytes()[start..].iter().copied().enumerate() {
        if let Some(current) = quote {
            if byte == current {
                quote = None;
            }
        } else if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
        } else if byte == b'>' {
            return Ok(start + offset + 1);
        }
    }
    anyhow::bail!("unterminated HTML tag in Native UI composition")
}

// This recursive parser threads shared cursor, limits, package context, counts, and bindings.
#[allow(clippy::too_many_arguments)]
fn region<const N: usize>(
    input: &str,
    cursor: &mut usize,
    closing: Option<&str>,
    slots_allowed: bool,
    depth: usize,
    package: Option<&Package>,
    counts: &mut [usize; N],
    bindings: &mut Vec<Binding>,
) -> Result<Children> {
    ensure!(
        depth <= MAX_DEPTH,
        "Native UI composition nesting exceeds {MAX_DEPTH}"
    );
    let mut result = Children::default();
    let mut html_stack: Vec<String> = Vec::new();
    while *cursor < input.len() {
        let Some(relative) = input[*cursor..].find('<') else {
            result.body.push_str(&input[*cursor..]);
            *cursor = input.len();
            break;
        };
        let start = *cursor + relative;
        result.body.push_str(&input[*cursor..start]);
        if input[start..].starts_with("<!--") {
            let end = input[start + 4..]
                .find("-->")
                .context("unterminated HTML comment")?
                + start
                + 7;
            result.body.push_str(&input[start..end]);
            *cursor = end;
            continue;
        }
        let is_end = input[start..].starts_with("</");
        let name_start = start + if is_end { 2 } else { 1 };
        let mut name_end = name_start;
        while input
            .as_bytes()
            .get(name_end)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
        {
            name_end += 1;
        }
        if name_end == name_start {
            result.body.push('<');
            *cursor = start + 1;
            continue;
        }
        let original_name = &input[name_start..name_end];
        let name = original_name.to_ascii_lowercase();
        if name.starts_with("cui-") {
            ensure!(
                original_name == name,
                "Native UI declaration names must be lowercase"
            );
            if is_end {
                ensure!(
                    closing == Some(name.as_str()),
                    "mismatched Native UI closing declaration: {name}"
                );
                ensure!(
                    html_stack.is_empty(),
                    "HTML children cannot cross Native UI composition boundaries"
                );
                let end = tag_end(input, start)?;
                ensure!(
                    input[name_end..end - 1].trim().is_empty(),
                    "malformed Native UI closing declaration"
                );
                *cursor = end;
                return Ok(result);
            }
            let tag = format!("<{name}");
            let (end, declaration, self_closing) = parse_opening(input, start, &tag)?;
            *cursor = end;
            if name == "cui-definition" {
                ensure!(
                    slots_allowed && html_stack.is_empty(),
                    "cui-definition must be a direct child of cui-definition-list"
                );
                checked_attributes(&declaration, &["term"], "definition")?;
                let term = declaration
                    .attrs
                    .get("term")
                    .context("cui-definition requires term")?;
                ensure!(
                    !self_closing,
                    "cui-definition requires paired host-admitted value markup"
                );
                let child = region(
                    input,
                    cursor,
                    Some("cui-definition"),
                    false,
                    depth + 1,
                    package,
                    counts,
                    bindings,
                )?;
                ensure!(
                    child.slots.is_empty()
                        && child.definitions.is_empty()
                        && !child.body.trim().is_empty(),
                    "cui-definition requires nonblank host-admitted value markup"
                );
                result
                    .definitions
                    .push(format!("{}\u{0}{}", term, child.body));
                continue;
            }
            let package = package.context("cui declarations require ui/clanker-ui.lock.json")?;
            if name == "cui-slot" {
                ensure!(
                    slots_allowed && html_stack.is_empty(),
                    "cui-slot must be a direct child of a supported composition"
                );
                checked_attributes(&declaration, &["name"], "slot")?;
                let slot = declaration
                    .attrs
                    .get("name")
                    .context("cui-slot requires name")?;
                ensure!(
                    [
                        "actions",
                        "start",
                        "end",
                        "top",
                        "primary",
                        "bottom",
                        "base",
                        "foreground",
                        "header",
                        "footer",
                        "body",
                        "main",
                        "aside",
                    ]
                    .contains(&slot.as_str()),
                    "unsupported composition slot: {slot}"
                );
                ensure!(!self_closing, "cui-slot requires paired child markup");
                let child = region(
                    input,
                    cursor,
                    Some("cui-slot"),
                    false,
                    depth + 1,
                    Some(package),
                    counts,
                    bindings,
                )?;
                ensure!(
                    child.slots.is_empty(),
                    "nested slot declarations are not allowed"
                );
                ensure!(
                    result.slots.insert(slot.clone(), child.body).is_none(),
                    "duplicate composition slot: {slot}"
                );
                continue;
            }
            let component = name.strip_prefix("cui-").expect("checked prefix");
            let index = NAMES
                .iter()
                .position(|candidate| *candidate == component)
                .context("unsupported cui- syntax")?;
            ensure!(index < N, "Native UI component counter is undersized");
            counts[index] += 1;
            let max = if index == 0 {
                MAX_BUTTONS
            } else if index == 1 {
                MAX_ICONS
            } else if index == 5 {
                MAX_FORM_FIELDS
            } else {
                MAX_STATIC
            };
            ensure!(counts[index] <= max, "Native UI {name} count exceeds {max}");
            for (attribute, value) in &declaration.attrs {
                if let Some(field_path) = interpolation(value) {
                    let expected_kind = binding_kind(component, attribute)
                        .context("binding is not supported for this component attribute")?;
                    bindings.push(Binding {
                        field_path: field_path.to_owned(),
                        expected_kind: expected_kind.to_owned(),
                        component: component.to_owned(),
                        attribute: attribute.clone(),
                    });
                }
            }
            let enterprise = (26..=30).contains(&index);
            let ports_component = (31..=37).contains(&index);
            let native_control = (38..=40).contains(&index);
            let composite = index == 13 || index >= 14;

            ensure!(
                self_closing || composite,
                "cui-{component} must be self-closing"
            );
            if ports_component {
                if [31, 35, 36, 37].contains(&index) {
                    ensure!(self_closing, "cui-{component} must be self-closing");
                }
                if [33, 34].contains(&index) {
                    ensure!(
                        !self_closing,
                        "cui-{component} requires paired child markup"
                    );
                }
            }
            let enterprise_body = if enterprise && !self_closing {
                Some(enterprise::take_body(input, cursor, &name)?)
            } else {
                None
            };
            let children = if self_closing || enterprise {
                Children::default()
            } else {
                region(
                    input,
                    cursor,
                    Some(&name),
                    true,
                    depth + 1,
                    Some(package),
                    counts,
                    bindings,
                )?
            };
            ensure!(
                index == 32 || children.definitions.is_empty(),
                "cui-definition is supported only inside cui-definition-list"
            );
            let rendered = if native_control {
                native_controls::render(component, &declaration, package, bindings)?
            } else if matches!(
                component,
                "copy-field" | "theme-switcher" | "tooltip" | "toast"
            ) {
                enhancements::render(component, &declaration, &children, package)?
            } else if ports_component {
                ports::render(index, &declaration, &children, package)?
            } else if enterprise {
                enterprise::render(
                    index,
                    &declaration,
                    enterprise_body.as_deref(),
                    package,
                    counts,
                    bindings,
                )?
            } else {
                match index {
                    0 => render(&declaration, package)?,
                    1 => render_icon(&declaration, package)?,
                    2 => render_badge(&declaration, package)?,
                    3 => render_divider(&declaration, package)?,
                    4 => render_status_indicator(&declaration, package)?,
                    5 => render_form_field(&declaration, package)?,
                    6 => render_tag(&declaration, package)?,
                    7 => render_alert(&declaration, package)?,
                    8 => render_progress(&declaration, package)?,
                    9 => render_avatar(&declaration, package)?,
                    10 => render_empty_state(&declaration, package)?,
                    11 => render_metric(&declaration, package)?,
                    12 => render_skeleton(&declaration, package)?,
                    13 => {
                        ensure!(
                            children.body.trim().is_empty()
                                && children.slots.keys().all(|s| s == "actions"),
                            "page-header accepts only its named actions slot"
                        );
                        render_page_header_actions(
                            &declaration,
                            package,
                            children.slots.get("actions").map(String::as_str),
                        )?
                    }
                    _ => render_layout(component, &declaration, &children, package)?,
                }
            };
            result.body.push_str(&rendered);
        } else {
            let end = tag_end(input, start)?;
            // Inside a wrapper require independently balanced children. This prevents
            // a div child closing the generated div wrapper and silently changing intent.
            if closing.is_some() {
                if is_end {
                    ensure!(
                        html_stack.pop().as_deref() == Some(name.as_str()),
                        "unbalanced HTML children in Native UI composition"
                    );
                } else if !VOID.contains(&name.as_str())
                    && !input[start..end].trim_end().ends_with("/>")
                {
                    html_stack.push(name);
                }
            }
            result.body.push_str(&input[start..end]);
            *cursor = end;
        }
        ensure!(
            result.body.len() <= MAX_FILE,
            "Native UI expanded composition exceeds file budget"
        );
    }
    ensure!(
        closing.is_none(),
        "unterminated Native UI paired declaration"
    );
    Ok(result)
}

fn render_page_header_actions(
    declaration: &Button,
    package: &Package,
    actions: Option<&str>,
) -> Result<String> {
    let header = render_page_header(declaration, package)?;
    if let Some(actions) = actions {
        ensure!(
            !actions.trim().is_empty(),
            "page-header actions slot cannot be blank"
        );
        let end = header
            .rfind("</header>")
            .context("page-header fragment has no closing header")?;
        let mut output = header[..end].to_owned();
        output.push_str(&format!(
            "<div class=\"cui-page-header__actions\">{actions}</div>"
        ));
        output.push_str(&header[end..]);
        Ok(output)
    } else {
        Ok(header)
    }
}

fn choice<'a>(
    declaration: &'a Button,
    key: &str,
    default: &'a str,
    allowed: &[&str],
) -> Result<&'a str> {
    let value = declaration
        .attrs
        .get(key)
        .map(String::as_str)
        .unwrap_or(default);
    ensure!(allowed.contains(&value), "invalid layout {key}");
    Ok(value)
}

fn render_layout(
    name: &str,
    declaration: &Button,
    children: &Children,
    package: &Package,
) -> Result<String> {
    let gap = || {
        choice(
            declaration,
            "gap",
            "standard",
            &["none", "compact", "standard", "spacious", "generous"],
        )
    };
    let alignment = |default| {
        choice(
            declaration,
            "alignment",
            default,
            &["stretch", "start", "center", "end", "baseline"],
        )
    };
    let breakpoint = || {
        choice(
            declaration,
            "collapse-at",
            "standard",
            &["compact", "standard", "wide", "never"],
        )
    };
    // A named body is an alternative to implicit children, never an extra region.
    // Its markup already passed the same balanced-child admission and budgets.
    let mut children = Children {
        body: children.body.clone(),
        slots: children.slots.clone(),
        definitions: children.definitions.clone(),
    };
    if [
        "container",
        "stack",
        "cluster",
        "grid",
        "card",
        "pane",
        "reel",
        "switch",
    ]
    .contains(&name)
    {
        if let Some(body) = children.slots.remove("body") {
            ensure!(
                children.body.trim().is_empty() && !body.trim().is_empty(),
                "cui-{name} requires either implicit content or one nonblank body slot"
            );
            children.body = body;
        }
    }
    let mut fills: Vec<(&str, String)> = vec![("[[body]]", children.body.clone())];
    let class = match name {
        "container" => {
            checked_attributes(declaration, &["width", "gutter"], name)?;
            let width = choice(
                declaration,
                "width",
                "standard",
                &["reading", "compact", "standard", "wide", "full"],
            )?;
            let gutter = choice(
                declaration,
                "gutter",
                "standard",
                &["none", "compact", "standard", "generous"],
            )?;
            format!("cui-container cui-container--{width} cui-container--gutter-{gutter}")
        }
        "stack" => {
            checked_attributes(declaration, &["gap", "alignment"], name)?;
            format!(
                "cui-stack cui-stack--gap-{} cui-stack--align-{}",
                gap()?,
                alignment("stretch")?
            )
        }
        "cluster" => {
            checked_attributes(
                declaration,
                &["gap", "alignment", "justification", "wrapping"],
                name,
            )?;
            let gap = choice(
                declaration,
                "gap",
                "compact",
                &["none", "compact", "standard", "spacious", "generous"],
            )?;
            let justification = choice(
                declaration,
                "justification",
                "start",
                &["start", "center", "end", "between"],
            )?;
            let wrapping = choice(declaration, "wrapping", "wrap", &["wrap", "nowrap"])?;
            format!(
                "cui-cluster cui-cluster--gap-{gap} cui-cluster--align-{} cui-cluster--justify-{justification} cui-cluster--{wrapping}",
                alignment("center")?
            )
        }
        "grid" => {
            checked_attributes(
                declaration,
                &["columns", "minimum", "gap", "alignment", "collapse-at"],
                name,
            )?;
            let columns = choice(
                declaration,
                "columns",
                "responsive",
                &["responsive", "two", "three", "four"],
            )?;
            let minimum = choice(
                declaration,
                "minimum",
                "standard",
                &["narrow", "standard", "wide"],
            )?;
            format!(
                "cui-grid cui-grid--columns-{columns} cui-grid--minimum-{minimum} cui-grid--gap-{} cui-grid--align-{} cui-grid--collapse-{}",
                gap()?,
                alignment("stretch")?,
                breakpoint()?
            )
        }
        "split" => {
            checked_attributes(
                declaration,
                &["ratio", "gap", "alignment", "collapse-at"],
                name,
            )?;
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.len() == 2
                    && children.slots.contains_key("start")
                    && children.slots.contains_key("end"),
                "split requires exactly its named start and end slots"
            );
            let ratio = choice(
                declaration,
                "ratio",
                "equal",
                &[
                    "equal",
                    "start-wide",
                    "end-wide",
                    "start-dominant",
                    "end-dominant",
                ],
            )?;
            fills = vec![
                ("[[start]]", children.slots["start"].clone()),
                ("[[end]]", children.slots["end"].clone()),
            ];
            format!(
                "cui-split cui-split--ratio-{ratio} cui-split--gap-{} cui-split--align-{} cui-split--collapse-{}",
                gap()?,
                alignment("stretch")?,
                breakpoint()?
            )
        }
        "card" => {
            checked_attributes(
                declaration,
                &["title", "subtitle", "padding", "heading-level"],
                name,
            )?;
            let padding = choice(
                declaration,
                "padding",
                "standard",
                &["none", "compact", "standard"],
            )?;
            let heading = choice(declaration, "heading-level", "h2", &["h2", "h3", "h4"])?;
            let title = declaration
                .attrs
                .get("title")
                .map(|text| checked_template_text(text, "card title"))
                .transpose()?
                .map(|text| format!("<{heading} class=\"cui-card__title\">{text}</{heading}>"))
                .unwrap_or_default();
            let subtitle = declaration
                .attrs
                .get("subtitle")
                .map(|text| checked_template_text(text, "card subtitle"))
                .transpose()?
                .map(|text| format!("<p class=\"cui-card__subtitle\">{text}</p>"))
                .unwrap_or_default();
            let header = if title.is_empty() && subtitle.is_empty() {
                String::new()
            } else {
                format!("<header class=\"cui-card__header\">{title}{subtitle}</header>")
            };
            let actions = children
                .slots
                .get("actions")
                .map(|markup| format!("<footer class=\"cui-card__actions\">{markup}</footer>"))
                .unwrap_or_default();
            fills.push(("[[header]]", header));
            fills.push(("[[actions]]", actions));
            format!("cui-card cui-card--{padding}")
        }
        "cover" => {
            checked_attributes(declaration, &["height", "gap"], name)?;
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.contains_key("primary")
                    && !children.slots["primary"].trim().is_empty(),
                "cover requires nonblank primary content in its named slot and no unnamed content"
            );
            ensure!(
                children
                    .slots
                    .keys()
                    .all(|slot| ["top", "primary", "bottom"].contains(&slot.as_str())),
                "unsupported child slot for cui-cover"
            );
            let height = choice(
                declaration,
                "height",
                "standard",
                &["compact", "standard", "fill", "viewport"],
            )?;
            fills = vec![
                (
                    "[[top]]",
                    children
                        .slots
                        .get("top")
                        .map(String::as_str)
                        .unwrap_or("")
                        .to_owned(),
                ),
                ("[[primary]]", children.slots["primary"].clone()),
                (
                    "[[bottom]]",
                    children
                        .slots
                        .get("bottom")
                        .map(String::as_str)
                        .unwrap_or("")
                        .to_owned(),
                ),
            ];
            format!(
                "cui-cover cui-cover--height-{height} cui-cover--gap-{}",
                gap()?
            )
        }
        "layer" => {
            checked_attributes(declaration, &["placement", "inset"], name)?;
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.contains_key("base")
                    && children.slots.contains_key("foreground")
                    && !children.slots["base"].trim().is_empty()
                    && !children.slots["foreground"].trim().is_empty(),
                "layer requires nonblank content in exactly its named base and foreground slots"
            );
            ensure!(
                children.slots.len() == 2,
                "layer accepts only base and foreground slots"
            );
            let placement = choice(
                declaration,
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
            )?;
            let inset = choice(
                declaration,
                "inset",
                "standard",
                &["none", "compact", "standard", "spacious", "generous"],
            )?;
            fills = vec![
                ("[[base]]", children.slots["base"].clone()),
                ("[[foreground]]", children.slots["foreground"].clone()),
            ];
            format!("cui-layer cui-layer--placement-{placement} cui-layer--inset-{inset}")
        }
        "pane" => {
            checked_attributes(declaration, &["height", "body-scrolling"], name)?;
            ensure!(
                !children.body.trim().is_empty(),
                "pane requires nonblank body content"
            );
            ensure!(
                children
                    .slots
                    .keys()
                    .all(|slot| ["header", "footer"].contains(&slot.as_str())),
                "unsupported child slot for cui-pane"
            );
            let height = choice(
                declaration,
                "height",
                "content",
                &["content", "compact", "standard", "fill", "viewport"],
            )?;
            let scrolling = match declaration.attrs.get("body-scrolling").map(String::as_str) {
                None => height != "content",
                Some("true") => true,
                Some("false") => false,
                Some(_) => anyhow::bail!("pane body-scrolling must be true or false"),
            };
            let body = if scrolling {
                format!(
                    "<div class=\"cui-pane__body\" tabindex=\"0\">{}</div>",
                    children.body
                )
            } else {
                format!("<div class=\"cui-pane__body\">{}</div>", children.body)
            };
            let header = children
                .slots
                .get("header")
                .map(|markup| format!("<div class=\"cui-pane__header\">{markup}</div>"))
                .unwrap_or_default();
            let footer = children
                .slots
                .get("footer")
                .map(|markup| format!("<div class=\"cui-pane__footer\">{markup}</div>"))
                .unwrap_or_default();
            fills = vec![
                ("[[header]]", header),
                ("[[body]]", body),
                ("[[footer]]", footer),
            ];
            let mut class = format!("cui-pane cui-pane--height-{height}");
            if height != "content" {
                class.push_str(" cui-pane--bounded");
            }
            if scrolling {
                class.push_str(" cui-pane--scroll");
            }
            if children.slots.contains_key("header") {
                class.push_str(" cui-pane--has-header");
            }
            if children.slots.contains_key("footer") {
                class.push_str(" cui-pane--has-footer");
            }
            class
        }
        "reel" => {
            checked_attributes(declaration, &["label", "item-width", "snap", "gap"], name)?;
            ensure!(
                children.slots.is_empty(),
                "reel accepts no named child slots"
            );
            let label = declaration
                .attrs
                .get("label")
                .context("reel requires label")?;
            ensure!(
                interpolation(label).is_none(),
                "reel label must be a literal, not a page binding"
            );
            let label = checked_template_text(label, "reel label")?;
            let width = choice(
                declaration,
                "item-width",
                "standard",
                &["narrow", "standard", "wide", "content"],
            )?;
            let snap = choice(declaration, "snap", "free", &["free", "start", "center"])?;
            fills = vec![("[[label]]", label), ("[[body]]", children.body.clone())];
            format!(
                "cui-reel cui-reel--width-{width} cui-reel--snap-{snap} cui-reel--gap-{}",
                gap()?
            )
        }
        "sidebar" => {
            checked_attributes(
                declaration,
                &["side", "width", "gap", "alignment", "collapse-at"],
                name,
            )?;
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.contains_key("main")
                    && children.slots.contains_key("aside")
                    && children.slots.len() == 2
                    && !children.slots["main"].trim().is_empty()
                    && !children.slots["aside"].trim().is_empty(),
                "sidebar requires nonblank content in exactly its named main and aside slots"
            );
            let side = choice(declaration, "side", "end", &["start", "end"])?;
            let width = choice(
                declaration,
                "width",
                "standard",
                &["narrow", "standard", "wide"],
            )?;
            let main = format!(
                "<div class=\"cui-sidebar__main\">{}</div>",
                children.slots["main"]
            );
            let aside = format!(
                "<div class=\"cui-sidebar__aside\">{}</div>",
                children.slots["aside"]
            );
            let regions = if side == "start" {
                format!("{aside}{main}")
            } else {
                format!("{main}{aside}")
            };
            fills = vec![("[[regions]]", regions)];
            format!(
                "cui-sidebar cui-sidebar--side-{side} cui-sidebar--width-{width} cui-sidebar--gap-{} cui-sidebar--align-{} cui-sidebar--collapse-{}",
                gap()?,
                alignment("start")?,
                breakpoint()?
            )
        }
        "switch" => {
            checked_attributes(
                declaration,
                &["sizing", "gap", "alignment", "justification", "collapse-at"],
                name,
            )?;
            ensure!(
                children.slots.is_empty(),
                "switch accepts no named child slots"
            );
            let sizing = choice(declaration, "sizing", "natural", &["natural", "equal"])?;
            let justification = choice(
                declaration,
                "justification",
                "start",
                &["start", "center", "end", "between"],
            )?;
            fills = vec![("[[body]]", children.body.clone())];
            format!(
                "cui-switch cui-switch--sizing-{sizing} cui-switch--gap-{} cui-switch--align-{} cui-switch--justify-{justification} cui-switch--collapse-{}",
                gap()?,
                alignment("stretch")?,
                breakpoint()?
            )
        }
        _ => anyhow::bail!("unsupported layout component"),
    };
    if !["split", "cover", "layer", "pane", "sidebar"].contains(&name) {
        ensure!(
            children
                .slots
                .keys()
                .all(|key| name == "card" && key == "actions"),
            "unsupported child slot for cui-{name}"
        );
    }
    fills.push(("[[class]]", class));
    let borrowed = fills
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect::<Vec<_>>();
    fill_slots(static_fragment(package, name)?, &borrowed)
}
