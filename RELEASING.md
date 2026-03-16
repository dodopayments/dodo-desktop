# Releasing Dodo Payments Desktop App

| Platform | Artifacts | Channels |
|----------|-----------|----------|
| Windows | `.msi`, `.exe` | GitHub Releases, Microsoft Store |
| macOS | `.dmg`, `.app` | GitHub Releases, Mac App Store |
| Linux | `.deb`, `.AppImage` | GitHub Releases |

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

---

## Code Signing

Signing steps are already in `build.yml` and activate automatically when the corresponding secrets are set. Without secrets, the build still succeeds — just unsigned.

### macOS

Requires an [Apple Developer Program](https://developer.apple.com/programs) membership ($99/year).

1. In [Certificates, IDs & Profiles](https://developer.apple.com/account/resources/certificates/list), create a **Developer ID Application** certificate (for GitHub Releases) or **Apple Distribution** certificate (for Mac App Store).
2. Export it from Keychain Access → right-click → Export → save as `.p12`.
3. Encode and add to GitHub secrets:
   ```bash
   openssl base64 -A -in certificate.p12 -out certificate-base64.txt
   ```

4. In `src-tauri/Entitlements.plist`, replace both `TEAM_ID` placeholders with your 10-character Apple Developer Team ID.

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

## Microsoft Store

Tauri 2 does not produce `.msix`. The Store supports **EXE or MSI app** listings linked to a hosted installer URL.

### One-time setup

1. [Partner Center](https://partner.microsoft.com) → Apps and Games → New Product → **EXE or MSI app** → reserve "Dodo Payments". Cost: free (individual) / ~$99 one-time (company).
2. `publisher` in `tauri.conf.json` is already set to `"Dodo Payments Inc."` — the Store requires this to differ from `productName`.

### Store build

`src-tauri/tauri.microsoftstore.conf.json` is already created. Run:

```powershell
pnpm tauri build -- --no-bundle
pnpm tauri bundle -- --config src-tauri/tauri.microsoftstore.conf.json
```

This produces an installer with the offline WebView2 bootstrapper required by the Store.

### Submit

1. Upload the `.exe` or `.msi` to a public URL (GitHub Release asset works).
2. Partner Center → your app → Start a submission → Packages → link the URL.
3. Fill in listing, screenshots (1366×768 or 1920×1080), age rating, privacy policy URL.
4. Submit. Certification: 1–3 business days.

---

## Mac App Store

### One-time setup

1. In [Certificates, IDs & Profiles](https://developer.apple.com/account/resources/certificates/list) create:
   - **Apple Distribution** certificate
   - **Mac Installer Distribution** certificate
2. Create an **App ID** with Bundle ID `com.dodopayments.desktop`.
3. Create a **Mac App Store** provisioning profile linked to the App ID. Download the `.provisionprofile`.
4. Add the provisioning profile to `tauri.conf.json`:
   ```json
   {
     "bundle": {
       "macOS": {
         "files": {
           "embedded.provisionprofile": "/path/to/your.provisionprofile"
         }
       }
     }
   }
   ```
5. Register the app in [App Store Connect](https://appstoreconnect.apple.com) using Bundle ID `com.dodopayments.desktop`.
6. Create an **App Store Connect API key** (Users and Access → Integrations → Individual Keys). Save the `.p8` file at `~/.appstoreconnect/private_keys/AuthKey_KEY_ID.p8`.

### Build and upload

```bash
pnpm tauri build --target universal-apple-darwin --bundles app

xcrun productbuild \
  --sign "3rd Party Mac Developer Installer: Dodo Payments Inc (TEAM_ID)" \
  --component "src-tauri/target/universal-apple-darwin/release/bundle/macos/Dodo Payments.app" \
  /Applications \
  "Dodo Payments.pkg"

xcrun altool --upload-app --type macos \
  --file "Dodo Payments.pkg" \
  --apiKey YOUR_KEY_ID \
  --apiIssuer YOUR_ISSUER_ID
```

### Submit

1. App Store Connect → your app → new version → select the uploaded build (~15 min to appear).
2. Fill in "What's New", screenshots (1280×800 or 2560×1600), review info.
3. Submit. App Review: 1–2 business days.

---

## Release Checklist

**Every release:**
- [ ] Bump version in `package.json`, `tauri.conf.json`, `Cargo.toml`
- [ ] Test locally: app loads, tray, menus, offline page
- [ ] `git tag v1.0.0 && git push origin main --tags`
- [ ] CI completes → edit and publish the GitHub Release draft

**Microsoft Store:**
- [ ] Build with `tauri.microsoftstore.conf.json` → upload installer to a public URL
- [ ] Partner Center → new submission → link installer URL → submit

**Mac App Store:**
- [ ] Build universal binary → sign → create `.pkg` → upload via `altool`
- [ ] App Store Connect → new version → select build → submit

---

## Quick Reference

| Action | Command |
|--------|---------|
| Dev | `pnpm dev` |
| Build | `pnpm build` |
| Microsoft Store build | `pnpm tauri bundle -- --config src-tauri/tauri.microsoftstore.conf.json` |
| macOS universal build | `pnpm tauri build --target universal-apple-darwin --bundles app` |
| Regenerate icons | `pnpm icons` |
| Tag a release | `git tag v1.0.0 && git push origin v1.0.0` |
