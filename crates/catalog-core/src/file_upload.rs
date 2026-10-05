//! Typed static file-upload presentation. File bytes and transport remain host/application-owned.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileUploadInstance {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub drop_label: Option<String>,
    #[serde(default)]
    pub file_list_label: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub accept: Option<String>,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub files: Vec<FileRow>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileRow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub meta: Option<String>,
    pub status: FileStatus,
    pub status_label: String,
    #[serde(default)]
    pub action: Option<FileAction>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "lowercase", deny_unknown_fields)]
pub enum FileStatus {
    Ready,
    Uploading { progress: u8 },
    Complete,
    Failed,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileAction {
    pub label: String,
    pub href: String,
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
}
fn plain(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}
fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn safe_href(value: &str) -> bool {
    if value.is_empty()
        || value != value.trim()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return false;
    }
    if value.starts_with('/') {
        return !value.starts_with("//");
    }
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}
fn valid_accept(value: &str) -> bool {
    let parts: Vec<_> = value.split(',').collect();
    !parts.is_empty()
        && parts.len() <= 32
        && parts.iter().all(|part| {
            let part = part.trim();
            if let Some(ext) = part.strip_prefix('.') {
                !ext.is_empty()
                    && ext.len() <= 32
                    && ext
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            } else {
                let Some((kind, subtype)) = part.split_once('/') else {
                    return false;
                };
                !kind.is_empty()
                    && !subtype.is_empty()
                    && kind
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
                    && (subtype == "*"
                        || subtype
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'+')))
            }
        })
}
impl FileUploadInstance {
    pub fn parse(json: &str) -> Result<Self, String> {
        let value: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), String> {
        let id = self.id.as_deref().unwrap_or(&self.name);
        if !valid_token(&self.name) || !valid_token(id) || !plain(&self.label) {
            return Err("file upload requires safe name/id tokens and a nonblank label".into());
        }
        for text in [
            &self.prompt,
            &self.drop_label,
            &self.file_list_label,
            &self.hint,
            &self.error,
        ]
        .into_iter()
        .flatten()
        {
            if !plain(text) {
                return Err(
                    "file upload labels and descriptions must be nonblank plain text".into(),
                );
            }
        }
        if self.accept.as_ref().is_some_and(|v| !valid_accept(v)) {
            return Err("accept must contain only MIME types or file extensions".into());
        }
        if self.files.len() > 25 {
            return Err("file upload supports at most 25 server-owned file rows".into());
        }
        let mut ids = BTreeSet::new();
        for file in &self.files {
            if !valid_token(&file.id)
                || !ids.insert(&file.id)
                || file.id == id
                || !plain(&file.name)
                || !plain(&file.status_label)
            {
                return Err(
                    "file rows require unique safe IDs, names, and localized status labels".into(),
                );
            }
            if file.meta.as_ref().is_some_and(|v| !plain(v)) {
                return Err("file metadata must be nonblank plain text".into());
            }
            if let FileStatus::Uploading { progress } = &file.status {
                if *progress > 100 {
                    return Err("upload progress must be in 0..=100".into());
                }
            }
            if let Some(action) = &file.action {
                if !plain(&action.label) || !safe_href(&action.href) {
                    return Err(
                        "file actions require a label and safe HTTPS or app-relative href".into(),
                    );
                }
            }
        }
        Ok(())
    }
}
fn truthy(value: bool, attr: &str) -> String {
    if value {
        format!(" {attr}")
    } else {
        String::new()
    }
}
/// Render the validated instance into the package fragment. The fragment must expose every named slot once.
pub fn render(instance: &FileUploadInstance, fragment: &str) -> Result<String, String> {
    instance.validate()?;
    let id = instance.id.as_deref().unwrap_or(&instance.name);
    let prompt = instance.prompt.as_deref().unwrap_or(if instance.multiple {
        "Choose files"
    } else {
        "Choose a file"
    });
    let description = instance.hint.is_some() || instance.error.is_some();
    let input = format!(
        "<label class=\"cui-file-upload__label\" for=\"{}\">{}{}</label><div class=\"cui-file-upload__dropzone\" data-cui-file-upload-dropzone><strong>{}</strong><span>{}</span><input class=\"cui-file-upload__input\" id=\"{}\" name=\"{}\" type=\"file\"{}{}{}{}{}{} data-cui-file-upload-input><ul class=\"cui-file-upload__local-selection\" data-cui-file-upload-local-selection aria-live=\"polite\" hidden></ul><p class=\"cui-file-upload__local-feedback\" data-cui-file-upload-local-feedback data-cui-selection-error-label=\"Files could not be handed to the application.\" data-cui-cancel-error-label=\"File selection could not be cancelled.\" role=\"status\" hidden></p></div>",
        esc(id),
        esc(&instance.label),
        if instance.required {
            " <span aria-hidden=\"true\">*</span>"
        } else {
            ""
        },
        esc(prompt),
        esc(instance.drop_label.as_deref().unwrap_or("or drag and drop")),
        esc(id),
        esc(&instance.name),
        instance
            .accept
            .as_ref()
            .map(|a| format!(" accept=\"{}\"", esc(a)))
            .unwrap_or_default(),
        truthy(instance.multiple, "multiple"),
        truthy(instance.required, "required"),
        truthy(instance.disabled, "disabled"),
        if instance.error.is_some() {
            " aria-invalid=\"true\""
        } else {
            ""
        },
        if description {
            format!(" aria-describedby=\"{}-description\"", esc(id))
        } else {
            String::new()
        }
    );
    let mut files = String::new();
    if !instance.files.is_empty() {
        files.push_str(&format!(
            "<ul class=\"cui-file-upload__files\" aria-label=\"{}\">",
            esc(instance
                .file_list_label
                .as_deref()
                .unwrap_or("Selected files"))
        ));
        for file in &instance.files {
            let (status, progress) = match &file.status {
                FileStatus::Ready => ("ready", None),
                FileStatus::Uploading { progress } => ("uploading", Some(*progress)),
                FileStatus::Complete => ("complete", None),
                FileStatus::Failed => ("failed", None),
            };
            files.push_str(&format!("<li id=\"{}\" class=\"cui-file-upload__file cui-file-upload__file--{}\"><span class=\"cui-file-upload__name\">{}</span>{}<span class=\"cui-file-upload__status\">{}</span>{}", esc(&file.id), status, esc(&file.name), file.meta.as_ref().map(|v| format!("<span class=\"cui-file-upload__meta\">{}</span>", esc(v))).unwrap_or_default(), esc(&file.status_label), progress.map(|p| format!("<progress value=\"{p}\" max=\"100\" aria-label=\"{}\"></progress>", esc(&file.status_label))).unwrap_or_default()));
            if let Some(action) = &file.action {
                files.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    esc(&action.href),
                    esc(&action.label)
                ));
            }
            files.push_str("</li>");
        }
        files.push_str("</ul>");
    }
    let mut description_markup = String::new();
    if description {
        description_markup.push_str(&format!(
            "<div class=\"cui-file-upload__description\" id=\"{}-description\">",
            esc(id)
        ));
        if let Some(error) = &instance.error {
            description_markup.push_str(&format!(
                "<p class=\"cui-file-upload__error\">{}</p>",
                esc(error)
            ));
        }
        if let Some(hint) = &instance.hint {
            description_markup.push_str(&format!(
                "<p class=\"cui-file-upload__hint\">{}</p>",
                esc(hint)
            ));
        }
        description_markup.push_str("</div>");
    }
    let attrs = format!(
        "class=\"cui-file-upload{}{}\" data-cui-component=\"file-upload\"{}{}",
        if instance.error.is_some() {
            " cui-file-upload--invalid"
        } else {
            ""
        },
        if instance.disabled {
            " cui-file-upload--disabled"
        } else {
            ""
        },
        if instance.error.is_some() {
            " aria-invalid=\"true\""
        } else {
            ""
        },
        if instance.accept.is_some() {
            format!(
                " data-cui-accept=\"{}\"",
                esc(instance.accept.as_deref().unwrap())
            )
        } else {
            String::new()
        }
    );
    crate::fragment::fill(
        fragment,
        &[
            ("[[attributes]]", &attrs),
            ("[[field]]", &input),
            ("[[description]]", &description_markup),
            ("[[files]]", &files),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const FRAGMENT: &str = "<div [[attributes]]>[[field]][[description]][[files]]</div>";
    fn instance() -> FileUploadInstance {
        FileUploadInstance {
            name: "attachments".into(),
            id: None,
            label: "Attachments".into(),
            prompt: None,
            drop_label: None,
            file_list_label: None,
            hint: Some("PDF only".into()),
            accept: Some(".pdf,application/pdf".into()),
            multiple: true,
            required: false,
            disabled: false,
            error: None,
            files: vec![FileRow {
                id: "file-1".into(),
                name: "a<&>.pdf".into(),
                meta: None,
                status: FileStatus::Uploading { progress: 50 },
                status_label: "Uploading".into(),
                action: None,
            }],
        }
    }
    #[test]
    fn escapes_text_and_renders_server_status_only() {
        let h = render(&instance(), FRAGMENT).unwrap();
        assert!(h.contains("a&lt;&amp;&gt;.pdf"));
        assert!(h.contains("value=\"50\""));
        assert!(h.contains("type=\"file\""));
    }
    #[test]
    fn rejects_invalid_accept_progress_ids_and_urls() {
        let mut x = instance();
        x.accept = Some("image/*;onload=x".into());
        assert!(x.validate().is_err());
        let mut x = instance();
        x.files[0].status = FileStatus::Uploading { progress: 101 };
        assert!(x.validate().is_err());
        let mut x = instance();
        x.files[0].action = Some(FileAction {
            label: "Open".into(),
            href: "javascript:alert(1)".into(),
        });
        assert!(x.validate().is_err());
    }
    #[test]
    fn requires_exact_fragment_slots() {
        assert!(render(&instance(), "[[attributes]]").is_err());
    }
}
