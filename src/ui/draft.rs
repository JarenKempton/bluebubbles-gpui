use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, Result};
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::*;

use crate::{
    api::{Attachment, Message},
    media::decode_image,
    omarchy::Palette,
    ui::time,
};

const THUMB: f32 = 64.;

#[derive(Clone)]
pub struct Draft {
    pub name: String,
    pub mime: String,
    pub bytes: Arc<Vec<u8>>,
    pub preview: Option<Arc<Image>>,
    pub dims: Option<Size<u32>>,
}

impl Draft {
    pub fn from_bytes(name: String, mime: String, bytes: Vec<u8>) -> Self {
        let is_image = mime.starts_with("image/");
        let dims = is_image
            .then(|| imagesize::blob_size(&bytes).ok())
            .flatten()
            .map(|d| size(d.width as u32, d.height as u32));
        let preview = is_image.then(|| decode_image(bytes.clone())).flatten();
        Self {
            name,
            mime,
            bytes: Arc::new(bytes),
            preview,
            dims,
        }
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("couldn't read {}", path.display()))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "attachment".to_string());
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        Ok(Self::from_bytes(name, mime, bytes))
    }

    pub fn from_clipboard(image: &Image) -> Self {
        let name = format!("Pasted image.{}", image.format.extension());
        Self::from_bytes(
            name,
            image.format.mime_type().to_string(),
            image.bytes.clone(),
        )
    }

    pub fn placeholder(&self, temp_guid: &str) -> Message {
        Message {
            guid: temp_guid.to_string(),
            temp_guid: Some(temp_guid.to_string()),
            is_from_me: true,
            date_created: time::now_ms(),
            attachments: vec![Attachment {
                guid: temp_guid.to_string(),
                mime_type: Some(self.mime.clone()),
                transfer_name: Some(self.name.clone()),
                width: self.dims.map(|d| d.width as f64),
                height: self.dims.map(|d| d.height as f64),
                total_bytes: Some(self.bytes.len() as i64),
                hide_attachment: false,
            }],
            ..Default::default()
        }
    }
}

pub fn file_uris(text: &str) -> Option<Vec<PathBuf>> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }
    lines
        .iter()
        .map(|line| {
            line.strip_prefix("file://")
                .map(|path| PathBuf::from(percent_decode(path)))
        })
        .collect()
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let decoded = (bytes[i] == b'%')
            .then(|| input.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match decoded {
            Some(byte) => {
                out.push(byte);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn thumbnail(draft: &Draft, p: &Palette) -> Div {
    let frame = div()
        .size(px(THUMB))
        .flex_none()
        .rounded(px(10.))
        .overflow_hidden()
        .bg(p.surface)
        .border_1()
        .border_color(p.muted.opacity(0.6));
    match &draft.preview {
        Some(image) => frame.child(
            img(ImageSource::Image(image.clone()))
                .size_full()
                .rounded(px(10.))
                .object_fit(ObjectFit::Cover),
        ),
        None => frame
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .px_1()
            .child(Icon::new(IconName::BookOpen).small().text_color(p.accent))
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_center()
                    .truncate()
                    .text_color(p.dim_foreground)
                    .child(draft.name.clone()),
            ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::file_uris;

    #[test]
    fn copied_files_become_paths_and_plain_text_does_not() {
        assert_eq!(
            file_uris("file:///home/j/My%20Photo.jpg\r\nfile:///tmp/a.pdf\r\n"),
            Some(vec![
                PathBuf::from("/home/j/My Photo.jpg"),
                PathBuf::from("/tmp/a.pdf")
            ])
        );
        assert_eq!(file_uris("see file:///tmp/a.pdf"), None);
        assert_eq!(file_uris("hello"), None);
    }
}
