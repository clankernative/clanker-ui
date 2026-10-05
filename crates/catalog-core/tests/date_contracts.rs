#[test]
fn date_package_manifests_are_complete_with_explicit_native_gaps() {
    for component in ["date-calendar", "date-picker"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/vanilla/components")
            .join(component)
            .join("component.json");
        let manifest = catalog_core::Component::parse(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(manifest.status, catalog_core::Status::Ready);
        assert_eq!(
            manifest.integration.as_ref().unwrap().native.status,
            catalog_core::integration::IntegrationStatus::AdapterRequired
        );
        assert!(manifest.contract.required_fields.iter().all(|field| {
            !field.contains("slot")
                && field
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }));
    }
}
