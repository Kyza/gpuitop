# Cumulative resources are aggregated bottom-up, depth-sorted

`CumulativeResources` (a process plus all descendants) is computed by sorting processes deepest-first and summing each child into its parent in one pass, memoized in `cum_cache`. Depth-sorting guarantees each parent sees fully-aggregated children without recursion or repeated walks — O(n log n) once per snapshot, cached and invalidated wholesale.

The alternative (walk each subtree recursively per process) is O(n²) on deep trees and re-walks shared ancestors. The depth-sort keeps the hot filtering/rendering path cheap at the cost of a per-snapshot sort and a whole-map cache.
