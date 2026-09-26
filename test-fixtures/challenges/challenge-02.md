# Challenge 02 — Depth-2 capacity trade-off

**Goal:** Feel why LSB depth 2 is noisier.

1. Hide the same short text in a small PNG with `--lsb-depth 1` and again with `--lsb-depth 2`.
2. Open both outputs and zoom. Which looks dirtier?
3. Run `stego plan --cover … --json` for each depth setting via the GUI depth control / CLI option.

**Instructor notes:** Depth 2 roughly doubles channel bit budget but flips more low-order bits; prefer depth 1 for stealth demos.
