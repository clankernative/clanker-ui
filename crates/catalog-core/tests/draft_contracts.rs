//! Draft is an honest readiness boundary, not permission for malformed contracts.
use catalog_core::{Component, PackageManifest, Status};
use std::{fs, path::PathBuf};

#[test]
fn every_declared_manifest_and_asset_path_is_valid_even_before_registration() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    PackageManifest::parse(&fs::read(root.join("ui-package.json")).unwrap()).unwrap();
    for entry in fs::read_dir(root.join("components")).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().join("component.json");
        if !path.exists() {
            continue;
        }
        let component = Component::parse(&fs::read(&path).unwrap())
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(component.name, entry.file_name().to_str().unwrap());
        for asset in component
            .assets
            .scripts
            .iter()
            .chain([&component.assets.template, &component.assets.styles])
            .chain(component.fixtures.iter())
        {
            let bytes = fs::read(root.join(asset))
                .unwrap_or_else(|error| panic!("{}: {error}", root.join(asset).display()));
            assert!(!bytes.is_empty(), "empty declared asset: {asset}");
        }
        if component.status == Status::Ready {
            assert!(
                !component.fixtures.is_empty(),
                "ready component {} needs fixtures",
                component.name
            );
        }
    }
}

#[test]
fn activity_timestamp_validator_covers_the_gregorian_leap_cycle_and_invalid_controls() {
    // Cover every year in a full Gregorian leap cycle, using boundary dates as an
    // independent calendar oracle. No third-party generator dependency is needed.
    let mut leap_count = 0;
    for year in 2000..2400 {
        assert!(catalog_core::activity_feed::valid_rfc3339(&format!(
            "{year}-02-28T23:59:59Z"
        )));
        leap_count += usize::from(catalog_core::activity_feed::valid_rfc3339(&format!(
            "{year}-02-29T12:00:00+05:30"
        )));
        assert!(!catalog_core::activity_feed::valid_rfc3339(&format!(
            "{year}-02-30T00:00:00Z"
        )));
        for month in [4, 6, 9, 11] {
            assert!(!catalog_core::activity_feed::valid_rfc3339(&format!(
                "{year}-{month:02}-31T00:00:00Z"
            )));
        }
    }
    assert_eq!(
        leap_count, 97,
        "A Gregorian 400-year cycle has 97 leap days"
    );
    assert!(catalog_core::activity_feed::valid_rfc3339(
        "2000-02-29T12:00:00Z"
    ));
    for year in [1900, 2100, 2200, 2300] {
        assert!(!catalog_core::activity_feed::valid_rfc3339(&format!(
            "{year}-02-29T12:00:00Z"
        )));
    }
    for byte in 0u8..=31 {
        let value = format!("2026-10-01T00:00:00Z{}", char::from(byte));
        assert!(!catalog_core::activity_feed::valid_rfc3339(&value));
    }
}
