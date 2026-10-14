mod api;
mod config;
mod contacts;
mod live;
mod media;
mod runtime;
mod ui;

use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::*;

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            ui::bind_keys(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1120.), px(760.)), cx)),
                titlebar: Some(TitlebarOptions {
                    title: Some("BlueBubbles".into()),
                    ..Default::default()
                }),
                app_id: Some("bluebubbles-gpui".into()),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| ui::AppView::new(window, cx)))
                .expect("failed to open window");
        });
}
