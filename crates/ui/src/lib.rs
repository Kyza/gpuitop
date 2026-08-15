#![recursion_limit = "512"]

pub mod app;
pub mod assets;
pub mod performance;
pub mod processes;
pub mod properties_window;
pub mod settings;
pub mod theme;
pub mod themes;

#[allow(dead_code)]
mod built {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
