use std::sync::Arc;

use gpui_kit::component::{StyledExt as _, theme::Theme};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{api::Attachment, media::Media};

pub const BUBBLE_MAX_WIDTH: f32 = 460.;
const IMAGE_MAX_WIDTH: f32 = 280.;
const IMAGE_MAX_HEIGHT: f32 = 360.;

pub fn outgoing_blue() -> Hsla {
    rgb(0x0a84ff).into()
}

pub fn divider(label: String, theme: &Theme) -> Div {
    div()
        .w_full()
        .pt_4()
        .pb_1()
        .flex()
        .justify_center()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(label)
}

pub fn sender(name: String, theme: &Theme) -> Div {
    div()
        .pl_3()
        .pt_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(name)
}

pub fn system(text: String, theme: &Theme) -> Div {
    div()
        .w_full()
        .py_1()
        .flex()
        .justify_center()
        .text_xs()
        .italic()
        .text_color(theme.muted_foreground)
        .child(text)
}

pub fn bubble(text: String, outgoing: bool, failed: bool, theme: &Theme) -> Div {
    let (bg, fg) = match (outgoing, failed) {
        (_, true) => (theme.danger, theme.danger_foreground),
        (true, false) => (outgoing_blue(), gpui_kit::white()),
        (false, false) => (theme.secondary, theme.secondary_foreground),
    };
    div()
        .max_w(px(BUBBLE_MAX_WIDTH))
        .px_3()
        .py_1p5()
        .rounded(px(18.))
        .bg(bg)
        .text_color(fg)
        .text_sm()
        .child(text)
}

pub struct LoadedImage {
    pub image: Option<Arc<Image>>,
    pub dims: Option<Size<u32>>,
    pub failed: bool,
}

pub fn image_size(attachment: &Attachment, loaded: Option<Size<u32>>) -> Size<Pixels> {
    let (w, h) = match (attachment.width, attachment.height, loaded) {
        (Some(w), Some(h), _) if w > 0. && h > 0. => (w as f32, h as f32),
        (_, _, Some(d)) if d.width > 0 && d.height > 0 => (d.width as f32, d.height as f32),
        _ => (240., 180.),
    };
    let scale = (IMAGE_MAX_WIDTH / w).min(IMAGE_MAX_HEIGHT / h).min(1.);
    size(px((w * scale).max(60.)), px((h * scale).max(60.)))
}

pub fn image(attachment: &Attachment, loaded: LoadedImage, theme: &Theme) -> Div {
    let dims = image_size(attachment, loaded.dims);
    let frame = div()
        .w(dims.width)
        .h(dims.height)
        .rounded(px(16.))
        .overflow_hidden()
        .bg(theme.muted)
        .cursor_pointer();
    match loaded.image {
        Some(image) => frame.child(
            img(ImageSource::Image(image))
                .size_full()
                .object_fit(ObjectFit::Cover),
        ),
        None => frame
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(if loaded.failed {
                "Couldn't load image"
            } else {
                "Loading…"
            }),
    }
}

pub fn file(attachment: &Attachment, outgoing: bool, theme: &Theme) -> Div {
    let name = attachment
        .transfer_name
        .clone()
        .unwrap_or_else(|| "Attachment".to_string());
    let size = attachment.total_bytes.map(human_size).unwrap_or_default();
    div()
        .max_w(px(BUBBLE_MAX_WIDTH))
        .h_flex()
        .gap_2()
        .px_3()
        .py_2()
        .rounded(px(14.))
        .border_1()
        .border_color(theme.border)
        .when(outgoing, |el| el.bg(theme.secondary))
        .cursor_pointer()
        .child(div().text_sm().child("📎"))
        .child(div().text_sm().truncate().child(name))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(size),
        )
}

pub fn reactions(emojis: Vec<&str>, theme: &Theme) -> Div {
    div()
        .h_flex()
        .gap_0p5()
        .px_1p5()
        .py_0p5()
        .rounded_full()
        .bg(theme.secondary)
        .border_1()
        .border_color(theme.background)
        .text_xs()
        .children(emojis.into_iter().map(|e| e.to_string()))
}

pub fn status(label: &'static str, theme: &Theme) -> Div {
    div()
        .pr_2()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(label)
}

pub fn media_state(media: &Media) -> LoadedImage {
    match media {
        Media::Ready(image, dims) => LoadedImage {
            image: Some(image.clone()),
            dims: *dims,
            failed: false,
        },
        Media::Loading => LoadedImage {
            image: None,
            dims: None,
            failed: false,
        },
        Media::Failed => LoadedImage {
            image: None,
            dims: None,
            failed: true,
        },
    }
}

fn human_size(bytes: i64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1 << 20) as f64),
        b if b >= 1 << 10 => format!("{} KB", b >> 10),
        b => format!("{b} B"),
    }
}
