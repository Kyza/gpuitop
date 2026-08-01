pub mod collector;
pub mod gpu;
pub mod process;
pub mod wayland;

pub use collector::SystemCollector;
pub use gpu::detect_gpu;
pub use wayland::get_focused_window_app_id;
