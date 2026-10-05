//! Typed, build-time admissions for the seven locked pure-presentation ports.
use super::composition::Children;
use super::*;
use serde::Deserialize;

const VOID: &[&str] = &["area", "br", "col", "hr", "img", "input", "wbr"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivityFeed {
    label: String,
    entries: Vec<ActivityEntry>,
    #[serde(default)]
    compact: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActivityEntry {
    title: String,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    actor: Option<String>,
    occurred_at: String,
    time_label: String,
    #[serde(default)]
    tone: Tone,
    #[serde(default)]
    href: Option<String>,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionList {
    items: Vec<DefinitionItem>,
    #[serde(default)]
    density: Density,
    #[serde(default = "yes")]
    dividers: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionItem {
    term: String,
    value: String,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Density {
    #[default]
    Standard,
    Compact,
}
fn yes() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Disclosure {
    summary: String,
    #[serde(default)]
    appearance: DisclosureAppearance,
    #[serde(default)]
    open: bool,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DisclosureAppearance {
    Plain,
    #[default]
    Contained,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ButtonGroup {
    label: String,
    #[serde(default)]
    appearance: GroupAppearance,
    #[serde(default)]
    full_width: bool,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum GroupAppearance {
    #[default]
    Joined,
    Spaced,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Tabs {
    #[serde(default = "tabs_label")]
    label: String,
    items: Vec<TabItem>,
}
fn tabs_label() -> String {
    "Sections".into()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TabItem {
    label: String,
    href: String,
    #[serde(default)]
    active: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Segmented {
    #[serde(default = "segments_label")]
    label: String,
    #[serde(default)]
    equal_width: bool,
    items: Vec<SegmentItem>,
}
fn segments_label() -> String {
    "Options".into()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SegmentItem {
    kind: SegmentKind,
    label: String,
    #[serde(default)]
    href: Option<String>,
}
#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum SegmentKind {
    Link,
    Current,
    Disabled,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProgressSteps {
    #[serde(default = "progress_label")]
    label: String,
    #[serde(default)]
    orientation: Orientation,
    #[serde(default)]
    appearance: ProgressAppearance,
    #[serde(default)]
    state_labels: StateLabels,
    items: Vec<ProgressStep>,
}
fn progress_label() -> String {
    "Progress".into()
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ProgressAppearance {
    #[default]
    Detailed,
    Compact,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StateLabels {
    #[serde(default = "completed")]
    complete: String,
    #[serde(default = "current")]
    current: String,
    #[serde(default = "attention")]
    error: String,
    #[serde(default = "upcoming")]
    upcoming: String,
}
fn completed() -> String {
    "Completed".into()
}
fn current() -> String {
    "Current".into()
}
fn attention() -> String {
    "Needs attention".into()
}
fn upcoming() -> String {
    "Upcoming".into()
}
impl Default for StateLabels {
    fn default() -> Self {
        Self {
            complete: completed(),
            current: current(),
            error: attention(),
            upcoming: upcoming(),
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProgressStep {
    state: StepState,
    label: String,
    #[serde(default)]
    href: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}
#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum StepState {
    Complete,
    Current,
    Error,
    Upcoming,
}

fn parse<T: for<'de> Deserialize<'de>>(value: &str, what: &str) -> Result<T> {
    serde_json::from_str(value).with_context(|| format!("invalid {what} JSON"))
}
fn plain(value: &str) -> bool {
    !value.trim().is_empty() && text(value)
}
fn text(value: &str) -> bool {
    !value.chars().any(char::is_control) && !value.contains("{{") && !value.contains("}}")
}
fn checked_text(value: &str, what: &str) -> Result<String> {
    ensure!(plain(value), "{what} must be nonblank plain text");
    Ok(escape(value))
}
fn fragment<'a>(package: &'a Package, name: &str, expected: &[&str]) -> Result<&'a str> {
    let value = static_fragment(package, name)?;
    for slot in expected {
        ensure!(
            value.matches(slot).count() == 1,
            "cui-{name} fragment must contain exactly one {slot}"
        );
    }
    ensure!(
        value.matches("[[").count() == expected.len(),
        "cui-{name} fragment has unsupported slots"
    );
    Ok(value)
}
fn fill_nav(package: &Package, name: &str, label: &str, items: &str) -> Result<String> {
    let f = fragment(package, name, &["[[attributes]]", "[[label]]", "[[items]]"])?;
    let a = format!("class=\"cui-{name}\" data-cui-component=\"{name}\"");
    fill_slots(
        f,
        &[
            ("[[attributes]]", &a),
            ("[[label]]", label),
            ("[[items]]", items),
        ],
    )
}
fn valid_rfc3339(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20
        || !b.is_ascii()
        || !(0..4).all(|i| b[i].is_ascii_digit())
        || b.get(4) != Some(&b'-')
        || !(5..7).all(|i| b[i].is_ascii_digit())
        || b.get(7) != Some(&b'-')
        || !(8..10).all(|i| b[i].is_ascii_digit())
        || b.get(10) != Some(&b'T')
        || !(11..13).all(|i| b[i].is_ascii_digit())
        || b.get(13) != Some(&b':')
        || !(14..16).all(|i| b[i].is_ascii_digit())
        || b.get(16) != Some(&b':')
        || !(17..19).all(|i| b[i].is_ascii_digit())
    {
        return false;
    }
    let num = |a: usize, z: usize| {
        std::str::from_utf8(&b[a..z])
            .ok()
            .and_then(|x| x.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    };
    let (y, m, d, h, mi, se) = (
        num(0, 4),
        num(5, 7),
        num(8, 10),
        num(11, 13),
        num(14, 16),
        num(17, 19),
    );
    if !(1..=12).contains(&m) || h > 23 || mi > 59 || se > 60 {
        return false;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if d == 0 || d > days {
        return false;
    }
    let mut p = 19;
    if b.get(p) == Some(&b'.') {
        p += 1;
        let begin = p;
        while b.get(p).is_some_and(u8::is_ascii_digit) {
            p += 1;
        }
        if p == begin {
            return false;
        }
    }
    match b.get(p) {
        Some(b'Z') => p + 1 == b.len(),
        Some(b'+' | b'-') if p + 6 == b.len() => {
            b[p + 3] == b':'
                && num(p + 1, p + 3) <= 23
                && num(p + 4, p + 6) <= 59
                && b[p + 1..p + 3].iter().all(u8::is_ascii_digit)
                && b[p + 4..p + 6].iter().all(u8::is_ascii_digit)
        }
        _ => false,
    }
}
fn tone(t: Tone) -> &'static str {
    match t {
        Tone::Neutral => "neutral",
        Tone::Info => "info",
        Tone::Success => "success",
        Tone::Warning => "warning",
        Tone::Danger => "danger",
    }
}
fn activity(value: &str, package: &Package) -> Result<String> {
    let i: ActivityFeed = parse(value, "activity-feed")?;
    ensure!(
        plain(&i.label),
        "activity feed label must be nonblank plain text"
    );
    ensure!(
        (1..=100).contains(&i.entries.len()),
        "activity feed requires 1..=100 entries"
    );
    let mut entries = String::new();
    for e in i.entries {
        let title = checked_text(&e.title, "activity title")?;
        let time = checked_text(&e.time_label, "activity time label")?;
        for v in [&e.detail, &e.actor].into_iter().flatten() {
            ensure!(
                plain(v),
                "activity text fields must be nonblank plain text when present"
            );
        }
        ensure!(
            valid_rfc3339(&e.occurred_at),
            "occurredAt must be a real RFC3339 timestamp"
        );
        let title = if let Some(h) = e.href {
            format!(
                "<a class=\"cui-activity-feed__link\" href=\"{}\">{title}</a>",
                enterprise::destination(&h)?
            )
        } else {
            title
        };
        entries.push_str(&format!("<li class=\"cui-activity-feed__item\"><span class=\"cui-activity-feed__marker cui-activity-feed__tone--{}\" aria-hidden=\"true\"></span><div class=\"cui-activity-feed__content\"><div class=\"cui-activity-feed__title\">{title}</div>{}<div class=\"cui-activity-feed__meta\">{}<time class=\"cui-activity-feed__time\" datetime=\"{}\">{time}</time></div></div></li>",tone(e.tone),e.detail.map(|v|format!("<p class=\"cui-activity-feed__detail\">{}</p>",escape(&v))).unwrap_or_default(),e.actor.map(|v|format!("<span class=\"cui-activity-feed__actor\">{}</span>",escape(&v))).unwrap_or_default(),escape(&e.occurred_at)));
    }
    let f = fragment(
        package,
        "activity-feed",
        &["[[attributes]]", "[[label]]", "[[entries]]"],
    )?;
    let a = format!(
        "class=\"cui-activity-feed{}\" data-cui-component=\"activity-feed\"",
        if i.compact {
            " cui-activity-feed--compact"
        } else {
            ""
        }
    );
    fill_slots(
        f,
        &[
            ("[[attributes]]", &a),
            ("[[label]]", &escape(&i.label)),
            ("[[entries]]", &entries),
        ],
    )
}
fn definition_list(value: &str, children: &Children, package: &Package) -> Result<String> {
    let i: DefinitionList = parse(value, "definition-list")?;
    ensure!(
        (1..=100).contains(&i.items.len()),
        "definition list requires 1..=100 items"
    );
    ensure!(
        children.body.trim().is_empty(),
        "definition-list accepts only cui-definition children"
    );
    ensure!(
        children.definitions.is_empty() || children.definitions.len() == i.items.len(),
        "definition values must match item count"
    );
    let mut rich = BTreeMap::new();
    for entry in &children.definitions {
        let (term, value) = entry
            .split_once('\0')
            .context("malformed cui-definition helper")?;
        ensure!(
            rich.insert(term, value).is_none(),
            "duplicate cui-definition term"
        );
    }
    let mut items = String::new();
    for item in i.items {
        ensure!(
            plain(&item.term) && text(&item.value),
            "definition terms must be nonblank and all text must be control-free"
        );
        let val = if let Some(value) = rich.remove(item.term.as_str()) {
            value.to_owned()
        } else {
            ensure!(
                children.definitions.is_empty(),
                "cui-definition terms must match items exactly"
            );
            escape(&item.value)
        };
        items.push_str(&format!("<div class=\"cui-definition-list__item\"><dt class=\"cui-definition-list__term\">{}</dt><dd class=\"cui-definition-list__value\">{val}</dd></div>",escape(&item.term)));
    }
    ensure!(
        rich.is_empty(),
        "cui-definition term does not match an item"
    );
    let attrs = format!(
        "class=\"cui-definition-list cui-definition-list--{}{}\" data-cui-component=\"definition-list\"",
        if matches!(i.density, Density::Compact) {
            "compact"
        } else {
            "standard"
        },
        if i.dividers {
            ""
        } else {
            " cui-definition-list--plain"
        }
    );
    let f = fragment(package, "definition-list", &["[[attributes]]", "[[items]]"])?;
    fill_slots(f, &[("[[attributes]]", &attrs), ("[[items]]", &items)])
}
fn disclosure(value: &str, children: &Children, package: &Package) -> Result<String> {
    let i: Disclosure = parse(value, "disclosure")?;
    ensure!(
        plain(&i.summary),
        "disclosure summary must be nonblank plain text"
    );
    ensure!(
        children.slots.len() == 1
            && children.slots.contains_key("body")
            && children.body.trim().is_empty(),
        "disclosure requires exactly one body slot"
    );
    let a = format!(
        "class=\"cui-disclosure cui-disclosure--{}\" data-cui-component=\"disclosure\"{}",
        if matches!(i.appearance, DisclosureAppearance::Contained) {
            "contained"
        } else {
            "plain"
        },
        if i.open { " open" } else { "" }
    );
    let f = fragment(
        package,
        "disclosure",
        &["[[attributes]]", "[[summary]]", "[[body]]"],
    )?;
    fill_slots(
        f,
        &[
            ("[[attributes]]", &a),
            ("[[summary]]", &escape(&i.summary)),
            ("[[body]]", &children.slots["body"]),
        ],
    )
}
fn button_group(value: &str, children: &Children, package: &Package) -> Result<String> {
    let i: ButtonGroup = parse(value, "button-group")?;
    ensure!(
        plain(&i.label),
        "button group label must be nonblank plain text"
    );
    ensure!(
        children.slots.is_empty() && children.definitions.is_empty(),
        "button-group accepts button children only"
    );
    let n = validate_button_children(&children.body)?;
    ensure!(
        (2..=6).contains(&n),
        "button group requires 2..=6 host-admitted buttons"
    );
    let a = format!(
        "class=\"cui-button-group cui-button-group--{}{}\" data-cui-component=\"button-group\" role=\"group\" aria-label=\"{}\"",
        if matches!(i.appearance, GroupAppearance::Joined) {
            "joined"
        } else {
            "spaced"
        },
        if i.full_width {
            " cui-button-group--full-width"
        } else {
            ""
        },
        escape(&i.label)
    );
    let f = fragment(package, "button-group", &["[[attributes]]", "[[buttons]]"])?;
    fill_slots(
        f,
        &[("[[attributes]]", &a), ("[[buttons]]", &children.body)],
    )
}
fn safe_group_href(value: &str) -> bool {
    safe_href(value)
        || value
            .strip_prefix("{{ routes.")
            .and_then(|route| route.strip_suffix("() }}"))
            .is_some_and(|route| route.split('.').count() == 1 && field_path(route).is_some())
}
fn validate_button_children(body: &str) -> Result<usize> {
    let mut count = 0;
    let mut stack: Vec<String> = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = body[cursor..].find('<') {
        let start = cursor + offset;
        if stack.is_empty() {
            ensure!(
                body[cursor..start].trim().is_empty(),
                "button-group accepts element children only"
            );
        }
        let end = composition::tag_end(body, start)?;
        let tag = &body[start..end];
        if tag.starts_with("<!--") {
            cursor = end;
            continue;
        }
        let close = tag.starts_with("</");
        let mut p = if close { 2 } else { 1 };
        while tag
            .as_bytes()
            .get(p)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
        {
            p += 1;
        }
        let name = &tag[if close { 2 } else { 1 }..p];
        if !close {
            let lower = tag.to_ascii_lowercase();
            ensure!(
                !lower
                    .split_whitespace()
                    .any(|a| a.starts_with("on") && a.contains('=')),
                "event attributes are forbidden in button-group children"
            );
            if let Some(href) = tag
                .as_bytes()
                .windows(5)
                .position(|bytes| bytes.eq_ignore_ascii_case(b"href="))
            {
                let tail = &tag[href + 5..];
                let quote = tail
                    .as_bytes()
                    .first()
                    .copied()
                    .context("malformed child href")?;
                ensure!(
                    quote == b'\"' || quote == 39,
                    "button-group child href must be quoted"
                );
                let end = tail[1..]
                    .find(quote as char)
                    .context("unterminated child href")?
                    + 1;
                ensure!(
                    safe_group_href(&tail[1..end]),
                    "unsafe button-group child href"
                );
            }
        }
        if close {
            ensure!(
                stack.pop().as_deref() == Some(name),
                "malformed button-group child markup"
            );
        } else if !tag.trim_end().ends_with("/>") && !VOID.contains(&name) {
            if stack.is_empty() {
                ensure!(
                    name == "button" || name == "a",
                    "button-group children must be button or link elements"
                );
                count += 1;
            } else {
                ensure!(
                    name != "button" && name != "a",
                    "nested interactive button-group children are forbidden"
                );
            }
            stack.push(name.to_owned());
        } else if stack.is_empty() {
            ensure!(
                name == "button" || name == "a",
                "button-group children must be button or link elements"
            );
            count += 1;
        }
        if stack.is_empty() && tag.trim_end().ends_with("/>") && !VOID.contains(&name) {
            anyhow::bail!("button-group button/link children cannot be self-closing");
        }
        cursor = end;
    }
    ensure!(
        body[cursor..].trim().is_empty(),
        "button-group accepts element children only"
    );
    ensure!(stack.is_empty(), "unclosed button-group child markup");
    Ok(count)
}
fn tabs(value: &str, package: &Package) -> Result<String> {
    let i: Tabs = parse(value, "tabs")?;
    ensure!(
        plain(&i.label),
        "tabs accessible label must be nonblank plain text"
    );
    ensure!(
        (2..=32).contains(&i.items.len()),
        "tabs require 2 to 32 destinations"
    );
    ensure!(
        i.items.iter().filter(|x| x.active).count() == 1,
        "tabs require exactly one active destination"
    );
    let mut list = String::from("<ul class=\"cui-tabs__list\">");
    for x in i.items {
        ensure!(plain(&x.label), "tab labels must be nonblank plain text");
        let destination = enterprise::destination(&x.href)?;
        list.push_str(&format!(
            "<li class=\"cui-tabs__item\"><a class=\"cui-tabs__link{}\" href=\"{}\"{}>{}</a></li>",
            if x.active {
                " cui-tabs__link--active"
            } else {
                ""
            },
            destination,
            if x.active {
                " aria-current=\"page\""
            } else {
                ""
            },
            escape(&x.label)
        ));
    }
    list.push_str("</ul>");
    fill_nav(package, "tabs", &escape(&i.label), &list)
}
fn segmented(value: &str, package: &Package) -> Result<String> {
    let i: Segmented = parse(value, "segmented-control")?;
    ensure!(
        plain(&i.label),
        "segmented control label must be nonblank plain text"
    );
    ensure!(
        (2..=8).contains(&i.items.len()),
        "segmented control requires 2 to 8 items"
    );
    ensure!(
        i.items
            .iter()
            .filter(|x| x.kind == SegmentKind::Current)
            .count()
            == 1,
        "segmented control requires exactly one current item"
    );
    let mut list = String::from("<ul class=\"cui-segmented-control__list\">");
    for x in i.items {
        ensure!(
            plain(&x.label),
            "segment labels must be nonblank plain text"
        );
        let (kind, content) = match x.kind {
            SegmentKind::Link => {
                let h = x.href.as_deref().context("linked segment requires href")?;
                let destination = enterprise::destination(h)?;
                (
                    "link",
                    format!(
                        "<a class=\"cui-segmented-control__link\" href=\"{}\">{}</a>",
                        destination,
                        escape(&x.label)
                    ),
                )
            }
            SegmentKind::Current => {
                ensure!(
                    x.href.is_none(),
                    "current and disabled segments forbid href"
                );
                (
                    "current",
                    format!(
                        "<span class=\"cui-segmented-control__current\" aria-current=\"page\">{}</span>",
                        escape(&x.label)
                    ),
                )
            }
            SegmentKind::Disabled => {
                ensure!(
                    x.href.is_none(),
                    "current and disabled segments forbid href"
                );
                (
                    "disabled",
                    format!(
                        "<span class=\"cui-segmented-control__disabled\" aria-disabled=\"true\">{}</span>",
                        escape(&x.label)
                    ),
                )
            }
        };
        list.push_str(&format!("<li class=\"cui-segmented-control__item cui-segmented-control__item--{kind}\">{content}</li>"));
    }
    list.push_str("</ul>");
    let attr = format!(
        "class=\"cui-segmented-control{}\" data-cui-component=\"segmented-control\"",
        if i.equal_width {
            " cui-segmented-control--equal"
        } else {
            ""
        }
    );
    let f = fragment(
        package,
        "segmented-control",
        &["[[attributes]]", "[[label]]", "[[items]]"],
    )?;
    fill_slots(
        f,
        &[
            ("[[attributes]]", &attr),
            ("[[label]]", &escape(&i.label)),
            ("[[items]]", &list),
        ],
    )
}
fn progress(value: &str, package: &Package) -> Result<String> {
    let i: ProgressSteps = parse(value, "progress-steps")?;
    ensure!(
        plain(&i.label),
        "progress steps label must be nonblank plain text"
    );
    ensure!(
        (2..=10).contains(&i.items.len()),
        "progress steps require 2 to 10 items"
    );
    for l in [
        &i.state_labels.complete,
        &i.state_labels.current,
        &i.state_labels.error,
        &i.state_labels.upcoming,
    ] {
        ensure!(plain(l), "state labels must be nonblank plain text");
    }
    let active: Vec<_> = i
        .items
        .iter()
        .enumerate()
        .filter(|(_, x)| matches!(x.state, StepState::Current | StepState::Error))
        .map(|(n, _)| n)
        .collect();
    ensure!(
        active.len() == 1,
        "progress steps require exactly one current or error step"
    );
    let aidx = active[0];
    for (n, x) in i.items.iter().enumerate() {
        ensure!(
            plain(&x.label) && x.detail.as_deref().is_none_or(plain),
            "step labels and details must be nonblank plain text"
        );
        match x.state {
            StepState::Complete if n < aidx => {
                let h = x.href.as_deref().context("completed steps require href")?;
                enterprise::destination(h)?;
            }
            StepState::Complete => {
                anyhow::bail!("completed steps must precede the current or error step")
            }
            StepState::Current | StepState::Error if n == aidx => {
                ensure!(x.href.is_none(), "current and error steps forbid href")
            }
            StepState::Upcoming if n > aidx => {
                ensure!(x.href.is_none(), "upcoming steps forbid href")
            }
            _ => anyhow::bail!("steps must be ordered complete*, current-or-error, upcoming*"),
        }
    }
    let mut list = String::from("<ol class=\"cui-progress-steps__list\">");
    for (n, x) in i.items.iter().enumerate() {
        let (state, marker) = match x.state {
            StepState::Complete => (
                "complete",
                if package.icons.contains_key("check") {
                    render_package_icon("check", "small", package)?
                } else {
                    (n + 1).to_string()
                },
            ),
            StepState::Current => ("current", (n + 1).to_string()),
            StepState::Error => (
                "error",
                if package.icons.contains_key("alert") {
                    render_package_icon("alert", "small", package)?
                } else {
                    (n + 1).to_string()
                },
            ),
            StepState::Upcoming => ("upcoming", (n + 1).to_string()),
        };
        let marker = format!(
            "<span class=\"cui-progress-steps__marker\" aria-hidden=\"true\">{marker}</span>"
        );
        let (sl, href) = match x.state {
            StepState::Complete => (&i.state_labels.complete, Some(x.href.as_deref().unwrap())),
            StepState::Current => (&i.state_labels.current, None),
            StepState::Error => (&i.state_labels.error, None),
            StepState::Upcoming => (&i.state_labels.upcoming, None),
        };
        let detail = x
            .detail
            .as_deref()
            .map(|v| {
                format!(
                    "<span class=\"cui-progress-steps__detail\">{}</span>",
                    escape(v)
                )
            })
            .unwrap_or_default();
        let content = format!(
            "<span class=\"cui-progress-steps__content\"><span class=\"cui-progress-steps__sr-only\">{}: </span><span class=\"cui-progress-steps__label\">{}</span>{detail}</span>",
            escape(sl),
            escape(&x.label)
        );
        let target = if let Some(h) = href {
            format!(
                "<a class=\"cui-progress-steps__target\" href=\"{}\">{marker}{content}</a>",
                enterprise::destination(h)?
            )
        } else {
            format!(
                "<span class=\"cui-progress-steps__target\"{}>{marker}{content}</span>",
                if matches!(x.state, StepState::Current | StepState::Error) {
                    " aria-current=\"step\""
                } else {
                    ""
                }
            )
        };
        list.push_str(&format!(
            "<li class=\"cui-progress-steps__item cui-progress-steps__item--{state}\">{target}</li>"
        ));
    }
    list.push_str("</ol>");
    let attr = format!(
        "class=\"cui-progress-steps cui-progress-steps--{} cui-progress-steps--{}\" data-cui-component=\"progress-steps\"",
        if matches!(i.orientation, Orientation::Horizontal) {
            "horizontal"
        } else {
            "vertical"
        },
        if matches!(i.appearance, ProgressAppearance::Detailed) {
            "detailed"
        } else {
            "compact"
        }
    );
    let f = fragment(
        package,
        "progress-steps",
        &["[[attributes]]", "[[label]]", "[[items]]"],
    )?;
    fill_slots(
        f,
        &[
            ("[[attributes]]", &attr),
            ("[[label]]", &escape(&i.label)),
            ("[[items]]", &list),
        ],
    )
}

pub(super) fn render(
    index: usize,
    declaration: &Button,
    children: &Children,
    package: &Package,
) -> Result<String> {
    match index {
        31 => {
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.is_empty()
                    && children.definitions.is_empty(),
                "cui-activity-feed does not accept children"
            );
            checked_attributes(
                declaration,
                &["label", "entries", "compact"],
                "activity-feed",
            )?;
            let v = declaration
                .attrs
                .get("entries")
                .context("cui-activity-feed requires entries")?;
            let compact = declaration
                .attrs
                .get("compact")
                .map(String::as_str)
                .unwrap_or("false");
            ensure!(
                compact == "true" || compact == "false",
                "activity-feed compact must be literal true or false"
            );
            let json = format!(
                "{{\"label\":{},\"compact\":{compact},\"entries\":{v}}}",
                serde_json::to_string(
                    declaration
                        .attrs
                        .get("label")
                        .context("cui-activity-feed requires label")?
                )?
            );
            activity(&json, package)
        }
        32 => {
            checked_attributes(
                declaration,
                &["items", "density", "dividers"],
                "definition-list",
            )?;
            let mut json = if let Some(items) = declaration.attrs.get("items") {
                items.clone()
            } else {
                ensure!(
                    !children.definitions.is_empty(),
                    "cui-definition-list requires items or cui-definition children"
                );
                let values = children
                    .definitions
                    .iter()
                    .map(|x| {
                        let (term, _) = x.split_once('\0').unwrap_or(("", ""));
                        serde_json::json!({"term":term,"value":""})
                    })
                    .collect::<Vec<_>>();
                serde_json::to_string(&values)?
            };
            if !json.trim_start().starts_with('{') {
                json = format!("{{\"items\":{json}}}");
            }
            let mut v: serde_json::Value = parse(&json, "definition-list")?;
            if let Some(obj) = v.as_object_mut() {
                if let Some(x) = declaration.attrs.get("density") {
                    obj.insert("density".into(), serde_json::Value::String(x.clone()));
                }
                if let Some(x) = declaration.attrs.get("dividers") {
                    obj.insert(
                        "dividers".into(),
                        serde_json::from_str(x)
                            .context("dividers must be literal true or false")?,
                    );
                }
            }
            definition_list(&serde_json::to_string(&v)?, children, package)
        }
        33 => {
            checked_attributes(
                declaration,
                &["summary", "appearance", "open"],
                "disclosure",
            )?;
            let json = serde_json::json!({"summary":declaration.attrs.get("summary").context("cui-disclosure requires summary")?,"appearance":declaration.attrs.get("appearance").map(String::as_str).unwrap_or("contained"),"open":declaration.attrs.get("open").is_some_and(|x|x=="true")});
            ensure!(
                declaration
                    .attrs
                    .get("open")
                    .is_none_or(|x| x == "true" || x == "false"),
                "disclosure open must be literal true or false"
            );
            disclosure(&json.to_string(), children, package)
        }
        34 => {
            checked_attributes(
                declaration,
                &["label", "appearance", "full-width"],
                "button-group",
            )?;
            let json = serde_json::json!({"label":declaration.attrs.get("label").context("cui-button-group requires label")?,"appearance":declaration.attrs.get("appearance").map(String::as_str).unwrap_or("joined"),"fullWidth":declaration.attrs.get("full-width").is_some_and(|x|x=="true")});
            ensure!(
                declaration
                    .attrs
                    .get("full-width")
                    .is_none_or(|x| x == "true" || x == "false"),
                "button-group full-width must be literal true or false"
            );
            button_group(&json.to_string(), children, package)
        }
        35 => {
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.is_empty()
                    && children.definitions.is_empty(),
                "cui-tabs does not accept children"
            );
            checked_attributes(declaration, &["label", "items"], "tabs")?;
            let json = format!(
                "{{\"label\":{},\"items\":{}}}",
                serde_json::to_string(
                    declaration
                        .attrs
                        .get("label")
                        .context("cui-tabs requires label")?
                )?,
                declaration
                    .attrs
                    .get("items")
                    .context("cui-tabs requires items")?
            );
            tabs(&json, package)
        }
        36 => {
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.is_empty()
                    && children.definitions.is_empty(),
                "cui-segmented-control does not accept children"
            );
            checked_attributes(
                declaration,
                &["label", "equal-width", "items"],
                "segmented-control",
            )?;
            let json = format!(
                "{{\"label\":{},\"equalWidth\":{},\"items\":{}}}",
                serde_json::to_string(
                    declaration
                        .attrs
                        .get("label")
                        .context("cui-segmented-control requires label")?
                )?,
                declaration
                    .attrs
                    .get("equal-width")
                    .map(String::as_str)
                    .unwrap_or("false"),
                declaration
                    .attrs
                    .get("items")
                    .context("cui-segmented-control requires items")?
            );
            ensure!(
                declaration
                    .attrs
                    .get("equal-width")
                    .is_none_or(|x| x == "true" || x == "false"),
                "equal-width must be literal true or false"
            );
            segmented(&json, package)
        }
        37 => {
            ensure!(
                children.body.trim().is_empty()
                    && children.slots.is_empty()
                    && children.definitions.is_empty(),
                "cui-progress-steps does not accept children"
            );
            checked_attributes(
                declaration,
                &[
                    "label",
                    "orientation",
                    "appearance",
                    "state-labels",
                    "items",
                ],
                "progress-steps",
            )?;
            let labels = declaration
                .attrs
                .get("state-labels")
                .map(|x| format!("\"stateLabels\":{x},"))
                .unwrap_or_default();
            let json = format!(
                "{{\"label\":{},\"orientation\":{},\"appearance\":{},{}\"items\":{}}}",
                serde_json::to_string(
                    declaration
                        .attrs
                        .get("label")
                        .context("cui-progress-steps requires label")?
                )?,
                serde_json::to_string(
                    declaration
                        .attrs
                        .get("orientation")
                        .map(String::as_str)
                        .unwrap_or("horizontal")
                )?,
                serde_json::to_string(
                    declaration
                        .attrs
                        .get("appearance")
                        .map(String::as_str)
                        .unwrap_or("detailed")
                )?,
                labels,
                declaration
                    .attrs
                    .get("items")
                    .context("cui-progress-steps requires items")?
            );
            progress(&json, package)
        }
        _ => anyhow::bail!("unsupported Native UI port index"),
    }
}
