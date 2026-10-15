use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    message_scroller::{MessageScroller, MessageScrollerState},
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{
    api::{Attachment, Client, Message},
    contacts::ContactBook,
    media::{self, MediaCache},
    runtime,
    ui::{message_row as row, reactions::Reactions, time},
};

const PAGE: usize = 50;

pub enum ThreadEvent {
    Sent(Message),
}

impl EventEmitter<ThreadEvent> for ThreadView {}

struct Row {
    msg: Message,
    pending: bool,
    failure: Option<SharedString>,
}

impl Row {
    fn new(msg: Message) -> Self {
        Self {
            msg,
            pending: false,
            failure: None,
        }
    }
}

pub struct ThreadView {
    client: Client,
    media: Entity<MediaCache>,
    contacts: Rc<ContactBook>,
    pub chat_guid: String,
    title: SharedString,
    subtitle: SharedString,
    is_group: bool,
    rows: Vec<Row>,
    reactions: Reactions,
    scroller: Entity<MessageScrollerState>,
    composer: Entity<InputState>,
    loading: bool,
    has_more: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl ThreadView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: Client,
        media: Entity<MediaCache>,
        contacts: Rc<ContactBook>,
        chat_guid: String,
        title: SharedString,
        subtitle: SharedString,
        is_group: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let scroller = cx.new(|cx| MessageScrollerState::new(1, cx));
        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("iMessage"));
        let subscriptions = vec![
            cx.subscribe_in(
                &composer,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => this.send(window, cx),
                    InputEvent::Change => cx.notify(),
                    _ => {}
                },
            ),
            cx.observe(&media, |this, _, cx| {
                this.scroller.update(cx, |s, cx| s.remeasure(cx))
            }),
        ];
        composer.update(cx, |state, cx| state.focus(window, cx));
        let mut this = Self {
            client,
            media,
            contacts,
            chat_guid,
            title,
            subtitle,
            is_group,
            rows: Vec::new(),
            reactions: Reactions::default(),
            scroller,
            composer,
            loading: false,
            has_more: true,
            error: None,
            _subscriptions: subscriptions,
        };
        this.load(None, cx);
        this
    }

    pub fn set_contacts(&mut self, contacts: Rc<ContactBook>, cx: &mut Context<Self>) {
        self.contacts = contacts;
        cx.notify();
    }

    pub fn focus_composer(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.composer
            .update(cx, |state, cx| state.focus(window, cx));
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let (client, guid) = (self.client.clone(), self.chat_guid.clone());
        cx.spawn(async move |this, cx| {
            let Ok(mut batch) =
                runtime::run(async move { client.messages(&guid, None, PAGE).await }).await
            else {
                return;
            };
            batch.sort_by_key(|m| m.date_created);
            this.update(cx, |this, cx| {
                batch.into_iter().for_each(|m| this.ingest(m, cx))
            })
            .ok();
        })
        .detach();
    }

    fn load(&mut self, before: Option<i64>, cx: &mut Context<Self>) {
        if self.loading || (before.is_some() && !self.has_more) {
            return;
        }
        self.loading = true;
        cx.notify();
        let (client, guid) = (self.client.clone(), self.chat_guid.clone());
        cx.spawn(async move |this, cx| {
            let result =
                runtime::run(async move { client.messages(&guid, before, PAGE).await }).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(batch) => {
                        this.error = None;
                        this.has_more = batch.len() == PAGE;
                        this.absorb_history(batch, before.is_none(), cx);
                    }
                    Err(err) => this.error = Some(format!("{err:#}").into()),
                }
                this.scroller
                    .update(cx, |s, cx| s.remeasure_items(0..1, cx));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn absorb_history(&mut self, mut batch: Vec<Message>, initial: bool, cx: &mut Context<Self>) {
        batch.sort_by_key(|m| m.date_created);
        let mut older = Vec::new();
        for msg in batch {
            if msg.is_reaction() {
                self.reactions.apply(&msg);
            } else if displayable(&msg) && !self.rows.iter().any(|r| r.msg.guid == msg.guid) {
                older.push(Row::new(msg));
            }
        }
        let count = older.len();
        self.rows.splice(0..0, older);
        self.scroller.update(cx, |s, cx| {
            if initial {
                s.reset(count + 1, cx);
            } else {
                s.splice(1..1, count, cx);
                s.remeasure_items(count + 1..(count + 2).min(s.item_count()), cx);
            }
        });
    }

    pub fn ingest(&mut self, msg: Message, cx: &mut Context<Self>) {
        if msg.is_reaction() {
            if let Some(target) = self.reactions.apply(&msg)
                && let Some(ix) = self.rows.iter().position(|r| r.msg.guid == target)
            {
                self.remeasure(ix, cx);
            }
            return;
        }
        let existing = self.rows.iter().position(|r| {
            r.msg.guid == msg.guid || msg.temp_guid.as_deref().is_some_and(|t| t == r.msg.guid)
        });
        if let Some(ix) = existing {
            self.rows[ix] = Row::new(msg);
            self.remeasure(ix, cx);
            return;
        }
        if !displayable(&msg) || msg.chat_guid().is_some_and(|g| g != self.chat_guid) {
            return;
        }
        self.insert(Row::new(msg), cx);
    }

    fn insert(&mut self, row: Row, cx: &mut Context<Self>) {
        let previous_outgoing = self.rows.iter().rposition(|r| r.msg.is_from_me);
        let ix = self
            .rows
            .partition_point(|r| r.msg.date_created <= row.msg.date_created);
        self.rows.insert(ix, row);
        self.scroller.update(cx, |s, cx| {
            s.splice(ix + 1..ix + 1, 1, cx);
        });
        if let Some(prev) = previous_outgoing.filter(|p| *p < ix) {
            self.remeasure(prev, cx);
        }
        cx.notify();
    }

    fn remeasure(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.scroller
            .update(cx, |s, cx| s.remeasure_items(ix + 1..ix + 2, cx));
        cx.notify();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.composer
            .update(cx, |state, cx| state.set_value("", window, cx));
        let temp = format!("temp-{}", uuid::Uuid::new_v4());
        let draft = Message {
            guid: temp.clone(),
            temp_guid: Some(temp.clone()),
            text: Some(text.clone()),
            is_from_me: true,
            date_created: time::now_ms(),
            ..Default::default()
        };
        self.insert(
            Row {
                msg: draft,
                pending: true,
                failure: None,
            },
            cx,
        );
        self.scroller.update(cx, |s, cx| s.scroll_to_end(cx));
        let (client, guid) = (self.client.clone(), self.chat_guid.clone());
        cx.spawn(async move |this, cx| {
            let sent_temp = temp.clone();
            let result =
                runtime::run(async move { client.send_text(&guid, &sent_temp, &text).await }).await;
            this.update(cx, |this, cx| match result {
                Ok(mut msg) => {
                    msg.temp_guid = Some(temp);
                    cx.emit(ThreadEvent::Sent(msg.clone()));
                    this.ingest(msg, cx);
                }
                Err(err) => {
                    if let Some(ix) = this.rows.iter().position(|r| r.msg.guid == temp) {
                        this.rows[ix].pending = false;
                        this.rows[ix].failure = Some(format!("{err:#}").into());
                        this.remeasure(ix, cx);
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    fn open_attachment(&self, attachment: &Attachment) {
        media::open_attachment(self.client.clone(), attachment.clone());
    }

    fn render_row(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        if ix == 0 {
            return self.render_header_row(cx).into_any_element();
        }
        let i = ix - 1;
        let Some(current) = self.rows.get(i) else {
            return div().into_any_element();
        };
        let msg = &current.msg;
        let previous = i
            .checked_sub(1)
            .and_then(|p| self.rows.get(p))
            .map(|r| &r.msg);
        let outgoing = msg.is_from_me;
        let last_outgoing = outgoing && !self.rows[i + 1..].iter().any(|r| r.msg.is_from_me);

        let mut column = div().w_full().v_flex().px_4().pt(px(2.));
        let new_section = time::needs_divider(previous.map(|p| p.date_created), msg.date_created);
        if new_section {
            column = column.child(row::divider(time::divider(msg.date_created), &theme));
        }
        if msg.item_type != 0 {
            let text = if msg.body().is_empty() {
                "Conversation updated".to_string()
            } else {
                msg.body().to_string()
            };
            return column.child(row::system(text, &theme)).into_any_element();
        }
        let sender_changed = new_section
            || previous
                .is_none_or(|p| p.is_from_me || p.item_type != 0 || sender_of(p) != sender_of(msg));
        if self.is_group && !outgoing && sender_changed {
            column = column.child(row::sender(self.contacts.display(&sender_of(msg)), &theme));
        }

        let align = |el: Div| {
            if outgoing {
                el.justify_end()
            } else {
                el.justify_start()
            }
        };
        for attachment in msg.visible_attachments() {
            let element = if attachment.is_image() {
                let loaded = self
                    .media
                    .update(cx, |m, cx| row::media_state(m.image(&attachment.guid, cx)));
                row::image(attachment, loaded, &theme)
            } else {
                row::file(attachment, outgoing, &theme)
            };
            let attachment = attachment.clone();
            column = column.child(
                align(div().w_full().flex().py_0p5()).child(
                    element
                        .id(SharedString::from(format!("att-{}", attachment.guid)))
                        .on_click(
                            cx.listener(move |this, _, _, _| this.open_attachment(&attachment)),
                        ),
                ),
            );
        }
        if !msg.body().is_empty() {
            let bubble = row::bubble(
                msg.body().to_string(),
                outgoing,
                current.failure.is_some(),
                &theme,
            )
            .when(current.pending, |el| el.opacity(0.65));
            column = column.child(align(div().w_full().flex()).child(bubble));
        }
        let emojis = self.reactions.for_message(&msg.guid);
        if !emojis.is_empty() {
            column = column
                .child(align(div().w_full().flex().px_2()).child(row::reactions(emojis, &theme)));
        }
        if let Some(failure) = &current.failure {
            column = column.child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .pr_2()
                    .text_xs()
                    .text_color(theme.danger)
                    .child(format!("Not delivered — {failure}")),
            );
        } else if last_outgoing {
            let label = if current.pending {
                "Sending…"
            } else if msg.error != 0 {
                "Not delivered"
            } else if msg.date_read.is_some() {
                "Read"
            } else if msg.date_delivered.is_some() || msg.is_delivered {
                "Delivered"
            } else {
                "Sent"
            };
            column = column.child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .child(row::status(label, &theme)),
            );
        }
        column.into_any_element()
    }

    fn render_header_row(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let content: AnyElement = if self.loading {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child("Loading…")
                .into_any_element()
        } else if let Some(error) = &self.error {
            div()
                .text_xs()
                .text_color(theme.danger)
                .child(error.clone())
                .into_any_element()
        } else if self.has_more && !self.rows.is_empty() {
            Button::new("older")
                .ghost()
                .small()
                .label("Load earlier messages")
                .on_click(cx.listener(|this, _, _, cx| {
                    let before = this.rows.first().map(|r| r.msg.date_created);
                    this.load(before, cx);
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        };
        div().w_full().flex().justify_center().py_2().child(content)
    }
}

fn tight_rows() -> StyleRefinement {
    let mut style = StyleRefinement::default();
    style.padding.bottom = Some(px(0.).into());
    style
}

fn displayable(msg: &Message) -> bool {
    msg.item_type != 0 || !msg.body().is_empty() || msg.visible_attachments().next().is_some()
}

fn sender_of(msg: &Message) -> String {
    msg.handle
        .as_ref()
        .map(|h| h.address.clone())
        .unwrap_or_default()
}

impl Render for ThreadView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let theme = cx.theme();
        let can_send = !self.composer.read(cx).value().trim().is_empty();
        div()
            .size_full()
            .v_flex()
            .bg(theme.background)
            .child(
                div()
                    .flex_none()
                    .h(px(56.))
                    .px_5()
                    .v_flex()
                    .justify_center()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(div().font_semibold().child(self.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(self.subtitle.clone()),
                    ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    MessageScroller::new(
                        "messages",
                        self.scroller.clone(),
                        move |ix, window, cx| {
                            view.update(cx, |this, cx| this.render_row(ix, window, cx))
                        },
                    )
                    .with_row_style(tight_rows())
                    .size_full(),
                ),
            )
            .child(
                div()
                    .flex_none()
                    .h_flex()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.composer).rounded_full()),
                    )
                    .child(
                        Button::new("send")
                            .icon(IconName::ArrowUp)
                            .primary()
                            .rounded_full()
                            .disabled(!can_send)
                            .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))),
                    ),
            )
    }
}
