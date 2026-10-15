use std::{collections::HashMap, fs, path::PathBuf, process::Command, time::Duration};

use gpui_kit::component::theme::{Theme, ThemeMode, ThemeTokens};
use gpui_kit::*;

use crate::oklch::Ramp;

#[derive(Clone)]
pub struct Palette {
    pub dark: bool,
    pub accent: Hsla,
    pub background: Hsla,
    pub lighter_background: Hsla,
    pub selection: Hsla,
    pub muted: Hsla,
    pub foreground: Hsla,
    pub dim_foreground: Hsla,
    pub bright_foreground: Hsla,
    pub red: Hsla,
    pub yellow: Hsla,
    pub green: Hsla,
    pub identity: [Hsla; 6],
    pub sidebar: Hsla,
    pub row_hover: Hsla,
    pub row_selected: Hsla,
    pub surface: Hsla,
    pub raised: Hsla,
    pub message: Hsla,
    pub action: Hsla,
    pub action_foreground: Hsla,
}

impl Global for Palette {}

fn theme_file() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    [
        ".local/state/omarchy/current/theme/colors.toml",
        ".config/omarchy/current/theme/colors.toml",
    ]
    .into_iter()
    .map(|p| home.join(p))
    .find(|p| p.exists())
}

fn read_source() -> String {
    let Some(colors) = theme_file() else {
        return String::new();
    };
    let mut source = fs::read_to_string(&colors).unwrap_or_default();
    source.push('\n');
    source.push_str(&fs::read_to_string(colors.with_file_name("t3code.json")).unwrap_or_default());
    source
}

fn hex(value: &str) -> Option<Hsla> {
    u32::from_str_radix(value.trim().trim_start_matches('#'), 16)
        .ok()
        .map(|h| rgb(h).into())
}

fn parse(source: &str) -> Palette {
    let values: HashMap<&str, &str> = source
        .lines()
        .filter_map(|line| line.split_once(['=', ':']))
        .map(|(k, v)| {
            let value = v.trim().trim_end_matches(',').trim_matches('"');
            (k.trim().trim_matches('"'), value)
        })
        .collect();
    let color = |key: &str, fallback: u32| -> Hsla {
        values
            .get(key)
            .and_then(|v| hex(v))
            .unwrap_or_else(|| rgb(fallback).into())
    };
    let background = color("background", 0x1a1b26);
    let accent = color("accent", 0x7aa2f7);
    let lighter = color("lighter_background", 0x24283b);
    let dark = values.get("mode").is_none_or(|m| *m != "light");
    let ramp = Ramp::new(color("canvas", 0x1a1b26), accent, dark);
    Palette {
        dark,
        sidebar: color("sidebar", 0x13141c),
        row_hover: color("sidebarRowHover", 0x1c1d27),
        row_selected: color("sidebarRowActive", 0x262e43),
        surface: ramp.surface(0.07, 0., 0.),
        raised: ramp.surface(0.05, 0., 0.),
        message: ramp.surface(0.16, 0.6, 0.13),
        action: ramp.action(),
        action_foreground: background,
        accent,
        background,
        lighter_background: lighter,
        selection: color("selection", 0x292e42),
        muted: color("muted", 0x414868),
        foreground: color("foreground", 0xa9b1d6),
        dim_foreground: color("foreground", 0xa9b1d6).opacity(0.6),
        bright_foreground: color("bright_foreground", 0xc0caf5),
        red: color("red", 0xf7768e),
        yellow: color("yellow", 0xe0af68),
        green: color("green", 0x9ece6a),
        identity: [
            color("blue", 0x7aa2f7),
            color("magenta", 0xad8ee6),
            color("cyan", 0x449dab),
            color("green", 0x9ece6a),
            color("yellow", 0xe0af68),
            color("orange", 0xeb927b),
        ],
    }
}

fn font_family(generic: &str) -> SharedString {
    Command::new("fc-match")
        .args([generic, "--format=%{family}"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|families| families.split(',').next().map(|f| f.trim().to_string()))
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| "sans-serif".to_string())
        .into()
}

fn apply(palette: Palette, cx: &mut App) {
    Theme::change(
        if palette.dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    let (sans, mono) = (font_family("sans-serif"), font_family("monospace"));
    let p = palette.clone();
    let theme = Theme::global_mut(cx);
    theme.font_family = sans;
    theme.mono_font_family = mono;
    theme.font_size = px(16.);
    theme.radius = px(8.);
    theme.radius_lg = px(12.);
    theme.shadow = false;
    theme.focus_ring = false;

    let c = &mut theme.colors;
    c.background = p.background;
    c.foreground = p.foreground;
    c.border = p.muted.opacity(0.6);
    c.input = p.muted.opacity(0.6);
    c.ring = p.accent;
    c.caret = p.bright_foreground;
    c.selection = p.accent.opacity(0.35);
    c.muted = p.lighter_background;
    c.muted_foreground = p.dim_foreground;
    c.accent = p.selection;
    c.accent_foreground = p.bright_foreground;
    c.primary = p.accent;
    c.primary_hover = p.accent.opacity(0.85);
    c.primary_active = p.accent.opacity(0.7);
    c.primary_foreground = p.background;
    c.secondary = p.surface;
    c.secondary_hover = p.row_selected;
    c.secondary_active = p.selection;
    c.secondary_foreground = p.foreground;
    c.button_primary = p.accent;
    c.button_primary_hover = p.accent.opacity(0.85);
    c.button_primary_active = p.accent.opacity(0.7);
    c.button_primary_foreground = p.background;
    c.list = p.sidebar;
    c.list_hover = p.row_hover;
    c.list_active = p.row_selected;
    c.list_active_border = p.row_selected;
    c.sidebar = p.sidebar;
    c.sidebar_border = p.muted;
    c.sidebar_foreground = p.foreground;
    c.popover = p.surface;
    c.popover_foreground = p.foreground;
    c.title_bar = p.background;
    c.title_bar_border = p.muted;
    c.window_border = p.muted;
    c.scrollbar_thumb = p.muted;
    c.scrollbar_thumb_hover = p.dim_foreground;
    c.danger = p.red;
    c.danger_foreground = p.background;
    c.red = p.red;
    c.green = p.green;
    c.yellow = p.yellow;
    c.blue = p.accent;
    c.link = p.accent;
    theme.tokens = ThemeTokens::from(&theme.colors);
    cx.set_global(palette);
    cx.refresh_windows();
}

pub fn init(cx: &mut App) {
    let mut source = read_source();
    apply(parse(&source), cx);
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let next = read_source();
            if next != source {
                source = next;
                let palette = parse(&source);
                cx.update(|cx| apply(palette, cx));
            }
        }
    })
    .detach();
}

pub fn palette(cx: &App) -> &Palette {
    cx.global::<Palette>()
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Hsla, rgb};

    use super::parse;

    #[test]
    fn reads_omarchy_colors_with_fallbacks() {
        let palette = parse("mode = \"light\"\naccent = \"#ff0000\"\n");
        assert!(!palette.dark);
        assert_eq!(palette.accent, Hsla::from(rgb(0xff0000)));
        assert_eq!(palette.background, Hsla::from(rgb(0x1a1b26)));
    }
}
