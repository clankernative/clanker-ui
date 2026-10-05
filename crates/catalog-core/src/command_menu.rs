//! Typed static rendering for searchable, URL-backed command destinations.
use crate::{
    button, fragment,
    icon::{self, IconInstance, IconSize},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandMenuInstance {
    pub id: String,
    pub title: String,
    pub trigger_label: String,
    pub fallback_href: String,
    pub groups: Vec<Group>,
    #[serde(default = "default_search_label")]
    pub search_label: String,
    #[serde(default = "default_placeholder")]
    pub search_placeholder: String,
    #[serde(default = "default_empty_text")]
    pub empty_text: String,
    #[serde(default = "default_instructions")]
    pub instructions: String,
    #[serde(default = "default_close_label")]
    pub close_label: String,
    #[serde(default)]
    pub shortcut: Option<GlobalShortcut>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub items: Vec<Item>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub href: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlobalShortcut {
    pub key: String,
    pub label: String,
}
fn default_search_label() -> String {
    "Search commands".into()
}
fn default_placeholder() -> String {
    "Search actions and destinations".into()
}
fn default_empty_text() -> String {
    "No commands match your search.".into()
}
fn default_instructions() -> String {
    "Type to filter. Use arrow keys to move and Enter to open.".into()
}
fn default_close_label() -> String {
    "Close command menu".into()
}

impl CommandMenuInstance {
    pub fn parse(bytes: &[u8], icons: &BTreeMap<String, String>) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        value.validate(icons)?;
        Ok(value)
    }
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("command menu id must be lowercase kebab-case".into());
        }
        for (value, name) in [
            (&self.title, "title"),
            (&self.trigger_label, "triggerLabel"),
            (&self.search_label, "searchLabel"),
            (&self.search_placeholder, "searchPlaceholder"),
            (&self.empty_text, "emptyText"),
            (&self.instructions, "instructions"),
            (&self.close_label, "closeLabel"),
        ] {
            text(value, name)?;
        }
        if !button::safe_href(&self.fallback_href) {
            return Err("command menu fallbackHref must be a safe destination".into());
        }
        if self.groups.is_empty() || self.groups.len() > 8 {
            return Err("command menu requires one to eight groups".into());
        }
        let mut group_ids = BTreeSet::new();
        let mut item_ids = BTreeSet::new();
        let mut count = 0usize;
        for group in &self.groups {
            if !valid_id(&group.id) || !group_ids.insert(&group.id) {
                return Err("command group ids must be valid and unique".into());
            }
            text(&group.label, "group label")?;
            if group.items.is_empty() || group.items.len() > 20 {
                return Err("each command group requires one to twenty items".into());
            }
            for item in &group.items {
                count += 1;
                if !valid_id(&item.id) || !item_ids.insert(&item.id) {
                    return Err("command item ids must be valid and unique across the menu".into());
                }
                if !button::safe_href(&item.href) {
                    return Err("command href must be a safe destination".into());
                }
                text(&item.label, "item label")?;
                if let Some(v) = &item.description {
                    text(v, "item description")?;
                }
                if let Some(v) = &item.shortcut {
                    text(v, "item shortcut")?;
                }
                if item.keywords.len() > 12 {
                    return Err("a command supports at most twelve search keywords".into());
                }
                for keyword in &item.keywords {
                    text(keyword, "search keyword")?;
                }
                if let Some(name) = &item.icon {
                    render_icon(name, IconSize::Medium, icons)?;
                }
            }
        }
        if count > 100 {
            return Err("command menu supports at most one hundred items".into());
        }
        if let Some(shortcut) = &self.shortcut {
            if shortcut.key.len() != 1 || !shortcut.key.as_bytes()[0].is_ascii_alphanumeric() {
                return Err("global shortcut key must be one ASCII letter or digit".into());
            }
            text(&shortcut.label, "shortcut label")?;
        }
        for name in ["command", "search", "close"] {
            render_icon(name, IconSize::Medium, icons)?;
        }
        Ok(())
    }
    pub fn derived_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::from([
            self.id.clone(),
            format!("{}-title", self.id),
            format!("{}-search", self.id),
            format!("{}-results", self.id),
            format!("{}-instructions", self.id),
        ]);
        for group in &self.groups {
            ids.insert(format!("{}-group-{}", self.id, group.id));
            for item in &group.items {
                ids.insert(format!("{}-item-{}", self.id, item.id));
            }
        }
        ids
    }
}
/// Render only typed data and closed-catalog icons. The fallback is an ordinary safe link.
pub fn render(
    instance: &CommandMenuInstance,
    fragment_text: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    let groups=instance.groups.iter().map(|g| {
        let items=g.items.iter().map(|i| {
            let icon= i.icon.as_ref().map(|name| render_icon(name,IconSize::Medium,icons).map(|svg|format!("<span class=\"cui-command-menu__item-icon\" aria-hidden=\"true\">{svg}</span>"))).transpose()?.unwrap_or_default();
            let description=i.description.as_ref().map(|v|format!("<span class=\"cui-command-menu__item-description\">{}</span>",escape(v))).unwrap_or_default();
            let shortcut=i.shortcut.as_ref().map(|v|format!("<kbd class=\"cui-command-menu__item-shortcut\">{}</kbd>",escape(v))).unwrap_or_default();
            let searchable=std::iter::once(i.label.as_str()).chain(i.description.iter().map(String::as_str)).chain(i.keywords.iter().map(String::as_str)).collect::<Vec<_>>().join(" ");
            Ok(format!("<a class=\"cui-command-menu__item\" id=\"{}-item-{}\" href=\"{}\" role=\"option\" aria-selected=\"false\" tabindex=\"-1\" data-cui-command-item data-cui-command-text=\"{}\">{icon}<span class=\"cui-command-menu__item-content\"><span class=\"cui-command-menu__item-label\">{}</span>{description}</span>{shortcut}</a>",escape(&instance.id),escape(&i.id),escape(&i.href),escape(&searchable),escape(&i.label)))
        }).collect::<Result<Vec<_>,String>>()?.join("");
        Ok(format!("<section class=\"cui-command-menu__group\" role=\"group\" aria-labelledby=\"{}-group-{}\" data-cui-command-group><h3 class=\"cui-command-menu__group-label\" id=\"{}-group-{}\">{}</h3><div class=\"cui-command-menu__items\">{items}</div></section>",escape(&instance.id),escape(&g.id),escape(&instance.id),escape(&g.id),escape(&g.label)))
    }).collect::<Result<Vec<_>,String>>()?.join("");
    let command = render_icon("command", IconSize::Medium, icons)?;
    let search = render_icon("search", IconSize::Medium, icons)?;
    let close = render_icon("close", IconSize::Medium, icons)?;
    let shortcut = instance
        .shortcut
        .as_ref()
        .map(|s| format!(" data-cui-command-shortcut=\"{}\"", escape(&s.key)))
        .unwrap_or_default();
    let hint = instance
        .shortcut
        .as_ref()
        .map(|s| {
            format!(
                "<kbd class=\"cui-command-menu__trigger-shortcut\">{}</kbd>",
                escape(&s.label)
            )
        })
        .unwrap_or_default();
    let html = format!(
        "<div class=\"cui-command-menu\" data-cui-component=\"command-menu\"{shortcut}><a class=\"cui-command-menu__trigger cui-button cui-button--secondary\" href=\"{}\" aria-haspopup=\"dialog\" aria-controls=\"{}\" data-cui-command-trigger><span aria-hidden=\"true\">{command}</span><span>{}</span>{hint}</a><dialog class=\"cui-command-menu__dialog\" id=\"{}\" aria-labelledby=\"{}-title\" data-cui-command-dialog><div class=\"cui-command-menu__surface\"><h2 class=\"cui-command-menu__visually-hidden\" id=\"{}-title\">{}</h2><div class=\"cui-command-menu__search\"><label class=\"cui-command-menu__visually-hidden\" for=\"{}-search\">{}</label><span aria-hidden=\"true\">{search}</span><input class=\"cui-command-menu__search-control\" id=\"{}-search\" type=\"search\" placeholder=\"{}\" autocomplete=\"off\" role=\"combobox\" aria-autocomplete=\"list\" aria-expanded=\"true\" aria-controls=\"{}-results\" aria-describedby=\"{}-instructions\" data-cui-command-search><button class=\"cui-command-menu__close\" type=\"button\" aria-label=\"{}\" data-cui-command-close>{close}</button></div><div class=\"cui-command-menu__results\" id=\"{}-results\" role=\"listbox\" aria-label=\"{}\" data-cui-command-results>{groups}<p class=\"cui-command-menu__empty\" role=\"status\" hidden data-cui-command-empty>{}</p></div><p class=\"cui-command-menu__instructions\" id=\"{}-instructions\">{}</p></div></dialog></div>",
        escape(&instance.fallback_href),
        escape(&instance.id),
        escape(&instance.trigger_label),
        escape(&instance.id),
        escape(&instance.id),
        escape(&instance.id),
        escape(&instance.title),
        escape(&instance.id),
        escape(&instance.search_label),
        escape(&instance.id),
        escape(&instance.search_placeholder),
        escape(&instance.id),
        escape(&instance.id),
        escape(&instance.close_label),
        escape(&instance.id),
        escape(&instance.title),
        escape(&instance.empty_text),
        escape(&instance.id),
        escape(&instance.instructions)
    );
    fragment::fill(fragment_text, &[("[[command_menu]]", &html)])
}
fn render_icon(
    name: &str,
    size: IconSize,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    icon::render(
        &IconInstance {
            name: name.into(),
            size,
            label: None,
        },
        "<svg [[attributes]]>[[geometry]]</svg>",
        icons,
    )
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('-')
        && !v.ends_with('-')
        && v.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn text(v: &str, n: &str) -> Result<(), String> {
    if v.trim().is_empty() || v.chars().any(char::is_control) {
        Err(format!("command menu {n} must contain safe, nonblank text"))
    } else {
        Ok(())
    }
}
fn escape(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn icons() -> BTreeMap<String, String> {
        ["command", "search", "close", "plus"]
            .into_iter()
            .map(|x| (x.into(), "<path d=\"M1 1\"></path>".into()))
            .collect()
    }
    fn sample() -> CommandMenuInstance {
        CommandMenuInstance {
            id: "app-commands".into(),
            title: "Commands".into(),
            trigger_label: "Open commands".into(),
            fallback_href: "/search".into(),
            groups: vec![Group {
                id: "actions".into(),
                label: "Actions".into(),
                items: vec![Item {
                    id: "new-report".into(),
                    href: "/reports/new".into(),
                    label: "Create <report>".into(),
                    description: Some("Start here".into()),
                    shortcut: Some("Ctrl R".into()),
                    icon: Some("plus".into()),
                    keywords: vec!["analytics".into()],
                }],
            }],
            search_label: default_search_label(),
            search_placeholder: default_placeholder(),
            empty_text: default_empty_text(),
            instructions: default_instructions(),
            close_label: default_close_label(),
            shortcut: Some(GlobalShortcut {
                key: "k".into(),
                label: "Ctrl K".into(),
            }),
        }
    }
    #[test]
    fn all_items_render_escaped_with_real_destinations_and_complete_ids() {
        let x = sample();
        let h = render(&x, "<main>[[command_menu]]</main>", &icons()).unwrap();
        for s in [
            "href=\"/search\"",
            "href=\"/reports/new\"",
            "Create &lt;report&gt;",
            "data-cui-command-text=\"Create &lt;report&gt; Start here analytics\"",
            "data-cui-command-shortcut=\"k\"",
            "id=\"app-commands-item-new-report\"",
        ] {
            assert!(h.contains(s), "{s}");
        }
        assert_eq!(x.derived_ids().len(), 7);
    }
    #[test]
    fn declared_variant_goldens_match_exact_rendering() {
        let package_icons: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap();
        for fixture in [
            include_str!(
                "../../../packages/vanilla/components/command-menu/fixtures/typical-golden.json"
            ),
            include_str!(
                "../../../packages/vanilla/components/command-menu/fixtures/dense-golden.json"
            ),
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let instance: CommandMenuInstance =
                serde_json::from_value(value["options"].clone()).unwrap();
            assert_eq!(
                render(&instance, "[[command_menu]]\n", &package_icons).unwrap(),
                value["expectedHtml"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn rejects_malformed_unknown_count_collision_href_icon_and_fragment() {
        assert!(CommandMenuInstance::parse(br#"{"id":"bad id"}"#, &icons()).is_err());
        let mut x = sample();
        x.groups.push(x.groups[0].clone());
        assert!(x.validate(&icons()).is_err());
        let mut x = sample();
        x.groups[0].items[0].href = "javascript:alert(1)".into();
        assert!(x.validate(&icons()).is_err());
        let mut x = sample();
        x.groups[0].items[0].icon = Some("unknown".into());
        assert!(x.validate(&icons()).is_err());
        for f in [
            "",
            "[[command_menu]][[command_menu]]",
            "[[command_menu]][[other]]",
        ] {
            assert!(render(&sample(), f, &icons()).is_err());
        }
    }
    #[test]
    fn rejects_keyword_group_item_and_total_count_bounds_and_unknown_props() {
        let mut x = sample();
        x.groups[0].items[0].keywords = (0..13).map(|n| format!("k{n}")).collect();
        assert!(x.validate(&icons()).is_err());
        let mut x = sample();
        x.groups[0].items = (0..21)
            .map(|n| Item {
                id: format!("item-{n}"),
                href: "/x".into(),
                label: "X".into(),
                description: None,
                shortcut: None,
                icon: None,
                keywords: vec![],
            })
            .collect();
        assert!(x.validate(&icons()).is_err());
        let mut x = sample();
        x.groups = (0..8)
            .map(|g| Group {
                id: format!("group-{g}"),
                label: "Group".into(),
                items: (0..13)
                    .map(|n| Item {
                        id: format!("item-{g}-{n}"),
                        href: "/x".into(),
                        label: "X".into(),
                        description: None,
                        shortcut: None,
                        icon: None,
                        keywords: vec![],
                    })
                    .collect(),
            })
            .collect();
        assert!(x.validate(&icons()).is_err());
        assert!(CommandMenuInstance::parse(br#"{"id":"ok","unexpected":true}"#, &icons()).is_err());
    }
}
