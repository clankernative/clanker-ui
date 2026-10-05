mod fragment {
    pub fn fill(fragment: &str, replacements: &[(&str, &str)]) -> Result<String, String> {
        for (slot, _) in replacements {
            if fragment.matches(slot).count() != 1 {
                return Err(format!("fragment must contain exactly one {slot} slot"));
            }
        }
        if fragment.matches("[[").count() != replacements.len() {
            return Err("fragment contains unsupported or incomplete slots".into());
        }
        let mut result = fragment.to_owned();
        for (slot, value) in replacements {
            result = result.replacen(slot, value, 1);
        }
        Ok(result)
    }
}
mod layout {
    #[derive(Clone, Debug)]
    pub struct AdmittedChildren(String);
    impl AdmittedChildren {
        pub fn from_host_admitted(value: impl Into<String>) -> Self {
            Self(value.into())
        }
        pub(crate) fn as_markup(&self) -> &str {
            &self.0
        }
    }
}
#[path = "../src/data_viewport.rs"]
mod data_viewport;
#[path = "../src/file_upload.rs"]
mod file_upload;

#[cfg(test)]
mod tests {
    use super::*;

    const UPLOAD_FRAGMENT: &str =
        include_str!("../../../packages/vanilla/components/file-upload/fragment.html");
    const VIEWPORT_FRAGMENT: &str =
        include_str!("../../../packages/vanilla/components/data-viewport/fragment.html");
    const UPLOAD_GOLDEN: &str = include_str!(
        "../../../packages/vanilla/components/file-upload/fixtures/typical.golden.html"
    );
    const VIEWPORT_GOLDEN: &str = include_str!(
        "../../../packages/vanilla/components/data-viewport/fixtures/windowed.golden.html"
    );

    #[test]
    fn package_file_upload_fixture_is_valid_and_rendered_without_client_success_claims() {
        let bytes =
            include_str!("../../../packages/vanilla/components/file-upload/fixtures/typical.json");
        let value = file_upload::FileUploadInstance::parse(bytes).unwrap();
        let html = file_upload::render(&value, UPLOAD_FRAGMENT).unwrap();
        assert_eq!(html, UPLOAD_GOLDEN);
        assert!(html.contains("type=\"file\""));
        assert!(html.contains("Uploading 48%"));
        assert!(html.contains("value=\"48\""));
        assert!(!html.contains("data-cui-upload-success"));
    }

    #[test]
    fn package_window_fixture_has_checked_bounds_ids_positions_and_spacers() {
        let bytes = include_str!(
            "../../../packages/vanilla/components/data-viewport/fixtures/windowed.json"
        );
        let value = data_viewport::ViewportInstance::parse(bytes).unwrap();
        let html = data_viewport::render(&value, VIEWPORT_FRAGMENT).unwrap();
        assert_eq!(html, VIEWPORT_GOLDEN);
        assert!(html.contains("id=\"audit-events-items\""));
        assert!(html.contains("aria-posinset=\"41\""));
        assert!(html.contains("aria-setsize=\"10000\""));
        assert!(html.contains("block-size:2880px"));
        assert!(html.contains("block-size:716976px"));
        assert!(html.contains("data-cui-window-start=\"40\""));
    }

    #[test]
    fn declared_file_upload_variants_render_native_disabled_and_server_states() {
        let mut value = file_upload::FileUploadInstance::parse(
            r#"{"name":"proof","label":"Proof files","multiple":false,"disabled":true,"files":[{"id":"prior","name":"prior.pdf","status":{"state":"failed"},"statusLabel":"Rejected"}]}"#,
        ).unwrap();
        let html = file_upload::render(&value, UPLOAD_FRAGMENT).unwrap();
        assert!(html.contains("type=\"file\" disabled"));
        assert!(html.contains("class=\"cui-file-upload__file cui-file-upload__file--failed\""));
        assert!(!html.contains("data-cui-upload-success"));
        value.disabled = false;
        value.files.clear();
        assert!(file_upload::render(&value, UPLOAD_FRAGMENT)
            .unwrap()
            .contains("type=\"file\""));
    }

    #[test]
    fn declared_viewport_mode_and_layout_combinations_stay_app_owned() {
        for (mode, layout) in [
            ("paged", "list"),
            ("incremental", "list"),
            ("incremental", "table"),
        ] {
            let content = if layout == "list" {
                r#""items":[{"id":"row-1","content":"long app-owned content"}]"#.to_owned()
            } else {
                r#""columns":["Name"],"rows":[{"id":"row-1","cells":["Long app-owned cell"]}]"#
                    .to_owned()
            };
            let json = format!(
                r#"{{"id":"results","label":"Results","height":"viewport","mode":"{mode}","pagination":"Next",{content}}}"#
            );
            let value = data_viewport::ViewportInstance::parse(&json).unwrap();
            let html = data_viewport::render(&value, VIEWPORT_FRAGMENT).unwrap();
            assert!(html.contains(&format!("data-cui-viewport-mode=\"{mode}\"")));
            assert!(html.contains("cui-data-viewport--viewport"));
        }
        let table = data_viewport::ViewportInstance::parse(
            r#"{"id":"window-table","label":"Window table","mode":"windowed","window":{"start":2,"total":10,"itemSize":40,"overscan":2},"pagination":"Next","columns":["Name"],"rows":[{"id":"row-2","cells":["Long content that remains inside its fixed row"]}]}"#,
        ).unwrap();
        let html = data_viewport::render(&table, VIEWPORT_FRAGMENT).unwrap();
        assert!(html.contains("--cui-data-viewport-item-size:40px"));
        assert!(html.contains("cui-data-viewport__cell-content\" tabindex=\"0\""));
        assert!(html.contains("style=\"block-size:80px\""));
    }

    #[test]
    fn configs_are_strict_and_do_not_accept_serialized_markup() {
        assert!(file_upload::FileUploadInstance::parse(
            r#"{"name":"x","label":"X","rawHtml":"<b>bad</b>"}"#
        )
        .is_err());
        assert!(data_viewport::ViewportInstance::parse(r#"{"id":"x","label":"X","pagination":"Next","items":[{"id":"i","content":"<b>not markup</b>"}],"rawHtml":"<b>bad</b>"}"#).is_err());
        let value = file_upload::FileUploadInstance::parse(r#"{"name":"x","label":"X","files":[{"id":"file","name":"<b>text</b>","status":{"state":"ready"},"statusLabel":"Ready"}]}"#).unwrap();
        assert!(file_upload::render(&value, UPLOAD_FRAGMENT)
            .unwrap()
            .contains("&lt;b&gt;text&lt;/b&gt;"));
    }
}
