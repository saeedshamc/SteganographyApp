# Building executables and installers

This guide covers **release** builds for:

1. **CLI** — `stego` / `stego.exe` (portable binary)
2. **Desktop GUI** — Tauri app **Open Stego**, including platform installers

Work from the repository root unless a command says otherwise:

```text
D:\Saeed\GitHub\SteganographyApp   (or your clone path)
```

---

## Prerequisites

### All platforms

| Tool | Check | Notes |
|------|--------|--------|
| Rust stable | `rustc --version` / `cargo --version` | Install via [rustup](https://rustup.rs/) |
| Node.js LTS | `node --version` / `npm --version` | Needed for the Tauri frontend |
| Git | `git --version` | Already required to clone |

First-time desktop deps:

```powershell
cd apps\desktop
npm install
cd ..\..
```

### Windows (GUI + installer)

1. **Visual Studio 2022** with workload **Desktop development with C++**  
   (MSVC linker, Windows SDK — required by Tauri / Rust on Windows)
2. **WebView2** — usually preinstalled on Windows 10/11. If the app fails to start, install the [Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
3. Optional for NSIS installers: Tauri’s bundler downloads/uses NSIS as needed when you build with the `nsis` target.

### Linux (GUI + packages)

Install a C toolchain and the usual Tauri/WebKit deps for your distro. Example (Debian/Ubuntu-style):

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
```

(Exact package names can vary by distro/version — follow [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/) if something is missing.)

---

## 1. CLI executable (`stego`)

### Build

```powershell
cargo build -p stego-cli --release
```

### Output location

| OS | Path |
|----|------|
| Windows | `target\release\stego.exe` |
| Linux / macOS | `target/release/stego` |

Copy that single file anywhere on `PATH`, or run it by full path. No installer is required for the CLI.

### Smoke test

```powershell
.\target\release\stego.exe --version
.\target\release\stego.exe --help
```

```bash
./target/release/stego --version
./target/release/stego --help
```

### Notes

- Release profile (workspace `Cargo.toml`) uses LTO, `opt-level = 3`, and strip for smaller/faster binaries.
- Passwords: `--password` or env `STEGO_PASSWORD` (see root `README.md`).

---

## 2. Desktop GUI — development vs release

### Dev (hot reload, not for distribution)

```powershell
cd apps\desktop
npm install
npm run tauri dev
```

### Release app + installers

From `apps/desktop`:

```powershell
cd apps\desktop
npm install
npm run tauri build
```

Equivalent:

```powershell
npx tauri build
```

What this does:

1. Runs `npm run build` (TypeScript + Vite → `apps/desktop/dist`)
2. Compiles the Rust side (`stego-desktop` / `stego-core`) in **release**
3. Bundles according to `apps/desktop/src-tauri/tauri.conf.json`  
   (`bundle.active: true`, `targets: "all"`)

### Windows — output artifacts

After a successful build, look under:

```text
apps\desktop\src-tauri\target\release\bundle\
```

Typical layout (names may include version `0.1.0`):

| Artifact | Role |
|----------|------|
| `nsis\Open Stego_*_x64-setup.exe` | **NSIS installer** — recommended for end users |
| `msi\Open Stego_*_x64_en-US.msi` | **MSI installer** (if produced for your toolchain) |
| `../Open Stego.exe` (also under `target\release\`) | Unpackaged GUI binary (portable-ish; still needs WebView2) |

Exact folder names depend on Tauri 2 / target triple. If unsure:

```powershell
Get-ChildItem -Recurse apps\desktop\src-tauri\target\release\bundle | Select-Object FullName
```

Also check the workspace target if your Cargo workspace redirects builds:

```powershell
Get-ChildItem -Recurse target\release\bundle -ErrorAction SilentlyContinue | Select-Object FullName
Get-ChildItem -Recurse apps\desktop\src-tauri\target\release -Filter "*.exe" | Select-Object FullName
```

### Linux — output artifacts

```text
apps/desktop/src-tauri/target/release/bundle/
```

Common packages when `targets` is `"all"`:

| Artifact | Role |
|----------|------|
| `deb/*.deb` | Debian/Ubuntu installer package |
| `rpm/*.rpm` | RPM-based distros (if tools available) |
| `appimage/*.AppImage` | Portable AppImage |

Install example (deb):

```bash
sudo dpkg -i apps/desktop/src-tauri/target/release/bundle/deb/*.deb
```

### Limit installer formats (optional)

To build only NSIS on Windows (faster, fewer tools):

```powershell
cd apps\desktop
npx tauri build --bundles nsis
```

Other useful values: `msi`, `deb`, `appimage`, `rpm`.

---

## 3. Version and product identity

Configured in `apps/desktop/src-tauri/tauri.conf.json`:

| Field | Current value |
|-------|----------------|
| `productName` | `Open Stego` |
| `version` | `0.1.0` |
| `identifier` | `com.shams.openstego` |

Bump `version` there (and ideally keep `apps/desktop/package.json` / workspace crate versions in sync) before cutting a release.

Icons used by the installer/app live in:

```text
apps/desktop/src-tauri/icons/
```

---

## 4. Recommended release checklist

1. `cargo test -p stego-core --lib`
2. `cargo build -p stego-cli --release` → archive `stego.exe` / `stego`
3. `cd apps/desktop && npm ci` (or `npm install`)
4. `npm run tauri build` (or `--bundles nsis` on Windows)
5. Manually smoke-test: Hide → Extract round-trip on the built app
6. Attach CLI binary + installer(s) to the GitHub Release
7. Note WebView2 requirement for Windows GUI in the release notes

---

## 5. Troubleshooting

| Symptom | What to try |
|---------|-------------|
| `link.exe` / MSVC not found | Install VS 2022 **Desktop development with C++** |
| Tauri build fails on `npm run build` | Run `cd apps/desktop && npm install && npm run build` alone; fix TypeScript errors |
| App starts then blank window (Windows) | Install/repair WebView2 Evergreen Runtime |
| Linux: missing `webkit2gtk` | Install distro WebKitGTK dev packages (see Tauri docs) |
| Cannot find `bundle\` folder | Search both workspace `target\` and `apps\desktop\src-tauri\target\` (Cargo may use either depending on config) |
| Slow builds | Normal for first release (LTO). Later builds are incremental |
| Geo-blocked crates / npm | Use a working network/VPN or configured mirrors before `cargo` / `npm` |

---

## 6. What not to commit

Do **not** commit build outputs:

- `target/`
- `apps/desktop/dist/`
- `apps/desktop/node_modules/`
- installer `.exe` / `.msi` / `.deb` / `.AppImage` from `bundle/`

Ship those via GitHub Releases (or similar), not the git tree.

---

## See also

- Root [README.md](../README.md) — CLI usage examples
- [LEARNING.md](LEARNING.md) — how the core works
- [Tauri 2 — Distribute](https://v2.tauri.app/distribute/) — upstream bundler details
