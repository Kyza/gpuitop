use std::collections::HashMap;

use wayland_client::{
	Connection, Dispatch, Proxy, QueueHandle,
	backend::ObjectId,
	globals::{registry_queue_init, GlobalListContents},
	protocol::wl_registry,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
	zwlr_foreign_toplevel_handle_v1,
	zwlr_foreign_toplevel_manager_v1,
};

struct HandleInfo {
	app_id: String,
	title: String,
	activated: bool,
}

struct AppData {
	handles: HashMap<ObjectId, HandleInfo>,
	_keep_alive: Vec<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1>,
}

impl AppData {
	fn new() -> Self {
		Self { handles: HashMap::new(), _keep_alive: Vec::new() }
	}
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppData {
	fn event(
		_state: &mut Self,
		_registry: &wl_registry::WlRegistry,
		_event: wl_registry::Event,
		_data: &GlobalListContents,
		_conn: &Connection,
		_qh: &QueueHandle<Self>,
	) {
	}
}

impl Dispatch<zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1, ()>
	for AppData
{
	fn event(
		state: &mut Self,
		_manager: &zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1,
		event: zwlr_foreign_toplevel_manager_v1::Event,
		_data: &(),
		_conn: &Connection,
		_qh: &QueueHandle<Self>,
	) {
		if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } =
			event
		{
			let id = toplevel.id();
		state.handles.entry(id).or_insert(HandleInfo {
			app_id: String::new(),
			title: String::new(),
			activated: false,
		});
			state._keep_alive.push(toplevel);
		}
	}

	fn event_created_child(
		opcode: u16,
		qh: &QueueHandle<Self>,
	) -> std::sync::Arc<dyn wayland_client::backend::ObjectData> {
		if opcode == zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE {
			qh.make_data::<zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1, _>(
				(),
			)
		} else {
			panic!(
				"Unexpected child opcode from foreign_toplevel_manager: \
				 {opcode}"
			);
		}
	}
}

impl
	Dispatch<
		zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
		(),
	> for AppData
{
	fn event(
		state: &mut Self,
		handle: &zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
		event: zwlr_foreign_toplevel_handle_v1::Event,
		_data: &(),
		_conn: &Connection,
		_qh: &QueueHandle<Self>,
	) {
		let id = handle.id();
		let info = state.handles.entry(id).or_insert(HandleInfo {
			app_id: String::new(),
			title: String::new(),
			activated: false,
		});
		match event {
			zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
				info.app_id = app_id;
			}
			zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
				info.title = title;
			}
			zwlr_foreign_toplevel_handle_v1::Event::State { state: states } => {
				info.activated = states.iter().any(|s| {
					*s == zwlr_foreign_toplevel_handle_v1::State::Activated
						as u8
				});
			}
			_ => {}
		}
	}

	fn event_created_child(
		_opcode: u16,
		_qh: &QueueHandle<Self>,
	) -> std::sync::Arc<dyn wayland_client::backend::ObjectData> {
		unreachable!("foreign_toplevel_handle has no children");
	}
}

/// Query the compositor for all toplevel windows, returning their app_id
/// and title (skipping entries with empty app_id). Entries are deduplicated
/// by app_id.
pub fn get_toplevels() -> Vec<(String, String)> {
	let Some(conn) = Connection::connect_to_env().ok() else {
		return Vec::new();
	};

	let Some((globals, mut event_queue)) = registry_queue_init::<AppData>(&conn).ok() else {
		return Vec::new();
	};

	let qh = event_queue.handle();

	let _manager: zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1 =
		match globals.bind(&qh, 1..=3, ()) {
			Ok(m) => m,
			Err(_) => return Vec::new(),
		};

	let mut app_data = AppData::new();

	if event_queue.roundtrip(&mut app_data).is_err() {
		return Vec::new();
	}

	let mut seen = std::collections::HashSet::new();
	let mut results = Vec::new();
	for info in app_data.handles.values() {
		if info.app_id.is_empty() || !seen.insert(info.app_id.clone()) {
			continue;
		}
		let label = if info.title.is_empty() {
			info.app_id.clone()
		} else {
			format!("{} — {}", info.app_id, info.title)
		};
		results.push((info.app_id.clone(), label));
	}
	results
}

/// Query the compositor for the currently active (focused) toplevel's
/// app_id. Skips gpuitop itself. Returns `None` if no other toplevel
/// is focused.
pub fn get_focused_window_app_id() -> Option<String> {
	let conn = Connection::connect_to_env().ok()?;
	let (globals, mut event_queue) =
		registry_queue_init::<AppData>(&conn).ok()?;
	let qh = event_queue.handle();
	let _manager: zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1 =
		globals.bind(&qh, 1..=3, ()).ok()?;
	let mut app_data = AppData::new();
	event_queue.roundtrip(&mut app_data).ok()?;
	app_data
		.handles
		.values()
		.find(|info| info.activated)
		.and_then(|info| {
			if info.app_id.is_empty()
				|| info.app_id == crate::GPUITOP_APP_ID
			{
				None
			} else {
				Some(info.app_id.clone())
			}
		})
}
