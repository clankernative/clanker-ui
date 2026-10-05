//! Closed Toolframe reference themes for the Native UI package.
//!
//! This module is a pure authority: it performs no filesystem or host I/O. The
//! generated CSS is an opt-in resource; consumers must attach a named theme to
//! the document root explicitly.

/// The five built-in reference palettes, in stable CSS emission order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    Ocean,
    Forest,
    Plum,
}

impl Theme {
    pub const ALL: [Self; 5] = [
        Self::Light,
        Self::Dark,
        Self::Ocean,
        Self::Forest,
        Self::Plum,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Ocean => "ocean",
            Self::Forest => "forest",
            Self::Plum => "plum",
        }
    }

    const fn scheme(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            _ => "light",
        }
    }

    const fn declarations(self) -> &'static str {
        match self {
            Self::Light => LIGHT,
            Self::Dark => DARK,
            Self::Ocean => OCEAN,
            Self::Forest => FOREST,
            Self::Plum => PLUM,
        }
    }
}

impl TryFrom<&str> for Theme {
    type Error = UnknownTheme;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.as_str() == value)
            .ok_or(UnknownTheme)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnknownTheme;

impl std::fmt::Display for UnknownTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown reference theme (expected light, dark, ocean, forest, or plum)")
    }
}

impl std::error::Error for UnknownTheme {}

const COMMON: &str = "\
  --cui-theme-surface-canvas: {canvas};\n\
  --cui-theme-surface-raised: {raised};\n\
  --cui-theme-surface-overlay: {overlay};\n\
  --cui-theme-surface-sunken: {sunken};\n\
  --cui-theme-surface-hover: {hover};\n\
  --cui-theme-surface-selected: {selected};\n\
  --cui-theme-text-primary: {primary};\n\
  --cui-theme-text-secondary: {secondary};\n\
  --cui-theme-text-muted: {muted};\n\
  --cui-theme-text-inverse: {inverse};\n\
  --cui-theme-border-subtle: {border_subtle};\n\
  --cui-theme-border-strong: {border_strong};\n\
  --cui-theme-accent: {accent};\n\
  --cui-theme-accent-hover: {accent_hover};\n\
  --cui-theme-accent-active: {accent_active};\n\
  --cui-theme-accent-subtle: {accent_subtle};\n\
  --cui-theme-accent-text: {accent_text};\n\
  --cui-theme-link: {link};\n\
  --cui-theme-link-hover: {link_hover};\n\
  --cui-theme-focus-ring: {focus};\n\
  --cui-theme-selection-background: {selection_background};\n\
  --cui-theme-selection-text: {selection_text};\n\
  --cui-theme-info-surface: {info_surface};\n\
  --cui-theme-info-text: {info_text};\n\
  --cui-theme-info-border: {info_border};\n\
  --cui-theme-success-surface: {success_surface};\n\
  --cui-theme-success-text: {success_text};\n\
  --cui-theme-success-border: {success_border};\n\
  --cui-theme-warning-surface: {warning_surface};\n\
  --cui-theme-warning-text: {warning_text};\n\
  --cui-theme-warning-border: {warning_border};\n\
  --cui-theme-danger-surface: {danger_surface};\n\
  --cui-theme-danger-text: {danger_text};\n\
  --cui-theme-danger-border: {danger_border};\n\
  --cui-theme-shadow-control: {shadow_control};\n\
  --cui-theme-shadow-overlay: {shadow_overlay};\n\
  --cui-theme-shadow-raised: {shadow_raised};\n\
  --cui-surface: var(--cui-theme-surface-canvas);\n\
  --cui-surface-hover: var(--cui-theme-surface-hover);\n\
  --cui-surface-subtle: var(--cui-theme-surface-sunken);\n\
  --cui-text: var(--cui-theme-text-primary);\n\
  --cui-heading-text: var(--cui-theme-text-primary);\n\
  --cui-text-secondary: var(--cui-theme-text-secondary);\n\
  --cui-text-muted: var(--cui-theme-text-muted);\n\
  --cui-muted: var(--cui-theme-text-muted);\n\
  --cui-border: var(--cui-theme-border-subtle);\n\
  --cui-border-strong: var(--cui-theme-border-strong);\n\
  --cui-accent: var(--cui-theme-accent);\n\
  --cui-accent-hover: var(--cui-theme-accent-hover);\n\
  --cui-accent-active: var(--cui-theme-accent-active);\n\
  --cui-accent-subtle: var(--cui-theme-accent-subtle);\n\
  --cui-accent-text: var(--cui-theme-accent-text);\n\
  --cui-link: var(--cui-theme-link);\n\
  --cui-link-hover: var(--cui-theme-link-hover);\n\
  --cui-focus: var(--cui-theme-focus-ring);\n\
  --cui-danger: var(--cui-theme-danger-text);\n\
  --cui-danger-hover: var(--cui-theme-danger-text);\n\
  --cui-info: var(--cui-theme-info-text);\n\
  --cui-success: var(--cui-theme-success-text);\n\
  --cui-warning: var(--cui-theme-warning-text);\n\
  --cui-selection-background: var(--cui-theme-selection-background);\n\
  --cui-selection-text: var(--cui-theme-selection-text);\n\
  --cui-font-family-body: Geist, ui-sans-serif, system-ui, sans-serif;\n\
  --cui-font-family-mono: \"Geist Mono\", ui-monospace, SFMono-Regular, monospace;\n\
  --cui-motion-duration-fast: 120ms;\n\
  --cui-motion-duration: 200ms;\n\
  --cui-motion-duration-slow: 320ms;\n\
  --cui-motion-distance-small: 0.25rem;\n\
  --cui-motion-distance-medium: 0.5rem;\n\
  --cui-motion-scale-enter: 0.98;\n\
  --cui-motion-transition: var(--cui-motion-duration-fast) ease-out;\n\
  --cui-shadow: var(--cui-theme-shadow-control);\n";

