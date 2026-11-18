#!/usr/bin/env bash
set -euo pipefail

repo="${BLUEBUBBLES_GPUI_REPO:-JarenKempton/bluebubbles-gpui}"
release="https://github.com/$repo/releases/download/main-build"
asset="bluebubbles-gpui-$(uname -m)-linux.tar.gz"
bin_dir="$HOME/.local/bin"
data_dir="$HOME/.local/share/bluebubbles-gpui"
installed="$(cat "$data_dir/VERSION" 2>/dev/null || true)"

latest="$(curl -fsSL "$release/VERSION")" || { echo "could not reach $release" >&2; exit 0; }
[ -n "$latest" ] && [ "$latest" != "$installed" ] || exit 0

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl -fsSL -o "$work/$asset" "$release/$asset"
tar -xzf "$work/$asset" -C "$work"
pkg="$work/bluebubbles-gpui"
[ "$(cat "$pkg/VERSION")" = "$latest" ] || { echo "version mismatch in download" >&2; exit 1; }

mkdir -p "$bin_dir" "$data_dir" "$HOME/.local/share/applications" "$HOME/.local/share/icons/hicolor/scalable/apps"
install -m 755 "$pkg/bluebubbles-gpui" "$bin_dir/.bluebubbles-gpui.new"
mv -f "$bin_dir/.bluebubbles-gpui.new" "$bin_dir/bluebubbles-gpui"
install -m 755 "$pkg/update.sh" "$bin_dir/bluebubbles-gpui-update"
install -m 644 "$pkg/bluebubbles-gpui.desktop" "$HOME/.local/share/applications/bluebubbles-gpui.desktop"
install -m 644 "$pkg/bluebubbles-gpui.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/bluebubbles-gpui.svg"
echo "$latest" > "$data_dir/VERSION"
echo "installed $latest"
