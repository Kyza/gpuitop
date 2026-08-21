pub fn desktop() -> String {
	"[Desktop Entry]
Type=Application
Name=gpuitop
Comment=A GPU-accelerated task manager with per-process VRAM tracking
Exec=gpuitop
Terminal=false
Categories=System;Monitor;
"
	.into()
}
