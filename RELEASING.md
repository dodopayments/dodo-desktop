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

Uses [**Azure Artifact Signing**](https://learn.microsoft.com/en-us/azure/trusted-signing/) (formerly Trusted Signing) — Microsoft's managed code signing service. No `.pfx` file, no physical HSM, no certificate renewal. Signing keys live in Azure; CI authenticates via OIDC (no long-lived secrets in GitHub).

#### One-time Azure setup

1. **Identity validation** — request via [Azure Portal → Trusted Signing → Identity validations](https://portal.azure.com). Takes ~3 business days. One validation is reusable across all accounts in the subscription.
2. **Register the resource provider**:
   ```bash
   az provider register --namespace Microsoft.CodeSigning
   ```
3. **Create a Trusted Signing Account** in East US (endpoint: `https://eus.codesigning.azure.net`).
4. **Create a Certificate Profile** of type `PublicTrust` on that account. Note the exact names of the account and profile.
5. **Plug the names into `src-tauri/tauri.conf.json`** → `bundle.windows.signCommand` (replace `<ARTIFACT_SIGNING_ACCOUNT_NAME>` and `<CERTIFICATE_PROFILE_NAME>`).

#### CI authentication (OIDC, no secrets)

1. Create a Microsoft Entra **App Registration** (Azure Portal → Microsoft Entra ID → App registrations → New).
2. On that app, add a **Federated Credential** scoped to this repo:
   - Issuer: `https://token.actions.githubusercontent.com`
   - Subject: `repo:dodopayments/dodo-desktop:ref:refs/heads/main` — and add another for `repo:dodopayments/dodo-desktop:ref:refs/tags/v*` so tag-triggered builds authenticate.
   - Audience: `api://AzureADTokenExchange`
3. **Assign RBAC** (critical — Owner/Contributor alone is *not* enough):
   ```bash
   az role assignment create \
     --assignee <app-client-id> \
     --role "Trusted Signing Certificate Profile Signer" \
     --scope "/subscriptions/<sub-id>/resourceGroups/<rg>/providers/Microsoft.CodeSigning/codeSigningAccounts/<account-name>"
   ```
   > Azure Portal may show this role as **"Artifact Signing Certificate Profile Signer"** — same role, renamed.

#### GitHub repository variables (not secrets — these are not sensitive)

Set under **Settings → Secrets and variables → Actions → Variables**:

| Variable | Value |
|--------|-------|
| `AZURE_CLIENT_ID` | App registration's Application (client) ID |
| `AZURE_TENANT_ID` | Microsoft Entra tenant ID |
| `AZURE_SUBSCRIPTION_ID` | Subscription ID holding the signing account |

CI auto-skips signing when `AZURE_CLIENT_ID` is unset — the Windows build still succeeds, just unsigned (useful for forks and PR builds).

#### Verifying a signed build

```powershell
signtool verify /v /pa ".\Dodo Payments_x.y.z_x64-setup.exe"
```

Should show the Microsoft ID Verified CS EOC CA chain and "Successfully verified".

> `TAURI_SIGNING_PRIVATE_KEY` is for the **updater plugin** only — not code signing.
>
> SmartScreen reputation is **not instant** with Artifact Signing (unlike old EV certs). The first few thousand downloads will still trigger SmartScreen warnings; reputation accrues automatically over time.

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
