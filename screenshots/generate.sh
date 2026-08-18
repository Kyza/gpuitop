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

BIN="$(cargo metadata --format-version 1 --no-deps \
	| grep -oP '"target_directory":"[^"]+"' | cut -d'"' -f4)/debug/gpuitop"

shot() {
	local appid="$1"
	local output="$2"
	local abs
	abs="$(realpath "$output")"
	local reply
	reply=$(echo "{\"Screenshot\":{\"target\":{\"Window\":{\"window\":\"$appid\"}},\"scale\":1.0,\"path\":\"$abs\"}}" \
		| socat -t3 - "UNIX-CONNECT:$SOCK" 2>&1)
	if echo "$reply" | grep -q '"Ok"'; then
		local dims
		dims=$(echo "$reply" | grep -oP '"width":\d+,"height":\d+')
		echo "  -> $output ($dims)"
	else
		echo "  ERROR: screenshot failed: $reply"
	fi
}

# job = "appid|output|theme|extra args..."
JOB_DARK="gpuitop-shot-dark|list-view-dark.png|Default Dark|--page|processes.list|--search|vesktop"
JOB_TREE_DARK="gpuitop-shot-dark2|tree-view-dark.png|Default Dark|--page|processes.tree|--search|vesktop"
JOB_ABOUT_DARK="gpuitop-shot-dark3|settings-about-dark.png|Default Dark|--page|settings.about"
JOB_CPU_DARK="gpuitop-shot-dark4|perf-cpu-dark.png|Default Dark|--page|performance.cpu"

JOB_LIGHT="gpuitop-shot-light|list-view-light.png|Default Light|--page|processes.list|--search|vesktop"
JOB_TREE_LIGHT="gpuitop-shot-light2|tree-view-light.png|Default Light|--page|processes.tree|--search|vesktop"
JOB_ABOUT_LIGHT="gpuitop-shot-light3|settings-about-light.png|Default Light|--page|settings.about"
JOB_CPU_LIGHT="gpuitop-shot-light4|perf-cpu-light.png|Default Light|--page|performance.cpu"

JOB_MOCHA="gpuitop-shot-mocha|settings-about-catppuccin-mocha.png|Catppuccin Mocha|--page|settings.about"

JOBS=("$JOB_DARK" "$JOB_TREE_DARK" "$JOB_ABOUT_DARK" "$JOB_CPU_DARK" "$JOB_LIGHT" "$JOB_TREE_LIGHT" "$JOB_ABOUT_LIGHT" "$JOB_CPU_LIGHT" "$JOB_MOCHA")

echo "=== Building (debug) ==="
cargo build 2>&1 | tail -1

PIDS=()
for jobstr in "${JOBS[@]}"; do
	IFS='|' read -r -a job <<<"$jobstr"
	appid="${job[0]}"
	theme="${job[2]}"
	echo "  Launching gpuitop ($theme, $appid)..."
	"$BIN" \
		"--app-id" "$appid" \
		"--override" "(general: (interface: (theme: \"$theme\")))" \
		"--override" "(window_size: (1100, 700))" \
		"${job[@]:3}" &
	PIDS+=("$!")
done

# Let every instance's collector run several ticks so the table and history
# graph populate before screenshotting.
sleep 10

for jobstr in "${JOBS[@]}"; do
	IFS='|' read -r -a job <<<"$jobstr"
	shot "${job[0]}" "$SCREENSHOTS_DIR/${job[1]}"
done

for pid in "${PIDS[@]}"; do
	kill "$pid" 2>/dev/null || true
	wait "$pid" 2>/dev/null || true
done

echo ""
echo "Done:"
ls -lh "$SCREENSHOTS_DIR"/*.png
