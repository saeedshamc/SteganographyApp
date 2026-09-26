# Open Stego

Open-source steganography toolkit: hide encrypted payloads in any cover file, extract them later, and learn how it works.

**Desktop first (this repo):** Rust core + CLI + Tauri GUI (Windows / Linux).  
**Mobile later:** Flutter app in [`apps/mobile`](apps/mobile) — same on-disk format, after desktop stages land.

## Layout

| Path | Role |
|------|------|
| `crates/stego-core` | Crypto, metadata, Method A/B embedding |
| `crates/stego-cli` | `stego` command-line tool |
| `apps/desktop` | Tauri 2 GUI |
| `apps/mobile` | Flutter placeholder (post-desktop) |
| `docs/` | Open format, crypto, threat model, learning path |

## Quick start (desktop)

```powershell
# CLI
cargo build -p stego-cli
cargo run -p stego-cli -- --help

# GUI
cd apps/desktop
npm install
npm run tauri dev
```

## Docs (outlines; filled as stages land)

- [FORMAT.md](docs/FORMAT.md) — binary container layout
- [CRYPTO.md](docs/CRYPTO.md) — Argon2id + AES-256-GCM
- [EMBEDDING.md](docs/EMBEDDING.md) — Method A (LSB) and Method B (EOF)
- [THREAT_MODEL.md](docs/THREAT_MODEL.md) — what this resists / does not
- [LEARNING.md](docs/LEARNING.md) — how to study the code

## License

MIT — see [LICENSE](LICENSE).
