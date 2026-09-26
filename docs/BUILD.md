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

### Regenerating icons (desktop)

```powershell
cd apps\desktop
npm run icons
```

This refreshes `src-tauri/icons/`. For Flutter Android/iOS icons see [ANDROID.md](ANDROID.md).

(The committed `open-stego-icon-1024.png` is already prepared; re-run only if you replace the logo.)

---

## Version / identity

Workspace version lives in root `Cargo.toml` → `[workspace.package] version`.

| Surface | Where | Current line |
|---------|--------|----------------|
| Rust crates / CLI / Tauri | workspace + `tauri.conf.json` | `0.4.1` |
| Desktop npm | `apps/desktop/package.json` | `0.4.1` |
| Flutter | `apps/mobile/pubspec.yaml` | `0.4.1+41` |
| Android applicationId | `android/app/build.gradle.kts` | `com.shams.openstego` |

Bump all of the above together before a public release.

---

## Mobile (Flutter → Android)

Full guide: **[ANDROID.md](ANDROID.md)**.

```powershell
# NDK + cargo-ndk required (see ANDROID.md)
.\scripts\build-mobile-native.ps1 -Android
cd apps\mobile
flutter pub get
flutter run
flutter build apk --release
```

Launcher icons: from `assets/open-stego-icon-1024.png` via `dart run flutter_launcher_icons` in `apps/mobile`.

---

## Release checklist

1. `cargo test -p stego-core --lib`
2. Windows desktop: `.\scripts\build-windows.ps1`
3. Linux `.deb`: on Linux `./scripts/build-linux.sh --deb-only`, **or** on Windows `.\scripts\build-linux-deb-docker.ps1`
4. Android: `.\scripts\build-mobile-native.ps1 -Android` then `flutter build apk --release` (see [ANDROID.md](ANDROID.md))
5. Smoke-test Hide → Extract
6. Attach CLI + installers + optional APK to the GitHub Release

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `link.exe` / MSVC missing | Install VS 2022 **Desktop development with C++** |
| No `.deb` after `build:win` | Expected — build `.deb` on Linux or with Docker script |
| Docker script fails | Start Docker Desktop; ensure WSL2 backend works |
| Linux WebKit errors | Install `libwebkit2gtk-4.1-dev` (see above) |
| Blank window on Windows | Install/repair WebView2 Evergreen Runtime |
| Icons look old (desktop) | `cd apps/desktop && npm run icons`, then rebuild |
| Icons look old (Android) | `cd apps/mobile && dart run flutter_launcher_icons`, reinstall app |
| Android Hide fails / no native | Install NDK + `cargo-ndk`, run `-Android` (see [ANDROID.md](ANDROID.md)) |

---

## Do not commit

- `target/`, `apps/desktop/dist/`, `node_modules/`
- Built `.exe` / `.msi` / `.deb` / `.AppImage` under `bundle/`
- `apps/mobile/android/.../jniLibs/**/*.so` and `apps/mobile/native/*.dll`

Ship artifacts via GitHub Releases.

---

## See also

- [README.md](../README.md)
- [ANDROID.md](ANDROID.md) — Flutter Android only
- [Tauri 2 — Distribute](https://v2.tauri.app/distribute/)
