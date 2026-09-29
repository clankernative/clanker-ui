//! Typed selection and safe static rendering for operational metrics.

use crate::icon::{self, IconInstance, IconSize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Trend {
    Up,
    Down,
    #[default]
    Flat,
}

impl Trend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Flat => "flat",
        }
    }

    fn icon_name(self) -> &'static str {
        match self {
            Self::Up => "arrow-up",
            Self::Down => "arrow-down",
            Self::Flat => "minus",
        }
    }

    fn announcement(self) -> &'static str {
        match self {
            Self::Up => "Increased",
            Self::Down => "Decreased",
            Self::Flat => "Unchanged",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendTone {
    Positive,
    Negative,
    #[default]
    Neutral,
}

impl TrendTone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }

    fn announcement(self) -> &'static str {
        match self {
            Self::Positive => "favorable change",
            Self::Negative => "unfavorable change",
            Self::Neutral => "neutral change",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Plain,
    Contained,
}

impl Appearance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Contained => "contained",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricInstance {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub trend: Option<Trend>,
    #[serde(default)]
    pub trend_tone: Option<TrendTone>,
    #[serde(default)]
    pub trend_label: Option<String>,
    #[serde(default)]
    pub trend_announcement: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub appearance: Appearance,
}

impl MetricInstance {
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        for (name, value) in [("label", &self.label), ("value", &self.value)] {
            if !valid_text(value) {
                return Err(format!("metric {name} must contain safe, nonblank text"));
            }
        }
        for (name, value) in [
            ("detail", &self.detail),
            ("trendLabel", &self.trend_label),
            ("trendAnnouncement", &self.trend_announcement),
        ] {
            if let Some(value) = value {
                if !valid_text(value) {
                    return Err(format!("metric {name} must contain safe, nonblank text"));
                }
            }
        }
        let trend_fields = [
            self.trend.is_some(),
            self.trend_tone.is_some(),
            self.trend_label.is_some(),
        ];
        if trend_fields.iter().any(|present| *present)
            && !trend_fields.iter().all(|present| *present)
        {
            return Err("metric trend, trendTone, and trendLabel must be supplied together".into());
        }
        if self.trend_announcement.is_some() && self.trend.is_none() {
            return Err("metric trendAnnouncement requires a trend".into());
        }
        if let Some(name) = &self.icon {
            IconInstance {
                name: name.clone(),
                size: IconSize::Small,
                label: None,
            }
            .validate(icons)?;
        }
        Ok(())
    }
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Renders a deterministic static metric into a fragment with one `[[metric]]` slot.
pub fn render(
    instance: &MetricInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;

    let mut content = String::new();
    if let Some(name) = &instance.icon {
        let icon = icon::render(
            &IconInstance {
                name: name.clone(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        content.push_str(&format!(
            "<span class=\"cui-metric__icon\" aria-hidden=\"true\">{icon}</span>"
        ));
    }
    content.push_str(&format!(
        "<dl class=\"cui-metric__content\"><dt class=\"cui-metric__label\">{}</dt><dd class=\"cui-metric__value\">{}</dd></dl>",
        escape_html(&instance.label),
        escape_html(&instance.value)
    ));

    let mut footer = String::new();
    if let (Some(trend), Some(tone), Some(label)) =
        (instance.trend, instance.trend_tone, &instance.trend_label)
    {
        let announcement = instance
            .trend_announcement
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}, {}", trend.announcement(), tone.announcement()));
        let trend_icon = icon::render(
            &IconInstance {
                name: trend.icon_name().into(),
                size: IconSize::Small,
                label: None,
            },
            "<svg [[attributes]]>[[geometry]]</svg>",
            icons,
        )?;
        footer.push_str(&format!(
            "<span class=\"cui-metric__change cui-metric__change--{} cui-metric__change--{}\"><span class=\"cui-visually-hidden\">{}: </span>{trend_icon}<span>{}</span></span>",
            trend.as_str(), tone.as_str(), escape_html(&announcement), escape_html(label)
        ));
    }
    if let Some(detail) = &instance.detail {
        footer.push_str(&format!(
            "<span class=\"cui-metric__detail\">{}</span>",
            escape_html(detail)
        ));
    }
    if !footer.is_empty() {
        content.push_str(&format!("<p class=\"cui-metric__footer\">{footer}</p>"));
    }
    let element = format!(
        "<article class=\"cui-metric cui-metric--{}\" data-cui-component=\"metric\">{content}</article>",
        instance.appearance.as_str()
    );
    crate::fragment::fill(fragment, &[("[[metric]]", &element)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "[[metric]]";

    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("server".into(), "<path d=\"M1 1\"></path>".into()),
            ("arrow-up".into(), "<path d=\"M2 2\"></path>".into()),
            ("arrow-down".into(), "<path d=\"M3 3\"></path>".into()),
            ("minus".into(), "<path d=\"M4 4\"></path>".into()),
        ])
    }

    fn metric() -> MetricInstance {
        MetricInstance {
            label: "Active users".into(),
            value: "2,420".into(),
            detail: None,
            trend: None,
            trend_tone: None,
            trend_label: None,
            trend_announcement: None,
            icon: None,
            appearance: Appearance::Plain,
        }
    }

    #[test]
    fn renders_all_direction_and_tone_combinations_independently() {
        for trend in [Trend::Up, Trend::Down, Trend::Flat] {
            for tone in [TrendTone::Positive, TrendTone::Negative, TrendTone::Neutral] {
                let instance = MetricInstance {
                    trend: Some(trend),
                    trend_tone: Some(tone),
                    trend_label: Some("8.2%".into()),
                    ..metric()
                };
                let html = render(&instance, FRAGMENT, &icons()).unwrap();
                assert!(html.contains(&format!("cui-metric__change--{}", trend.as_str())));
                assert!(html.contains(&format!("cui-metric__change--{}", tone.as_str())));
                assert!(html.contains("8.2%"));
                assert!(!html.contains("aria-live"));
            }
        }
    }

    #[test]
    fn renders_optional_icon_detail_override_and_formatted_value_safely() {
        let instance = MetricInstance {
            label: "Revenue <net> & [[metric]]".into(),
            value: "$12 & rising".into(),
            detail: Some("vs. <week>".into()),
            trend: Some(Trend::Down),
            trend_tone: Some(TrendTone::Negative),
            trend_label: Some("3% <week>".into()),
            trend_announcement: Some("Fell <unexpectedly>".into()),
            icon: Some("server".into()),
            appearance: Appearance::Contained,
        };
        let html = render(&instance, FRAGMENT, &icons()).unwrap();
        assert!(html.contains("Revenue &lt;net&gt; &amp; [[metric]]"));
        assert!(html.contains("$12 &amp; rising"));
        assert!(html.contains("vs. &lt;week&gt;"));
        assert!(html.contains("Fell &lt;unexpectedly&gt;: "));
        assert!(html.contains("3% &lt;week&gt;"));
        assert!(html.contains("data-cui-icon=\"server\""));
        assert!(html.contains("aria-hidden=\"true\""));
        assert!(!html.contains("aria-live"));
    }

    #[test]
    fn rejects_unknown_props_bad_text_incomplete_trends_and_unknown_icons() {
        assert!(serde_json::from_str::<MetricInstance>(
            r#"{"label":"x","value":"1","unknown":true}"#
        )
        .is_err());
        assert!(serde_json::from_str::<MetricInstance>(
            r#"{"label":"x","value":"1","appearance":"compact"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<MetricInstance>(
            r#"{"label":"x","value":"1","trend":"sideways"}"#
        )
        .is_err());
        let mut value = metric();
        value.label = "  ".into();
        assert!(value.validate(&icons()).is_err());
        value = metric();
        value.value = "1\n2".into();
        assert!(value.validate(&icons()).is_err());
        value = metric();
        value.trend = Some(Trend::Up);
        assert!(value.validate(&icons()).is_err());
        value = metric();
        value.trend_announcement = Some("change".into());
        assert!(value.validate(&icons()).is_err());
        value = metric();
        value.icon = Some("missing".into());
        assert!(value.validate(&icons()).is_err());
    }

    #[test]
    fn package_gold_fixtures_match_rendered_output() {
        let package =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
        let icons: BTreeMap<String, String> =
            serde_json::from_str(&std::fs::read_to_string(package.join("icons.json")).unwrap())
                .unwrap();
        let fragment =
            std::fs::read_to_string(package.join("components/metric/fragment.html")).unwrap();
        for file in [
            "plain.json",
            "contained.json",
            "up.json",
            "down.json",
            "flat.json",
            "positive.json",
            "negative.json",
            "neutral.json",
            "icon-detail.json",
        ] {
            let mut fixture: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(package.join("components/metric/fixtures").join(file))
                    .unwrap(),
            )
            .unwrap();
            let expected = fixture["expectedHtml"].as_str().unwrap().to_owned();
            fixture.as_object_mut().unwrap().remove("expectedHtml");
            let instance: MetricInstance = serde_json::from_value(fixture).unwrap();
            assert_eq!(
                render(&instance, &fragment, &icons).unwrap(),
                expected,
                "{file}"
            );
        }
    }

    #[test]
    fn rejects_invalid_fragment_slots() {
        assert!(render(&metric(), "[[metric]][[metric]]", &icons()).is_err());
        assert!(render(&metric(), "[[metric]][[other]]", &icons()).is_err());
        assert!(render(&metric(), "without a slot", &icons()).is_err());
    }
}