// Values follow Toolframe's _themes.scss in its semantic-token order.
const LIGHT: &str = "surface-canvas:#fff;surface-raised:#fff;surface-overlay:#fff;surface-sunken:#f9fafb;surface-hover:#f9fafb;surface-selected:#f4ebff;text-primary:#181d27;text-secondary:#414651;text-muted:#717680;text-inverse:#fff;border-subtle:#e9eaeb;border-strong:#d5d7da;accent:#6941c6;accent-hover:#53389e;accent-active:#42307d;accent-subtle:#f4ebff;accent-text:#fff;link:#6941c6;link-hover:#53389e;focus:#9e77ed;selection-background:#e9d7fe;selection-text:#181d27;info-surface:#eff8ff;info-text:#175cd3;info-border:#b2ddff;success-surface:#ecfdf3;success-text:#067647;success-border:#abefc6;warning-surface:#fffaeb;warning-text:#b54708;warning-border:#fedf89;danger-surface:#fef3f2;danger-text:#b42318;danger-border:#fecdca;shadow-control:0 0.0625rem 0.125rem rgb(10 13 18 / 5%);shadow-overlay:0 0.25rem 0.375rem -0.125rem rgb(10 13 18 / 8%);shadow-raised:0 0.75rem 1rem -0.25rem rgb(10 13 18 / 8%)";
const DARK: &str = "surface-canvas:#0c111d;surface-raised:#161b26;surface-overlay:#1f242f;surface-sunken:#0c111d;surface-hover:#1f242f;surface-selected:#2c1c4e;text-primary:#f5f5f6;text-secondary:#d5d7da;text-muted:#94979c;text-inverse:#181d27;border-subtle:#1f242f;border-strong:#333741;accent:#7f56d9;accent-hover:#6941c6;accent-active:#53389e;accent-subtle:#2c1c4e;accent-text:#fff;link:#b692f6;link-hover:#d6bbfb;focus:#9e77ed;selection-background:#53389e;selection-text:#fff;info-surface:#102a56;info-text:#b2ddff;info-border:#1849a9;success-surface:#082e23;success-text:#75e0a7;success-border:#087443;warning-surface:#3d2605;warning-text:#fec84b;warning-border:#b54708;danger-surface:#3b1113;danger-text:#fda29b;danger-border:#b42318;shadow-control:0 0.0625rem 0.125rem rgb(0 0 0 / 32%);shadow-overlay:0 0.25rem 0.375rem -0.125rem rgb(0 0 0 / 40%);shadow-raised:0 0.75rem 1rem -0.25rem rgb(0 0 0 / 48%)";
const OCEAN: &str = "surface-canvas:#fff;surface-raised:#fff;surface-overlay:#fff;surface-sunken:#f9fafb;surface-hover:#f8fafc;surface-selected:#eff8ff;text-primary:#181d27;text-secondary:#414651;text-muted:#717680;text-inverse:#fff;border-subtle:#e9eaeb;border-strong:#d5d7da;accent:#1570ef;accent-hover:#175cd3;accent-active:#1849a9;accent-subtle:#eff8ff;accent-text:#fff;link:#175cd3;link-hover:#1849a9;focus:#2e90fa;selection-background:#d1e9ff;selection-text:#102a56;info-surface:#eff8ff;info-text:#175cd3;info-border:#b2ddff;success-surface:#ecfdf3;success-text:#067647;success-border:#abefc6;warning-surface:#fffaeb;warning-text:#b54708;warning-border:#fedf89;danger-surface:#fef3f2;danger-text:#b42318;danger-border:#fecdca;shadow-control:0 0.0625rem 0.125rem rgb(10 13 18 / 5%);shadow-overlay:0 0.25rem 0.375rem -0.125rem rgb(10 13 18 / 8%);shadow-raised:0 0.75rem 1rem -0.25rem rgb(10 13 18 / 8%)";
const FOREST: &str = "surface-canvas:#fff;surface-raised:#fff;surface-overlay:#fff;surface-sunken:#f9fafb;surface-hover:#f8fafc;surface-selected:#ecfdf3;text-primary:#181d27;text-secondary:#414651;text-muted:#717680;text-inverse:#fff;border-subtle:#e9eaeb;border-strong:#d5d7da;accent:#067647;accent-hover:#05603a;accent-active:#054f31;accent-subtle:#ecfdf3;accent-text:#fff;link:#067647;link-hover:#05603a;focus:#067647;selection-background:#d3f8df;selection-text:#074d31;info-surface:#eff8ff;info-text:#175cd3;info-border:#b2ddff;success-surface:#ecfdf3;success-text:#067647;success-border:#abefc6;warning-surface:#fffaeb;warning-text:#b54708;warning-border:#fedf89;danger-surface:#fef3f2;danger-text:#b42318;danger-border:#fecdca;shadow-control:0 0.0625rem 0.125rem rgb(10 13 18 / 5%);shadow-overlay:0 0.25rem 0.375rem -0.125rem rgb(10 13 18 / 8%);shadow-raised:0 0.75rem 1rem -0.25rem rgb(10 13 18 / 8%)";
const PLUM: &str = "surface-canvas:#fff;surface-raised:#fff;surface-overlay:#fff;surface-sunken:#f9fafb;surface-hover:#fdf2fa;surface-selected:#fdf2fa;text-primary:#181d27;text-secondary:#414651;text-muted:#717680;text-inverse:#fff;border-subtle:#e9eaeb;border-strong:#d5d7da;accent:#c11574;accent-hover:#9e165f;accent-active:#851651;accent-subtle:#fdf2fa;accent-text:#fff;link:#9e165f;link-hover:#851651;focus:#ee46bc;selection-background:#fcceee;selection-text:#4e0d30;info-surface:#eff8ff;info-text:#175cd3;info-border:#b2ddff;success-surface:#ecfdf3;success-text:#067647;success-border:#abefc6;warning-surface:#fffaeb;warning-text:#b54708;warning-border:#fedf89;danger-surface:#fef3f2;danger-text:#b42318;danger-border:#fecdca;shadow-control:0 0.0625rem 0.125rem rgb(10 13 18 / 5%);shadow-overlay:0 0.25rem 0.375rem -0.125rem rgb(10 13 18 / 8%);shadow-raised:0 0.75rem 1rem -0.25rem rgb(10 13 18 / 8%)";

