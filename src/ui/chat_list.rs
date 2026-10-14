use std::sync::Arc;

use gpui_kit::component::{StyledExt as _, avatar::Avatar, theme::Theme};
use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;

use crate::{
    api::{Chat, Message},
    contacts::ContactBook,
    ui::time,
};

pub const ROW_HEIGHT: f32 = 68.;

pub struct ChatEntry {
    pub chat: Chat,
    pub title: SharedString,
    pub preview: SharedString,
    pub stamp: SharedString,
    pub unread: bool,
    pub avatar: Option<Arc<Image>>,
}

impl ChatEntry {
    pub fn build(chat: Chat, contacts: &ContactBook) -> Self {
        let last = chat.last_message.as_ref();
        let last_at = last.map(|m| m.date_created).unwrap_or_default();
        let avatar = match chat.participants.as_slice() {
            [only] if !chat.is_group() => contacts.avatar(&only.address),
            _ => None,
        };
        Self {
            title: title(&chat, contacts).into(),
            preview: last.map(preview).unwrap_or_default().into(),
            stamp: if last_at > 0 { time::list_stamp(last_at).into() } else { SharedString::default() },
            unread: last.is_some_and(|m| !m.is_from_me && m.date_read.is_none() && !m.is_reaction()),
            avatar,
            chat,
        }
    }
}

pub fn title(chat: &Chat, contacts: &ContactBook) -> String {
    if let Some(name) = chat.display_name.as_deref().filter(|n| !n.trim().is_empty()) {
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

pub fn row(entry: &ChatEntry, selected: bool, theme: &Theme) -> Div {
    let mut avatar = Avatar::new();
    let initials_source: Vec<&str> = entry.title.split([' ', ',']).filter(|w| w.starts_with(char::is_alphabetic)).collect();
    if !initials_source.is_empty() {
        avatar = avatar.name(initials_source.join(" "));
    }
    if let Some(image) = &entry.avatar {
        avatar = avatar.src(ImageSource::Image(image.clone()));
    }
    div()
        .h(px(ROW_HEIGHT))
        .px_2()
        .child(
            div()
                .size_full()
                .h_flex()
                .gap_3()
                .px_2()
                .rounded_lg()
                .when(selected, |el| el.bg(theme.list_active))
                .when(!selected, |el| el.hover(|el| el.bg(theme.list_hover)))
                .child(
                    div()
                        .w(px(8.))
                        .flex_none()
                        .child(div().when(entry.unread, |el| el.size(px(8.)).rounded_full().bg(theme.blue))),
                )
                .child(avatar)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .v_flex()
                        .gap_0p5()
                        .child(
                            div()
                                .h_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .font_semibold()
                                        .text_sm()
                                        .child(entry.title.clone()),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(entry.stamp.clone()),
                                ),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(entry.preview.clone()),
                        ),
                ),
        )
}

