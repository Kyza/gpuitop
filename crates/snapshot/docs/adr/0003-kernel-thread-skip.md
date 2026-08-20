# Kernel threads skip /proc reads

Kernel threads (parented by `kthreadd` or with a bracketed name) skip cmdline, cgroup, environ, and VRAM reads entirely during collection. On a typical desktop hundreds of kernel threads exist; reading their cmdline/environ per tick is wasted syscalls with no displayable content.

The heuristic (ppid 2 or `[name]`) is cheap and near-universally accurate on Linux. A kernel thread that somehow escapes the heuristic reads its (empty) files harmlessly.
