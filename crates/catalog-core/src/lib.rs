//! Pure package and component rules. No filesystem, process, or browser dependencies.

pub use clanker_ui_runtime as template_values;

pub mod activity_feed;
pub mod alert;
pub mod authoring;
pub mod avatar;
pub mod badge;
pub mod breadcrumbs;
pub mod button;
pub mod button_group;
pub mod checkbox_group;
pub mod command_menu;
pub mod confirm_dialog;
pub mod copy_field;
pub mod css_check;
pub mod data_table;
pub mod data_viewport;
pub mod date;
pub mod date_calendar;
pub mod date_picker;
pub mod definition_list;
pub mod disclosure;
pub mod divider;
pub mod drawer;
pub mod empty_state;
pub mod expansion;
pub mod file_upload;
pub mod filter_bar;
pub mod form_field;
pub mod fragment;
pub mod icon;
pub mod integration;
pub mod layout;
pub mod metric;
pub mod modal;
pub mod page_header;
pub mod pagination;
pub mod popover;
pub mod progress;
pub mod progress_steps;
pub mod properties;
pub mod radio_group;
pub mod segmented_control;
pub mod select_field;
pub mod skeleton;
pub mod status_indicator;
pub mod tabs;
pub mod tag;
pub mod theme;
pub mod theme_switcher;
pub mod toast;
pub mod toggle;
pub mod tokens;
pub mod tooltip;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub name: String,
    pub version: String,
    pub summary: String,
    pub theme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_metadata: Option<String>,
    #[serde(default)]
    pub resources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub schema_version: u32,
    pub name: String,
    pub status: Status,
    pub target: String,
    pub category: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub agent: AgentGuidance,
    pub contract: Contract,
    pub tokens: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub token_descriptions: Vec<tokens::TokenDefinition>,
    pub dependencies: Vec<String>,
    pub assets: Assets,
    pub fixtures: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_authoring: Option<authoring::TemplateAuthoring>,
    /// Ready means component-complete; this separately describes host wiring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration: Option<integration::ComponentIntegration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Draft,
    Ready,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentGuidance {
    pub use_when: String,
    pub avoid_when: String,
    pub compose_with: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contract {
    pub role: String,
    pub variants: Vec<String>,
    pub required_fields: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assets {
    pub template: String,
    pub styles: String,
    pub scripts: Vec<String>,
    /// Locked, non-executable type contracts for app-owned adapter ports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contracts: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub package: PackageManifest,
    components: BTreeMap<String, Component>,
}

pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn meaningful(value: &str) -> bool {
    !value.trim().is_empty() && !value.contains(['\r', '\n'])
}

fn field_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

impl PackageManifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let Some((owner, name)) = manifest
            .name
            .strip_prefix('@')
            .and_then(|n| n.split_once('/'))
        else {
            return Err("package name must be @organization/name".into());
        };
        if manifest.schema_version != SCHEMA_VERSION
            || !identifier(owner)
            || !identifier(name)
            || !meaningful(&manifest.version)
            || !meaningful(&manifest.summary)
            || !safe_path(&manifest.theme)
            || manifest.token_metadata.as_ref().is_some_and(|path| {
                !safe_path(path) || !path.ends_with(".json") || !manifest.resources.contains(path)
            })
            || manifest.resources.iter().any(|path| !safe_path(path))
            || !unique(&manifest.resources)
        {
            return Err("invalid package schema version, identity, version, or summary".into());
        }
        Ok(manifest)
    }
}

