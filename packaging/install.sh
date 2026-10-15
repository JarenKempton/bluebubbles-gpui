#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
gh_path="$(command -v gh)"
units="$HOME/.config/systemd/user"

mkdir -p "$HOME/.local/bin" "$units"
install -m 755 "$here/update.sh" "$HOME/.local/bin/bluebubbles-gpui-update"

cat > "$units/bluebubbles-gpui-update.service" <<UNIT
[Unit]
Description=Install the latest bluebubbles-gpui build from GitHub

[Service]
Type=oneshot
Environment=BLUEBUBBLES_GPUI_GH=$gh_path
ExecStart=%h/.local/bin/bluebubbles-gpui-update
UNIT

cat > "$units/bluebubbles-gpui-update.timer" <<UNIT
[Unit]
Description=Check for bluebubbles-gpui builds every two minutes

[Timer]
OnStartupSec=30s
OnUnitActiveSec=2min

[Install]
WantedBy=timers.target
UNIT

omarchy="$HOME/.config/omarchy"
if [ -d "$omarchy" ]; then
  template="$omarchy/themed/bluebubbles-gpui.json.tpl"
  rendered="$HOME/.local/state/omarchy/current/theme/bluebubbles-gpui.json"
  mkdir -p "$omarchy/themed" "$omarchy/hooks/theme-set.d"
  install -m 755 "$here/omarchy/theme-set-hook.sh" "$omarchy/hooks/theme-set.d/bluebubbles-gpui"
  if ! cmp -s "$here/omarchy/bluebubbles-gpui.json.tpl" "$template" || [ ! -f "$rendered" ]; then
    install -m 644 "$here/omarchy/bluebubbles-gpui.json.tpl" "$template"
    omarchy-theme-set "$(cat "$HOME/.local/state/omarchy/current/theme.name")"
  fi
  "$omarchy/hooks/theme-set.d/bluebubbles-gpui"
fi

systemctl --user daemon-reload
systemctl --user enable --now bluebubbles-gpui-update.timer
systemctl --user start bluebubbles-gpui-update.service
