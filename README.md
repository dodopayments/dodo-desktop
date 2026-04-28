# Dodo Payments Desktop

<p align="left">
  <a href="https://github.com/dodopayments/dodo-desktop/releases/latest">
    <img src="https://img.shields.io/github/v/release/dodopayments/dodo-desktop?label=release&color=blue" alt="Latest release" />
  </a>
  <a href="https://github.com/dodopayments/dodo-desktop/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/dodopayments/dodo-desktop?color=green" alt="License" />
  </a>
  <a href="https://discord.gg/bYqAp4ayYh">
    <img src="https://img.shields.io/discord/1305511580854779984?label=Join%20Discord&logo=discord" alt="Join Discord" />
  </a>
</p>

The official [Dodo Payments](https://dodopayments.com) desktop app for macOS, Windows, and Linux. Run your payments dashboard as a native app — signed, auto-updating, and only ~5 MB.

## Table of Contents

- [Download](#download)
- [Features](#features)
- [Menu Bar](#menu-bar)
- [Prerequisites](#prerequisites)
- [Development](#development)
- [Build](#build)
- [App Icon](#app-icon)
- [Auto-Update](#auto-update)
- [Releases](#releases)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Security](#security)
- [License](#license)

## Download

Grab the latest installer for your platform from the [Releases page](https://github.com/dodopayments/dodo-desktop/releases/latest):

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `Dodo.Payments_<version>_aarch64.dmg` |
| macOS (Intel) | `Dodo.Payments_<version>_x64.dmg` |
| Windows | `Dodo.Payments_<version>_x64-setup.exe` or `.msi` |
| Linux (AppImage, auto-update) | `Dodo.Payments_<version>_amd64.AppImage` |
| Linux (Debian/Ubuntu) | `Dodo.Payments_<version>_amd64.deb` |
| Linux (Fedora/RHEL) | `Dodo.Payments-<version>-1.x86_64.rpm` |

macOS builds are signed with Apple's Developer ID and notarized — no Gatekeeper warning. Windows builds are currently unsigned; you'll see a SmartScreen prompt on first install (click **More info → Run anyway**).

## Features

- **Native webview** — no bundled Chromium, ~5 MB binary
- **System tray** with hide-to-tray on macOS
- **Full menu bar** (File, Edit, View, Help) with keyboard shortcuts
- **Deep link support** for magic-link authentication flows
- **Auto-update** via signed GitHub Releases (checks every 4 hours)
- **Cross-platform builds** signed + notarized in CI via GitHub Actions

### Menu Bar

| Menu | Items |
|:---|:---|
| **Dodo Payments** | About, Services, Hide, Quit, Check for Updates… |
| **File** | Go to Dashboard `⌘⇧H`, Reload `⌘R`, Hard Reload `⌘⇧R`, Close Window |
| **Edit** | Undo, Redo, Cut, Copy, Paste, Select All |
| **View** | Zoom In/Out/Reset, Full Screen, Toggle Dev Tools `⌘⌥I` |
| **Help** | Documentation, Support, Copy Current URL `⌘L` |

## Prerequisites

- [Rust](https://rustup.rs/) 1.77+
- [Node.js](https://nodejs.org/) 22+
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

## Auto-Update

Installed apps poll `https://github.com/dodopayments/dodo-desktop/releases/latest/download/latest.json` every 4 hours (and on startup). When a new version is detected:

1. The update is downloaded and staged silently in the background.
2. A native notification informs the user that the update will apply on next restart.
3. If the user doesn't restart within 24 hours, a dialog prompts them to restart now.
4. Users can manually check via `Dodo Payments → Check for Updates…`.

Update bundles are signed with a Tauri-specific minisign key (separate from OS code-signing). `.deb` packages are not auto-updatable — Linux users who want auto-update should install the `.AppImage` instead.

## Releases

Releases are cut by maintainers from `main`. The short version:

```bash
# bump version in package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml
git commit -am "chore: bump version to X.Y.Z"
git tag vX.Y.Z
git push origin main --tags
```

Pushing a `v*` tag triggers [`.github/workflows/build.yml`](.github/workflows/build.yml), which builds all platforms, signs + notarizes the macOS artifacts, and creates a draft GitHub Release. See [RELEASING.md](RELEASING.md) for the full process including signing key setup and rotation.

## Project Structure

```
├── package.json                   # Tauri CLI + scripts
├── .github/
│   ├── workflows/build.yml        # CI: cross-platform builds + signing + notarization
│   ├── ISSUE_TEMPLATE/            # Bug report + feature request templates
│   ├── SECURITY.md                # Vulnerability disclosure policy
│   └── pull_request_template.md
├── RELEASING.md                   # Release process, signing keys, secrets
├── CONTRIBUTING.md                # How to develop + submit changes
├── LICENSE                        # GPL-3.0
└── src-tauri/
    ├── Cargo.toml                 # Rust dependencies
    ├── tauri.conf.json            # Tauri configuration (bundle, updater, plugins)
    ├── capabilities/default.json  # Webview permissions
    ├── icons/                     # Generated platform icons
    ├── app-icon.png               # Source icon (replace this)
    └── src/
        ├── main.rs                # Desktop entry point
        └── lib.rs                 # App logic, menus, system tray, deep links
```

## Contributing

Pull requests are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow, coding guidelines, and submission process.

For bugs and feature requests, use the [issue templates](https://github.com/dodopayments/dodo-desktop/issues/new/choose).

## Security

Report vulnerabilities privately via [GitHub Security Advisories](https://github.com/dodopayments/dodo-desktop/security/advisories/new). See [SECURITY.md](.github/SECURITY.md) for the full policy, SLAs, and safe-harbor terms.

## License

[GPL-3.0](LICENSE) © Dodo Payments Inc.
