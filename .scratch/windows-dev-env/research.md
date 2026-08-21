# Windows Dev Environment for gpuitop — Research

Question: how to set up a Windows development environment on Arch Linux (x86_64, KVM-capable) for building/testing gpuitop's Windows code paths, ideally a "light" VM.

Researched 2026-08-20 against primary sources. Repo examined at current trunk.

---

## 1. Current state of the repo's Windows support

All nine `windows.rs` platform files are stubs — zero real Windows code:

| Crate | File | Returns |
|---|---|---|
| snapshot | `crates/snapshot/src/windows.rs:5-7` | `SystemSnapshot::empty()` |
| gpu | `crates/gpu/src/windows.rs:5-15` | empty `HashMap` / empty vecs |
| window_picker | `crates/window_picker/src/windows.rs:3-9` | `false` / `None` |
| elevation | `crates/elevation/src/windows.rs:1-7` | `false` / `Err("not supported on Windows")` |
| core/service_manager | `crates/core/src/service_manager/windows.rs:6-27` | `InitSystem::Unknown`, empty sets, `false` |
| icons | `crates/icons/src/windows.rs:5-13` | empty cache / `None` |
| properties | `crates/properties/src/windows.rs:5-7` | `Err(ReadError::Dead)` |
| processes/affinity | `crates/processes/src/affinity/windows.rs:1-7` | `Err("CPU affinity is not supported")` |
| properties/window | `crates/properties/src/window/windows.rs:7-20` | "not available on this platform" body |

macOS is the same shape. Linux is real (`crates/snapshot/src/linux/` + linux.rs, ~1,240 lines, all `/proc` via `procfs`). The project is effectively Linux-only with a clean stub seam.

### Does it compile on Windows today? No — two hard blockers

1. **`users` crate (0.11)** — non-conditional dep of `snapshot` and `properties` (`Cargo.toml`s). It's Unix-only (`libc::getpwuid_r` in `users-0.11.0/src/base.rs:336`) and only *used* in `crates/snapshot/src/linux/proc_basics.rs:67`. Will not compile for a Windows target; takes the whole workspace with it.
2. **`processes/src/context_menu.rs`** calls `libc::kill` + SIGKILL/SIGCONT/SIGSTOP/SIGHUP/SIGQUIT/SIGUSR1/SIGUSR2 unconditionally (`context_menu.rs:16,63-113`). libc's Windows module defines no `kill` and only SIGINT/SIGILL/SIGFPE/SIGSEGV/SIGTERM/SIGABRT (`libc-0.2.189/src/windows/mod.rs:236-249`). Compile error.

Everything else is cross-platform or properly gated:
- `procfs` 0.16 has no `target_os` gating — compiles on Windows, dead weight there (only linux.rs calls it). Move to `[target.'cfg(target_os = "linux")'.dependencies]` like `linicon` already is.
- `linicon` correctly gated Linux-only; `nvml-wrapper` cross-platform; wayland/x11rb window_picker deps compile anywhere (fail at runtime without a display); `core/src/cpu.rs:2-14` properly gated.
- No `.github/` directory at all — no Windows CI exists.

### The UI stack supports Windows

- gpui pinned at zed `5e1fd39`: full Win32/DX11 backend (`gpui_windows` crate). `crates/gpui_platform/src/gpui_platform.rs` dispatches `Rc::new(gpui_windows::WindowsPlatform::new(...))` on `target_os = "windows"`.
- Rendering is **Direct3D 11 + DirectWrite**. `directx_devices.rs:126-150` requires **Feature Level 10.1+ with compute/StructuredBuffer** (`ComputeShaders_Plus_RawAndStructuredBuffers_Via_Shader_4_x`); no WARP/software fallback in the adapter loop — ends in `unreachable!()` if none matches.
- Zed's own CI runs Windows (MSVC on Windows Server 2022 runners): `.github/workflows/run_tests.yml:191,327`, `release.yml:119,226,637`.
- `gpui_windows/build.rs` only compiles HLSL shaders in **release** builds and needs `fxc.exe` from a Windows SDK (or `GPUI_FXC_PATH`). Debug/`cargo check` builds skip it.

Sources: zed repo at pinned rev `5e1fd39`; https://github.com/zed-industries/zed

---

## 2. Can it cross-compile to Windows from Linux?

