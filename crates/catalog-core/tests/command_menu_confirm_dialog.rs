#[path = "../src/button.rs"]
mod button;
#[path = "../src/command_menu.rs"]
mod command_menu;
#[path = "../src/confirm_dialog.rs"]
mod confirm_dialog;
#[path = "../src/fragment.rs"]
mod fragment;
#[path = "../src/icon.rs"]
mod icon;
use std::collections::BTreeMap;
const COMMAND_FRAGMENT: &str = "<main>[[command_menu]]</main>";
const CONFIRM_FRAGMENT: &str = "<main>[[confirm_dialog]]</main>";
fn icons() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!("../../../packages/vanilla/icons.json")).unwrap()
}
#[test]
fn command_menu_fixture_output_is_deterministic_and_preserves_every_semantic_field() {
    for fixture in [
        include_str!("../../../packages/vanilla/components/command-menu/fixtures/typical.json"),
        include_str!("../../../packages/vanilla/components/command-menu/fixtures/dense.json"),
    ] {
        let instance =
            command_menu::CommandMenuInstance::parse(fixture.as_bytes(), &icons()).unwrap();
        let rendered = command_menu::render(&instance, COMMAND_FRAGMENT, &icons()).unwrap();
        assert_eq!(
            rendered,
            command_menu::render(&instance, COMMAND_FRAGMENT, &icons()).unwrap()
        );
        assert!(rendered.contains("<dialog"));
        assert!(rendered.contains("role=\"combobox\""));
        assert!(rendered.contains("role=\"listbox\""));
        assert!(rendered.contains("href=\"/search\""));
        for group in &instance.groups {
            assert!(rendered.contains(&format!("{}-group-{}", instance.id, group.id)));
            for item in &group.items {
                assert!(rendered.contains(&format!("href=\"{}\"", item.href)));
                assert!(rendered.contains(&format!("{}-item-{}", instance.id, item.id)));
            }
        }
    }
}
#[test]
fn command_menu_shortcut_and_localized_copy_render_without_markup_interpretation() {
    let mut instance = command_menu::CommandMenuInstance::parse(
        include_str!("../../../packages/vanilla/components/command-menu/fixtures/typical.json")
            .as_bytes(),
        &icons(),
    )
    .unwrap();
    instance.title = "Actions <today>".into();
    instance.search_label = "Rechercher".into();
    instance.groups[0].items[0].label = "Créer <rapport>".into();
    let rendered = command_menu::render(&instance, COMMAND_FRAGMENT, &icons()).unwrap();
    assert!(rendered.contains("Actions &lt;today&gt;"));
    assert!(rendered.contains("Rechercher"));
    assert!(rendered.contains("Créer &lt;rapport&gt;"));
    assert!(rendered.contains("data-cui-command-shortcut=\"k\""));
    assert!(!rendered.contains("<rapport>"));
}
#[test]
fn confirmation_fixture_outputs_cover_neutral_danger_headings_and_submitter() {
    for fixture in [
        include_str!("../../../packages/vanilla/components/confirm-dialog/fixtures/neutral.json"),
        include_str!("../../../packages/vanilla/components/confirm-dialog/fixtures/danger.json"),
    ] {
        let instance = confirm_dialog::ConfirmDialogInstance::parse(fixture.as_bytes()).unwrap();
        let rendered = confirm_dialog::render(&instance, CONFIRM_FRAGMENT, &icons()).unwrap();
        assert_eq!(
            rendered,
            confirm_dialog::render(&instance, CONFIRM_FRAGMENT, &icons()).unwrap()
        );
        let tone = match instance.tone {
            confirm_dialog::Tone::Neutral => "neutral",
            confirm_dialog::Tone::Danger => "danger",
        };
        let level = match instance.heading_level {
            confirm_dialog::HeadingLevel::H2 => "h2",
            confirm_dialog::HeadingLevel::H3 => "h3",
            confirm_dialog::HeadingLevel::H4 => "h4",
        };
        assert!(rendered.contains(&format!("cui-confirm-dialog--{tone}")));
        assert!(rendered.contains(&format!("form=\"{}\"", instance.form_id)));
        assert!(rendered.contains(&format!("<{level} ")));
        assert!(rendered.contains("data-cui-confirm-accept"));
        assert!(rendered.contains("data-cui-confirm-cancel"));
        if let (Some(name), Some(value)) = (&instance.submit_name, &instance.submit_value) {
            assert!(rendered.contains(&format!("name=\"{name}\" value=\"{value}\"")));
        }
    }
}
