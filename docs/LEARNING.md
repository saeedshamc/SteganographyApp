# Learning path

1. Read [THREAT_MODEL.md](THREAT_MODEL.md) — purpose and limits  
2. Read [CRYPTO.md](CRYPTO.md) — password → keys → AES-GCM  
3. Read [FORMAT.md](FORMAT.md) — on-disk / in-image layout  
4. Read [EMBEDDING.md](EMBEDDING.md) — Method A vs B  
5. Trace `crates/stego-core/src/ops.rs` — hide/extract orchestration  
6. Trace `crypto.rs` → `metadata.rs` → `embedding/*` → `detection.rs`  
7. Try the CLI round-trip in the root README  
8. Open `apps/desktop` to see the thin GUI over the same core  

## Tests as examples

```powershell
cargo test -p stego-core --lib
```

Unit tests cover crypto round-trips, metadata checksums, EOF fixtures (PDF/MP3/ZIP-like), LSB PNG round-trips, and detection rules.

## Contributing

Doc clarity and extra fixtures are welcome. Keep the container versioned; bump `FORMAT_VERSION` only with a migration story.
