# User / System / Services / Kernel filter buckets are disjoint

The four "ownership" filters partition the process set by construction:

- `User` = owned by the current user **and not a GUI app**
- `System` = not a kernel thread, not owned by the current user, and `ppid != 1`
- `Services` = managed by the detected init system
- `Kernel` = kernel thread

`User` deliberately excludes GUI apps, and `System` deliberately excludes both the current user's processes and init's direct children. The buckets never overlap. This is narrower than the toolbar tooltips imply ("not owned by any user"), so the definitions above are the contract.
