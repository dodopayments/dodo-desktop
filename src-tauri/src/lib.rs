mod updater;

use std::{
    net::{TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use tauri::{
    menu::{AboutMetadataBuilder, MenuBuilder, MenuItem, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::WebviewBuilder,
    window::WindowBuilder,
    ActivationPolicy, AppHandle, Manager, Webview, WebviewUrl,
};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::Mutex as TokioMutex;

use crate::updater::{check_manual, spawn_update_loop, SharedUpdateState, UpdateState};

type SharedConnectivityState = Arc<AtomicBool>;
type SharedLastRemoteUrl = Arc<Mutex<Option<String>>>;

const DOCS_URL: &str = "https://docs.dodopayments.com";
const SUPPORT_URL: &str = "https://dodopayments.com/support";
const HOME_URL: &str = "https://app.dodopayments.com";
const STATUS_URL: &str = "https://status.dodopayments.com";
const AUTH_CALLBACK_URL: &str = "https://app.dodopayments.com/login/magic-link";
const APP_HOST_PORT: &str = "app.dodopayments.com:443";
// Local pages served via the `dodo://` custom URI scheme.
// Using a real URL (instead of about:blank + eval) avoids a macOS WKWebView race
// where eval calls queued before the first navigation commit can be lost,
// leaving the content webview stuck on a blank screen at startup.
//
// We use the canonical `<scheme>://localhost/<path>` form documented by Tauri:
// the URL the handler sees has a stable `/<path>` shape on every platform
// (macOS/Linux keep `dodo://localhost/<path>`; Windows rewrites to
// `http://dodo.localhost/<path>` under the hood).
const OFFLINE_PAGE_URL: &str = "dodo://localhost/offline";
const TOOLBAR_PAGE_URL: &str = "dodo://localhost/toolbar";
const CONNECT_TIMEOUT_SECS: u64 = 3;
const CONNECTIVITY_CHECK_INTERVAL_SECS: u64 = 10;

// macOS: toolbar sits in the native titlebar row (28px), offset past traffic lights.
// Windows/Linux: toolbar replaces the native titlebar entirely (36px), starts at x=0.
#[cfg(target_os = "macos")]
const TOOLBAR_HEIGHT: f64 = 28.0;
#[cfg(not(target_os = "macos"))]
const TOOLBAR_HEIGHT: f64 = 36.0;

// On macOS the traffic lights occupy ~76 logical px on the left.
// We start our toolbar webview to the right of them.
#[cfg(target_os = "macos")]
const TOOLBAR_OFFSET_X: f64 = 76.0;
#[cfg(not(target_os = "macos"))]
const TOOLBAR_OFFSET_X: f64 = 0.0;

// Served via the "dodo" custom URI scheme at dodo://toolbar.
// Platform is detected via navigator.platform so one file handles both.
const TOOLBAR_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  :root {
    color-scheme: light dark;
    --bg:           #ececec;
    --border:       rgba(0,0,0,0.10);
    --btn-color:    #444;
    --btn-hover:    rgba(0,0,0,0.08);
    --btn-active:   rgba(0,0,0,0.15);
    --close-hover:  #c42b1c;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg:          #2c2c2c;
      --border:      rgba(255,255,255,0.08);
      --btn-color:   #ccc;
      --btn-hover:   rgba(255,255,255,0.10);
      --btn-active:  rgba(255,255,255,0.18);
      --close-hover: #c42b1c;
    }
  }
  html, body {
    width: 100%; height: 100%; overflow: hidden;
    -webkit-user-select: none; user-select: none;
  }
  body {
    display: flex; align-items: center;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  /* macOS: transparent so the native titlebar shows through */
  body.mac { background: transparent; border-bottom: none; }

  button {
    all: unset;
    display: inline-flex; align-items: center; justify-content: center;
    cursor: pointer; flex-shrink: 0;
  }
  svg { pointer-events: none; }

  /* ── Nav buttons (back / forward) ─────────────────────── */
  .nav { display: flex; align-items: center; gap: 2px; padding: 0 6px; height: 100%; }
  .nav-btn {
    width: 28px; height: 24px; border-radius: 5px;
    color: var(--btn-color);
  }
  .nav-btn:hover  { background: var(--btn-hover); }
  .nav-btn:active { background: var(--btn-active); }

  /* ── Drag region ───────────────────────────────────────── */
  .drag { flex: 1; height: 100%; }

  /* ── Windows / Linux close-min-max ────────────────────── */
  .winctrl { display: none; height: 100%; }
  body.win .winctrl { display: flex; }
  .wbtn {
    width: 46px; height: 100%;
    color: var(--btn-color); font-size: 10px;
    cursor: default;
  }
  .wbtn:hover  { background: var(--btn-hover); }
  .wbtn:active { background: var(--btn-active); }
  .wbtn.close:hover  { background: var(--close-hover); color: #fff; }
  .wbtn.close:active { background: #a52016; color: #fff; }
</style>
</head>
<body>
  <div class="nav">
    <button class="nav-btn" title="Back" onclick="nav('go_back')">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2.5"
           stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6"/>
      </svg>
    </button>
    <button class="nav-btn" title="Forward" onclick="nav('go_forward')">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2.5"
           stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 18 15 12 9 6"/>
      </svg>
    </button>
  </div>

  <div class="drag" data-tauri-drag-region></div>

  <!-- min / max / close — shown only on Windows & Linux -->
  <div class="winctrl">
    <button class="wbtn" title="Minimize" onclick="wm('minimize')">
      <svg width="10" height="1" viewBox="0 0 10 1" fill="currentColor">
        <rect width="10" height="1"/>
      </svg>
    </button>
    <button class="wbtn" title="Maximize" onclick="wm('toggle_maximize')">
      <svg width="10" height="10" viewBox="0 0 10 10"
           fill="none" stroke="currentColor" stroke-width="1">
        <rect x="0.5" y="0.5" width="9" height="9"/>
      </svg>
    </button>
    <button class="wbtn close" title="Close" onclick="wm('close')">
      <svg width="10" height="10" viewBox="0 0 10 10"
           stroke="currentColor" stroke-width="1.2"
           stroke-linecap="round">
        <line x1="0" y1="0" x2="10" y2="10"/>
        <line x1="10" y1="0" x2="0" y2="10"/>
      </svg>
    </button>
  </div>

  <script>
    const isMac = /Mac/i.test(navigator.platform);
    document.body.classList.add(isMac ? 'mac' : 'win');

    function nav(cmd) {
      window.__TAURI_INTERNALS__.invoke(cmd);
    }

    function wm(action) {
      window.__TAURI_INTERNALS__.invoke('plugin:window|' + action, { label: 'main' });
    }
  </script>
</body>
</html>"##;

fn can_reach_app_host() -> bool {
    let timeout = Duration::from_secs(CONNECT_TIMEOUT_SECS);
    let Ok(addrs) = APP_HOST_PORT.to_socket_addrs() else {
        return false;
    };
    addrs
        .into_iter()
        .any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok())
}

// Served via the "dodo" custom URI scheme at dodo://offline.
const OFFLINE_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Dodo Payments – Unable to Connect</title>
    <style>
      *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
      :root {
        color-scheme: light dark;
        --bg: #050710; --fg: #f9fafb; --muted: #9ca3af; --accent: #bef264;
      }
      @media (prefers-color-scheme: light) {
        :root { --bg: #f3f4f6; --fg: #030712; --muted: #6b7280; --accent: #a3e635; }
      }
      body {
        min-height: 100vh; display: flex; align-items: center;
        justify-content: center; padding: 32px 24px;
        font-family: "Inter", "Segoe UI", system-ui, -apple-system, sans-serif;
        background: var(--bg); color: var(--fg);
        -webkit-font-smoothing: antialiased;
      }
      .wrap { max-width: 460px; width: 100%; text-align: left; }
      h1 { font-size: 1.8rem; font-weight: 700; line-height: 1.2; margin-bottom: 10px; }
      .desc { font-size: 0.95rem; line-height: 1.55; color: var(--muted); margin-bottom: 10px; }
      .btns { display: inline-flex; gap: 10px; flex-wrap: wrap; margin: 24px 0 20px; }
      button {
        padding: 9px 20px; border-radius: 8px; font-size: 0.9rem;
        font-weight: 600; cursor: pointer; border: none; font-family: inherit; line-height: 1;
      }
      .btn-retry { background: var(--accent); color: #020308; }
      .btn-retry:hover { background: #a3e635; }
      .btn-status {
        background: transparent; color: var(--fg);
        border: 1px solid rgba(148,163,184,0.5);
      }
      .btn-status:hover { background: rgba(148,163,184,0.08); }
      .note { font-size: 0.83rem; color: var(--muted); }
      @media (max-width: 480px) { button { width: 100%; } }
    </style>
  </head>
  <body>
    <main class="wrap" role="alert" aria-live="polite">
      <h1>Unable to connect</h1>
      <p class="desc">We couldn't reach your dashboard. Please check your connection and try again.</p>
      <div class="btns">
        <button class="btn-retry" onclick="window.__TAURI_INTERNALS__.invoke('retry_connection')">Retry</button>
        <button class="btn-status" onclick="window.__TAURI_INTERNALS__.invoke('open_status_page')">Service Status</button>
      </div>
      <p class="note">The app reconnects automatically when connectivity is available.</p>
    </main>
  </body>
</html>"#;

fn navigate_to<R: tauri::Runtime>(webview: &Webview<R>, url: &str) {
    match url.parse() {
        Ok(parsed) => {
            if let Err(e) = webview.navigate(parsed) {
                eprintln!("[connectivity] navigate to {url} failed: {e}");
            }
        }
        Err(e) => eprintln!("[connectivity] failed to parse url {url}: {e}"),
    }
}

fn show_no_internet_popup<R: tauri::Runtime>(app: &AppHandle<R>) {
    app.dialog()
        .message("No internet connection detected. Check your connection and retry to load Dodo Payments.")
        .title("No internet connection")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::Ok)
        .show(|_| {});
}

fn current_remote_url<R: tauri::Runtime>(webview: &Webview<R>) -> Option<String> {
    let url = webview.url().ok()?;
    let s = url.as_str();
    if s.starts_with(HOME_URL) {
        Some(s.to_owned())
    } else {
        None
    }
}

// Snapshot the webview's remote URL (if any) so a later reconnect can restore it.
fn snapshot_remote_url<R: tauri::Runtime>(app: &AppHandle<R>, webview: &Webview<R>) {
    if let (Some(state), Some(url)) = (
        app.try_state::<SharedLastRemoteUrl>(),
        current_remote_url(webview),
    ) {
        if let Ok(mut guard) = state.lock() {
            *guard = Some(url);
        }
    }
}

fn take_snapshotted_remote_url<R: tauri::Runtime>(app: &AppHandle<R>) -> Option<String> {
    let state = app.try_state::<SharedLastRemoteUrl>()?;
    let result = state.lock().ok()?.take();
    result
}

// Restore the user to where they were before the network blipped:
//   * If we still have the live remote URL on screen, reload it in place.
//   * Else if we captured one before going offline, navigate back to it.
//   * Else fall back to HOME_URL.
fn reload_or_home<R: tauri::Runtime>(app: &AppHandle<R>, webview: &Webview<R>) {
    if current_remote_url(webview).is_some() {
        if let Err(e) = webview.reload() {
            eprintln!("[connectivity] reload failed: {e}");
        }
        return;
    }

    let target = take_snapshotted_remote_url(app).unwrap_or_else(|| HOME_URL.to_owned());
    navigate_to(webview, &target);
}

fn store_connectivity_state<R: tauri::Runtime>(app: &AppHandle<R>, is_online: bool) {
    if let Some(state) = app.try_state::<SharedConnectivityState>() {
        state.store(is_online, Ordering::Relaxed);
    }
}

#[cfg(target_os = "macos")]
fn set_activation_policy<R: tauri::Runtime>(app: &AppHandle<R>, policy: ActivationPolicy) {
    if let Err(err) = app.set_activation_policy(policy) {
        eprintln!("Failed to set activation policy: {err}");
    }
}

#[tauri::command]
fn go_back(app: AppHandle) {
    if let Some(wv) = app.get_webview("content") {
        let _ = wv.eval("window.history.back()");
    }
}

#[tauri::command]
fn go_forward(app: AppHandle) {
    if let Some(wv) = app.get_webview("content") {
        let _ = wv.eval("window.history.forward()");
    }
}

#[tauri::command]
fn retry_connection(app: AppHandle) {
    let is_online = can_reach_app_host();
    store_connectivity_state(&app, is_online);

    if let Some(wv) = app.get_webview("content") {
        if is_online {
            reload_or_home(&app, &wv);
        } else {
            snapshot_remote_url(&app, &wv);
            navigate_to(&wv, OFFLINE_PAGE_URL);
        }
    }
}

#[tauri::command]
fn open_status_page() {
    let _ = open::that(STATUS_URL);
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            go_back,
            go_forward,
            retry_connection,
            open_status_page
        ])
        // Serve local HTML pages via the dodo:// custom scheme.
        // We use canonical `<scheme>://localhost/<path>` URLs (see OFFLINE_PAGE_URL /
        // TOOLBAR_PAGE_URL), so the handler always sees a stable `/<path>` regardless
        // of platform. Unknown paths return 404 to surface routing mistakes loudly
        // instead of silently falling through to one of the known pages.
        .register_uri_scheme_protocol("dodo", |_app, req| {
            let path = req.uri().path();
            let (status, body) = match path {
                "/offline" => (200, OFFLINE_HTML.as_bytes().to_vec()),
                "/toolbar" => (200, TOOLBAR_HTML.as_bytes().to_vec()),
                _ => (404, format!("Not Found: {path}").into_bytes()),
            };
            tauri::http::Response::builder()
                .status(status)
                .header("Content-Type", "text/html; charset=utf-8")
                .body(body)
                .unwrap()
        })
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // ── Auto-updater ─────────────────────────────────────────

            let update_state: SharedUpdateState =
                Arc::new(TokioMutex::new(UpdateState::default()));
            app.manage(update_state.clone());

            #[cfg(desktop)]
            spawn_update_loop(app.handle().clone(), update_state);

            // ── Window ───────────────────────────────────────────────

            let mut builder = WindowBuilder::new(app, "main")
                .title("Dodo Payments")
                .inner_size(1400.0, 900.0)
                .min_inner_size(800.0, 600.0)
                .center();

            // macOS: keep native traffic lights, extend content under titlebar
            #[cfg(target_os = "macos")]
            {
                builder = builder
                    .title_bar_style(tauri::TitleBarStyle::Overlay)
                    .hidden_title(true);
            }

            // Windows / Linux: remove native chrome entirely
            #[cfg(not(target_os = "macos"))]
            {
                builder = builder.decorations(false);
            }

            let window = builder.build()?;

            let scale = window.scale_factor()?;
            let size = window.inner_size()?.to_logical::<f64>(scale);

            // Toolbar webview — local HTML, serves back/forward icons
            // and (on Win/Linux) min/max/close buttons
            window.add_child(
                WebviewBuilder::new(
                    "toolbar",
                    WebviewUrl::External(TOOLBAR_PAGE_URL.parse()?),
                ),
                tauri::LogicalPosition::new(TOOLBAR_OFFSET_X, 0.0),
                tauri::LogicalSize::new(size.width - TOOLBAR_OFFSET_X, TOOLBAR_HEIGHT),
            )?;

            // Pre-flight the network check so the content webview can be constructed
            // with its real initial URL — going through `about:blank` and then eval'ing
            // `window.location.replace(...)` is racy on macOS WKWebView and can leave
            // the webview stuck on a blank screen.
            let is_online_on_startup = can_reach_app_host();
            let initial_content_url = if is_online_on_startup {
                HOME_URL
            } else {
                OFFLINE_PAGE_URL
            };

            // Content webview — the remote Dodo Payments app (or local offline page).
            window.add_child(
                WebviewBuilder::new(
                    "content",
                    WebviewUrl::External(initial_content_url.parse()?),
                )
                .user_agent("DodoDesktop"),
                tauri::LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
                tauri::LogicalSize::new(size.width, size.height - TOOLBAR_HEIGHT),
            )?;

            let connectivity_state: SharedConnectivityState =
                Arc::new(AtomicBool::new(is_online_on_startup));
            app.manage(connectivity_state.clone());

            let last_remote_url: SharedLastRemoteUrl = Arc::new(Mutex::new(None));
            app.manage(last_remote_url);

            if !is_online_on_startup {
                show_no_internet_popup(&app.handle());
            }

            // ── Deep link ─────────────────────────────────────────────

            let dl_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                if let Some(window) = dl_handle.get_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                if let Some(url) = event.urls().first() {
                    if let Some(wv) = dl_handle.get_webview("content") {
                        let query = url.query().unwrap_or("");
                        let callback = format!("{AUTH_CALLBACK_URL}?{query}&desktop_app=1");
                        navigate_to(&wv, &callback);
                    }
                }
            });

            #[cfg(any(target_os = "windows", target_os = "linux"))]
            app.deep_link().register_all()?;

            // ── App menu bar (macOS only) ─────────────────────────────

            #[cfg(target_os = "macos")]
            {
                let about_meta = AboutMetadataBuilder::new()
                    .name(Some("Dodo Payments"))
                    .version(Some(env!("CARGO_PKG_VERSION")))
                    .copyright(Some("© 2026 Dodo Payments"))
                    .website(Some("https://dodopayments.com"))
                    .website_label(Some("dodopayments.com"))
                    .build();

                let app_menu = SubmenuBuilder::new(app, "Dodo Payments")
                    .about(Some(about_meta))
                    .item(&MenuItem::with_id(
                        app, "check_for_updates", "Check for Updates…", true, None::<&str>,
                    )?)
                    .separator()
                    .services()
                    .separator()
                    .hide()
                    .hide_others()
                    .show_all()
                    .separator()
                    .quit()
                    .build()?;

                let file_menu = SubmenuBuilder::new(app, "File")
                    .item(&MenuItem::with_id(
                        app, "go_home", "Go to Dashboard", true,
                        Some("CmdOrCtrl+Shift+H"),
                    )?)
                    .separator()
                    .item(&MenuItem::with_id(
                        app, "reload", "Reload", true, Some("CmdOrCtrl+R"),
                    )?)
                    .item(&MenuItem::with_id(
                        app, "hard_reload", "Hard Reload", true, Some("CmdOrCtrl+Shift+R"),
                    )?)
                    .separator()
                    .close_window()
                    .build()?;

                let edit_menu = SubmenuBuilder::new(app, "Edit")
                    .undo().redo().separator()
                    .cut().copy().paste().select_all()
                    .build()?;

                #[cfg(debug_assertions)]
                let view_menu = SubmenuBuilder::new(app, "View")
                    .item(&MenuItem::with_id(app, "zoom_in",  "Zoom In",    true, Some("CmdOrCtrl+="))?)
                    .item(&MenuItem::with_id(app, "zoom_out", "Zoom Out",   true, Some("CmdOrCtrl+-"))?)
                    .item(&MenuItem::with_id(app, "zoom_reset","Actual Size",true, Some("CmdOrCtrl+0"))?)
                    .separator().fullscreen().separator()
                    .item(&MenuItem::with_id(app, "dev_tools", "Toggle Developer Tools", true, Some("CmdOrCtrl+Alt+I"))?)
                    .build()?;

                #[cfg(not(debug_assertions))]
                let view_menu = SubmenuBuilder::new(app, "View")
                    .item(&MenuItem::with_id(app, "zoom_in",  "Zoom In",    true, Some("CmdOrCtrl+="))?)
                    .item(&MenuItem::with_id(app, "zoom_out", "Zoom Out",   true, Some("CmdOrCtrl+-"))?)
                    .item(&MenuItem::with_id(app, "zoom_reset","Actual Size",true, Some("CmdOrCtrl+0"))?)
                    .separator().fullscreen()
                    .build()?;

                let history_menu = SubmenuBuilder::new(app, "History")
                    .item(&MenuItem::with_id(app, "go_back",    "Back",    true, Some("CmdOrCtrl+["))?)
                    .item(&MenuItem::with_id(app, "go_forward", "Forward", true, Some("CmdOrCtrl+]"))?)
                    .build()?;

                let help_menu = SubmenuBuilder::new(app, "Help")
                    .item(&MenuItem::with_id(app, "documentation", "Documentation", true, None::<&str>)?)
                    .item(&MenuItem::with_id(app, "support",       "Support",       true, None::<&str>)?)
                    .separator()
                    .item(&MenuItem::with_id(app, "copy_url", "Copy Current URL", true, Some("CmdOrCtrl+L"))?)
                    .build()?;

                let menu = MenuBuilder::new(app)
                    .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &history_menu, &help_menu])
                    .build()?;

                app.set_menu(menu)?;
            }

            // ── Menu event handler (macOS only) ───────────────────────

            #[cfg(target_os = "macos")]
            app.on_menu_event(move |app_handle, event| {
                if event.id().as_ref() == "check_for_updates" {
                    if let Some(state) = app_handle.try_state::<SharedUpdateState>() {
                        let app = app_handle.clone();
                        let state = state.inner().clone();
                        tauri::async_runtime::spawn(async move {
                            check_manual(app, state).await;
                        });
                    }
                    return;
                }

                let Some(wv) = app_handle.get_webview("content") else { return };
                match event.id().as_ref() {
                    "go_home" => { navigate_to(&wv, HOME_URL); }
                    "go_back" => { let _ = wv.eval("window.history.back()"); }
                    "go_forward" => { let _ = wv.eval("window.history.forward()"); }
                    "reload" => { let _ = wv.reload(); }
                    "hard_reload" => {
                        let _ = wv.eval(
                            "caches.keys().then(ks=>Promise.all(ks.map(k=>caches.delete(k)))).then(()=>window.location.reload())"
                        );
                    }
                    "zoom_in" => {
                        let _ = wv.eval("document.body.style.zoom=(parseFloat(document.body.style.zoom||1)+0.1).toString()");
                    }
                    "zoom_out" => {
                        let _ = wv.eval("document.body.style.zoom=Math.max(0.5,parseFloat(document.body.style.zoom||1)-0.1).toString()");
                    }
                    "zoom_reset" => { let _ = wv.eval("document.body.style.zoom='1'"); }
                    #[cfg(debug_assertions)]
                    "dev_tools" => {
                        if wv.is_devtools_open() { wv.close_devtools(); } else { wv.open_devtools(); }
                    }
                    "documentation" => { let _ = open::that(DOCS_URL); }
                    "support"       => { let _ = open::that(SUPPORT_URL); }
                    "copy_url" => {
                        let _ = wv.eval("navigator.clipboard.writeText(window.location.href)");
                    }
                    _ => {}
                }
            });

            // ── System tray ───────────────────────────────────────────

            let tray_quit = MenuItem::with_id(app, "tray_quit", "Quit Dodo Payments", true, None::<&str>)?;
            let tray_show = MenuItem::with_id(app, "tray_show", "Show Window",         true, None::<&str>)?;
            let tray_menu = tauri::menu::Menu::with_items(app, &[&tray_show, &tray_quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("Dodo Payments")
                .on_menu_event(|app_handle, event| match event.id.as_ref() {
                    "tray_quit" => { app_handle.exit(0); }
                    "tray_show" => {
                        if let Some(window) = app_handle.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();

                            #[cfg(target_os = "macos")]
                            set_activation_policy(app_handle, ActivationPolicy::Regular);
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app_handle = tray.app_handle();
                        if let Some(window) = app_handle.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // ── Connectivity monitor ──────────────────────────────────

            let app_handle = app.handle().clone();
            thread::spawn(move || {
                let connectivity_state = connectivity_state.clone();
                loop {
                    thread::sleep(Duration::from_secs(CONNECTIVITY_CHECK_INTERVAL_SECS));
                    let was_online = connectivity_state.load(Ordering::Relaxed);
                    let is_online = can_reach_app_host();
                    if is_online != was_online {
                        if let Some(wv) = app_handle.get_webview("content") {
                            if is_online {
                                reload_or_home(&app_handle, &wv);
                            } else {
                                snapshot_remote_url(&app_handle, &wv);
                                navigate_to(&wv, OFFLINE_PAGE_URL);
                            }
                        }
                        connectivity_state.store(is_online, Ordering::Relaxed);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Keep toolbar and content webviews stacked on resize
            if let tauri::WindowEvent::Resized(physical_size) = event {
                if let Ok(scale) = window.scale_factor() {
                    let size = physical_size.to_logical::<f64>(scale);
                    if let Some(tb) = window.get_webview("toolbar") {
                        let _ = tb.set_size(tauri::LogicalSize::new(
                            size.width - TOOLBAR_OFFSET_X,
                            TOOLBAR_HEIGHT,
                        ));
                    }
                    if let Some(cv) = window.get_webview("content") {
                        let _ = cv.set_position(tauri::LogicalPosition::new(0.0, TOOLBAR_HEIGHT));
                        let _ = cv.set_size(tauri::LogicalSize::new(
                            size.width,
                            (size.height - TOOLBAR_HEIGHT).max(0.0),
                        ));
                    }
                }
            }

            // Hide to tray instead of closing on macOS
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
                
                #[cfg(target_os = "macos")]
                set_activation_policy(window.app_handle(), ActivationPolicy::Accessory);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Dodo Payments");
}
