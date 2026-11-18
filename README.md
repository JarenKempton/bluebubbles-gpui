# bluebubbles-gpui

A native Linux desktop client for iMessage, written in Rust on [GPUI](https://www.gpui.rs/) (the UI framework behind the Zed editor) through [gpui-kit](https://crates.io/crates/gpui-kit).

It talks to a [BlueBubbles](https://bluebubbles.app/) server running on a Mac. That Mac does the actual iMessage sending and receiving. This app replaces the Electron and Flutter clients with a single GPU-rendered binary that starts instantly and follows your desktop theme live.

![Conversation view with the multi-line composer](https://github.com/JarenKempton/bluebubbles-gpui/releases/download/pr-2-screenshots/pr2-composer-wrap.png)

## Features

- **Live messaging**: new messages and delivery or read receipts arrive over the BlueBubbles Socket.IO stream, with automatic reconnects. A red **Offline** label appears only while the connection is down.
- **Conversations**: searchable chat list, contact names and photos, group icons, and sender avatars in group threads.
- **Attachments**: paste images or copied files, pick files with `+`, or drag and drop. A local preview shows while the upload runs. Attachments that only exist in iCloud are labeled as iCloud-only.
- **Composer**: text wraps and the box grows up to 8 rows. Enter sends and Shift+Enter adds a new line.
- **Desktop notifications** for incoming messages.
- **Omarchy theming**: on [Omarchy](https://omarchy.org/), the app renders a native theme from your current Omarchy theme and recolors live when you switch themes.
- **Self-updating**: a systemd user timer installs each new `main` build. When the binary is replaced, the app shows **Update ready** so you can restart into the new build.

### Keyboard shortcuts

| Keys | Action |
| --- | --- |
| `Ctrl+K` | Search conversations |
| `Alt+↓` / `Ctrl+Tab` | Next conversation |
| `Alt+↑` / `Ctrl+Shift+Tab` | Previous conversation |
| `Enter` / `Shift+Enter` | Send / new line |

## Requirements

- Linux on Wayland or X11, with a Vulkan-capable GPU
- A running [BlueBubbles server](https://bluebubbles.app/install) on a Mac, reachable from this machine, plus its password

## Install

### Prebuilt (x86_64)

CI publishes every push to `main` as the rolling [`main-build`](https://github.com/JarenKempton/bluebubbles-gpui/releases/tag/main-build) release. The installer needs the [GitHub CLI](https://cli.github.com/):

```sh
git clone https://github.com/JarenKempton/bluebubbles-gpui
cd bluebubbles-gpui
packaging/install.sh
```

This installs `bluebubbles-gpui` into `~/.local/bin`, adds a **Messages** launcher entry, and enables a systemd user timer that checks for a new build every two minutes. On Omarchy it also installs the theme template and the theme-set hook.

### From source (any architecture)

Install Rust through [rustup](https://rustup.rs/) or [mise](https://mise.jdx.dev/). You also need the native libraries GPUI uses.

```sh
# Arch
sudo pacman -S --needed base-devel openssl fontconfig freetype2 wayland libxkbcommon libxkbcommon-x11 libxcb vulkan-icd-loader zstd alsa-lib

# Debian / Ubuntu
sudo apt install pkg-config libssl-dev libfontconfig-dev libfreetype-dev libwayland-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev libvulkan-dev libzstd-dev libasound2-dev
```

Then build and install it:

```sh
cargo build --release
install -Dm755 target/release/bluebubbles-gpui ~/.local/bin/bluebubbles-gpui
install -Dm644 packaging/bluebubbles-gpui.desktop ~/.local/share/applications/bluebubbles-gpui.desktop
install -Dm644 packaging/bluebubbles-gpui.svg ~/.local/share/icons/hicolor/scalable/apps/bluebubbles-gpui.svg
```

## Usage

Launch **Messages** from your app launcher, or run `bluebubbles-gpui`. Enter your BlueBubbles server address (for example `http://your-mac:1234`) and the server password.

The connection is saved to `~/.config/bluebubbles-gpui/config.json` with `0600` permissions. **Sign out** deletes it.

## Development

```sh
cargo run
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

CI runs the same checks on every pull request.

| Path | What it holds |
| --- | --- |
| `src/api` | BlueBubbles REST client and models |
| `src/live.rs` | Socket.IO event stream |
| `src/ui` | Views: connect screen, chat list, thread, message rows, composer |
| `src/omarchy.rs`, `src/oklch.rs` | Theme loading and palette math |
| `packaging/` | Launcher entry, icon, installer, updater, Omarchy theme template |
