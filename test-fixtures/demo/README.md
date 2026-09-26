# Demo payload fixtures

Small, harmless scripts for the educational **executable** hide → extract → optional Run path.

| File | Use |
|------|-----|
| `demo-hello.bat` | Windows demo script |
| `demo-hello.sh` | Unix demo script |

Example (CLI):

```powershell
$env:STEGO_PASSWORD = "demo-pass"
cargo run -p stego-cli -- hide --cover .\photo.png --payload .\test-fixtures\demo\demo-hello.bat -o .\demo_stego.png --verify
cargo run -p stego-cli -- extract --input .\demo_stego.png -o .\recovered.bat
# Optional run (transparent demo only):
cargo run -p stego-cli -- extract --input .\demo_stego.png -o .\recovered.bat --run --i-understand
```

Opening `demo_stego.png` in a normal image viewer never runs the script — that is intentional and documented in `docs/LEARNING.md`.
