use std::sync::Arc;

use gpui_kit::component::StyledExt as _;
use gpui_kit::*;

use crate::{api::Attachment, media::Media, omarchy::Palette};

const TEXT_MAX_WIDTH: f32 = 640.;
const CARD_MAX_WIDTH: f32 = 540.;
const RADIUS: f32 = 12.;
const IMAGE_MAX_WIDTH: f32 = 280.;
const IMAGE_MAX_HEIGHT: f32 = 360.;

pub fn divider(label: String, p: &Palette) -> Div {
    let rule = || div().flex_1().h(px(1.)).bg(p.muted);
    div()
        .w_full()
        .pt_2()
        .pb_1()
        .h_flex()
        .gap_3()
        .text_xs()
        .text_color(p.dim_foreground)
        .child(rule())
        .child(label)
        .child(rule())
}

pub fn meta(name: String, time: String, p: &Palette) -> Div {
    div()
        .h_flex()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.foreground)
                .child(name),
        )
        .child(div().text_xs().text_color(p.dim_foreground).child(time))
}

pub fn system(text: String, p: &Palette) -> Div {
    div()
        .w_full()
        .py_1()
        .flex()
        .justify_center()
        .text_xs()
        .italic()
        .text_color(p.dim_foreground)
        .child(text)
}

pub fn text(text: String, p: &Palette) -> Div {
    div()
        .max_w(px(TEXT_MAX_WIDTH))
        .text_sm()
        .line_height(px(22.))
        .text_color(p.foreground)
        .child(text)
}

pub fn card(text: String, failed: bool, p: &Palette) -> Div {
    let (bg, border, fg) = if failed {
        (p.red.opacity(0.12), p.red.opacity(0.4), p.red)
    } else {
        (p.surface, p.muted, p.foreground)
    };
    div()
        .max_w(px(CARD_MAX_WIDTH))
        .px_3()
        .py(px(7.))
        .rounded(px(RADIUS))
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_sm()
        .line_height(px(22.))
        .text_color(fg)
        .child(text)
}

pub enum LoadedImage {
    Loading,
    Ready(Arc<Image>, Option<Size<u32>>),
    Unavailable(&'static str),
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

pub fn image(attachment: &Attachment, loaded: LoadedImage, p: &Palette) -> Div {
    let frame = |dims: Size<Pixels>| {
        div()
            .w(dims.width)
            .h(dims.height)
            .rounded(px(RADIUS))
            .overflow_hidden()
            .bg(p.lighter_background)
    };
    match loaded {
        LoadedImage::Ready(image, dims) => {
            frame(image_size(attachment, dims)).cursor_pointer().child(
                img(ImageSource::Image(image))
                    .size_full()
                    .rounded(px(RADIUS))
                    .object_fit(ObjectFit::Cover),
            )
        }
        LoadedImage::Loading => frame(image_size(attachment, None)),
        LoadedImage::Unavailable(reason) => div()
            .h_flex()
            .gap_2()
            .px_3()
            .py_2()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(p.muted)
            .text_xs()
            .text_color(p.dim_foreground)
            .child(div().text_color(p.yellow).child("\u{f0c2}"))
            .child(format!(
                "{} — {reason}",
                attachment.transfer_name.as_deref().unwrap_or("Image")
            )),
    }
}

pub fn file(attachment: &Attachment, p: &Palette) -> Div {
    let name = attachment
        .transfer_name
        .clone()
        .unwrap_or_else(|| "Attachment".to_string());
    let size = attachment.total_bytes.map(human_size).unwrap_or_default();
    div()
        .max_w(px(CARD_MAX_WIDTH))
        .h_flex()
        .gap_2()
        .px_3()
        .py_2()
        .rounded(px(RADIUS))
        .border_1()
        .border_color(p.muted)
        .cursor_pointer()
        .hover(|el| el.bg(p.foreground.opacity(0.06)))
        .child(div().text_color(p.accent).child("\u{f0c6}"))
        .child(
            div()
                .text_sm()
                .text_color(p.bright_foreground)
                .truncate()
                .child(name),
        )
        .child(div().text_xs().text_color(p.dim_foreground).child(size))
}

pub fn reactions(emojis: Vec<&str>, p: &Palette) -> Div {
    div()
        .h(px(22.))
        .px_2()
        .h_flex()
        .gap_1()
        .rounded_full()
        .border_1()
        .border_color(p.muted)
        .text_xs()
        .text_color(p.dim_foreground)
        .children(emojis.into_iter().map(|e| e.to_string()))
}

pub fn status(label: String, failed: bool, p: &Palette) -> Div {
    div()
        .w_full()
        .flex()
        .justify_end()
        .pt_1()
        .text_xs()
        .text_color(if failed { p.red } else { p.dim_foreground })
        .child(label)
}

pub fn media_state(media: &Media) -> LoadedImage {
    match media {
        Media::Ready(image, dims) => LoadedImage::Ready(image.clone(), *dims),
        Media::Loading => LoadedImage::Loading,
        Media::NotOnMac => LoadedImage::Unavailable("only in iCloud, not on your Mac"),
        Media::Failed => LoadedImage::Unavailable("couldn't load"),
    }
}

fn human_size(bytes: i64) -> String {
    match bytes {
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1 << 20) as f64),
        b if b >= 1 << 10 => format!("{} KB", b >> 10),
        b => format!("{b} B"),
    }
}