impl Component {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let component: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if component.schema_version != SCHEMA_VERSION
            || !identifier(&component.name)
            || !identifier(&component.category)
            || !meaningful(&component.target)
            || !meaningful(&component.summary)
            || !meaningful(&component.agent.use_when)
            || !meaningful(&component.agent.avoid_when)
            || !meaningful(&component.contract.role)
            || component.contract.variants.is_empty()
            || component.contract.invariants.is_empty()
            || !unique(&component.dependencies)
            || !unique(&component.contract.variants)
            || !unique(&component.contract.required_fields)
            || !unique(&component.tokens)
            || !unique(
                &component
                    .token_descriptions
                    .iter()
                    .map(|token| token.name.clone())
                    .collect::<Vec<_>>(),
            )
            || component
                .token_descriptions
                .iter()
                .any(|token| token.validate().is_err() || !component.tokens.contains(&token.name))
            || component
                .contract
                .required_fields
                .iter()
                .any(|field| !field_name(field))
            || component
                .tokens
                .iter()
                .any(|token| !token.starts_with("--cui-"))
            || component
                .keywords
                .iter()
                .any(|keyword| !meaningful(keyword))
        {
            return Err("invalid component identity, target, guidance, or contract".into());
        }
        if let Some(integration) = &component.integration {
            integration.validate(&component.name, &component.assets.contracts)?;
        }
        if !unique(&component.assets.contracts)
            || component.assets.contracts.iter().any(|path| {
                !safe_path(path)
                    || !path.ends_with(".d.ts")
                    || !path.starts_with(&format!("components/{}/", component.name))
            })
        {
            return Err("invalid component-owned port type assets".into());
        }
        if component.dependencies.iter().any(|name| !identifier(name))
            || component
                .contract
                .variants
                .iter()
                .any(|variant| !identifier(variant))
            || !component.assets.template.ends_with(".html")
            || !component.assets.styles.ends_with(".css")
            || component
                .assets
                .scripts
                .iter()
                .any(|path| !path.ends_with(".js"))
            || component
                .fixtures
                .iter()
                .any(|path| !path.ends_with(".json"))
            || component
                .assets
                .scripts
                .iter()
                .chain(component.assets.contracts.iter())
                .chain([&component.assets.template, &component.assets.styles])
                .chain(component.fixtures.iter())
                .any(|path| !safe_path(path))
        {
            return Err("invalid dependency, variant, or asset path".into());
        }
        if let Some(metadata) = &component.template_authoring {
            metadata.validate(&component.name, &component.target)?;
        }
        Ok(component)
    }

    pub fn id(&self, package: &str) -> String {
        format!("{package}/{}", self.name)
    }
}

impl Catalog {
    pub fn new(package: PackageManifest, components: Vec<Component>) -> Result<Self, String> {
        let mut ready = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for component in components {
            if !seen.insert(component.name.clone()) {
                return Err(format!("duplicate component: {}", component.name));
            }
            if component.status == Status::Ready {
                ready.insert(component.name.clone(), component);
            }
        }
        for component in ready.values() {
            for dependency in &component.dependencies {
                if !ready.contains_key(dependency) {
                    return Err(format!(
                        "{} requires unavailable component {dependency}",
                        component.name
                    ));
                }
            }
        }
        let catalog = Self {
            package,
            components: ready,
        };
        for name in catalog.components.keys() {
            catalog.resolve(name)?;
        }
        Ok(catalog)
    }

    pub fn components(&self) -> impl Iterator<Item = &Component> {
        self.components.values()
    }

    pub fn get(&self, name: &str) -> Option<&Component> {
        self.components.get(name)
    }

    pub fn find(&self, query: &str, target: &str) -> Vec<&Component> {
        let query = query.trim().to_ascii_lowercase();
        self.components
            .values()
            .filter(|component| {
                component.target == target
                    && (query.is_empty()
                        || [
                            component.name.as_str(),
                            component.category.as_str(),
                            component.summary.as_str(),
                            component.agent.use_when.as_str(),
                        ]
                        .into_iter()
                        .chain(component.keywords.iter().map(String::as_str))
                        .any(|text| text.to_ascii_lowercase().contains(&query)))
            })
            .collect()
    }

