use std::sync::Arc;

use gpui_kit::*;

use crate::omarchy::palette;

#[derive(Clone, Default)]
pub struct Face {
    pub name: SharedString,
    pub image: Option<Arc<Image>>,
}

fn initials(name: &str) -> String {
    name.split([' ', ','])
        .filter(|w| w.starts_with(char::is_alphabetic))
        .take(2)
        .filter_map(|w| w.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

fn tint_index(name: &str) -> usize {
    name.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    }) % 6
}

pub fn face(face: &Face, size: Pixels, cx: &App) -> Div {
    let p = palette(cx);
    let radius = size / 2.;
    let base = div()
        .size(size)
        .flex_none()
        .rounded(radius)
        .overflow_hidden();
    if let Some(image) = &face.image {
        return base.child(
            img(ImageSource::Image(image.clone()))
                .size_full()
                .rounded(radius)
                .object_fit(ObjectFit::Cover),
        );
    }
    let text = initials(&face.name);
    let tint = p.identity[tint_index(&face.name)];
    base.flex()
        .items_center()
        .justify_center()
        .bg(tint.opacity(0.16))
        .text_color(tint)
        .text_size(size * 0.38)
        .font_weight(FontWeight::BOLD)
        .child(if text.is_empty() {
            "\u{f007}".to_string()
        } else {
            text
        })
}

pub fn group(faces: &[Face], icon: Option<Arc<Image>>, size: Pixels, ring: Hsla, cx: &App) -> Div {
    if icon.is_some() {
        return face(
            &Face {
                name: SharedString::default(),
                image: icon,
            },
            size,
            cx,
        );
    }
    match faces {
        [] => face(&Face::default(), size, cx),
        [only] => face(only, size, cx),
        [first, second, ..] => {
            let small = size * 0.6;
            div()
                .size(size)
                .flex_none()
                .relative()
                .child(face(second, small, cx).absolute().top_0().right_0())
                .child(
                    face(first, small + px(3.), cx)
                        .absolute()
                        .bottom(px(-2.))
                        .left(px(-2.))
                        .border_2()
                        .border_color(ring),
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn initials_skip_numbers_and_punctuation() {
        assert_eq!(initials("Curt Kempton"), "CK");
        assert_eq!(initials("Logan, (480) 328-6541"), "L");
        assert_eq!(initials("(480) 328-6541"), "");
    }
}
