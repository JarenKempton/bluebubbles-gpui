use std::path::PathBuf;

use gpui_kit::component::theme::{Theme, ThemeMode, ThemeRegistry};
use gpui_kit::*;

const THEME_NAME: &str = "Omarchy";

#[derive(Clone)]
pub struct Palette {
    pub accent: Hsla,
    pub background: Hsla,
    pub lighter_background: Hsla,
    pub muted: Hsla,
    pub foreground: Hsla,
    pub dim_foreground: Hsla,
    pub bright_foreground: Hsla,
    pub red: Hsla,
    pub yellow: Hsla,
    pub sidebar: Hsla,
    pub row_hover: Hsla,
    pub row_selected: Hsla,
    pub surface: Hsla,
    pub raised: Hsla,
}

impl Global for Palette {}

impl Palette {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            accent: theme.primary,
            background: theme.background,
            lighter_background: theme.muted,
            muted: theme.border,
            foreground: theme.foreground,
            dim_foreground: theme.muted_foreground,
            bright_foreground: theme.accent_foreground,
            red: theme.red,
            yellow: theme.yellow,
            sidebar: theme.sidebar,
            row_hover: theme.list_hover,
            row_selected: theme.list_active,
            surface: theme.secondary,
            raised: theme.popover,
        }
    }
}

pub fn themes_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bluebubbles-gpui")
        .join("themes")
}

pub fn init(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    cx.set_global(Palette::from_theme(Theme::global(cx)));
    cx.observe_global::<Theme>(|cx| {
        let palette = Palette::from_theme(Theme::global(cx));
        cx.set_global(palette);
    })
    .detach();
    let _ = ThemeRegistry::watch_dir(themes_dir(), cx, select_omarchy);
}

fn select_omarchy(cx: &mut App) {
    let Some(config) = ThemeRegistry::global(cx).themes().get(THEME_NAME).cloned() else {
        return;
    };
    let mode = config.mode;
    Theme::global_mut(cx).apply_config(&config);
    Theme::change(mode, None, cx);
    cx.refresh_windows();
}

pub fn palette(cx: &App) -> &Palette {
    cx.global::<Palette>()
}
