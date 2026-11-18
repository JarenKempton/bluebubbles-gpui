use std::{ops::Range, rc::Rc, time::Duration};

use futures::StreamExt as _;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{
    api::{Chat, Client, Message},
    contacts::ContactBook,
    live::{self, LiveEvent},
    media::MediaCache,
    omarchy::{Palette, palette},
    runtime,
    self_update::InstalledBinary,
    ui::{
        chat_list::{self, ChatEntry},
        thread::{ThreadEvent, ThreadHeader, ThreadView},
    },
};

actions!(bluebubbles, [NextChat, PreviousChat, FocusSearch]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-down", NextChat, Some("MainView")),
        KeyBinding::new("alt-up", PreviousChat, Some("MainView")),
        KeyBinding::new("ctrl-tab", NextChat, Some("MainView")),
        KeyBinding::new("ctrl-shift-tab", PreviousChat, Some("MainView")),
        KeyBinding::new("ctrl-k", FocusSearch, Some("MainView")),
    ]);
}

const FIRST_PAGE: usize = 60;
const PAGE: usize = 250;

pub struct SignedOut;

impl EventEmitter<SignedOut> for MainView {}

#[derive(Clone, Copy, PartialEq)]
enum Link {
    Connecting,
    Online,
    Offline,
}

pub struct MainView {
    client: Client,
    contacts: Rc<ContactBook>,
    media: Entity<MediaCache>,
    chats: Vec<Chat>,
    entries: Vec<ChatEntry>,
    visible: Vec<usize>,
    search: Entity<InputState>,
    selected: Option<String>,
    thread: Option<Entity<ThreadView>>,
    link: Link,
    load_error: Option<SharedString>,
    has_connected: bool,
    list_scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
    update_ready: bool,
    _subscriptions: Vec<Subscription>,
    _thread_subscription: Option<Subscription>,
    _live: Task<()>,
}

impl MainView {
    pub fn new(client: Client, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
        let media = cx.new(|_| MediaCache::new(client.clone()));
        let subscriptions = vec![
            cx.observe(&media, |_, _, cx| cx.notify()),
            cx.subscribe_in(
                &search,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => this.refilter(cx),
                    InputEvent::PressEnter { .. } => this.select(0, window, cx),
                    _ => {}
                },
            ),
        ];
        let mut events = live::subscribe(&client);
        let live = cx.spawn_in(window, async move |this, cx| {
            while let Some(event) = events.next().await {
                if this
                    .update_in(cx, |this, window, cx| this.on_live(event, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut this = Self {
            media,
            client,
            contacts: Rc::default(),
            chats: Vec::new(),
            entries: Vec::new(),
            visible: Vec::new(),
            search,
            selected: None,
            thread: None,
            link: Link::Connecting,
            load_error: None,
            has_connected: false,
            list_scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            update_ready: false,
            _subscriptions: subscriptions,
            _thread_subscription: None,
            _live: live,
        };
        this.focus_handle.focus(window, cx);
        this.watch_for_update(cx);
        this.load_contacts(cx);
        this.load_chats(cx);
        this
    }

    fn watch_for_update(&mut self, cx: &mut Context<Self>) {
        let Some(binary) = InstalledBinary::current() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(30))
                    .await;
                if binary.replaced() {
                    this.update(cx, |this, cx| {
                        this.update_ready = true;
                        cx.notify();
                    })
                    .ok();
                    return;
                }
            }
        })
        .detach();
    }

    fn restart(&mut self, cx: &mut Context<Self>) {
        if InstalledBinary::current().is_some_and(|binary| binary.relaunch()) {
            cx.quit();
        }
    }

