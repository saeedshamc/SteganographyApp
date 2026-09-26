# Building executables and installers

Release builds for:

1. **CLI** — portable `stego` / `stego.exe`
2. **Desktop GUI** — **Open Stego** with platform installers

| Host OS | Typical outputs |
|---------|-----------------|
| **Windows** | `stego.exe`, NSIS setup (`.exe`), MSI (`.msi`) |
| **Linux** | `stego`, **`.deb`**, AppImage |
| **Windows + Docker** | Linux **`.deb`** (without a Linux machine) |

App icons are generated from [`assets/open-stego-icon-1024.png`](../assets/open-stego-icon-1024.png) into `apps/desktop/src-tauri/icons/`.

---

## Prerequisites

### All platforms

| Tool | Check |
|------|--------|
| Rust stable | `rustc --version` / `cargo --version` |
| Node.js LTS | `node --version` / `npm --version` |

```powershell
cd apps\desktop
npm install
cd ..\..
```

### Windows (native GUI / NSIS / MSI)

1. Visual Studio 2022 — workload **Desktop development with C++**
2. WebView2 (usually already on Win10/11)

### Linux (native GUI / deb / AppImage)

Debian/Ubuntu example:

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
```

### Docker (optional — Linux `.deb` from Windows)

- Docker Desktop (Windows) or Docker Engine (Linux)
- Used by `scripts/build-linux-deb-docker.ps1`

---

## One-command scripts (recommended)

### Windows host → Windows CLI + installers

```powershell
# From repo root
.\scripts\build-windows.ps1
```

Options:

```powershell
.\scripts\build-windows.ps1 -CliOnly
.\scripts\build-windows.ps1 -GuiOnly
```

### Linux host → Linux CLI + `.deb` + AppImage

```bash
chmod +x scripts/build-linux.sh
./scripts/build-linux.sh
```

Options:

```bash
./scripts/build-linux.sh --cli-only
./scripts/build-linux.sh --gui-only
./scripts/build-linux.sh --deb-only    # only .deb
```

### Windows host → Linux `.deb` via Docker

```powershell
.\scripts\build-linux-deb-docker.ps1
```

This builds image `openstego-linux-build` from `scripts/Dockerfile.linux-build`, then runs the Linux deb build with your repo mounted.

---

## npm scripts (GUI only)

From `apps/desktop`:

| Script | What it builds |
|--------|----------------|
| `npm run build:win` | NSIS + MSI |
| `npm run build:nsis` | NSIS only |
| `npm run build:linux` | `.deb` + AppImage |
| `npm run build:deb` | **`.deb` only** |
| `npm run build:all` | All configured targets for the current OS |
| `npm run icons` | Regenerate `src-tauri/icons` from the 1024 master PNG |

Configured targets in `tauri.conf.json`:

```json
"targets": ["nsis", "msi", "deb", "appimage"]
```

Tauri only emits packages valid for the **current** OS (e.g. `.deb` is produced on Linux or in the Linux Docker image — not by a bare Windows MSVC toolchain).

---

## CLI only (any OS)

```powershell
cargo build -p stego-cli --release
```

| OS | Binary |
|----|--------|
| Windows | `target\release\stego.exe` |
| Linux | `target/release/stego` |

---

## Where installers land

Search both trees (Cargo may use the workspace `target/` or the crate-local one):

```text
apps/desktop/src-tauri/target/release/bundle/
target/release/bundle/
```

### Windows

```text
bundle/nsis/Open Stego_*_x64-setup.exe
bundle/msi/Open Stego_*_x64_*.msi
```

### Linux

```text
bundle/deb/*.deb
bundle/appimage/*.AppImage
```

Install `.deb`:

```bash
sudo dpkg -i path/to/Open_Stego_*.deb
# if deps missing:
sudo apt-get install -f
```

PowerShell helper to list artifacts after a build:

```powershell
Get-ChildItem -Recurse apps\desktop\src-tauri\target\release\bundle, target\release\bundle `
  -Include *.exe,*.msi,*.deb,*.AppImage -ErrorAction SilentlyContinue
```

---

## Regenerating app icons from the logo

Master art:

- UI / brand: `assets/open-stego-logo.png`
- Icon pipeline (1024×1024, transparent): `assets/open-stego-icon-1024.png`

```powershell
cd apps\desktop
npm run icons
```

This refreshes `src-tauri/icons/` (PNG, ICO, ICNS, Store logos, etc.). Commit the updated icons when branding changes.

To rebuild the 1024 master from the logo (Python + Pillow):

```powershell
python -c "from PIL import Image; print('use repo script or existing open-stego-icon-1024.png')"
```

(The committed `open-stego-icon-1024.png` is already prepared; re-run the crop/scale step only if you replace the logo.)

---

## Version / identity

In `apps/desktop/src-tauri/tauri.conf.json`:

| Field | Value |
|-------|--------|
| `productName` | Open Stego |
| `version` | 0.1.0 |
| `identifier` | com.shams.openstego |

Bump `version` before a public release (keep `apps/desktop/package.json` in sync if you care about npm metadata).

---

## Release checklist

1. `cargo test -p stego-core --lib`
2. Windows: `.\scripts\build-windows.ps1`
3. Linux `.deb`: on Linux `./scripts/build-linux.sh --deb-only`, **or** on Windows `.\scripts\build-linux-deb-docker.ps1`
4. Smoke-test Hide → Extract on the installed / portable app
5. Attach CLI binary + installer(s) to the GitHub Release
6. Mention WebView2 for Windows GUI in release notes

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `link.exe` / MSVC missing | Install VS 2022 **Desktop development with C++** |
| No `.deb` after `build:win` | Expected — build `.deb` on Linux or with Docker script |
| Docker script fails | Start Docker Desktop; ensure WSL2 backend works |
| Linux WebKit errors | Install `libwebkit2gtk-4.1-dev` (see above) |
| Blank window on Windows | Install/repair WebView2 Evergreen Runtime |
| Icons look old | `cd apps/desktop && npm run icons`, then rebuild |

---

## Do not commit

- `target/`, `apps/desktop/dist/`, `node_modules/`
- Built `.exe` / `.msi` / `.deb` / `.AppImage` under `bundle/`

Ship artifacts via GitHub Releases.

---

## See also

- [README.md](../README.md)
- [Tauri 2 — Distribute](https://v2.tauri.app/distribute/)
