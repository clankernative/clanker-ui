//! Read-only application boundary for locked token discovery and CSS diagnostics.
use crate::{css_analysis, ports::LoadedPackage};
use catalog_core::{
    css_check::{self, CssDiagnostic},
    tokens::{self, CssStylesheet, TokenCatalog, TokenMetadata, TokenSpec},
    Component,
};
use serde::Serialize;
use std::{collections::BTreeSet, path::Path};

pub fn package_styles(package: &LoadedPackage) -> Result<Vec<CssStylesheet>, String> {
    // Match the shared stylesheet closure and order emitted by expand. A
    // declared resource is not automatically part of the browser's cascade.
    let mut paths = vec![(package.catalog.package.theme.clone(), None)];
    if package.assets.contains_key("theme/reference.css") {
        paths.push(("theme/reference.css".into(), None));
    }
    // Readers include all complete component contracts, including components
    // whose separate Native integration still requires an adapter.
    for component in package.catalog.components() {
        paths.push((
            component.assets.styles.clone(),
            Some(component.name.clone()),
        ));
    }
    paths
        .into_iter()
        .map(|(path, owner)| {
            let bytes = package
                .assets
                .get(&path)
                .ok_or_else(|| format!("missing locked stylesheet: {path}"))?;
            let text = std::str::from_utf8(bytes).map_err(|error| format!("{path}: {error}"))?;
            css_analysis::parse_stylesheet(&path, owner.as_deref(), text)
        })
        .collect()
}

/// Defaults and readers derive from captured CSS; purposes/roles come from its
/// declared contracts. Nothing reads mutable package files after lock loading.
pub fn token_catalog(package: &LoadedPackage) -> Result<TokenCatalog, String> {
    let metadata_path = package.catalog.package.token_metadata.as_ref().ok_or(
        "locked package does not declare token metadata; explicitly upgrade its package and lock",
    )?;
    let metadata = TokenMetadata::parse(
        package
            .assets
            .get(metadata_path)
            .ok_or("token metadata is not a locked asset")?,
    )?;
    let mut specs = metadata
        .tokens
        .into_iter()
        .map(|definition| TokenSpec {
            definition,
            owner: "theme".into(),
            status: "shared".into(),
        })
        .collect::<Vec<_>>();
    // Draft manifests are captured, even though their implementation assets are
    // not admitted. Their descriptors do not imply readiness or default proof.
    for (path, bytes) in &package.assets {
        if path.starts_with("components/") && path.ends_with("/component.json") {
            let component = Component::parse(bytes).map_err(|error| format!("{path}: {error}"))?;
            for definition in component.token_descriptions {
                specs.push(TokenSpec {
                    definition,
                    owner: component.name.clone(),
                    status: match component.status {
                        catalog_core::Status::Ready => "ready",
                        catalog_core::Status::Draft => "draft",
                    }
                    .into(),
                });
            }
        }
    }
    let names = specs
        .iter()
        .map(|spec| spec.definition.name.as_str())
        .collect::<BTreeSet<_>>();
    for component in package.catalog.components() {
        for name in &component.tokens {
            if !names.contains(name.as_str()) {
                return Err(format!(
                    "{} token {name} has no component/theme description",
                    component.name
                ));
            }
        }
    }
    tokens::resolve_tokens(&specs, &package_styles(package)?)
}

pub fn described_component(
    package: &LoadedPackage,
    component: &Component,
) -> Result<serde_json::Value, String> {
    let mut value = serde_json::to_value(component).map_err(|error| error.to_string())?;
    // Older packages stay discoverable; the explicit new commands require a
    // declared metadata source rather than inventing token descriptions.
    if package.catalog.package.token_metadata.is_some() {
        let catalog = token_catalog(package)?;
        value["tokenDetails"] = serde_json::to_value(
            catalog
                .tokens
                .iter()
                .filter(|token| {
                    token.owner == component.name
                        || token.readers.contains(&component.name)
                        || component.tokens.contains(&token.name)
                })
                .collect::<Vec<_>>(),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(value)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CssCheckReport {
    pub package_digest: String,
    pub files: Vec<String>,
    pub errors: usize,
    pub warnings: usize,
    pub limitations: Vec<String>,
    #[serde(skip)]
    pub diagnostics: Vec<CssDiagnostic>,
}

pub fn check_css(ui: &Path, package: &LoadedPackage) -> Result<CssCheckReport, String> {
    let catalog = token_catalog(package)?;
    let inspected = css_analysis::inspect_ui(ui, package, &catalog, &package_styles(package)?)?;
    let diagnostics = css_check::check_css(&catalog, &inspected.rules, &inspected.pages);
    Ok(CssCheckReport {
        package_digest: package.digest.clone(),
        files: inspected.files,
        errors: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "error")
            .count(),
        warnings: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "warning")
            .count(),
        limitations: inspected.limitations,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unused_css_resource_cannot_invent_a_baseline_default() {
        let manifest = catalog_core::PackageManifest::parse(br#"{"schemaVersion":1,"name":"@clanker/test","version":"1","summary":"Proof","theme":"theme/default.css","tokenMetadata":"theme/tokens.json","resources":["theme/tokens.json","theme/extra.css"]}"#).unwrap();
        let package = LoadedPackage {
            root: std::path::PathBuf::from("/never-read"),
            catalog: catalog_core::Catalog::new(manifest, vec![]).unwrap(),
            digest: "sha256:proof".into(),
            assets: std::collections::BTreeMap::from([
                ("theme/default.css".into(), b":root { --cui-accent: green; }".to_vec()),
                ("theme/extra.css".into(), b":root { --cui-accent: red; }".to_vec()),
                ("theme/tokens.json".into(), br#"{"schemaVersion":1,"tokens":[{"name":"--cui-accent","role":"color","purpose":"Shared accent"}]}"#.to_vec()),
            ]),
        };
        let catalog = token_catalog(&package).unwrap();
        assert_eq!(
            catalog
                .get("--cui-accent")
                .unwrap()
                .default_value
                .as_deref(),
            Some("green")
        );
        assert!(package_styles(&package)
            .unwrap()
            .iter()
            .all(|sheet| sheet.path != "theme/extra.css"));
    }
}
