use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[derive(Debug, Default)]
pub struct Cli {
	pub config_path: Option<PathBuf>,
	pub overrides: Vec<String>,
	pub page: Option<String>,
	pub search: Option<String>,
	pub properties_pid: Option<i32>,
	pub app_id: Option<String>,
	pub print_version: bool,
	pub print_help: bool,
}

impl Cli {
	pub fn active_tab(&self) -> usize {
		match self.page.as_deref() {
			Some(s) if s == "processes" || s.starts_with("processes.") => 0,
			Some(s)
				if s == "performance" || s.starts_with("performance.") =>
			{
				1
			}
			Some(s) if s == "settings" || s.starts_with("settings.") => 2,
			_ => 0,
		}
	}

	pub fn performance_tab_index(&self) -> Option<usize> {
		match self.page.as_deref() {
			Some("performance") => None,
			Some("performance.cpu") => Some(0),
			Some("performance.memory") => Some(1),
			Some("performance.gpu") => Some(2),
			Some("performance.disks") => Some(3),
			Some("performance.network") => Some(4),
			_ => None,
		}
	}

	pub fn settings_page_index(&self) -> Option<usize> {
		match self.page.as_deref() {
			Some("settings") => None,
			Some("settings.general") => Some(0),
			Some("settings.processes") => Some(1),
			Some("settings.about") => Some(2),
			_ => None,
		}
	}

	pub fn override_view(&self) -> Option<bool> {
		match self.page.as_deref() {
			Some("processes.tree") => Some(true),
			Some("processes.list") => Some(false),
			_ => None,
		}
	}
}

fn print_version() {
	println!("gpuitop {VERSION}");
}

fn print_help() {
	print!(
		r"gpuitop {VERSION} — {DESCRIPTION}

USAGE:
    gpuitop [OPTIONS]

OPTIONS:
    -c, --config <PATH>       Load config from file instead of default
    --override <RON>          Partial RON config layered over loaded config
                              (repeatable; last occurrence wins per section)
    -p, --page <PATH>         Navigate to tab/page:
                                processes       Processes tab
                                processes.tree  Processes tab, tree view
                                processes.list  Processes tab, list view
                                performance     Performance tab
                                performance.cpu      Performance > CPU
                                performance.memory   Performance > Memory
                                performance.gpu      Performance > GPU
                                performance.disks    Performance > Disks
                                performance.network  Performance > Network
                                settings        Settings tab
                                settings.general   Settings > General
                                settings.processes Settings > Processes
                                settings.about     Settings > About
    -s, --search <TEXT>       Pre-fill the process search bar
    --properties <PID>        Open the properties window for a PID
                              (standalone, no main window)
    --app-id <ID>             Override the Wayland/x11 app_id used by the
                              window manager (default: com.github.kyza.gpuitop)
    -v, --version             Print version and exit
    -h, --help                Print this help and exit

EXAMPLES:
    gpuitop --page settings.about
    gpuitop --config ~/gaming.ron --override '(general: (interface: (refresh_ms: 500)))'
    gpuitop -p processes.tree -s firefox
"
	);
}