/// Render the complete, opt-in reference theme stylesheet deterministically.
pub fn reference_css() -> String {
    let mut css = String::from(
        "/* Generated by catalog-core::theme::reference_css; explicit root opt-in only. */\n",
    );
    for theme in Theme::ALL {
        css.push_str(&format!(
            ":root[data-cui-theme=\"{}\"] {{\n  color-scheme: {};\n",
            theme.as_str(),
            theme.scheme()
        ));
        let values = parse_values(theme.declarations());
        let mut placeholders = COMMON.to_owned();
        for (name, value) in values {
            placeholders = placeholders.replace(&format!("{{{}}}", placeholder_name(name)), value);
        }
        css.push_str(&placeholders.replace("\n--cui", "\n  --cui"));
        css.push_str("}\n\n");
        css.push_str(&format!(
            ":root[data-cui-theme=\"{}\"]::selection {{\n  background: var(--cui-theme-selection-background);\n  color: var(--cui-theme-selection-text);\n}}\n\n",
            theme.as_str()
        ));
    }
    css.push_str("@media (prefers-reduced-motion: reduce) {\n  :root[data-cui-theme] {\n    --cui-motion-distance-small: 0;\n    --cui-motion-distance-medium: 0;\n    --cui-motion-scale-enter: 1;\n    --cui-motion-transition: var(--cui-motion-duration-fast) ease-out;\n  }\n}\n");
    css
}

