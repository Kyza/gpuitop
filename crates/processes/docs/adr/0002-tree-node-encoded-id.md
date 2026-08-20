# Tree node identity is an encoded string

Tree nodes are keyed by the string `"pid-{pid}:match|ancestor"` — the same id doubles as the stable expand-state key and is parsed back into pid + match-status by the renderer. Because `set_items()` wipes expand state, `preserve_expand_from` walks the old ids and re-applies `.expanded(true)` on the matching new nodes.

The alternative (typed ids or a side table) would avoid the string-splitting, but the encoded form keeps the id and the render decision in one value and needs no bookkeeping object.
