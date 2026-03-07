# Dodo Payments Desktop App

Native desktop application for [Dodo Payments](https://dodopayments.com) — wraps [app.dodopayments.com](https://app.dodopayments.com) using [Tauri 2.0](https://v2.tauri.app/).

Builds for **macOS**, **Windows**, and **Linux** with a ~5MB binary size.

## Features

- Native webview rendering (no bundled Chromium)
- System tray with hide-to-tray on macOS
- Full menu bar (File, Edit, View, Help)
- Keyboard shortcuts for reload, zoom, dev tools
- Cross-platform builds via GitHub Actions

### Menu Bar

| Menu | Items |
|:---|:---|
| **Dodo Payments** | About, Services, Hide, Quit |
| **File** | Go to Dashboard `⌘⇧H`, Reload `⌘R`, Hard Reload `⌘⇧R`, Close Window |
| **Edit** | Undo, Redo, Cut, Copy, Paste, Select All |
| **View** | Zoom In/Out/Reset, Full Screen, Toggle Dev Tools `⌘⌥I` |
| **Help** | Documentation, Support, Copy Current URL `⌘L` |

## Prerequisites

- [Rust](https://rustup.rs/) (1.77+)
- [Node.js](https://nodejs.org/) (22+)
- [pnpm](https://pnpm.io/)
- **Linux only:** system dependencies (see below)

### Linux Dependencies

```bash
sudo apt-get install -y \
  libdbus-1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
  libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## Development

```bash
pnpm install
pnpm dev
```

## Build

```bash
pnpm build
```

Build output is in `src-tauri/target/release/bundle/`.

## App Icon

Replace `src-tauri/app-icon.png` with your logo (1024x1024 PNG with transparency), then regenerate all platform icons:

```bash
pnpm icons
```

## Releases

Push a version tag to trigger the GitHub Actions build workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This creates a draft release with binaries for:
- macOS (Apple Silicon + Intel)
- Windows (.msi + .exe)
- Linux (.deb + .AppImage)

## Project Structure

```
├── package.json
├── .github/workflows/build.yml    # CI/CD for cross-platform builds
└── src-tauri/
    ├── Cargo.toml                 # Rust dependencies
    ├── tauri.conf.json            # Tauri configuration
    ├── capabilities/default.json  # Webview permissions
    ├── icons/                     # Generated platform icons
    ├── app-icon.png               # Source icon (replace this)
    └── src/
        ├── main.rs                # Desktop entry point
        └── lib.rs                 # App logic, menus, system tray
```

## License

MIT