fn placeholder_name(role: &str) -> &str {
    match role {
        "surface-canvas" => "canvas",
        "surface-raised" => "raised",
        "surface-overlay" => "overlay",
        "surface-sunken" => "sunken",
        "surface-hover" => "hover",
        "surface-selected" => "selected",
        "text-primary" => "primary",
        "text-secondary" => "secondary",
        "text-muted" => "muted",
        "text-inverse" => "inverse",
        "border-subtle" => "border_subtle",
        "border-strong" => "border_strong",
        "accent-hover" => "accent_hover",
        "accent-active" => "accent_active",
        "accent-subtle" => "accent_subtle",
        "accent-text" => "accent_text",
        "link-hover" => "link_hover",
        "selection-background" => "selection_background",
        "selection-text" => "selection_text",
        "info-surface" => "info_surface",
        "info-text" => "info_text",
        "info-border" => "info_border",
        "success-surface" => "success_surface",
        "success-text" => "success_text",
        "success-border" => "success_border",
        "warning-surface" => "warning_surface",
        "warning-text" => "warning_text",
        "warning-border" => "warning_border",
        "danger-surface" => "danger_surface",
        "danger-text" => "danger_text",
        "danger-border" => "danger_border",
        "shadow-control" => "shadow_control",
        "shadow-overlay" => "shadow_overlay",
        "shadow-raised" => "shadow_raised",
        other => other,
    }
}

fn parse_values(values: &str) -> Vec<(&str, &str)> {
    values
        .split(';')
        .filter_map(|entry| entry.split_once(':'))
        .collect()
}

/// Optional font-face declarations. URLs are relative to `theme/reference.css`.
pub fn font_css() -> &'static str {
    "@font-face {\n  font-family: Geist;\n  font-style: normal;\n  font-weight: 100 900;\n  font-display: swap;\n  src: url(\"../fonts/geist-sans-variable.woff2\") format(\"woff2\");\n}\n\n@font-face {\n  font-family: \"Geist Mono\";\n  font-style: normal;\n  font-weight: 100 900;\n  font-display: swap;\n  src: url(\"../fonts/geist-mono-variable.woff2\") format(\"woff2\");\n}\n"
}
