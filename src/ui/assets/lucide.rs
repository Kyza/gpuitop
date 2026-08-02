use anyhow::anyhow;
use gpui::*;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "lucide/icons"]
#[include = "*.svg"]
pub struct LucideAssets;

impl AssetSource for LucideAssets {
	fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
		if path.is_empty() {
			return Ok(None);
		}
		Self::get(path)
			.map(|f| Some(f.data))
			.ok_or_else(|| anyhow!("asset not found: {path}"))
	}

	fn list(&self, path: &str) -> Result<Vec<SharedString>> {
		Ok(Self::iter()
			.filter_map(|p| p.starts_with(path).then(|| p.into()))
			.collect())
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LucideIcon {
	Activity,
	Atom,
	ChartPie,
	ChevronDown,
	ChevronRight,
	ChevronUp,
	CircleCheck,
	CircleOff,
	CircleX,
	Close,
	Copy,
	Cpu,
	Crosshair,
	ExternalLink,
	Eye,
	File,
	FolderTree,
	Funnel,
	Gpu,
	HardDrive,
	HardDriveDownload,
	HardDriveUpload,
	Hash,
	Info,
	LayoutDashboard,
	List,
	MemoryStick,
	Menu,
	Moon,
	Palette,
	Pause,
	Pin,
	Play,
	Search,
	Settings,
	Settings2,
	SquareTerminal,
	Sun,
	User,
}

impl LucideIcon {
	pub fn path(self) -> &'static str {
		match self {
			Self::Activity => "activity.svg",
			Self::Atom => "atom.svg",
			Self::ChartPie => "chart-pie.svg",
			Self::ChevronDown => "chevron-down.svg",
			Self::ChevronRight => "chevron-right.svg",
			Self::ChevronUp => "chevron-up.svg",
			Self::CircleCheck => "circle-check.svg",
			Self::CircleOff => "circle-off.svg",
			Self::CircleX => "circle-x.svg",
			Self::Close => "x.svg",
			Self::Copy => "copy.svg",
			Self::Cpu => "cpu.svg",
			Self::Crosshair => "crosshair.svg",
			Self::ExternalLink => "external-link.svg",
			Self::Eye => "eye.svg",
			Self::File => "file.svg",
			Self::FolderTree => "folder-tree.svg",
			Self::Funnel => "funnel.svg",
			Self::Gpu => "gpu.svg",
			Self::HardDrive => "hard-drive.svg",
			Self::HardDriveDownload => "hard-drive-download.svg",
			Self::HardDriveUpload => "hard-drive-upload.svg",
			Self::Hash => "hash.svg",
			Self::Info => "info.svg",
			Self::LayoutDashboard => "layout-dashboard.svg",
			Self::List => "list.svg",
			Self::MemoryStick => "memory-stick.svg",
			Self::Menu => "menu.svg",
			Self::Moon => "moon.svg",
			Self::Palette => "palette.svg",
			Self::Pause => "pause.svg",
			Self::Pin => "pin.svg",
			Self::Play => "play.svg",
			Self::Search => "search.svg",
			Self::Settings => "settings.svg",
			Self::Settings2 => "settings-2.svg",
			Self::SquareTerminal => "square-terminal.svg",
			Self::Sun => "sun.svg",
			Self::User => "user.svg",
		}
	}

	pub fn icon(self) -> gpui_component::Icon {
		gpui_component::Icon::default().path(SharedString::from(self.path()))
	}
}