- `rustup target add x86_64-pc-windows-msvc` + a linker is the mechanism; MSVC needs VS install, MSVC is the recommended ABI. https://rust-lang.github.io/rustup/cross-compilation.html, https://rust-lang.github.io/rustup/installation/windows.html
- **cargo-xwin** is the canonical cross-compile tool: downloads MSVC CRT + Windows SDK via `xwin`, drives `clang-cl`/`lld-link`. Usage: `cargo xwin build --target x86_64-pc-windows-msvc`. `cargo xwin test` runs under Wine. Downloading the SDK accepts the MS license (https://go.microsoft.com/fwlink/?LinkId=2086102). https://github.com/rust-cross/cargo-xwin
- `procfs` is Linux-only (WSL2 supported): https://docs.rs/procfs/latest/procfs/ — but not a compile blocker here since it's never invoked on Windows.
- **Cross-compiling answers only "does it build?"** The Windows data paths are all stubs, and the app is a D3D11 GPU-rendered GUI. A real Windows runtime (VM) is **required** to test Windows code paths. Wine can't reliably run a D3D11/GPU desktop app; MSVC target, not GNU (zed CI is MSVC-only, gpui uses `windows-registry` at runtime).
- Release cross-builds need `fxc.exe`; debug/check builds don't.

---

## 3. Light Windows VM on Arch/KVM

### Host

- KVM needs VT-x/AMD-V + `kvm`/`kvm_intel`/`kvm_amd` modules; use `-accel kvm` + `-cpu host`. Without KVM Windows is unusably slow. https://wiki.archlinux.org/title/QEMU, https://wiki.archlinux.org/title/KVM
- Packages: `qemu-desktop` (GUI) or `qemu-base` (headless); libvirt/virt-manager is the documented management path. Windows 11 needs **UEFI/OVMF** (`edk2-ovmf`) + **software TPM 2.0** via `swtpm`. https://wiki.archlinux.org/title/QEMU

### Guest OS

- **Windows 11 Enterprise eval ISO** — free, no product key, **90 days**; after expiry desktop goes black + hourly shutdown. https://www.microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise
- Win11 min reqs: 1 GHz 2-core, 4 GB RAM, 64 GB disk, DX12/WDDM 2.0, UEFI Secure Boot, TPM 2.0. VM: gen-2 UEFI, vTPM, 4 GB, 2+ vCPUs, 64 GB. https://learn.microsoft.com/en-us/windows/whats-new/windows-11-requirements
- Windows Server 2025 eval = 180 days but Server Core has no desktop → can't run the GUI app. https://www.microsoft.com/en-us/evalcenter/evaluate-windows-server-2025
- Prebuilt eval VM images page (`developer.microsoft.com/.../virtual-machines/`) currently redirects to a generic hub — verify at download time; plan on the eval ISO (~30 min install).

### Drivers / display (the decisive question)

- virtio-win drivers from Fedora: https://fedorapeople.org/groups/virt/virtio-win/direct-downloads/stable-virtio/virtio-win.iso (also `virtio-win` AUR). virtio-blk/virtio-net are the performance levers. https://wiki.archlinux.org/title/QEMU "Preparing a Windows guest"
- virtio-win is signed with Red Hat test certs, not WHQL — disable Secure Boot in-guest or install the cert. https://github.com/virtio-win/virtio-win-pkg-scripts
- `std`/`qxl`/`cirrus` VGAs have no Windows D3D11 driver. virtio-win ships a **virtio-gpu WDDM display driver for Windows** (`viogpu`, `Class=Display`): https://github.com/virtio-win/virtio-win-guest-tools-installer README. Whether it meets gpui's FL 10.1 + StructuredBuffer requirement must be **verified empirically**.
- **Fallback: WARP** — Microsoft Basic Display Adapter exposes WARP (full D3D11 software rasterizer, FL 11_1) through DXGI. https://learn.microsoft.com/en-us/windows-hardware/drivers/display/microsoft-basic-display-driver — gpui's `EnumAdapters` loop will find it; slow but functional for a dev loop. GPU passthrough not required upfront.
- RDP into the guest is the documented way to see the desktop (RDP sessions standardly use WARP). https://wiki.archlinux.org/title/QEMU "Windows-specific notes"

### Sizing

8–16 GB RAM, 4–8 host cores, 64 GB+ disk. qcow2; `nocow` on btrfs. First build is slow everywhere (`[profile.dev.package."*"] opt-level = 2`). https://wiki.archlinux.org/title/QEMU "Improve virtual machine performance"

---

## 4. Dev workflow inside the VM

- **Edit on host, build/test in guest.** Share the repo:
  - **virtiofs** — `Z:` drive auto-mapped; needs `viofs` driver + WinFsp. https://virtio-fs.gitlab.io/howto-windows.html — best choice.
  - QEMU built-in SMB: `-nic user,id=nic0,smb=shared_dir` → `\\10.0.2.4\qemu`; needs `samba` on host. https://wiki.archlinux.org/title/QEMU "QEMU's built-in SMB server"
  - 9pfs VirtFS also possible; sshfs/rsync last resort. https://code.visualstudio.com/docs/remote/ssh
- **SSH**: `-nic user,hostfwd=tcp::60022-:22` then `ssh guest@127.0.0.1 -p 60022`; vsock is faster. https://wiki.archlinux.org/title/QEMU "port forwarding" and "Accessing SSH via vsock"
- **VSCode Remote-SSH** is officially supported on Windows guests (1803+ with OpenSSH Server). https://code.visualstudio.com/docs/remote/ssh
- **Toolchain**: rustup default `stable-x86_64-pc-windows-msvc` + **VS Build Tools** (no IDE): `vs_buildtools.exe --add Microsoft.VisualStudio.Workload.VCTools`. https://learn.microsoft.com/en-us/visualstudio/install/use-command-line-parameters-to-install-visual-studio; bootstrapper https://aka.ms/vs/17/release/vs_buildtools.exe

### The one cargo rule that matters

- **Never put `target/` on the share** — building zed's gpui + opt-level-2 deps over 9p/SMB/virtiofs (file-locking semantics) is 10–50× slower or spuriously fails. Set `CARGO_TARGET_DIR` to guest-local disk; only the source tree lives on the share.

---

## 5. Licensing

- Win11 Enterprise eval: free, 90 days, no key, hourly-shutdown after expiry. https://www.microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise
- Win Server 2025 eval: 180 days, activate within 10 days, no desktop on Server Core. https://www.microsoft.com/en-us/evalcenter/evaluate-windows-server-2025
- virtio-win: free, Red Hat test certs, WHQL builds only with paid RHEL subscription. https://github.com/virtio-win/virtio-win-pkg-scripts
- cargo-xwin/xwin: downloading MS SDK accepts MS license. https://github.com/rust-cross/cargo-xwin
- gpui / gpui-component: Apache-2.0. https://github.com/zed-industries/zed, https://github.com/longbridge/gpui-component

---

## Open questions / decisions

1. **Prebuilt eval VM vs ISO** — VM images page redirects to a hub; decide whether to hunt archived VHDX or just do the ISO (~1–2 h total incl. toolchain).
2. **viogpu FL vs gpui's requirement** — empirically test current virtio-win viogpu against gpui's FL 10.1 + StructuredBuffer; else rely on WARP; GPU passthrough only if WARP is unusable.
3. **Fix the two compile blockers in the repo first** (unix-gate `users` + `context_menu.rs`) — nothing Windows builds without them.
4. **cargo-xwin** only worth it as a cheap compile gate (debug/check only; release needs fxc.exe).

## Recommended approach (lightest VM that works)

1. Fix the two compile blockers (`users` dep + `context_menu.rs` signal calls).
2. Host: `qemu-desktop` + `edk2-ovmf` + `swtpm` (+ `samba` if using SMB share). KVM, `-cpu host`, machine `q35`, OVMF pflash, swtpm TPM. Disable Secure Boot in-guest or install Red Hat test cert for virtio drivers.
3. Guest: Win11 Enterprise eval ISO, 8–16 GB RAM / 4–8 vCPUs / 64 GB+ disk. Load `virtio-win.iso` at install for virtio-blk/net; then guest tools (adds `viogpu`, `viofs`, guest-agent).
4. Display: virtio-gpu + viogpu WDDM; verify gpui renders (DXGI/D3D11). Fall back to WARP if feature level short; no GPU passthrough upfront.
5. Files: virtiofs `Z:` share (or QEMU SMB); `CARGO_TARGET_DIR` on guest-local disk.
6. Workflow: edit on Arch → VSCode Remote-SSH (port-forwarded) into guest → rustup MSVC + VS Build Tools → `cargo check`/`cargo test`. RDP to see the app.
7. Recycle: qcow2 overlay/snapshot the install so eval expiry means fresh VM, not re-toolchain; keep ISO around.
