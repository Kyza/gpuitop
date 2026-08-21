# 02 — Windows 11 dev VM on Arch/KVM

**What to build:** a bootable Windows 11 Enterprise eval VM on the Arch host (QEMU/KVM, OVMF + swtpm, virtio-blk/net/display drivers, virtiofs repo share, `CARGO_TARGET_DIR` on guest-local disk) with SSH access, rustup (MSVC), and VS Build Tools installed. The decisive acceptance check: run the gpuitop binary inside the VM and confirm the gpui window renders — via the viogpu driver or WARP fallback. This VM is the runtime every later Windows feature ticket tests on.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] VM boots Windows 11 Enterprise eval (90-day ISO, free, no key) under KVM with OVMF + swtpm
- [ ] virtio drivers installed (disk + net at minimum); SSH reachable from the host via port forwarding
- [ ] Repo shared via virtiofs; `CARGO_TARGET_DIR` on guest-local disk
- [ ] rustup MSVC + VS Build Tools installed; `cargo check` of the workspace succeeds inside the VM
- [ ] gpuitop launches in the VM and the window renders (viogpu or WARP; note which)

References: `.scratch/windows-dev-env/research.md` §3, §4, §5.