pub fn parse() -> Cli {
	let mut cli = Cli::default();
	let mut parser = lexopt::Parser::from_env();

	loop {
		let arg = match parser.next() {
			Ok(None) => break,
			Ok(Some(arg)) => arg,
			Err(e) => {
				eprintln!("{e}");
				std::process::exit(1);
			}
		};
		match arg {
			lexopt::Arg::Short('c') | lexopt::Arg::Long("config") => {
				cli.config_path = parser.value().ok().map(PathBuf::from);
			}
			lexopt::Arg::Long("override") => {
				if let Ok(val) = parser.value() {
					let s = val.to_str().unwrap_or("").to_string();
					cli.overrides.push(s);
				}
			}
			lexopt::Arg::Short('p') | lexopt::Arg::Long("page") => {
				cli.page = parser
					.value()
					.ok()
					.and_then(|v| v.to_str().map(|s| s.to_string()));
			}
			lexopt::Arg::Short('s') | lexopt::Arg::Long("search") => {
				cli.search = parser
					.value()
					.ok()
					.and_then(|v| v.to_str().map(|s| s.to_string()));
			}
			lexopt::Arg::Long("properties") => {
				cli.properties_pid = parser
					.value()
					.ok()
					.and_then(|v| v.to_str().and_then(|s| s.parse().ok()));
			}
			lexopt::Arg::Long("app-id") => {
				cli.app_id = parser
					.value()
					.ok()
					.and_then(|v| v.to_str().map(|s| s.to_string()));
			}
			lexopt::Arg::Short('v') | lexopt::Arg::Long("version") => {
				cli.print_version = true;
			}
			lexopt::Arg::Short('h') | lexopt::Arg::Long("help") => {
				cli.print_help = true;
			}
			lexopt::Arg::Value(val) => {
				eprintln!(
					"unexpected argument: {}",
					val.to_str().unwrap_or("?")
				);
				std::process::exit(1);
			}
			_ => {}
		}
	}

	if cli.print_version {
		print_version();
		std::process::exit(0);
	}

	if cli.print_help {
		print_help();
		std::process::exit(0);
	}

	cli
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_active_tab_processes() {
		let c = Cli {
			page: Some("processes".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 0);
	}

	#[test]
	fn test_active_tab_processes_tree() {
		let c = Cli {
			page: Some("processes.tree".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 0);
	}

	#[test]
	fn test_active_tab_performance() {
		let c = Cli {
			page: Some("performance".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 1);
	}

	#[test]
	fn test_active_tab_settings() {
		let c = Cli {
			page: Some("settings".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 2);
	}

	#[test]
	fn test_active_tab_settings_subpage() {
		let c = Cli {
			page: Some("settings.general".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 2);
	}

	#[test]
	fn test_active_tab_default() {
		let c = Cli::default();
		assert_eq!(c.active_tab(), 0);
	}

	#[test]
	fn test_active_tab_unknown() {
		let c = Cli {
			page: Some("bogus".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 0);
	}

	#[test]
	fn test_settings_page_index_none_for_settings_root() {
		let c = Cli {
			page: Some("settings".into()),
			..Default::default()
		};
		assert_eq!(c.settings_page_index(), None);
	}

	#[test]
	fn test_settings_page_index_general() {
		let c = Cli {
			page: Some("settings.general".into()),
			..Default::default()
		};
		assert_eq!(c.settings_page_index(), Some(0));
	}

	#[test]
	fn test_settings_page_index_processes() {
		let c = Cli {
			page: Some("settings.processes".into()),
			..Default::default()
		};
		assert_eq!(c.settings_page_index(), Some(1));
	}

	#[test]
	fn test_settings_page_index_about() {
		let c = Cli {
			page: Some("settings.about".into()),
			..Default::default()
		};
		assert_eq!(c.settings_page_index(), Some(2));
	}

	#[test]
	fn test_settings_page_index_not_settings() {
		let c = Cli {
			page: Some("processes".into()),
			..Default::default()
		};
		assert_eq!(c.settings_page_index(), None);
	}

	#[test]
	fn test_settings_page_index_none_for_no_page() {
		let c = Cli::default();
		assert_eq!(c.settings_page_index(), None);
	}

	#[test]
	fn test_override_view_tree() {
		let c = Cli {
			page: Some("processes.tree".into()),
			..Default::default()
		};
		assert_eq!(c.override_view(), Some(true));
	}

	#[test]
	fn test_performance_tab_index_root() {
		let c = Cli {
			page: Some("performance".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 1);
		assert_eq!(c.performance_tab_index(), None);
	}

	#[test]
	fn test_performance_tab_index_cpu() {
		let c = Cli {
			page: Some("performance.cpu".into()),
			..Default::default()
		};
		assert_eq!(c.active_tab(), 1);
		assert_eq!(c.performance_tab_index(), Some(0));
	}

	#[test]
	fn test_performance_tab_index_gpu() {
		let c = Cli {
			page: Some("performance.gpu".into()),
			..Default::default()
		};
		assert_eq!(c.performance_tab_index(), Some(2));
	}

	#[test]
	fn test_performance_tab_index_network() {
		let c = Cli {
			page: Some("performance.network".into()),
			..Default::default()
		};
		assert_eq!(c.performance_tab_index(), Some(4));
	}

	#[test]
	fn test_performance_tab_index_none_for_other() {
		let c = Cli {
			page: Some("processes".into()),
			..Default::default()
		};
		assert_eq!(c.performance_tab_index(), None);
	}

	#[test]
	fn test_override_view_list() {
		let c = Cli {
			page: Some("processes.list".into()),
			..Default::default()
		};
		assert_eq!(c.override_view(), Some(false));
	}

	#[test]
	fn test_override_view_none_for_processes() {
		let c = Cli {
			page: Some("processes".into()),
			..Default::default()
		};
		assert_eq!(c.override_view(), None);
	}

	#[test]
	fn test_override_view_none_for_no_page() {
		let c = Cli::default();
		assert_eq!(c.override_view(), None);
	}
}
