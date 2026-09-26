# Open Stego — desktop (Tauri)

GUI shell over `stego-core`. Branding icons live in `src-tauri/icons/` (from `assets/open-stego-icon-1024.png`).

## Develop

```powershell
npm install
npm run tauri dev
```

## Release (easy)

From **repo root** (preferred):

| Goal | Command |
|------|---------|
| Windows CLI + NSIS/MSI | `..\..\scripts\build-windows.ps1` or from root `.\scripts\build-windows.ps1` |
| Linux `.deb` + AppImage | on Linux: `../../scripts/build-linux.sh` |
| Linux `.deb` from Windows | `.\scripts\build-linux-deb-docker.ps1` (Docker) |

From **this folder**:

```powershell
npm run build:win      # NSIS + MSI
npm run build:linux    # deb + AppImage (Linux host)
npm run build:deb      # deb only (Linux host)
npm run icons          # regenerate icons from 1024 master
```

Full guide: **[docs/BUILD.md](../../docs/BUILD.md)**.
