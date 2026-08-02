#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCREENSHOTS_DIR="$SCRIPT_DIR"
mkdir -p "$SCREENSHOTS_DIR"

SOCK="$XDG_RUNTIME_DIR/driftwm/ipc-$WAYLAND_DISPLAY.sock"
if [ ! -S "$SOCK" ]; then
	echo "Error: driftwm IPC socket not found at $SOCK"
	exit 1
fi

capture() {
	local output="$1"
	local theme="$2"
	shift 2

	local abs
	abs="$(realpath "$output")"

	echo "  Launching gpuitop ($theme)..."

	cargo run -- \
		"--override" "(general: (interface: (theme: $theme)))" \
		"--override" "(window_size: (1100, 700))" \
		"$@" &
	local pid=$!

	sleep 3

	local reply
	reply=$(echo "{\"Screenshot\":{\"target\":{\"Window\":{\"window\":\"gpuitop\"}},\"scale\":1.0,\"path\":\"$abs\"}}" \
		| socat -t3 - "UNIX-CONNECT:$SOCK" 2>&1)

	if echo "$reply" | grep -q '"Ok"'; then
		local dims
		dims=$(echo "$reply" | grep -oP '"width":\d+,"height":\d+')
		echo "  -> $output ($dims)"
	else
		echo "  ERROR: screenshot failed: $reply"
	fi

	kill $pid 2>/dev/null || true
	wait $pid 2>/dev/null || true
	sleep 0.5
}

echo "=== Building (debug) ==="
cargo build 2>&1 | tail -1

for theme in Dark Light; do
	echo ""
	echo "=== Theme: $theme ==="

	capture "$SCREENSHOTS_DIR/list-view-${theme,,}.png" \
		"$theme" \
		"--page" "processes.list" "--search" "vesktop"

	capture "$SCREENSHOTS_DIR/tree-view-${theme,,}.png" \
		"$theme" \
		"--page" "processes.tree" "--search" "vesktop"

	capture "$SCREENSHOTS_DIR/settings-about-${theme,,}.png" \
		"$theme" \
		"--page" "settings.about"
done

echo ""
echo "Done:"
ls -lh "$SCREENSHOTS_DIR"/*.png
