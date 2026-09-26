# Learning path

1. Read [THREAT_MODEL.md](THREAT_MODEL.md) — purpose and limits  
2. Read [CRYPTO.md](CRYPTO.md) — password → keys → AES-GCM  
3. Read [FORMAT.md](FORMAT.md) — on-disk / in-image layout  
4. Read [EMBEDDING.md](EMBEDDING.md) — Method A vs B  
5. Trace `crates/stego-core/src/ops.rs` — hide/extract orchestration  
6. Trace `crypto.rs` → `metadata.rs` → `embedding/*` → `detection.rs`  
7. Try the CLI round-trip in the root README  
8. Open `apps/desktop` to see the thin GUI over the same core  
9. Walk the **Demo** tab in the GUI (or fixtures below)

## Executable demo (transparent)

Goal: show that a small program/script can be **hidden** in a cover, that the cover still opens normally, and that recovery + optional **Run** happen only inside Open Stego with an explicit confirm.

1. Hide `test-fixtures/demo/demo-hello.bat` (or `.sh`) into a PNG/PDF with a password.  
2. Open the stego cover in Photos/Acrobat — you only see the normal file. Trailing/LSB data is **not** executed by those apps.  
3. Extract in the GUI or CLI; if kind is `executable`, Save then optionally Run (GUI double-confirm, or CLI `--run --i-understand`).  

Why no auto-run on double-click? Viewers interpret their own format only. Stego payloads are opaque encrypted bytes until *this* tool decrypts them. That is how open formats and OS security models work — and it keeps the demo honest for classrooms.

See also [PHASE2_PLAN.md](PHASE2_PLAN.md) and [PHASE2_START.md](PHASE2_START.md).

## Tests as examples

```powershell
cargo test -p stego-core --lib
```

Unit tests cover crypto round-trips, metadata checksums (including executable flag), EOF fixtures (PDF/MP3/ZIP-like), LSB PNG round-trips, and detection rules.

## Contributing

Doc clarity and extra fixtures are welcome. Keep the container versioned; bump `FORMAT_VERSION` only with a migration story.
