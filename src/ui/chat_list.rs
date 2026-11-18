use gpui_kit::component::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{
    api::{Chat, Message},
    contacts::ContactBook,
    omarchy::palette,
    ui::time,
};

pub const ROW_HEIGHT: f32 = 32.;

pub struct ChatEntry {
    pub chat: Chat,
    pub title: SharedString,
    pub last_at: i64,
    pub unread: bool,
}

impl ChatEntry {
    pub fn build(chat: Chat, contacts: &ContactBook) -> Self {
        let last = chat.last_message.as_ref();
        let last_at = last.map(|m| m.date_created).unwrap_or_default();
        Self {
            title: title(&chat, contacts).into(),
            last_at,
            unread: last
                .is_some_and(|m| !m.is_from_me && m.date_read.is_none() && !m.is_reaction()),
            chat,
        }
    }
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

pub fn row(entry: &ChatEntry, selected: bool, cx: &App) -> Div {
    let p = palette(cx).clone();
    div().h(px(ROW_HEIGHT)).w_full().pb(px(2.)).child(
        div()
            .size_full()
            .h_flex()
            .gap_2()
            .px_2()
            .rounded(px(6.))
            .text_sm()
            .when(selected, |el| el.bg(p.row_selected))
            .when(!selected, |el| el.hover(|el| el.bg(p.row_hover)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when(entry.unread, |el| el.font_weight(FontWeight::SEMIBOLD))
                    .text_color(if entry.unread || selected {
                        p.foreground
                    } else {
                        p.dim_foreground
                    })
                    .child(entry.title.clone()),
            )
            .when(entry.last_at > 0, |el| {
                el.child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(if entry.unread {
                            p.foreground
                        } else {
                            p.dim_foreground.opacity(0.8)
                        })
                        .child(time::relative(entry.last_at)),
                )
            }),
    )
}
