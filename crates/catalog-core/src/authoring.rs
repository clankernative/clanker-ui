//! Declarative authoring guidance for template authors. This metadata is not an
//! admission policy or renderer instruction set.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_ELEMENTS: usize = 16;
const MAX_ATTRIBUTES_PER_ELEMENT: usize = 32;
const MAX_EXAMPLES: usize = 16;
const MAX_LIST_ITEMS: usize = 32;
const MAX_METADATA_BYTES: usize = 128 * 1024;
const MAX_TEMPLATE_BYTES: usize = 32 * 1024;
const MAX_TAG_BYTES: usize = 64;
const MAX_ATTRIBUTE_BYTES: usize = 64;
const MAX_TEXT_BYTES: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateAuthoring {
    pub schema_version: u32,
    pub root_tag: String,
    pub elements: Vec<TemplateElement>,
    pub rules: Vec<String>,
    pub examples: Vec<TemplateExample>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateElement {
    pub tag: String,
    pub form: ElementForm,
    pub allowed_parents: Vec<String>,
    pub attributes: Vec<TemplateAttribute>,
    pub content: ContentPolicy,
    pub allowed_children: Vec<String>,
    pub control_flow: Vec<String>,
    pub rules: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateAttribute {
    pub name: String,
    pub required: bool,
    pub value_forms: Vec<ValueForm>,
    pub rules: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateExample {
    pub name: String,
    pub summary: String,
    pub template: String,
    pub requirements: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ElementForm {
    Paired,
    SelfClosing,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ContentPolicy {
    None,
    Helpers,
    HostAdmitted,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ValueForm {
    Literal,
    StringField,
    BooleanField,
    KeyField,
    NamedRoute,
    JsonLiteral,
}

impl TemplateAuthoring {
    /// Validate authoring guidance against the component identity and target.
    /// This validates metadata shape only; it does not admit or render templates.
    pub fn validate(&self, component_name: &str, target: &str) -> Result<(), String> {
        let invalid = |message: &str| Err(message.to_string());
        if target != "native-html" {
            return invalid("template authoring target must be native-html");
        }
        if self.schema_version != 1 {
            return invalid("unsupported template authoring schema version");
        }
        let expected_root = format!("cui-{component_name}");
        if self.root_tag != expected_root {
            return invalid("rootTag must be cui-<component name>");
        }
        if self.elements.is_empty() || self.elements.len() > MAX_ELEMENTS {
            return invalid("elements must contain between 1 and 16 entries");
        }
        if self.examples.is_empty() || self.examples.len() > MAX_EXAMPLES {
            return invalid("examples must contain between 1 and 16 entries");
        }
        if self.rules.len() > MAX_LIST_ITEMS || !valid_texts(&self.rules, MAX_LIST_ITEMS) {
            return invalid("rules must be bounded meaningful plain text");
        }

        let tags: Vec<&str> = self
            .elements
            .iter()
            .map(|element| element.tag.as_str())
            .collect();
        let known: BTreeSet<&str> = tags.iter().copied().collect();
        if known.len() != tags.len()
            || !known.contains(self.root_tag.as_str())
            || self.elements.iter().any(|element| !valid_tag(&element.tag))
        {
            return invalid("element tags must be unique bounded cui-* names and include rootTag");
        }

        for element in &self.elements {
            if element.allowed_parents.len() > MAX_ELEMENTS
                || element.allowed_children.len() > MAX_ELEMENTS
                || !unique(&element.allowed_parents)
                || !unique(&element.allowed_children)
                || element
                    .allowed_parents
                    .iter()
                    .chain(element.allowed_children.iter())
                    .any(|reference| !known.contains(reference.as_str()))
            {
                return invalid(
                    "allowed parent/child references must be unique known element tags",
                );
            }
            if element.tag == self.root_tag {
                if !element.allowed_parents.is_empty() {
                    return invalid("root element must not declare allowed parents");
                }
            } else if element.allowed_parents.is_empty() {
                return invalid("non-root elements must declare at least one allowed parent");
            }
            if element.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT
                || !unique_by(&element.attributes, |attribute| &attribute.name)
            {
                return invalid("attributes must be unique and within the per-element limit");
            }
            for attribute in &element.attributes {
                if !valid_attribute_name(&attribute.name)
                    || attribute.value_forms.is_empty()
                    || attribute.value_forms.len() > 6
                    || !unique(&attribute.value_forms)
                    || attribute.rules.len() > MAX_LIST_ITEMS
                    || !valid_texts(&attribute.rules, MAX_LIST_ITEMS)
                {
                    return invalid("invalid attribute name, valueForms, or rules");
                }
            }
            if element.rules.len() > MAX_LIST_ITEMS
                || !valid_texts(&element.rules, MAX_LIST_ITEMS)
                || element.control_flow.len() > 6
                || !unique(&element.control_flow)
                || element.control_flow.iter().any(|flow| {
                    !matches!(
                        flow.as_str(),
                        "for" | "if" | "elif" | "else" | "endif" | "endfor"
                    )
                })
            {
                return invalid("invalid element rules or controlFlow");
            }
            match element.content {
                ContentPolicy::None
                    if !element.allowed_children.is_empty() || !element.control_flow.is_empty() =>
                {
                    return invalid("content none cannot declare children or controlFlow");
                }
                ContentPolicy::Helpers if element.allowed_children.is_empty() => {
                    return invalid("helpers content must declare allowedChildren");
                }
                _ => {}
            }
            if element.form == ElementForm::SelfClosing && element.content != ContentPolicy::None {
                return invalid("self-closing elements must have content none");
            }
        }

        // Both directions describe the same grammar edge, not independent hints.
        for element in &self.elements {
            for child in &element.allowed_children {
                let child = self
                    .elements
                    .iter()
                    .find(|entry| &entry.tag == child)
                    .unwrap();
                if !child.allowed_parents.contains(&element.tag) {
                    return invalid("parent/child references must agree");
                }
            }
            for parent in &element.allowed_parents {
                let parent = self
                    .elements
                    .iter()
                    .find(|entry| &entry.tag == parent)
                    .unwrap();
                if !parent.allowed_children.contains(&element.tag) {
                    return invalid("parent/child references must agree");
                }
            }
        }
        let mut reachable = BTreeSet::from([self.root_tag.as_str()]);
        loop {
            let previous = reachable.len();
            for element in &self.elements {
                if reachable.contains(element.tag.as_str()) {
                    reachable.extend(element.allowed_children.iter().map(String::as_str));
                }
            }
            if reachable.len() == previous {
                break;
            }
        }
        if reachable.len() != self.elements.len() {
            return invalid("all helper elements must be reachable from rootTag");
        }
        if !unique_by(&self.examples, |example| &example.name) {
            return invalid("example names must be unique");
        }
        for example in &self.examples {
            if example.name.len() > 128
                || !valid_kebab(&example.name)
                || !valid_plain_text(&example.summary, MAX_TEXT_BYTES)
                || example.template.trim().is_empty()
                || example.template.len() > MAX_TEMPLATE_BYTES
                || example.requirements.len() > MAX_LIST_ITEMS
                || !valid_texts(&example.requirements, MAX_LIST_ITEMS)
            {
                return invalid("invalid example name, summary, template, or requirements");
            }
        }
        let serialized = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        if serialized.len() > MAX_METADATA_BYTES {
            return invalid("template authoring metadata exceeds 128 KiB");
        }
        Ok(())
    }
}

fn valid_tag(value: &str) -> bool {
    value.len() <= MAX_TAG_BYTES && value.strip_prefix("cui-").is_some_and(valid_kebab)
}

fn valid_attribute_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ATTRIBUTE_BYTES && valid_kebab(value)
}

fn valid_kebab(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_plain_text(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes && !contains_control(value)
}

fn contains_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn valid_texts(values: &[String], max_items: usize) -> bool {
    values.len() <= max_items
        && values
            .iter()
            .all(|value| valid_plain_text(value, MAX_TEXT_BYTES))
}

fn unique<T: PartialEq>(values: &[T]) -> bool {
    (0..values.len()).all(|index| !values[..index].contains(&values[index]))
}

fn unique_by<T, K: PartialEq>(values: &[T], key: impl Fn(&T) -> &K) -> bool {
    (0..values.len())
        .all(|index| (0..index).all(|prior| key(&values[index]) != key(&values[prior])))
}