    fn load_contacts(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let book = runtime::run(async move {
                client
                    .contacts()
                    .await
                    .map(ContactBook::from_contacts)
                    .unwrap_or_default()
            })
            .await;
            this.update(cx, |this, cx| {
                this.contacts = Rc::new(book);
                if let Some(thread) = &this.thread {
                    let contacts = this.contacts.clone();
                    thread.update(cx, |t, cx| t.set_contacts(contacts, cx));
                }
                this.rebuild(cx);
            })
            .ok();
        })
        .detach();
    }

    fn load_chats(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let mut offset = 0;
            let mut limit = FIRST_PAGE;
            let mut all = Vec::new();
            loop {
                let page_client = client.clone();
                let page =
                    runtime::run(async move { page_client.chats(offset, limit).await }).await;
                let done = match page {
                    Ok(page) => {
                        let done = page.len() < limit;
                        all.extend(page);
                        offset = all.len();
                        limit = PAGE;
                        done
                    }
                    Err(err) => {
                        this.update(cx, |this, cx| {
                            this.load_error = Some(format!("{err:#}").into());
                            cx.notify();
                        })
                        .ok();
                        return;
                    }
                };
                let snapshot = all.clone();
                let alive = this
                    .update(cx, |this, cx| {
                        this.load_error = None;
                        this.chats = snapshot;
                        this.rebuild(cx);
                    })
                    .is_ok();
                if done || !alive {
                    return;
                }
            }
        })
        .detach();
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.chats.sort_by_key(|c| {
            std::cmp::Reverse(
                c.last_message
                    .as_ref()
                    .map(|m| m.date_created)
                    .unwrap_or_default(),
            )
        });
        self.entries = self
            .chats
            .iter()
            .cloned()
            .map(|c| ChatEntry::build(c, &self.contacts))
            .collect();
        self.refilter(cx);
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().trim().to_lowercase();
        self.visible = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                query.is_empty()
                    || e.title.to_lowercase().contains(&query)
                    || e.chat
                        .participants
                        .iter()
                        .any(|p| p.address.contains(&query))
            })
            .map(|(i, _)| i)
            .collect();
        cx.notify();
    }

    fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_deref()?;
        self.visible
            .iter()
            .position(|&i| self.entries[i].chat.guid == selected)
    }

    fn step(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.visible.len() as isize;
        if count == 0 {
            return;
        }
        let next = match self.selected_index() {
            Some(ix) => (ix as isize + delta).clamp(0, count - 1),
            None => 0,
        } as usize;
        self.select(next, window, cx);
        self.list_scroll
            .scroll_to_item(next, ScrollStrategy::Nearest);
    }

    fn select(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.visible.get(ix).and_then(|&i| self.entries.get(i)) else {
            return;
        };
        let guid = entry.chat.guid.clone();
        if self.selected.as_deref() == Some(&guid) {
            if let Some(thread) = &self.thread {
                thread.update(cx, |t, cx| t.focus_composer(window, cx));
            }
            return;
        }
        let header = ThreadHeader {
            title: entry.title.clone(),
            is_group: entry.chat.is_group(),
        };
        let (client, media, contacts) = (
            self.client.clone(),
            self.media.clone(),
            self.contacts.clone(),
        );
        let thread =
            cx.new(|cx| ThreadView::new(client, media, contacts, guid.clone(), header, window, cx));
        self._thread_subscription = Some(cx.subscribe(
            &thread,
            |this, _, event: &ThreadEvent, cx| match event {
                ThreadEvent::Sent(msg) => this.apply_message(msg.clone(), cx),
            },
        ));
        self.thread = Some(thread);
        self.mark_seen(&guid);
        self.selected = Some(guid);
        self.rebuild(cx);
    }

    fn mark_seen(&mut self, guid: &str) {
        let last = self
            .chats
            .iter_mut()
            .find(|c| c.guid == guid)
            .and_then(|c| c.last_message.as_mut());
        if let Some(last) = last.filter(|m| m.date_read.is_none()) {
            last.date_read = Some(crate::ui::time::now_ms());
        }
    }

    fn apply_message(&mut self, msg: Message, cx: &mut Context<Self>) {
        let Some(guid) = msg
            .chat_guid()
            .map(str::to_string)
            .or_else(|| self.selected_if_temp(&msg))
        else {
            return;
        };
        let Some(chat) = self.chats.iter_mut().find(|c| c.guid == guid) else {
            self.load_chats(cx);
            return;
        };
        let newer = chat
            .last_message
            .as_ref()
            .is_none_or(|last| last.guid == msg.guid || last.date_created <= msg.date_created);
        if newer {
            chat.last_message = Some(msg);
            if self.selected.as_deref() == Some(guid.as_str()) {
                self.mark_seen(&guid);
            }
            self.rebuild(cx);
        }
    }

    fn selected_if_temp(&self, msg: &Message) -> Option<String> {
        msg.temp_guid.as_ref().and(self.selected.clone())
    }

    fn on_live(&mut self, event: LiveEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            LiveEvent::Connected => {
                if self.has_connected {
                    self.load_chats(cx);
                    if let Some(thread) = &self.thread {
                        thread.update(cx, |t, cx| t.refresh(cx));
                    }
                }
                self.has_connected = true;
                self.link = Link::Online;
            }
            LiveEvent::Disconnected => self.link = Link::Offline,
            LiveEvent::NewMessage(msg) => {
                self.notify_if_needed(&msg, window);
                self.route(msg, cx);
            }
            LiveEvent::UpdatedMessage(msg) => self.route(msg, cx),
        }
        cx.notify();
    }

    fn route(&mut self, msg: Message, cx: &mut Context<Self>) {
        if let Some(thread) = &self.thread {
            let matches = msg
                .chat_guid()
                .is_none_or(|g| Some(g) == self.selected.as_deref());
            if matches {
                thread.update(cx, |t, cx| t.ingest(msg.clone(), cx));
            }
        }
        if !msg.is_reaction() {
            self.apply_message(msg, cx);
        }
    }

    fn notify_if_needed(&self, msg: &Message, window: &Window) {
        if msg.is_from_me {
            return;
        }
        let viewing = window.is_window_active() && msg.chat_guid() == self.selected.as_deref();
        if viewing {
            return;
        }
        let title = msg
            .chat_guid()
            .and_then(|g| self.entries.iter().find(|e| e.chat.guid == g))
            .map(|e| e.title.to_string())
            .or_else(|| {
                msg.handle
                    .as_ref()
                    .map(|h| self.contacts.display(&h.address))
            })
            .unwrap_or_else(|| "New message".to_string());
        let body = chat_list::preview(msg);
        runtime::spawn(async move {
            let _ = tokio::task::spawn_blocking(move || {
                notify_rust::Notification::new()
                    .appname("BlueBubbles")
                    .summary(&title)
                    .body(&body)
                    .show()
            })
            .await;
        });
    }

    fn render_rows(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        range
            .filter_map(|ix| {
                let entry = self.entries.get(*self.visible.get(ix)?)?;
                let selected = self.selected.as_deref() == Some(entry.chat.guid.as_str());
                Some(
                    chat_list::row(entry, selected, cx)
                        .id(ix)
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.select(ix, window, cx)),
                        )
                        .into_any_element(),
                )
            })
            .collect()
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx).clone();
        div()
            .w(px(264.))
            .flex_none()
            .h_full()
            .v_flex()
            .bg(p.sidebar)
            .border_r_1()
            .border_color(p.muted)
            .text_color(p.foreground)
            .child(
                div()
                    .h(px(48.))
                    .flex_none()
                    .px_4()
                    .h_flex()
                    .gap_2p5()
                    .border_b_1()
                    .border_color(p.muted)
                    .child(
                        Icon::new(IconName::Search)
                            .small()
                            .text_color(p.dim_foreground),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .child(Input::new(&self.search).appearance(false).cleanable(true)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_xs()
                            .text_color(p.dim_foreground)
                            .child("Ctrl K"),
                    ),
            )
            .children(
                self.load_error
                    .clone()
                    .map(|err| div().px_4().py_2().text_xs().text_color(p.red).child(err)),
            )
            .child(
                uniform_list("chats", self.visible.len(), cx.processor(Self::render_rows))
                    .track_scroll(&self.list_scroll)
                    .px_2()
                    .pt_2()
                    .flex_1(),
            )
            .child(self.render_account(&p, cx))
    }

    fn render_account(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let host: SharedString = server_host(self.client.root()).into();
        let label: SharedString = match (self.link, self.update_ready) {
            (Link::Offline, _) => "Reconnecting…".into(),
            (_, true) => "Update ready".into(),
            _ => host.clone(),
        };
        let status: SharedString = match self.link {
            Link::Online => "Connected".into(),
            Link::Connecting => "Connecting…".into(),
            Link::Offline => "Offline, reconnecting".into(),
        };
        let update_ready = self.update_ready;
        let view = cx.entity().downgrade();
        div()
            .h(px(48.))
            .flex_none()
            .px(px(6.))
            .h_flex()
            .border_t_1()
            .border_color(p.muted)
            .child(
                Button::new("account")
                    .ghost()
                    .h(px(32.))
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.dim_foreground)
                            .truncate()
                            .child(label),
                    )
                    .dropdown_menu_with_anchor(Anchor::BottomLeft, move |menu, _, _| {
                        let (restart, sign_out) = (view.clone(), view.clone());
                        menu.label(host.clone())
                            .label(status.clone())
                            .separator()
                            .when(update_ready, |menu| {
                                menu.item(PopupMenuItem::new("Restart to update").on_click(
                                    move |_, _, cx| {
                                        restart.update(cx, |this, cx| this.restart(cx)).ok();
                                    },
                                ))
                            })
                            .item(PopupMenuItem::new("Sign out").on_click(move |_, _, cx| {
                                sign_out.update(cx, |_, cx| cx.emit(SignedOut)).ok();
                            }))
                    }),
            )
    }
}

fn server_host(root: &str) -> String {
    reqwest::Url::parse(root)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| root.to_string())
}

impl Render for MainView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = self.render_sidebar(cx).into_any_element();
        let theme = cx.theme();
        let pane = match &self.thread {
            Some(thread) => thread.clone().into_any_element(),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted_foreground)
                .text_sm()
                .child(if self.entries.is_empty() {
                    "Loading conversations…"
                } else {
                    "Select a conversation"
                })
                .into_any_element(),
        };
        div()
            .size_full()
            .h_flex()
            .key_context("MainView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &NextChat, window, cx| this.step(1, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousChat, window, cx| this.step(-1, window, cx)))
            .on_action(cx.listener(|this, _: &FocusSearch, window, cx| {
                this.search.update(cx, |s, cx| s.focus(window, cx));
            }))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sidebar)
            .child(div().flex_1().h_full().min_w_0().child(pane))
    }
}
