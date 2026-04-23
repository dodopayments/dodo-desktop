# Contributing to Dodo Payments Desktop App

Thank you for your interest in contributing. This document explains how to set up the project, the development workflow, and expectations for pull requests.

## Prerequisites

- [Rust](https://rustup.rs/) 1.77+
- [Node.js](https://nodejs.org/) 22+
- [pnpm](https://pnpm.io/)
- Platform build toolchains:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Windows**: Microsoft C++ Build Tools + WebView2 Runtime (ships with Windows 11)
  - **Linux**: see system dependencies in the [README](README.md#linux-dependencies)

## Getting Started

1. Fork the repository and clone your fork.
2. Install dependencies:
   ```bash
   pnpm install
   ```
3. Start the app in development mode:
   ```bash
   pnpm dev
   ```
   This compiles the Rust backend, launches the Tauri window pointing at `app.dodopayments.com`, and enables hot-reload for Rust changes.

## Project Structure

```
├── package.json                   # Tauri CLI + scripts
├── .github/workflows/build.yml    # CI: cross-platform builds + signing + notarization
├── RELEASING.md                   # Release process, signing keys, secrets
└── src-tauri/
    ├── Cargo.toml                 # Rust dependencies
    ├── tauri.conf.json            # Tauri config (bundle, windows, updater, plugins)
    ├── capabilities/default.json  # Webview permissions
    ├── icons/                     # Generated platform icons
    ├── app-icon.png               # Source icon (replace this)
    └── src/
        ├── main.rs                # Desktop entry point
        └── lib.rs                 # App logic, menus, system tray, deep links
```

## Development Guidelines

- Match existing Rust code style; run `cargo fmt` before committing.
- Keep changes small and focused; one concern per PR.
- Avoid introducing new dependencies unless there's a clear reason.
- Prefer modifying existing abstractions over adding new ones.
- Menus, tray, and deep-link handling live in `src-tauri/src/lib.rs` — read it before touching.

## Scripts

| Command | Description |
|---|---|
| `pnpm dev` | Launch the app in development mode with hot-reload |
| `pnpm build` | Produce a release bundle in `src-tauri/target/release/bundle/` |
| `pnpm icons` | Regenerate all platform icons from `src-tauri/app-icon.png` |

## Icons and Assets

To update the app icon, replace `src-tauri/app-icon.png` (1024x1024 PNG with transparency) and run `pnpm icons`. Commit all generated icons together.

## Submitting Changes

1. Create a feature branch from `main`.
2. Make your changes with small, focused commits.
3. Verify locally:
   - `pnpm build` completes successfully on at least one platform.
   - The app launches and basic functionality (tray, menus, navigation) works.
4. Open a pull request:
   - Provide a clear title and description of the change and motivation.
   - Include screenshots or a short clip for UI changes.
   - Note any changes to signing, updater, or release infrastructure — these require special review.

## Code Reviews

- Keep PRs small and self-contained when possible.
- Address review feedback with follow-up commits (avoid force-pushing unless necessary).
- If a discussion stalls, summarize options and propose a decision to move forward.

## Reporting Issues

When filing an issue, include:

- What you expected to happen vs. what happened
- Steps to reproduce
- Environment details (OS, app version, architecture, install method)
- Logs or stack traces if available

For security issues, follow [SECURITY.md](.github/SECURITY.md) — do not file them as public issues.

## Release Process

Releases are cut by maintainers from `main`. See [RELEASING.md](RELEASING.md) for the full process — version bumping, signing keys, GitHub secrets, and publishing.

## License

Unless otherwise stated, contributions to this repository are made under the same license as the repository ([GPL-3.0](LICENSE)).
