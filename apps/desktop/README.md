# Open Stego — desktop (Tauri)

GUI shell over `stego-core`.

## Develop

```powershell
npm install
npm run tauri dev
```

## Release executable / installer

See the project guide: **[docs/BUILD.md](../../docs/BUILD.md)**.

Short version:

```powershell
npm install
npm run tauri build
```

Installers land under `src-tauri/target/release/bundle/` (or the workspace `target/` tree — details in BUILD.md).
