# First tick is a throwaway warm-up

All rates (CPU%, disk/network bytes-per-second) are deltas between consecutive samples, so the first tick has no previous sample and reports zeros. `prev_time` is stamped at the *end* of each tick so elapsed is always the gap between two real collections.

The alternative (seed with an initial sample before the first render) adds state and a second read for no user-visible benefit. The cost is accepted: a single zero-tick at startup.
