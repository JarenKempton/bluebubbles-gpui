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

systemctl --user daemon-reload
systemctl --user enable --now bluebubbles-gpui-update.timer
systemctl --user start bluebubbles-gpui-update.service
