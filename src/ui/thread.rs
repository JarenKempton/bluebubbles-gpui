use std::{path::PathBuf, rc::Rc, sync::Arc};

use gpui_kit::component::{
    Icon, IconName, Sizable as _, StyledExt as _,
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
    omarchy::{Palette, palette},
    runtime,
    ui::{
        avatar::{self, Face},
        chat_list::face_for,
        draft::{self, Draft},
        message_row as row,
        reactions::Reactions,
        time,
    },
};

pub struct ThreadHeader {
    pub title: SharedString,
    pub subtitle: SharedString,
    pub is_group: bool,
    pub faces: Vec<Face>,
}

const PAGE: usize = 50;
const COLUMN_WIDTH: f32 = 800.;

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
    header: ThreadHeader,
    rows: Vec<Row>,
    reactions: Reactions,
    scroller: Entity<MessageScrollerState>,
    composer: Entity<InputState>,
    loading: bool,
    has_more: bool,
    error: Option<SharedString>,
    drafts: Vec<Draft>,
    composer_error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl ThreadView {
    pub fn new(
        client: Client,
        media: Entity<MediaCache>,
        contacts: Rc<ContactBook>,
        chat_guid: String,
        header: ThreadHeader,
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
            header,
            rows: Vec::new(),
            reactions: Reactions::default(),
            scroller,
            composer,
            loading: false,
            has_more: true,
            error: None,
            drafts: Vec::new(),
            composer_error: None,
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
        if text.is_empty() && self.drafts.is_empty() {
            return;
        }
        self.composer
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.composer_error = None;
        let mut jobs = Vec::new();
        for draft in std::mem::take(&mut self.drafts) {
            let temp = temp_guid();
            if let Some(image) = &draft.preview {
                self.media
                    .update(cx, |m, _| m.insert_local(&temp, image.clone(), draft.dims));
            }
            self.insert(
                Row {
                    msg: draft.placeholder(&temp),
                    pending: true,
                    failure: None,
                },
                cx,
            );
            jobs.push((
                temp,
                Outgoing::File {
                    name: draft.name,
                    mime: draft.mime,
                    bytes: draft.bytes,
                },
            ));
        }
        if !text.is_empty() {
            let temp = temp_guid();
            let msg = Message {
                guid: temp.clone(),
                temp_guid: Some(temp.clone()),
                text: Some(text.clone()),
                is_from_me: true,
                date_created: time::now_ms(),
                ..Default::default()
            };
            self.insert(
                Row {
                    msg,
                    pending: true,
                    failure: None,
                },
                cx,
            );
            jobs.push((temp, Outgoing::Text(text)));
        }
        self.scroller.update(cx, |s, cx| s.scroll_to_end(cx));
        let (client, guid) = (self.client.clone(), self.chat_guid.clone());
        cx.spawn(async move |this, cx| {
            for (temp, job) in jobs {
                let (client, guid, sent_temp) = (client.clone(), guid.clone(), temp.clone());
                let result = runtime::run(async move {
                    match job {
                        Outgoing::Text(text) => client.send_text(&guid, &sent_temp, &text).await,
                        Outgoing::File { name, mime, bytes } => {
                            client
                                .send_attachment(&guid, &sent_temp, &name, &mime, (*bytes).clone())
                                .await
                        }
                    }
                })
                .await;
                if this
                    .update(cx, |this, cx| this.finish_send(temp, result, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn finish_send(
        &mut self,
        temp: String,
        result: anyhow::Result<Message>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(mut msg) => {
                msg.temp_guid = Some(temp);
                cx.emit(ThreadEvent::Sent(msg.clone()));
                self.ingest(msg, cx);
            }
            Err(err) => {
                if let Some(ix) = self.rows.iter().position(|r| r.msg.guid == temp) {
                    self.rows[ix].pending = false;
                    self.rows[ix].failure = Some(format!("{err:#}").into());
                    self.remeasure(ix, cx);
                }
            }
        }
    }

    fn paste(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let mut handled = false;
        for entry in item.entries() {
            match entry {
                ClipboardEntry::Image(image) => {
                    self.drafts.push(Draft::from_clipboard(image));
                    handled = true;
                }
                ClipboardEntry::ExternalPaths(paths) => {
                    self.add_paths(paths.paths().to_vec(), cx);
                    handled = true;
                }
                ClipboardEntry::String(text) => {
                    if let Some(paths) = draft::file_uris(text.text()) {
                        self.add_paths(paths, cx);
                        handled = true;
                    }
                }
            }
        }
        cx.notify();
        handled
    }

    fn pick_files(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await {
                this.update(cx, |this, cx| this.add_paths(paths, cx)).ok();
            }
        })
        .detach();
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let loaded = runtime::run(async move {
                paths
                    .iter()
                    .filter(|p| p.is_file())
                    .map(|p| Draft::from_path(p))
                    .collect::<Vec<_>>()
            })
            .await;
            this.update(cx, |this, cx| {
                for result in loaded {
                    match result {
                        Ok(draft) => this.drafts.push(draft),
                        Err(err) => this.composer_error = Some(format!("{err:#}").into()),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn remove_draft(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.drafts.len() {
            self.drafts.remove(ix);
            cx.notify();
        }
    }

    fn open_attachment(&self, attachment: &Attachment) {
        media::open_attachment(self.client.clone(), attachment.clone());
    }

    fn render_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(COLUMN_WIDTH))
                    .child(self.render_message(ix, window, cx)),
            )
            .into_any_element()
    }

    fn render_message(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if ix == 0 {
            return self.render_header_row(cx).into_any_element();
        }
        let p = palette(cx).clone();
        let i = ix - 1;
        let Some(current) = self.rows.get(i) else {
            return div().into_any_element();
        };
        let msg = &current.msg;
        let previous = i
            .checked_sub(1)
            .and_then(|p| self.rows.get(p))
            .map(|r| &r.msg);
        let next = self.rows.get(i + 1).map(|r| &r.msg);
        let outgoing = msg.is_from_me;
        let last_outgoing = outgoing && !self.rows[i + 1..].iter().any(|r| r.msg.is_from_me);

        let new_section = time::needs_divider(previous.map(|p| p.date_created), msg.date_created);
        let continues = !new_section
            && previous.is_some_and(|prev| {
                prev.item_type == 0
                    && prev.is_from_me == msg.is_from_me
                    && (outgoing || sender_of(prev) == sender_of(msg))
            });
        let mut column = div()
            .w_full()
            .v_flex()
            .px_6()
            .pt(px(if continues { 3. } else { 12. }));
        if new_section {
            column = column.child(row::divider(time::divider(msg.date_created), &p));
        }
        if msg.item_type != 0 {
            let text = if msg.body().is_empty() {
                "Conversation updated".to_string()
            } else {
                msg.body().to_string()
            };
            return column.child(row::system(text, &p)).into_any_element();
        }
        let same_run = |other: &Message| {
            !other.is_from_me && other.item_type == 0 && sender_of(other) == sender_of(msg)
        };
        let first_in_run = new_section || previous.is_none_or(|prev| !same_run(prev));
        let last_in_run = next.is_none_or(|n| {
            !same_run(n) || time::needs_divider(Some(msg.date_created), n.date_created)
        });
        let gutter = self.header.is_group && !outgoing;
        let sender = sender_of(msg);
        let face = face_for(&sender, &self.contacts);
        let tint_name = self.contacts.display(&sender);

        let align = |el: Div| {
            if outgoing {
                el.justify_end()
            } else {
                el.justify_start()
            }
        };
        let mut content = div().flex_1().min_w_0().v_flex().gap(px(3.));
        if gutter && first_in_run {
            content = content.child(row::sender(tint_name.clone(), p.dim_foreground));
        }
        for attachment in msg.visible_attachments() {
            let element = if attachment.is_image() {
                let loaded = self
                    .media
                    .update(cx, |m, cx| row::media_state(m.image(&attachment.guid, cx)));
                row::image(attachment, loaded, &p)
            } else {
                row::file(attachment, &p)
            };
            let attachment = attachment.clone();
            content = content.child(
                align(div().w_full().flex()).child(
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
                &p,
            )
            .when(current.pending, |el| el.opacity(0.6));
            content = content.child(align(div().w_full().flex()).child(bubble));
        }
        let emojis = self.reactions.for_message(&msg.guid);
        if !emojis.is_empty() {
            content = content
                .child(align(div().w_full().flex().px_2()).child(row::reactions(emojis, &p)));
        }

        column = if gutter {
            let slot = div().w(px(28.)).flex_none().when(last_in_run, |el| {
                el.child(avatar::face(
                    &Face {
                        name: tint_name.into(),
                        image: face.image,
                    },
                    px(28.),
                    cx,
                ))
            });
            column.child(
                div()
                    .w_full()
                    .h_flex()
                    .items_end()
                    .gap_2p5()
                    .child(slot)
                    .child(content),
            )
        } else {
            column.child(content)
        };

        if let Some(failure) = &current.failure {
            column = column.child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .pt_0p5()
                    .text_xs()
                    .text_color(p.red)
                    .child(format!("Not delivered — {failure}")),
            );
        } else if last_outgoing {
            let (label, failed) = if current.pending {
                ("Sending…", false)
            } else if msg.error != 0 {
                ("Not delivered", true)
            } else if msg.date_read.is_some() {
                ("Read", false)
            } else if msg.date_delivered.is_some() || msg.is_delivered {
                ("Delivered", false)
            } else {
                ("Sent", false)
            };
            column = column.child(row::status(label, failed, &p));
        }
        column.into_any_element()
    }

    fn render_header_row(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let content: AnyElement = if self.loading {
            div()
                .text_xs()
                .text_color(p.dim_foreground)
                .child("Loading…")
                .into_any_element()
        } else if let Some(error) = &self.error {
            div()
                .text_xs()
                .text_color(p.red)
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

enum Outgoing {
    Text(String),
    File {
        name: String,
        mime: String,
        bytes: Arc<Vec<u8>>,
    },
}

fn temp_guid() -> String {
    format!("temp-{}", uuid::Uuid::new_v4())
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

impl ThreadView {
    fn render_composer(&mut self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send = !self.drafts.is_empty() || !self.composer.read(cx).value().trim().is_empty();
        let weak = cx.entity().downgrade();
        let input = Input::new(&self.composer)
            .appearance(false)
            .on_paste(move |item, _, cx| {
                weak.update(cx, |this, cx| this.paste(item, cx))
                    .unwrap_or(false)
            });
        let tray = (!self.drafts.is_empty()).then(|| {
            div()
                .h_flex()
                .flex_wrap()
                .gap_2()
                .pb_2()
                .children(self.drafts.iter().enumerate().map(|(ix, d)| {
                    div().relative().child(draft::thumbnail(d, p)).child(
                        div()
                            .id(("remove-draft", ix))
                            .absolute()
                            .top(px(-6.))
                            .right(px(-6.))
                            .size(px(20.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(p.background)
                            .border_1()
                            .border_color(p.muted)
                            .cursor_pointer()
                            .child(Icon::new(IconName::Close).xsmall().text_color(p.foreground))
                            .on_click(cx.listener(move |this, _, _, cx| this.remove_draft(ix, cx))),
                    )
                }))
        });
        let round = |id: &'static str, icon: IconName, bg: Hsla, fg: Hsla| {
            div()
                .id(id)
                .size(px(32.))
                .flex_none()
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(bg)
                .text_color(fg)
                .cursor_pointer()
                .child(Icon::new(icon).small())
        };
        div()
            .flex_none()
            .w_full()
            .flex()
            .justify_center()
            .px_4()
            .pt_2()
            .pb_4()
            .child(
                div()
                    .w_full()
                    .max_w(px(COLUMN_WIDTH - 32.))
                    .v_flex()
                    .px_2()
                    .pt_2()
                    .pb_2()
                    .rounded(px(22.))
                    .bg(p.raised)
                    .when_some(tray, |el, tray| el.child(div().px_2().pt_1().child(tray)))
                    .when_some(self.composer_error.clone(), |el, err| {
                        el.child(div().px_2().pb_1().text_xs().text_color(p.red).child(err))
                    })
                    .child(
                        div()
                            .h_flex()
                            .gap_2()
                            .child(
                                round("attach", IconName::Plus, p.surface, p.foreground)
                                    .hover(|el| el.bg(p.row_selected))
                                    .on_click(cx.listener(|this, _, _, cx| this.pick_files(cx))),
                            )
                            .child(div().flex_1().child(input))
                            .child(
                                round(
                                    "send",
                                    IconName::ArrowUp,
                                    if can_send {
                                        p.action
                                    } else {
                                        p.action.opacity(0.4)
                                    },
                                    p.action_foreground,
                                )
                                .on_click(cx.listener(|this, _, window, cx| this.send(window, cx))),
                            ),
                    ),
            )
    }
}

impl Render for ThreadView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let p = palette(cx).clone();
        let icon = self
            .header
            .is_group
            .then(|| {
                self.media
                    .update(cx, |m, cx| m.group_icon(&self.chat_guid, cx))
            })
            .flatten();
        let header_avatar = if self.header.is_group {
            avatar::group(&self.header.faces, icon, px(26.), p.background, cx)
        } else {
            let fallback = Face {
                name: self.header.title.clone(),
                image: None,
            };
            avatar::face(
                self.header
                    .faces
                    .first()
                    .filter(|f| f.image.is_some())
                    .unwrap_or(&fallback),
                px(26.),
                cx,
            )
        };
        div()
            .id("thread")
            .size_full()
            .v_flex()
            .bg(p.background)
            .drag_over::<ExternalPaths>(|style, _, _, _| style.opacity(0.85))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.add_paths(paths.paths().to_vec(), cx);
            }))
            .child(
                div()
                    .flex_none()
                    .h(px(48.))
                    .px_4()
                    .h_flex()
                    .gap_2p5()
                    .child(header_avatar)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.bright_foreground)
                            .child(self.header.title.clone()),
                    )
                    .child(div().text_sm().text_color(p.dim_foreground).child("/"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.dim_foreground)
                            .child(self.header.subtitle.clone()),
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
                    .scrollbar(false)
                    .size_full(),
                ),
            )
            .child(self.render_composer(&p, cx))
    }
}
