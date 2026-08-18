use std::time::Duration;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
use x11rb::rust_connection::RustConnection;

use crate::PickedWindow;

pub fn pick_window() -> Option<PickedWindow> {
	for _ in 0..50 {
		if let Some(w) = try_get_active() {
			return Some(w);
		}
		std::thread::sleep(Duration::from_millis(200));
	}
	None
}

fn try_get_active() -> Option<PickedWindow> {
	let (conn, screen_num) = x11rb::connect(None).ok()?;
	let root = conn.setup().roots.get(screen_num)?.root;

	let net_active = intern_atom(&conn, b"_NET_ACTIVE_WINDOW")?;
	let reply = conn
		.get_property(false, root, net_active, AtomEnum::WINDOW, 0, 1)
		.ok()?
		.reply()
		.ok()?;
	let window = reply.value32()?.next()?;
	if window == 0 {
		return None;
	}

	let pid = window_pid(&conn, window);
	if pid == Some(std::process::id()) {
		return None;
	}

	match pid {
		Some(pid) => Some(PickedWindow::Pid(pid as i32)),
		None => wm_class(&conn, window)
			.filter(|c| c != "gpuitop")
			.map(PickedWindow::AppId),
	}
}

fn intern_atom(conn: &RustConnection, name: &[u8]) -> Option<u32> {
	Some(conn.intern_atom(false, name).ok()?.reply().ok()?.atom)
}

fn window_pid(conn: &RustConnection, window: u32) -> Option<u32> {
	let atom = intern_atom(conn, b"_NET_WM_PID")?;
	let reply = conn
		.get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)
		.ok()?
		.reply()
		.ok()?;
	let pid = reply.value32()?.next();
	pid
}

fn wm_class(conn: &RustConnection, window: u32) -> Option<String> {
	let atom = intern_atom(conn, b"WM_CLASS")?;
	let reply = conn
		.get_property(false, window, atom, AtomEnum::STRING, 0, 1)
		.ok()?
		.reply()
		.ok()?;
	let bytes: Vec<u8> = reply.value8()?.collect();
	let instance = bytes.split(|&b| b == 0).next()?;
	String::from_utf8(instance.to_vec()).ok()
}
