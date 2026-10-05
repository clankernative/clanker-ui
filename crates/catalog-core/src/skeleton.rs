//! Typed selection and safe static rendering for decorative skeleton placeholders.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Text,
    Rectangle,
    Circle,
}

impl Shape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Rectangle => "rectangle",
            Self::Circle => "circle",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Width {
    Short,
    Medium,
    #[default]
    Full,
}

impl Width {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Full => "full",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkeletonInstance {
    pub shape: Shape,
    #[serde(default)]
    pub size: Size,
    #[serde(default)]
    pub width: Width,
    #[serde(default = "default_animated")]
    pub animated: bool,
}

fn default_animated() -> bool {
    true
}

/// Renders a decorative skeleton into the adapter-owned fragment. The fragment
/// must contain exactly one `[[skeleton]]` slot and no other template slots.
pub fn render(instance: &SkeletonInstance, fragment: &str) -> Result<String, String> {
    let shape = instance.shape.as_str();
    let size = instance.size.as_str();
    let width = instance.width.as_str();
    let animation = if instance.animated {
        "animated"
    } else {
        "static"
    };
    let markup = format!(
        "<span class=\"cui-skeleton cui-skeleton--{shape} cui-skeleton--{size} cui-skeleton--width-{width} cui-skeleton--{animation}\" data-cui-component=\"skeleton\" aria-hidden=\"true\"></span>"
    );
    crate::fragment::fill(fragment, &[("[[skeleton]]", &markup)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = "[[skeleton]]";

    fn instance(shape: Shape, size: Size, width: Width, animated: bool) -> SkeletonInstance {
        SkeletonInstance {
            shape,
            size,
            width,
            animated,
        }
    }

    #[test]
    fn renders_every_shape_size_width_and_animation_combination_as_hidden_decoration() {
        for shape in [Shape::Text, Shape::Rectangle, Shape::Circle] {
            for size in [Size::Small, Size::Medium, Size::Large] {
                for width in [Width::Short, Width::Medium, Width::Full] {
                    for animated in [false, true] {
                        let html =
                            render(&instance(shape, size, width, animated), FRAGMENT).unwrap();
                        assert!(html.contains(&format!("cui-skeleton--{}", shape.as_str())));
                        assert!(html.contains(&format!("cui-skeleton--{}", size.as_str())));
                        assert!(html.contains(&format!("cui-skeleton--width-{}", width.as_str())));
                        assert!(html.contains(&format!(
                            "cui-skeleton--{}",
                            if animated { "animated" } else { "static" }
                        )));
                        assert!(
                            html.contains("data-cui-component=\"skeleton\" aria-hidden=\"true\"")
                        );
                        assert!(
                            !html.contains("role=")
                                && !html.contains("aria-live")
                                && !html.contains("aria-busy")
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn requires_shape_and_defaults_size_width_and_animation() {
        let value: SkeletonInstance = serde_json::from_str(r#"{"shape":"text"}"#).unwrap();
        assert_eq!(value.shape, Shape::Text);
        assert_eq!(value.size, Size::Medium);
        assert_eq!(value.width, Width::Full);
        assert!(value.animated);
        assert!(serde_json::from_str::<SkeletonInstance>("{}").is_err());
        assert!(
            serde_json::from_str::<SkeletonInstance>(r#"{"shape":"text","unknown":true}"#).is_err()
        );
    }

    #[test]
    fn rejects_invalid_enum_values_and_malformed_json_attributes() {
        for json in [
            r#"{"shape":"oval"}"#,
            r#"{"shape":"text","size":"tiny"}"#,
            r#"{"shape":"text","width":"wide"}"#,
            r#"{"shape":"text","animated":"yes"}"#,
            r#"{"shape":"text","shape":"circle"}"#,
            r#"{"shape":"text" "size":"small"}"#,
        ] {
            assert!(
                serde_json::from_str::<SkeletonInstance>(json).is_err(),
                "{json}"
            );
        }
    }

    #[test]
    fn rejects_missing_duplicate_and_unsupported_fragment_slots() {
        for fragment in [
            "<span></span>",
            "[[skeleton]][[skeleton]]",
            "[[skeleton]][[attributes]]",
            "[[skeleton]][[unterminated",
        ] {
            assert!(
                render(
                    &instance(Shape::Text, Size::Medium, Width::Full, true),
                    fragment
                )
                .is_err(),
                "{fragment}"
            );
        }
    }
}
