use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::*;

use crate::{api::Client, config, runtime};

pub struct Connected(pub Client);

impl EventEmitter<Connected> for ConnectView {}

pub struct ConnectView {
    url: Entity<InputState>,
    password: Entity<InputState>,
    connecting: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl ConnectView {
    pub fn new(
        previous: Option<config::Config>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("http://your-mac:1234"));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Server password")
                .masked(true)
        });
        if let Some(previous) = previous {
            url.update(cx, |state, cx| {
                state.set_value(previous.server_url, window, cx)
            });
        }
        let submit = |this: &mut Self,
                      _: &Entity<InputState>,
                      event: &InputEvent,
                      window: &mut Window,
                      cx: &mut Context<Self>| {
            if let InputEvent::PressEnter { .. } = event {
                this.connect(window, cx);
            }
        };
        let subscriptions = vec![
            cx.subscribe_in(&url, window, submit),
            cx.subscribe_in(&password, window, submit),
        ];
        let initial = if url.read(cx).value().is_empty() {
            &url
        } else {
            &password
        };
        initial.update(cx, |state, cx| state.focus(window, cx));
        Self {
            url,
            password,
            connecting: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn connect(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.connecting {
            return;
        }
        let url = self.url.read(cx).value().trim().to_string();
        let password = self.password.read(cx).value().to_string();
        if url.is_empty() || password.is_empty() {
            self.error = Some("Enter the server address and password.".into());
            cx.notify();
            return;
        }
        let client = Client::new(&url, &password);
        self.connecting = true;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let probe = client.clone();
            let result = runtime::run(async move { probe.server_info().await }).await;
            this.update(cx, |this, cx| {
                this.connecting = false;
                match result {
                    Ok(_) => {
                        let saved = config::Config {
                            server_url: client.root().to_string(),
                            password,
                        };
                        if let Err(err) = config::save(&saved) {
                            this.error = Some(
                                format!("Connected, but couldn't save settings: {err}").into(),
                            );
                        }
                        cx.emit(Connected(client));
                    }
                    Err(err) => this.error = Some(format!("{err:#}").into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for ConnectView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                div()
                    .v_flex()
                    .w(px(380.))
                    .gap_4()
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_semibold()
                                    .child("Connect to your Mac"),
                            )
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                "Use the BlueBubbles server address and password from your Mac.",
                            )),
                    )
                    .child(field("Server", Input::new(&self.url), theme))
                    .child(field(
                        "Password",
                        Input::new(&self.password).mask_toggle(),
                        theme,
                    ))
                    .children(
                        self.error
                            .clone()
                            .map(|error| div().text_sm().text_color(theme.danger).child(error)),
                    )
                    .child(
                        Button::new("connect")
                            .primary()
                            .w_full()
                            .label(if self.connecting {
                                "Connecting…"
                            } else {
                                "Connect"
                            })
                            .loading(self.connecting)
                            .on_click(cx.listener(|this, _, window, cx| this.connect(window, cx))),
                    ),
            )
    }
}

fn field(label: &'static str, input: Input, theme: &component::theme::Theme) -> impl IntoElement {
    div()
        .v_flex()
        .gap_1p5()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(input)
}
