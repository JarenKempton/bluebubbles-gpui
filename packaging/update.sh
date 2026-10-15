#!/usr/bin/env bash
set -euo pipefail

repo="${BLUEBUBBLES_GPUI_REPO:-JarenKempton/bluebubbles-gpui}"
gh="${BLUEBUBBLES_GPUI_GH:-gh}"
bin_dir="$HOME/.local/bin"
data_dir="$HOME/.local/share/bluebubbles-gpui"
installed="$(cat "$data_dir/VERSION" 2>/dev/null || true)"

latest="$("$gh" release view main-build --repo "$repo" --json name --jq .name 2>/dev/null)" || exit 0
[ -n "$latest" ] && [ "$latest" != "$installed" ] || exit 0

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
"$gh" release download main-build --repo "$repo" --pattern 'bluebubbles-gpui-x86_64-linux.tar.gz' --dir "$work"
tar -xzf "$work/bluebubbles-gpui-x86_64-linux.tar.gz" -C "$work"
pkg="$work/bluebubbles-gpui"
[ "$(cat "$pkg/VERSION")" = "$latest" ] || { echo "version mismatch in download" >&2; exit 1; }

mkdir -p "$bin_dir" "$data_dir" "$HOME/.local/share/applications" "$HOME/.local/share/icons/hicolor/scalable/apps"
install -m 755 "$pkg/bluebubbles-gpui" "$bin_dir/.bluebubbles-gpui.new"
mv -f "$bin_dir/.bluebubbles-gpui.new" "$bin_dir/bluebubbles-gpui"
install -m 644 "$pkg/bluebubbles-gpui.desktop" "$HOME/.local/share/applications/bluebubbles-gpui.desktop"
install -m 644 "$pkg/bluebubbles-gpui.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/bluebubbles-gpui.svg"
echo "$latest" > "$data_dir/VERSION"
echo "installed $latest"
