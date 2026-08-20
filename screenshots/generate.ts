#!/usr/bin/env -S deno run --allow-run --allow-read --allow-write --allow-env
// Regenerate the README screenshots.
//
// Self-contained (no socat/bash): builds the debug binary, launches one
// gpuitop instance per shot with a unique --app-id, waits for the collector
// to populate, then screenshots each window via driftwm's IPC by resolving
// the app_id to a stable window id first — so screenshots never cross wires.

const SCREENSHOTS_DIR = new URL(".", import.meta.url).pathname;

const JOBS = [
	["gpuitop-shot-list-view-dark", "list-view-dark.png", "Default Dark",
		"--page", "processes.list", "--search", "vesktop"],
	["gpuitop-shot-tree-view-dark", "tree-view-dark.png", "Default Dark",
		"--page", "processes.tree", "--search", "vesktop"],
	["gpuitop-shot-settings-about-dark", "settings-about-dark.png", "Default Dark",
		"--page", "settings.about"],
	["gpuitop-shot-perf-cpu-dark", "perf-cpu-dark.png", "Default Dark",
		"--page", "performance.cpu"],
	["gpuitop-shot-list-view-light", "list-view-light.png", "Default Light",
		"--page", "processes.list", "--search", "vesktop"],
	["gpuitop-shot-tree-view-light", "tree-view-light.png", "Default Light",
		"--page", "processes.tree", "--search", "vesktop"],
	["gpuitop-shot-settings-about-light", "settings-about-light.png", "Default Light",
		"--page", "settings.about"],
	["gpuitop-shot-perf-cpu-light", "perf-cpu-light.png", "Default Light",
		"--page", "performance.cpu"],
	["gpuitop-shot-settings-about-catppuccin-mocha", "settings-about-catppuccin-mocha.png",
		"Catppuccin Mocha", "--page", "settings.about"],
];

function run(cmd: string, args: string[]) {
	const res = new Deno.Command(cmd, { args, stdout: "piped", stderr: "piped" })
		.outputSync();
	return {
		code: res.code,
		out: new TextDecoder().decode(res.stdout),
		err: new TextDecoder().decode(res.stderr),
	};
}

function targetDir() {
	const out = run("cargo", ["metadata", "--format-version", "1", "--no-deps"]);
	const meta = JSON.parse(out.out);
	return meta.target_directory;
}

// Window id by app_id; returns null while the window hasn't mapped yet.
function windowId(appId: string): number | null {
	const res = run("driftwm", ["msg", "state", "--json"]);
	if (res.code !== 0) return null;
	const data = JSON.parse(res.out) as {
		Ok?: { State?: { windows?: { id: number; app_id: string }[] } };
		State?: { windows?: { id: number; app_id: string }[] };
	};
	const state = data.Ok?.State ?? data.State;
	if (!state) return null;
	const win = state.windows?.find((w) => w.app_id === appId);
	return win ? win.id : null;
}

function shot(appId: string, output: string): boolean {
	const id = windowId(appId);
	if (id === null) {
		console.log(`  ERROR: no window matching '${appId}'`);
		return false;
	}
	const res = run("driftwm", [
		"msg", "screenshot", "window", "--id", String(id),
		"--scale", "1.0", "--output", output,
	]);
	if (res.code !== 0) {
		console.log(`  ERROR: screenshot failed: ${res.err.trim()}`);
		return false;
	}
	console.log(`  -> ${output}`);
	return true;
}

console.log("=== Building (debug) ===");
const build = run("cargo", ["build"]);
console.log(build.err.trim().split("\n").pop() ?? build.err);

const bin = `${targetDir()}/debug/gpuitop`;
const children = JOBS.map(([appId, , theme, ...rest]) => {
	console.log(`  Launching gpuitop (${theme}, ${appId})...`);
	const args = [
		"--app-id", appId,
		"--override", `(general: (interface: (theme: "${theme}")))`,
		"--override", "(window_size: (1100, 700))",
		...rest,
	];
	return new Deno.Command(bin, {
		args,
		stdout: "null",
		stderr: "null",
	}).spawn();
});

// Let every instance's collector run several ticks so the table and history
// graph populate before screenshotting.
await new Promise((r) => setTimeout(r, 10_000));

for (const [appId, file] of JOBS) {
	shot(appId, SCREENSHOTS_DIR + file);
}

for (const child of children) {
	child.kill("SIGTERM");
	await child.status;
}

console.log("\nDone:");
for (const e of Deno.readDirSync(SCREENSHOTS_DIR)) {
	if (e.isFile && e.name.endsWith(".png")) {
		const info = Deno.statSync(SCREENSHOTS_DIR + e.name);
		console.log(
			`  ${(info.size / 1024).toFixed(1)}K ${e.name}`,
		);
	}
}
