# Orphan-rule newtype for `TableDelegate`

The data-pure core delegate is a foreign type to gpui_component, so the `TableDelegate` trait impl lives on a newtype wrapper in this crate: `ProcessTableDelegate(pub CoreDelegate)` with `Deref`/`DerefMut`. The core delegate stays gpui-free; the wrapper is a transparent handle.

This is the direct cost of ADR-0001 (data-pure core). The alternative — implementing the trait on the core type — would drag gpui into core and break the hard dependency rule.
