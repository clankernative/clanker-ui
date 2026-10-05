//! Typed selection and safe rendering for the package's closed SVG icon catalog.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Versioned human-readable metadata for the package's closed icon geometry map.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IconCatalog {
    pub schema_version: u32,
    pub categories: Vec<IconCategory>,
    pub icons: Vec<IconDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IconDefinition {
    pub name: String,
    pub label: String,
    pub category: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IconCategory {
    pub name: String,
    pub label: String,
}

impl IconCatalog {
    /// Parses and validates metadata against the existing closed geometry map.
    pub fn parse(bytes: &[u8], geometries: &BTreeMap<String, String>) -> Result<Self, String> {
        let catalog: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        catalog.validate(geometries)?;
        Ok(catalog)
    }

    pub fn validate(&self, geometries: &BTreeMap<String, String>) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported icon catalog schema version: {}",
                self.schema_version
            ));
        }
        let mut categories = BTreeSet::new();
        for category in &self.categories {
            if !valid_icon_identifier(&category.name) || !categories.insert(category.name.as_str())
            {
                return Err(format!(
                    "invalid or duplicate icon category: {}",
                    category.name
                ));
            }
            if category.label.trim().is_empty() {
                return Err(format!(
                    "icon category label must not be blank: {}",
                    category.name
                ));
            }
        }

        let mut names = BTreeSet::new();
        let mut previous: Option<&str> = None;
        for icon in &self.icons {
            if !valid_icon_identifier(&icon.name) || !names.insert(icon.name.as_str()) {
                return Err(format!("invalid or duplicate icon name: {}", icon.name));
            }
            if previous.is_some_and(|name| name >= icon.name.as_str()) {
                return Err("icon metadata names must be sorted".into());
            }
            previous = Some(&icon.name);
            if icon.label.trim().is_empty() {
                return Err(format!("icon label must not be blank: {}", icon.name));
            }
            if !categories.contains(icon.category.as_str()) {
                return Err(format!(
                    "unknown category for icon {}: {}",
                    icon.name, icon.category
                ));
            }
        }

        let geometry_names: BTreeSet<_> = geometries.keys().map(String::as_str).collect();
        if names != geometry_names {
            let missing: Vec<_> = geometry_names.difference(&names).copied().collect();
            let extra: Vec<_> = names.difference(&geometry_names).copied().collect();
            return Err(format!(
                "icon metadata does not match geometry names (missing: {missing:?}, extra: {extra:?})"
            ));
        }
        Ok(())
    }

    /// Returns every definition whose name, label, or category contains `query`.
    pub fn find(&self, query: &str) -> Vec<&IconDefinition> {
        let query = query.to_lowercase();
        self.icons
            .iter()
            .filter(|icon| {
                icon.name.to_lowercase().contains(&query)
                    || icon.label.to_lowercase().contains(&query)
                    || icon.category.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn lookup(&self, name: &str) -> Option<&IconDefinition> {
        self.icons.iter().find(|icon| icon.name == name)
    }

    pub fn category_label(&self, name: &str) -> Option<&str> {
        self.categories
            .iter()
            .find(|category| category.name == name)
            .map(|category| category.label.as_str())
    }
}

fn valid_icon_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IconSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl IconSize {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IconInstance {
    pub name: String,
    #[serde(default)]
    pub size: IconSize,
    #[serde(default)]
    pub label: Option<String>,
}

impl IconInstance {
    pub fn validate(&self, icons: &BTreeMap<String, String>) -> Result<(), String> {
        if !icons.contains_key(&self.name) {
            return Err(format!("unknown icon: {}", self.name));
        }
        if self.name.is_empty()
            || !self
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err("icon name must be a closed catalog name".into());
        }
        if let Some(label) = &self.label {
            if label.trim().is_empty() || label.chars().any(char::is_control) {
                return Err("icon label must contain safe text".into());
            }
        }
        Ok(())
    }
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn safe_geometry(value: &str) -> bool {
    // Package geometry is a fragment, never a complete SVG or executable markup.
    let lowercase = value.to_ascii_lowercase();
    !value.is_empty()
        && !lowercase.contains("<script")
        && !lowercase.contains("<svg")
        && !lowercase.contains("foreignobject")
        && !lowercase.contains("javascript:")
        && !lowercase.contains("href")
        && !lowercase.contains("style")
        && !value.contains("{{")
        && !value.contains("[[")
        && !value.contains('&')
        && !value.split_whitespace().any(|part| {
            let part = part.to_ascii_lowercase();
            part.starts_with("on") && part.contains('=')
        })
        && value
            .chars()
            .all(|character| !character.is_control() || character == '\n' || character == '\t')
}

/// Renders one checked icon against the adapter-owned fragment template.
/// The fragment must contain exactly one `[[attributes]]` and `[[geometry]]` slot.
pub fn render(
    instance: &IconInstance,
    fragment: &str,
    icons: &BTreeMap<String, String>,
) -> Result<String, String> {
    instance.validate(icons)?;
    if fragment.matches("[[attributes]]").count() != 1
        || fragment.matches("[[geometry]]").count() != 1
    {
        return Err("icon fragment must contain exactly one attributes and geometry slot".into());
    }
    if fragment.matches("[[").count() != 2 {
        return Err("icon fragment contains an unsupported slot".into());
    }
    let geometry = icons
        .get(&instance.name)
        .ok_or_else(|| format!("unknown icon: {}", instance.name))?;
    if !safe_geometry(geometry) {
        return Err(format!("unsafe SVG geometry for icon: {}", instance.name));
    }
    let attributes = format!(
        "class=\"cui-icon cui-icon--{}\" data-cui-component=\"icon\" data-cui-icon=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-linecap=\"round\" stroke-linejoin=\"round\" focusable=\"false\"{}",
        instance.size.as_str(),
        instance.name,
        match &instance.label {
            Some(label) => format!(" role=\"img\" aria-label=\"{}\"", escape_attribute(label)),
            None => " aria-hidden=\"true\"".into(),
        }
    );
    crate::fragment::fill(
        fragment,
        &[("[[attributes]]", &attributes), ("[[geometry]]", geometry)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icons() -> BTreeMap<String, String> {
        BTreeMap::from([("check".into(), "<path d=\"M1 1\"></path>".into())])
    }

    fn icon() -> IconInstance {
        IconInstance {
            name: "check".into(),
            size: IconSize::Medium,
            label: None,
        }
    }

    const FRAGMENT: &str = "<svg [[attributes]]>[[geometry]]</svg>";

    fn package_geometries() -> BTreeMap<String, String> {
        serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap()
    }

    #[test]
    fn package_icon_metadata_validates_and_supports_search_and_lookup() {
        let catalog = IconCatalog::parse(
            include_bytes!("../../../packages/vanilla/icon-catalog.json"),
            &package_geometries(),
        )
        .unwrap();
        assert_eq!(catalog.icons.len(), 100);
        assert_eq!(catalog.categories.len(), 7);
        assert_eq!(catalog.lookup("cpu").unwrap().label, "Processor");
        assert_eq!(catalog.category_label("system-tools"), Some("System"));
        assert_eq!(catalog.find("ARROW").len(), 4);
        assert_eq!(catalog.find("communication").len(), 11);
    }

    #[test]
    fn icon_catalog_rejects_duplicate_names_unknown_categories_and_geometry_mismatch() {
        let geometry = BTreeMap::from([("alpha".to_string(), "<path/>".to_string())]);
        let invalid = [
            r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"actions"},{"name":"alpha","label":"Again","category":"actions"}]}"#,
            r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"alpha","label":"Alpha","category":"unknown"}]}"#,
            r#"{"schemaVersion":1,"categories":[{"name":"actions","label":"Actions"}],"icons":[{"name":"beta","label":"Beta","category":"actions"}]}"#,
        ];
        for json in invalid {
            assert!(IconCatalog::parse(json.as_bytes(), &geometry).is_err());
        }
    }

    #[test]
    fn decorative_is_hidden_and_labeled_icon_is_an_image() {
        let decorative = render(&icon(), FRAGMENT, &icons()).unwrap();
        assert!(decorative.contains("aria-hidden=\"true\""));
        assert!(!decorative.contains("role=\"img\""));
        let labeled = IconInstance {
            label: Some("Status & <check>\"".into()),
            ..icon()
        };
        let labeled = render(&labeled, FRAGMENT, &icons()).unwrap();
        assert!(labeled.contains("role=\"img\""));
        assert!(labeled.contains("aria-label=\"Status &amp; &lt;check&gt;&quot;\""));
        assert!(!labeled.contains("aria-hidden"));
    }

    #[test]
    fn slot_looking_labels_cannot_inject_geometry_into_attributes() {
        let instance = IconInstance {
            label: Some("[[geometry]] & <check>".into()),
            ..icon()
        };
        let rendered = render(&instance, FRAGMENT, &icons()).unwrap();
        assert!(rendered.contains("aria-label=\"[[geometry]] &amp; &lt;check&gt;\""));
        assert_eq!(rendered.matches("<path").count(), 1);
    }

    #[test]
    fn closed_names_and_unsafe_values_are_rejected() {
        for name in ["missing", "check\"><script>", "Check", "../check"] {
            let instance = IconInstance {
                name: name.into(),
                ..icon()
            };
            assert!(instance.validate(&icons()).is_err(), "{name}");
        }
        let instance = IconInstance {
            label: Some("  ".into()),
            ..icon()
        };
        assert!(instance.validate(&icons()).is_err());
        let mut malicious_icons = icons();
        malicious_icons.insert("check".into(), "<script>alert(1)</script>".into());
        assert!(render(&icon(), FRAGMENT, &malicious_icons).is_err());
    }

    #[test]
    fn sizes_and_output_are_deterministic_and_slots_are_exact() {
        for (size, class) in [
            (IconSize::Small, "cui-icon--small"),
            (IconSize::Medium, "cui-icon--medium"),
            (IconSize::Large, "cui-icon--large"),
        ] {
            let instance = IconInstance { size, ..icon() };
            let first = render(&instance, FRAGMENT, &icons()).unwrap();
            assert_eq!(first, render(&instance, FRAGMENT, &icons()).unwrap());
            assert!(first.contains(class));
        }
        assert!(render(
            &icon(),
            "<svg [[attributes]]>[[geometry]][[geometry]]</svg>",
            &icons()
        )
        .is_err());
    }
}
