# `--properties` standalone window mode

The properties window can run as its own app: `gpuitop --properties <PID>` opens only the client-decorated "Properties — PID N" window with no main UI, and quits when the window closes. The normal app mode hosts the same `PropertiesWindow` as a popup over the process table.

One window component, two hosts. The alternative — a separate binary — would duplicate the bootstrap, window plumbing, and theme handling. The cost is a host-conditional path through `main.rs` that must branch on the CLI mode before constructing `App`.