    pub fn resolve(&self, name: &str) -> Result<Vec<&Component>, String> {
        fn visit<'a>(
            catalog: &'a Catalog,
            name: &str,
            visiting: &mut BTreeSet<String>,
            visited: &mut BTreeSet<String>,
            ordered: &mut Vec<&'a Component>,
        ) -> Result<(), String> {
            if visited.contains(name) {
                return Ok(());
            }
            if !visiting.insert(name.into()) {
                return Err(format!("component dependency cycle at {name}"));
            }
            let component = catalog
                .get(name)
                .ok_or_else(|| format!("unknown or unready component: {name}"))?;
            for dependency in &component.dependencies {
                visit(catalog, dependency, visiting, visited, ordered)?;
            }
            visiting.remove(name);
            visited.insert(name.into());
            ordered.push(component);
            Ok(())
        }
        let mut ordered = Vec::new();
        visit(
            self,
            name,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut ordered,
        )?;
        Ok(ordered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package() -> PackageManifest {
        PackageManifest::parse(br#"{"schemaVersion":1,"name":"@clanker/vanilla","version":"0.1.0","summary":"UI","theme":"theme/default.css"}"#).unwrap()
    }

    fn component(name: &str, status: Status, dependencies: &[&str]) -> Component {
        Component {
            schema_version: 1,
            name: name.into(),
            status,
            target: "native-html".into(),
            category: "actions".into(),
            summary: "A control".into(),
            keywords: vec!["action".into()],
            agent: AgentGuidance {
                use_when: "A user acts".into(),
                avoid_when: "No action exists".into(),
                compose_with: vec![],
            },
            contract: Contract {
                role: "button".into(),
                variants: vec!["primary".into()],
                required_fields: vec!["label".into()],
                invariants: vec!["Label is visible".into()],
            },
            tokens: vec!["--cui-accent".into()],
            token_descriptions: vec![],
            dependencies: dependencies.iter().map(|d| (*d).into()).collect(),
            assets: Assets {
                template: "components/button/template.html".into(),
                styles: "components/button/styles.css".into(),
                scripts: vec![],
                contracts: vec![],
            },
            fixtures: vec!["components/button/fixtures/primary.json".into()],
            template_authoring: None,
            integration: None,
        }
    }

    #[test]
    fn required_fields_accept_actual_serialized_camel_case_names() {
        let mut contract = component("button", Status::Ready, &[]);
        contract.contract.required_fields =
            vec!["monthLabels".into(), "weekday_short_labels".into()];
        assert!(Component::parse(&serde_json::to_vec(&contract).unwrap()).is_ok());
        contract.contract.required_fields.push("not a field".into());
        assert!(Component::parse(&serde_json::to_vec(&contract).unwrap()).is_err());
    }

    #[test]
    fn ready_only_and_dependency_order() {
        let catalog = Catalog::new(
            package(),
            vec![
                component("button", Status::Ready, &["icon"]),
                component("icon", Status::Ready, &[]),
                component("draft", Status::Draft, &[]),
            ],
        )
        .unwrap();
        assert_eq!(catalog.components().count(), 2);
        assert_eq!(
            catalog
                .resolve("button")
                .unwrap()
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            vec!["icon", "button"]
        );
        assert!(catalog.resolve("draft").is_err());
    }

    #[test]
    fn missing_and_cyclic_dependencies_fail() {
        assert!(Catalog::new(
            package(),
            vec![component("button", Status::Ready, &["icon"])]
        )
        .is_err());
        assert!(Catalog::new(
            package(),
            vec![
                component("button", Status::Ready, &["icon"]),
                component("icon", Status::Ready, &["button"])
            ]
        )
        .is_err());
    }

    #[test]
    fn malformed_declared_component_fails() {
        let value = serde_json::to_vec(&component("button", Status::Ready, &[])).unwrap();
        assert!(Component::parse(&value).is_ok());
        let invalid = String::from_utf8(value)
            .unwrap()
            .replace("components/button/template.html", "../secrets");
        assert!(Component::parse(invalid.as_bytes()).is_err());
    }
}
