pub mod collector;
pub mod gpu;
pub mod process;
pub mod system;
pub mod wayland;

pub use collector::SystemCollector;
pub use gpu::detect_gpu;
pub use system::detect_init;
pub use wayland::get_focused_window_app_id;
