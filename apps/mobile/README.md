# Mobile (Flutter) — Phase F

Desktop (Rust `stego-core` + CLI + Tauri) is the source of truth for the wire format in [`docs/FORMAT.md`](../../docs/FORMAT.md).

## Status

**Scaffold reserved.** Core format now includes:

- Executable payload kind (`flags bit1`)
- KDF profiles + optional keyfile (envelope v1/v2)
- Adaptive LSB (desktop/CLI)

## When implementing

1. `flutter create .` in this directory (keep system Flutter SDK).
2. Expose `stego-core` via **FFI** (`cbindgen` / `flutter_rust_bridge`) for `hide_with` / `extract_with`.
3. Mirror Hide / Extract / Demo UX from `apps/desktop` — Save then optional Run with confirm; no OS auto-run.
4. Same passwords / keyfiles / profiles as CLI.

See [`docs/PHASE2_PLAN.md`](../../docs/PHASE2_PLAN.md) Phase F.
