//! Component completeness and host integration are separate contracts.
//! Port type declarations are locked component-owned inputs, not executable adapters.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationStatus {
    Supported,
    AdapterRequired,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeIntegration {
    pub status: IntegrationStatus,
    /// Describes the proven scope or missing host seam; not an admission grant.
    pub scope: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortContract {
    pub name: String,
    pub direction: PortDirection,
    pub type_source: String,
    pub export: String,
    pub purpose: String,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentIntegration {
    pub native: NativeIntegration,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<PortContract>,
}

impl ComponentIntegration {
    pub fn validate(&self, component: &str, contracts: &[String]) -> Result<(), String> {
        fn prose(value: &str) -> bool {
            !value.trim().is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
        }
        if !prose(&self.native.scope) || self.ports.len() > 16 {
            return Err("invalid host integration scope or port count".into());
        }
        let mut names = BTreeSet::new();
        for port in &self.ports {
            if port.name.is_empty()
                || port.name.len() > 64
                || !port
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                || !names.insert(&port.name)
                || !prose(&port.purpose)
                || !crate::safe_path(&port.type_source)
                || !port
                    .type_source
                    .starts_with(&format!("components/{component}/"))
                || !port.type_source.ends_with(".d.ts")
                || !contracts.contains(&port.type_source)
                || port.export.is_empty()
                || port.export.len() > 128
                || !port.export.bytes().enumerate().all(|(i, b)| {
                    b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit())
                })
            {
                return Err(format!(
                    "invalid or undeclared component port: {}",
                    port.name
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn integration() -> ComponentIntegration {
        serde_json::from_value(serde_json::json!({
            "native":{"status":"adapter-required","scope":"Native multipart transport is not wired."},
            "ports":[{"name":"selection","direction":"output","typeSource":"components/file-upload/browser.d.ts","export":"FileUploadPorts","purpose":"The app receives native file selection intent.","required":false}]
        })).unwrap()
    }
    #[test]
    fn adapter_requirement_is_not_component_incompleteness() {
        let contract = integration();
        assert!(contract
            .validate(
                "file-upload",
                &["components/file-upload/browser.d.ts".into()]
            )
            .is_ok());
        assert_eq!(contract.native.status, IntegrationStatus::AdapterRequired);
    }
    #[test]
    fn ports_are_owned_declared_and_closed() {
        let mut contract = integration();
        assert!(contract.validate("file-upload", &[]).is_err());
        assert!(contract
            .validate("other", &["components/file-upload/browser.d.ts".into()])
            .is_err());
        contract.ports.push(contract.ports[0].clone());
        assert!(contract
            .validate(
                "file-upload",
                &["components/file-upload/browser.d.ts".into()]
            )
            .is_err());
        assert!(serde_json::from_value::<ComponentIntegration>(
            serde_json::json!({"native":{"status":"pretend","scope":"Unknown status"}})
        )
        .is_err());
        assert!(serde_json::from_value::<NativeIntegration>(serde_json::json!({"status":"supported","scope":"Literal declarations only","arbitraryAdapter":true})).is_err());
    }
}
