mod chat_list;
mod connect;
mod main_view;
mod message_row;
mod reactions;
mod thread;
mod time;

use gpui_kit::*;

use crate::{api::Client, config};
use connect::{ConnectView, Connected};
use main_view::{MainView, SignedOut};
pub use main_view::bind_keys;

enum Screen {
    Connect(Entity<ConnectView>),
    Main(Entity<MainView>),
}

pub struct AppView {
    screen: Screen,
    _subscription: Option<Subscription>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            screen: Screen::Connect(cx.new(|cx| ConnectView::new(None, window, cx))),
            _subscription: None,
        };
        match config::load() {
            Some(saved) => this.show_main(Client::new(&saved.server_url, &saved.password), window, cx),
            None => this.show_connect(None, window, cx),
        }
        this
    }

    fn show_connect(&mut self, previous: Option<config::Config>, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|cx| ConnectView::new(previous, window, cx));
        self._subscription = Some(cx.subscribe_in(&view, window, |this, _, event: &Connected, window, cx| {
            this.show_main(event.0.clone(), window, cx);
        }));
        self.screen = Screen::Connect(view);
        cx.notify();
    }

    fn show_main(&mut self, client: Client, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|cx| MainView::new(client, window, cx));
        self._subscription = Some(cx.subscribe_in(&view, window, |this, _, _: &SignedOut, window, cx| {
            let previous = config::load();
            config::clear();
            this.show_connect(previous.map(|c| config::Config { password: String::new(), ..c }), window, cx);
        }));
        self.screen = Screen::Main(view);
        cx.notify();
    }
}

impl Render for AppView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        match &self.screen {
            Screen::Connect(view) => view.clone().into_any_element(),
            Screen::Main(view) => view.clone().into_any_element(),
        }
    }
}
