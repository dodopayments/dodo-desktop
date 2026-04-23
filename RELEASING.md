# Releasing Dodo Payments Desktop App

| Platform | Artifacts |
|----------|-----------|
| Windows | `.msi`, `.exe` |
| macOS | `.dmg`, `.app` |
| Linux | `.deb`, `.AppImage` |

All releases are distributed via **GitHub Releases**. The website links to the latest release for downloads. Auto-update is driven by the same GitHub Releases endpoint.

---

## Version Bumping

Update the version in all three files before tagging:

1. `package.json` → `"version"`
2. `src-tauri/tauri.conf.json` → `"version"`
3. `src-tauri/Cargo.toml` → `version`

```powershell
git add -A
git commit -m "chore: bump version to 1.0.0"
git tag v1.0.0
git push origin main --tags
```

---

## GitHub Releases

Pushing a `v*` tag triggers `build.yml`, which builds all platforms and creates a draft GitHub Release.

1. Push tag → CI builds all platforms
2. GitHub → Releases → edit the draft → add release notes → publish

**Artifacts:**
- Windows: `Dodo Payments_x.y.z_x64-setup.exe`, `Dodo Payments_x.y.z_x64_en-US.msi`
- macOS: `Dodo Payments_x.y.z_aarch64.dmg`, `Dodo Payments_x.y.z_x64.dmg`
- Linux: `dodo-payments_x.y.z_amd64.deb`, `dodo-payments_x.y.z_amd64.AppImage`
- **Updater bundles** (when signing key is configured): `Dodo Payments.app.tar.gz`, `...setup.exe`, `...AppImage` + matching `.sig` files, plus a `latest.json` manifest

> **Only publish releases as "latest"** once you've verified them. The installed app polls `https://github.com/dodopayments/dodo-desktop/releases/latest/download/latest.json` every 4 hours — marking a broken release as "latest" will auto-update all users to it. Use pre-release flags for anything not production-ready.

---

## Auto-Update

Installed apps check for updates every 4 hours (and on startup). When a new version is detected:
1. The updater downloads and stages the update silently in the background.
2. A native notification informs the user that the update will apply on next restart.
3. If the user doesn't restart within 24 hours, a native dialog prompts them to restart now.
4. Users can manually check via `Dodo Payments → Check for Updates…` (macOS menu bar).

**Scope**: GitHub Releases only. Users who install via Mac App Store or Microsoft Store receive updates through those stores' own mechanisms (Tauri's updater is not involved).

**Not auto-updatable**: `.deb` packages (direct users to `.AppImage` for auto-update on Linux).

### Updater Signing Keys (one-time setup)

Tauri's updater requires its own signing keypair, distinct from OS code-signing certs. **If this key is lost, all existing installs become permanently un-updateable.**

1. Generate a keypair locally:
   ```bash
   pnpm tauri signer generate -w ~/.tauri/dodo-payments.key
   ```
   You'll be prompted for a password — store it somewhere durable (e.g. 1Password).

2. Back up `~/.tauri/dodo-payments.key` + password to 1Password. **No other copies.**

3. Copy the public key into `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` (replacing the `REPLACE_WITH_CONTENTS_OF_dodo-payments.key.pub` placeholder):
   ```bash
   cat ~/.tauri/dodo-payments.key.pub
   ```

4. Add GitHub secrets:

   | Secret | Value |
   |--------|-------|
   | `TAURI_SIGNING_PRIVATE_KEY` | Full contents of `~/.tauri/dodo-payments.key` |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password from step 1 |

Once these are set, CI will auto-sign updater bundles and upload `latest.json` alongside release artifacts. Without them, CI still produces installers but skips updater artifacts (builds continue to succeed).

### Key Rotation

If the private key is compromised, full rotation is not possible without forcing affected users to reinstall manually. See `.sisyphus/plans/auto-update.md` → Appendix A for the transitional-version procedure.

---

## Code Signing

Signing steps are already in `build.yml` and activate automatically when the corresponding secrets are set. Without secrets, the build still succeeds — just unsigned.

### macOS

Requires an [Apple Developer Program](https://developer.apple.com/programs) membership ($99/year).

1. In [Certificates, IDs & Profiles](https://developer.apple.com/account/resources/certificates/list), create a **Developer ID Application** certificate.
2. Export it from Keychain Access → right-click → Export → save as `.p12`.
3. Encode and add to GitHub secrets:
   ```bash
   openssl base64 -A -in certificate.p12 -out certificate-base64.txt
   ```

**GitHub secrets required:**

| Secret | Value |
|--------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `KEYCHAIN_PASSWORD` | Any password (used for the temporary CI keychain) |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | [App-specific password](https://support.apple.com/en-us/102654) for notarization |
| `APPLE_TEAM_ID` | Your 10-character team ID |

### Windows

Requires an OV (Organization Validated) code signing certificate from a CA (DigiCert, Sectigo, etc.).

1. Convert your certificate to `.pfx` and encode it:
   ```powershell
   certutil -encode certificate.pfx base64cert.txt
   ```
2. Open `certmgr.msc` → Personal → Certificates → find your cert → Details → copy the **Thumbprint**.
3. Set `certificateThumbprint` in `src-tauri/tauri.conf.json` → `bundle.windows`.

**GitHub secrets required:**

| Secret | Value |
|--------|-------|
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx` |

> `TAURI_SIGNING_PRIVATE_KEY` is for the **updater plugin** only — not code signing.

---

## Release Checklist

- [ ] Bump version in `package.json`, `tauri.conf.json`, `Cargo.toml`
- [ ] Test locally: app loads, tray, menus, offline page
- [ ] `git tag v1.0.0 && git push origin main --tags`
- [ ] CI completes → edit the GitHub Release draft
- [ ] Verify `latest.json` is present in the draft's assets (indicates updater signing worked)
- [ ] Publish — this makes the release visible to `/releases/latest/`, triggering auto-update for all existing installs. **Do not publish if the release is broken.**

---

## Quick Reference

| Action | Command |
|--------|---------|
| Dev | `pnpm dev` |
| Build | `pnpm build` |
| Regenerate icons | `pnpm icons` |
| Tag a release | `git tag v1.0.0 && git push origin v1.0.0` |
