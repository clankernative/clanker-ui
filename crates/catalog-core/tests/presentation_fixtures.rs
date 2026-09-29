//! Regression coverage against the actual declared fragment bytes, including whitespace.
use catalog_core::{avatar, empty_state, metric, page_header, skeleton, Component};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

#[test]
fn presentation_goldens_match_the_locked_fragment_shape() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/vanilla");
    let icons: BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(package.join("icons.json")).unwrap()).unwrap();
    for name in ["avatar", "empty-state", "metric", "skeleton", "page-header"] {
        let manifest: Component = serde_json::from_slice(
            &fs::read(package.join(format!("components/{name}/component.json"))).unwrap(),
        )
        .unwrap();
        let fragment = fs::read_to_string(package.join(&manifest.assets.template)).unwrap();
        assert!(!manifest.fixtures.is_empty(), "{name}");
        for path in &manifest.fixtures {
            let mut fixture: Value =
                serde_json::from_slice(&fs::read(package.join(path)).unwrap()).unwrap();
            let expected = fixture
                .as_object_mut()
                .unwrap()
                .remove("expectedHtml")
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            let rendered = match name {
                "avatar" => avatar::render(&serde_json::from_value(fixture).unwrap(), &fragment),
                "empty-state" => {
                    empty_state::render(&serde_json::from_value(fixture).unwrap(), &fragment)
                }
                "metric" => {
                    metric::render(&serde_json::from_value(fixture).unwrap(), &fragment, &icons)
                }
                "skeleton" => {
                    skeleton::render(&serde_json::from_value(fixture).unwrap(), &fragment)
                }
                "page-header" => {
                    page_header::render(&serde_json::from_value(fixture).unwrap(), &fragment)
                }
                _ => unreachable!(),
            }
            .unwrap();
            assert_eq!(rendered, expected, "{path}");
        }
    }
}
