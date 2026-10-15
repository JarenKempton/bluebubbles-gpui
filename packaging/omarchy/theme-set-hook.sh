#!/usr/bin/env bash
rendered="$HOME/.local/state/omarchy/current/theme/bluebubbles-gpui.json"
themes="${XDG_CONFIG_HOME:-$HOME/.config}/bluebubbles-gpui/themes"
[ -f "$rendered" ] || exit 0
mkdir -p "$themes"
cp "$rendered" "$themes/.omarchy.json.tmp" && mv -f "$themes/.omarchy.json.tmp" "$themes/omarchy.json"
