//! Auto-update machinery.
//!
//! Strategy:
//!   * On startup (after STARTUP_DELAY_SECS), and every CHECK_INTERVAL_SECS
//!     thereafter, silently check the updater endpoint.
//!   * If an update is found, silently download + stage. Show a native
//!     notification so the user knows a restart will apply it.
//!   * If the staged update is still unapplied after NAG_INTERVAL_SECS,
//!     show a native dialog asking to restart now.
//!   * Manual `Dodo Payments > Check for Updates…` menu item calls
//!     `check_manual` for on-demand checks.
//!
//! Since the app loads a remote web UI (`app.dodopayments.com`), we cannot
//! render update UI inside the webview. All prompts are native.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;

/// How long to wait after window open before the first check.
const STARTUP_DELAY_SECS: u64 = 10;

/// Interval between periodic background checks (4 hours).
const CHECK_INTERVAL_SECS: u64 = 4 * 60 * 60;

/// After an update is staged, nag the user this often until they restart (24 hours).
const NAG_INTERVAL_SECS: u64 = 24 * 60 * 60;

#[derive(Default)]
pub struct UpdateState {
    /// When the current pending update was downloaded and staged. `None` when
    /// there is no pending update.
    staged_at: Option<Instant>,
    /// When we last prompted the user to restart. Used to rate-limit the nag.
    last_nag: Option<Instant>,
    /// Version string of the staged update (for display).
    pending_version: Option<String>,
}

pub type SharedUpdateState = Arc<Mutex<UpdateState>>;

/// Spawn the background update loop. Call once from `setup`.
pub fn spawn_update_loop<R: Runtime>(app: AppHandle<R>, state: SharedUpdateState) {
    tauri::async_runtime::spawn(async move {
        // Don't block startup — wait for the UI to settle first.
        tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SECS)).await;

        loop {
            if let Err(e) = check_and_stage_silent(&app, &state).await {
                eprintln!("[updater] background check failed: {e}");
            }

            nag_if_due(&app, &state).await;

            tokio::time::sleep(Duration::from_secs(CHECK_INTERVAL_SECS)).await;
        }
    });
}

/// Silent: check → download → stage → notify. No dialogs.
async fn check_and_stage_silent<R: Runtime>(
    app: &AppHandle<R>,
    state: &SharedUpdateState,
) -> tauri_plugin_updater::Result<()> {
    // If an update is already staged, don't re-download on every tick.
    if state.lock().await.staged_at.is_some() {
        return Ok(());
    }

    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };

    let version = update.version.clone();
    println!("[updater] found v{version}, downloading in background...");

    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await?;

    println!("[updater] v{version} staged; will apply on next launch");

    {
        let mut s = state.lock().await;
        s.staged_at = Some(Instant::now());
        s.pending_version = Some(version.clone());
    }

    let _ = app
        .notification()
        .builder()
        .title("Dodo Payments updated")
        .body(format!(
            "Version {version} will install the next time you restart."
        ))
        .show();

    Ok(())
}

/// If an update is staged and 24h has passed since the last reminder, prompt.
async fn nag_if_due<R: Runtime>(app: &AppHandle<R>, state: &SharedUpdateState) {
    let (version, should_nag) = {
        let mut s = state.lock().await;
        let Some(staged_at) = s.staged_at else {
            return;
        };
        let reference = s.last_nag.unwrap_or(staged_at);
        if reference.elapsed() < Duration::from_secs(NAG_INTERVAL_SECS) {
            return;
        }
        s.last_nag = Some(Instant::now());
        (s.pending_version.clone().unwrap_or_else(|| "new".into()), true)
    };

    if !should_nag {
        return;
    }

    let app_for_restart = app.clone();
    app.dialog()
        .message(format!(
            "Dodo Payments v{version} has been downloaded. Restart now to apply?"
        ))
        .title("Update ready to install")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Restart Now".into(),
            "Later".into(),
        ))
        .show(move |restart_now| {
            if restart_now {
                app_for_restart.restart();
            }
        });
}

/// Manual menu-triggered check. Always shows a dialog.
pub async fn check_manual<R: Runtime>(app: AppHandle<R>, state: SharedUpdateState) {
    let result = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(e) => {
            show_error(&app, format!("Updater unavailable: {e}"));
            return;
        }
    };

    let maybe_update = match result {
        Ok(u) => u,
        Err(e) => {
            show_error(&app, format!("Failed to check for updates: {e}"));
            return;
        }
    };

    let Some(update) = maybe_update else {
        app.dialog()
            .message("You're on the latest version.")
            .title("No updates available")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::Ok)
            .show(|_| {});
        return;
    };

    let version = update.version.clone();
    let app_for_install = app.clone();

    app.dialog()
        .message(format!(
            "Version {version} is available. Install now and restart?"
        ))
        .title("Update available")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install".into(),
            "Later".into(),
        ))
        .show(move |install| {
            if !install {
                return;
            }
            let state = state.clone();
            let app = app_for_install.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = update
                    .download_and_install(|_chunk, _total| {}, || {})
                    .await
                {
                    show_error(&app, format!("Failed to install update: {e}"));
                    return;
                }
                {
                    let mut s = state.lock().await;
                    s.staged_at = Some(Instant::now());
                    s.pending_version = Some(version);
                }
                app.restart();
            });
        });
}

fn show_error<R: Runtime>(app: &AppHandle<R>, message: String) {
    app.dialog()
        .message(message)
        .title("Update error")
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}
