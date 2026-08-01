pub mod performance;
pub mod processes;
pub mod settings;

pub use performance::PerformanceTab;
pub use processes::ProcessesTab;
pub use settings::SettingsTab;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
	Processes,
	Performance,
	Settings,
}
