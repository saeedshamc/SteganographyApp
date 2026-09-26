# Mobile (Flutter) — deferred

This folder is reserved for the **Flutter** mobile client.

Desktop (Rust core + CLI + Tauri) ships first. After desktop stages are complete, this app will:

- Speak the same on-disk format documented in [`docs/FORMAT.md`](../../docs/FORMAT.md)
- Prefer calling `stego-core` via FFI, or a thin compatible layer if FFI is impractical

## Do not remove

- Keep this README so the mobile path stays visible in the monorepo
- System Flutter SDK (e.g. `C:\src\flutter`) is managed outside this repo — do not delete it while preparing mobile

## Status

**Not started.** Waiting on desktop Stage 10.
