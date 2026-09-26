# Mobile (Flutter) — deferred until desktop is done

This folder reserves the **Flutter** mobile client. Do not delete it.

Desktop (Rust `stego-core` + CLI + Tauri) is the source of truth for the wire format in [`docs/FORMAT.md`](../../docs/FORMAT.md).

## Planned approach (when starting this phase)

1. Keep system Flutter SDK (e.g. `C:\src\flutter`) intact — prepare `flutter doctor` separately.
2. Create a Flutter app here (`flutter create .` in this directory when ready).
3. Prefer **FFI** to `stego-core` (same encrypt/embed/extract behavior as desktop), or a thin Dart layer that matches `FORMAT.md` byte-for-byte if FFI is blocked on a target.
4. Reuse Hide / Extract UX concepts from `apps/desktop` (method badge, capacity, password strength warn, clear errors).

## Status

**Scaffold only.** Implementation starts after desktop Stage 10.
