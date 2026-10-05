//! Typed static activity-feed rendering with strict RFC 3339 timestamps.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Danger,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivityFeedInstance {
    pub label: String,
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub compact: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    pub occurred_at: String,
    pub time_label: String,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub href: Option<String>,
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}
/// RFC3339's fixed date/time/offset grammar plus Gregorian calendar validation.
pub fn valid_rfc3339(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || !b.is_ascii() {
        return false;
    }
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    if !(0..4).all(digit)
        || b.get(4) != Some(&b'-')
        || !(5..7).all(digit)
        || b.get(7) != Some(&b'-')
        || !(8..10).all(digit)
        || b.get(10) != Some(&b'T')
        || !(11..13).all(digit)
        || b.get(13) != Some(&b':')
        || !(14..16).all(digit)
        || b.get(16) != Some(&b':')
        || !(17..19).all(digit)
    {
        return false;
    }
    let num = |start: usize, end: usize| {
        std::str::from_utf8(&b[start..end])
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
    if d < 1 || d > days {
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
        Some(b'Z') if p + 1 == b.len() => true,
        Some(b'+' | b'-') if p + 6 == b.len() => {
            b[p + 3] == b':'
                && b[p + 1].is_ascii_digit()
                && b[p + 2].is_ascii_digit()
                && b[p + 4].is_ascii_digit()
                && b[p + 5].is_ascii_digit()
                && num(p + 1, p + 3) <= 23
                && num(p + 4, p + 6) <= 59
        }
        _ => false,
    }
}
impl ActivityFeedInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        v.validate()?;
        Ok(v)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !text(&self.label) {
            return Err("activity feed label must be nonblank plain text".into());
        }
        if !(1..=100).contains(&self.entries.len()) {
            return Err("activity feed requires 1..=100 entries".into());
        }
        for e in &self.entries {
            if !text(&e.title)
                || !text(&e.time_label)
                || e.detail.as_ref().is_some_and(|s| !text(s))
                || e.actor.as_ref().is_some_and(|s| !text(s))
            {
                return Err("activity text fields must be nonblank plain text when present".into());
            }
            if !valid_rfc3339(&e.occurred_at) {
                return Err("occurredAt must be a real RFC3339 timestamp".into());
            }
            if e.href
                .as_ref()
                .is_some_and(|h| !crate::button::safe_href(h))
            {
                return Err("activity href must be a safe HTTP(S) or relative href".into());
            }
        }
        Ok(())
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
pub fn render(i: &ActivityFeedInstance, fragment: &str) -> Result<String, String> {
    i.validate()?;
    let entries = i
        .entries
        .iter()
        .map(|e| {
            let title = if let Some(h) = &e.href {
                format!(
                    "<a class=\"cui-activity-feed__link\" href=\"{}\">{}</a>",
                    esc(h),
                    esc(&e.title)
                )
            } else {
                esc(&e.title)
            };
            format!(
                "<li class=\"cui-activity-feed__item\"><span class=\"cui-activity-feed__marker cui-activity-feed__tone--{}\" aria-hidden=\"true\"></span><div class=\"cui-activity-feed__content\"><div class=\"cui-activity-feed__title\">{}</div>{}<div class=\"cui-activity-feed__meta\">{}<time class=\"cui-activity-feed__time\" datetime=\"{}\">{}</time></div></div></li>",
                tone(e.tone),
                title,
                e.detail
                    .as_ref()
                    .map(|x| format!("<p class=\"cui-activity-feed__detail\">{}</p>", esc(x)))
                    .unwrap_or_default(),
                e.actor
                    .as_ref()
                    .map(|x| format!("<span class=\"cui-activity-feed__actor\">{}</span>", esc(x)))
                    .unwrap_or_default(),
                esc(&e.occurred_at),
                esc(&e.time_label)
            )
        })
        .collect::<String>();
    let attrs = format!(
        "class=\"cui-activity-feed{}\" data-cui-component=\"activity-feed\"",
        if i.compact {
            " cui-activity-feed--compact"
        } else {
            ""
        }
    );
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attrs),
            ("[[label]]", &esc(&i.label)),
            ("[[entries]]", &entries),
        ],
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    const F: &str = "<div [[attributes]]><ol aria-label=\"[[label]]\">[[entries]]</ol></div>";
    fn i() -> ActivityFeedInstance {
        ActivityFeedInstance {
            label: "Releases".into(),
            compact: false,
            entries: vec![Entry {
                title: "Ready <now>".into(),
                detail: Some("Updated & live".into()),
                actor: Some("operator".into()),
                occurred_at: "2024-02-29T12:30:00.12+02:30".into(),
                time_label: "Today".into(),
                tone: Tone::Success,
                href: Some("/release/2".into()),
            }],
        }
    }
    #[test]
    fn renders_and_escapes() {
        let h = render(&i(), F).unwrap();
        assert!(h.contains("datetime=\"2024-02-29T12:30:00.12+02:30\""));
        assert!(h.contains("Ready &lt;now&gt;"));
        assert!(h.contains("href=\"/release/2\""));
        assert!(h.contains("cui-activity-feed__tone--success"));
    }
    #[test]
    fn rejects_bad_dates_zone_controls_unknowns_and_links() {
        for t in [
            "2023-02-29T12:30:00Z",
            "2024-13-01T12:30:00Z",
            "2024-01-01T24:00:00Z",
            "2024-01-01T12:30:00+24:00",
            "2024-01-01 12:30:00Z",
            "2024-01-01T12:30:00.",
        ] {
            assert!(!valid_rfc3339(t), "{t}");
        }
        assert!(valid_rfc3339("2024-02-29T12:30:00.1Z"));
        let mut x = i();
        x.entries.clear();
        assert!(x.validate().is_err());
        x = i();
        let entry = x.entries[0].clone();
        x.entries = (0..101).map(|_| entry.clone()).collect();
        assert!(x.validate().is_err());
        x = i();
        x.entries[0].href = Some("javascript:alert(1)".into());
        assert!(x.validate().is_err());
        x = i();
        x.entries[0].title = "x\n".into();
        assert!(x.validate().is_err());
        assert!(ActivityFeedInstance::parse(r#"{"label":"x","entries":[],"unknown":1}"#).is_err());
        assert!(render(&i(), "[[attributes]]").is_err());
    }
}
