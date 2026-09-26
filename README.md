# Open Stego

Open-source steganography toolkit: hide **encrypted** payloads in almost any cover file, extract them later, and learn exactly how the bits move.

**Desktop (ready):** Rust core + `stego` CLI + Tauri GUI (Windows / Linux).  
**Mobile (next):** Flutter in [`apps/mobile`](apps/mobile) — same on-disk format after desktop.

## What it does

1. Pick any cover (image, PDF, audio, archive, …)
2. Pick a file or paste text/code as the payload
3. Enter a password (Argon2id → AES-256-GCM)
4. Save an output that still opens as the original format (when the format allows trailing data / lossless pixels)
5. Later: open the output, enter the password, recover the exact payload (name + checksum)
6. **Demo:** hide a small script from `test-fixtures/demo/`, open the cover in a normal viewer (still looks normal), then Extract in Open Stego and optionally **Run** with an explicit confirm — see the GUI **Demo** tab and [LEARNING.md](docs/LEARNING.md)

Executable payloads are tagged in metadata (`flags bit1`). CLI optional run:

```powershell
cargo run -p stego-cli -- extract --input .\demo_stego.png -o .\recovered.bat --run --i-understand
```

There is **no** silent auto-run when you double-click a cover in the OS; that keeps the project educational and transparent.

## Embedding methods

| Cover | Method | Notes |
|-------|--------|--------|
| PNG, BMP | **A — keyed random LSB** | Output always PNG. Capacity shown in UI/CLI. |
| JPEG | Rejected | Lossy compression destroys LSB; convert to PNG first. |
| Everything else | **B — keyed EOF append** | Trailing encrypted blob + HMAC locator (no fixed plaintext magic). |

See [docs/EMBEDDING.md](docs/EMBEDDING.md) for trade-offs and the Method B caveat.

## Quick start

### CLI

```powershell
cargo build -p stego-cli --release
$env:STEGO_PASSWORD = "your-passphrase"

cargo run -p stego-cli -- plan --cover .\photo.png
cargo run -p stego-cli -- hide --cover .\doc.pdf --payload .\secret.zip -o .\out.pdf --verify
cargo run -p stego-cli -- extract --input .\out.pdf -o .\recovered.zip
```

Hide text:

```powershell
cargo run -p stego-cli -- hide --cover .\a.pdf --text "notes" -o .\a_stego.pdf --verify
cargo run -p stego-cli -- extract --input .\a_stego.pdf --stdout
```

### GUI

```powershell
cd apps/desktop
npm install
npm run tauri dev
```

## Layout

| Path | Role |
|------|------|
| `crates/stego-core` | Crypto, metadata, Method A/B, detection, hide/extract ops |
| `crates/stego-cli` | `stego` CLI |
| `apps/desktop` | Tauri 2 GUI |
| `apps/mobile` | Flutter placeholder (post-desktop) |
| `docs/` | Open format, crypto, threat model, learning path, **build/installers** |

## Documentation

- [BUILD.md](docs/BUILD.md) — **release executables and installers** (CLI + Tauri GUI)
- [FORMAT.md](docs/FORMAT.md) — binary layout
- [CRYPTO.md](docs/CRYPTO.md) — Argon2id + AES-256-GCM + key schedule
- [EMBEDDING.md](docs/EMBEDDING.md) — Method A/B
- [THREAT_MODEL.md](docs/THREAT_MODEL.md) — what this resists / does not claim
- [LEARNING.md](docs/LEARNING.md) — how to study the code (+ executable demo)
- [PHASE2_PLAN.md](docs/PHASE2_PLAN.md) — full upgrade roadmap
- [PHASE2_START.md](docs/PHASE2_START.md) — current sprint checklist

## Release builds

See **[docs/BUILD.md](docs/BUILD.md)** for full details. Short paths:

```powershell
# Windows: CLI + NSIS/MSI
.\scripts\build-windows.ps1

# Linux .deb from Windows (Docker Desktop required)
.\scripts\build-linux-deb-docker.ps1
```

```bash
# Linux host: CLI + .deb + AppImage
chmod +x scripts/build-linux.sh
./scripts/build-linux.sh
./scripts/build-linux.sh --deb-only
```

## Branding

- Logo: [`assets/open-stego-logo.png`](assets/open-stego-logo.png)
- Icon master (1024): [`assets/open-stego-icon-1024.png`](assets/open-stego-icon-1024.png)
- Generated installer/app icons: `apps/desktop/src-tauri/icons/` (`npm run icons` in `apps/desktop`)

## Ethics

Built for privacy, education, and research. The design is intentionally transparent so others can audit and learn. It is not a guide for malware delivery or abuse.

## License

MIT — see [LICENSE](LICENSE).
