use std::sync::Arc;

use gpui_kit::component::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{
    api::{Chat, Message},
    contacts::ContactBook,
    omarchy::palette,
    ui::{
        avatar::{self, Face},
        time,
    },
};

pub const ROW_HEIGHT: f32 = 64.;

pub struct ChatEntry {
    pub chat: Chat,
    pub title: SharedString,
    pub preview: SharedString,
    pub stamp: SharedString,
    pub unread: bool,
    pub faces: Vec<Face>,
}

impl ChatEntry {
    pub fn build(chat: Chat, contacts: &ContactBook) -> Self {
        let last = chat.last_message.as_ref();
        let last_at = last.map(|m| m.date_created).unwrap_or_default();
        Self {
            title: title(&chat, contacts).into(),
            preview: last.map(preview).unwrap_or_default().into(),
            stamp: if last_at > 0 {
                time::list_stamp(last_at).into()
            } else {
                SharedString::default()
            },
            unread: last
                .is_some_and(|m| !m.is_from_me && m.date_read.is_none() && !m.is_reaction()),
            faces: faces(&chat, contacts),
            chat,
        }
    }
}

pub fn face_for(address: &str, contacts: &ContactBook) -> Face {
    Face {
        name: contacts
            .name(address)
            .unwrap_or_default()
            .to_string()
            .into(),
        image: contacts.avatar(address),
    }
}

fn faces(chat: &Chat, contacts: &ContactBook) -> Vec<Face> {
    let mut faces: Vec<Face> = chat
        .participants
        .iter()
        .map(|p| face_for(&p.address, contacts))
        .collect();
    faces.sort_by_key(|f| (f.image.is_none(), f.name.is_empty()));
    faces.truncate(2);
    faces
}

pub fn title(chat: &Chat, contacts: &ContactBook) -> String {
    if let Some(name) = chat
        .display_name
        .as_deref()
        .filter(|n| !n.trim().is_empty())
    {
        return name.trim().to_string();
    }
    if !chat.participants.is_empty() {
        let short = chat.is_group();
        return chat
            .participants
            .iter()
            .map(|p| match contacts.name(&p.address) {
                Some(name) if short => name.split(' ').next().unwrap_or(name).to_string(),
                _ => contacts.display(&p.address),
            })
            .collect::<Vec<_>>()
            .join(", ");
    }
    contacts.display(chat.chat_identifier.as_deref().unwrap_or(&chat.guid))
}

pub fn preview(msg: &Message) -> String {
    let body = msg.body();
    if !body.is_empty() {
        return body.lines().next().unwrap_or(body).to_string();
    }
    match msg.visible_attachments().count() {
        0 => String::new(),
        1 if msg.attachments.iter().any(|a| a.is_image()) => "Image".to_string(),
        1 => "Attachment".to_string(),
        n => format!("{n} attachments"),
    }
}

pub fn chat_avatar(
    entry: &ChatEntry,
    icon: Option<Arc<Image>>,
    size: Pixels,
    ring: Hsla,
    cx: &App,
) -> Div {
    if entry.chat.is_group() {
        avatar::group(&entry.faces, icon, size, ring, cx)
    } else {
        let fallback = Face {
            name: entry.title.clone(),
            image: None,
        };
        avatar::face(
            entry
                .faces
                .first()
                .filter(|f| f.image.is_some())
                .unwrap_or(&fallback),
            size,
            cx,
        )
    }
}

pub fn row(entry: &ChatEntry, icon: Option<Arc<Image>>, selected: bool, cx: &App) -> Div {
    let p = palette(cx).clone();
    div().h(px(ROW_HEIGHT)).w_full().pb(px(2.)).child(
        div()
            .size_full()
            .h_flex()
            .gap_2p5()
            .px(px(10.))
            .rounded(px(8.))
            .when(selected, |el| el.bg(p.row_selected))
            .when(!selected, |el| el.hover(|el| el.bg(p.row_hover)))
            .child(chat_avatar(
                entry,
                icon,
                px(36.),
                if selected { p.row_selected } else { p.sidebar },
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .v_flex()
                    .gap(px(2.))
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(if entry.unread {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(p.foreground.opacity(if entry.unread || selected {
                                        1.
                                    } else {
                                        0.9
                                    }))
                                    .child(entry.title.clone()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_xs()
                                    .text_color(if entry.unread {
                                        p.accent
                                    } else {
                                        p.dim_foreground
                                    })
                                    .child(entry.stamp.clone()),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_xs()
                                    .text_color(p.dim_foreground)
                                    .child(entry.preview.clone()),
                            )
                            .when(entry.unread, |el| {
                                el.child(div().flex_none().size(px(7.)).rounded_full().bg(p.accent))
                            }),
                    ),
            ),
    )
}
