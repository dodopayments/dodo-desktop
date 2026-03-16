use std::{
    net::{TcpStream, ToSocketAddrs},
    thread,
    time::Duration,
};

use tauri::{
    menu::{AboutMetadataBuilder, MenuBuilder, MenuItem, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};
use tauri_plugin_deep_link::DeepLinkExt;

const DOCS_URL: &str = "https://docs.dodopayments.com";
const SUPPORT_URL: &str = "https://dodopayments.com/support";
const HOME_URL: &str = "https://app.dodopayments.com";
const STATUS_URL: &str = "https://status.dodopayments.com";
const AUTH_CALLBACK_URL: &str = "https://app.dodopayments.com/login/magic-link";
const APP_HOST_PORT: &str = "app.dodopayments.com:443";
const CONNECT_TIMEOUT_SECS: u64 = 3;
const CONNECTIVITY_CHECK_INTERVAL_SECS: u64 = 10;

fn can_reach_app_host() -> bool {
    let timeout = Duration::from_secs(CONNECT_TIMEOUT_SECS);
    let Ok(addrs) = APP_HOST_PORT.to_socket_addrs() else {
        return false;
    };

    addrs
        .into_iter()
        .any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok())
}

fn load_home(window: &WebviewWindow) {
    let _ = window.eval(&format!("window.location.replace('{HOME_URL}')"));
}

fn render_offline_page(window: &WebviewWindow) {
        let offline_html = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Dodo Payments – Unable to Connect</title>
    <style>
      *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

      :root {
        color-scheme: light dark;
        --bg: #050710;
        --fg: #f9fafb;
        --muted: #9ca3af;
        --accent: #bef264;
      }

      @media (prefers-color-scheme: light) {
        :root {
          --bg: #f3f4f6;
          --fg: #030712;
          --muted: #6b7280;
          --accent: #a3e635;
        }
      }

      body {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 32px 24px;
        font-family: "Inter", "Segoe UI", system-ui, -apple-system, sans-serif;
        background: var(--bg);
        color: var(--fg);
        -webkit-font-smoothing: antialiased;
      }

      .wrap {
        max-width: 460px;
        width: 100%;
        text-align: left;
      }

      h1 {
        font-size: 1.8rem;
        font-weight: 700;
        line-height: 1.2;
        margin-bottom: 10px;
      }

      .desc {
        font-size: 0.95rem;
        line-height: 1.55;
        color: var(--muted);
        margin-bottom: 10px;
      }

      .btns {
        display: inline-flex;
        gap: 10px;
        flex-wrap: wrap;
        margin: 24px 0 20px;
      }

      button {
        padding: 9px 20px;
        border-radius: 8px;
        font-size: 0.9rem;
        font-weight: 600;
        cursor: pointer;
        border: none;
        font-family: inherit;
        line-height: 1;
      }

      .btn-retry {
        background: var(--accent);
        color: #020308;
      }

      .btn-retry:hover {
        background: #a3e635;
      }

      .btn-status {
        background: transparent;
        color: var(--fg);
        border: 1px solid rgba(148, 163, 184, 0.5);
      }

      .btn-status:hover {
        background: rgba(148, 163, 184, 0.08);
      }

      .note {
        font-size: 0.83rem;
        color: var(--muted);
      }

      @media (max-width: 480px) {
        button { width: 100%; }
      }
    </style>
  </head>
  <body>
    <main class="wrap" role="alert" aria-live="polite">
      <h1>Unable to connect</h1>
      <p class="desc">We couldn’t reach your dashboard. Please check your connection and try again.</p>
      <div class="btns">
        <button class="btn-retry" onclick="window.__TAURI_INTERNALS__.invoke('retry_connection')">Retry</button>
        <button class="btn-status" onclick="window.__TAURI_INTERNALS__.invoke('open_status_page')">Service Status</button>
      </div>
      <p class="note">The app reconnects automatically when connectivity is available.</p>
    </main>
  </body>
</html>"#;

    if let Ok(html_json) = serde_json::to_string(offline_html) {
        let _ = window.eval(&format!(
            "document.open();document.write({html_json});document.close();"
        ));
    }
}

fn apply_connectivity_state(window: &WebviewWindow, is_online: bool) {
    if is_online {
        load_home(window);
    } else {
        render_offline_page(window);
    }
}

#[tauri::command]
fn retry_connection(window: WebviewWindow) {
    if can_reach_app_host() {
        load_home(&window);
    } else {
        render_offline_page(&window);
    }
}

#[tauri::command]
fn open_status_page() {
    let _ = open::that(STATUS_URL);
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![retry_connection, open_status_page])
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let dl_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let Some(window) = dl_handle.get_webview_window("main") else {
                    return;
                };
                let _ = window.show();
                let _ = window.set_focus();
                if let Some(url) = event.urls().first() {
                    let query = url.query().unwrap_or("");
                    let callback = format!("{AUTH_CALLBACK_URL}?{query}");
                    let _ = window.eval(&format!("window.location.replace('{callback}')"));
                }
            });

            #[cfg(any(target_os = "windows", target_os = "linux"))]
            app.deep_link().register_all()?;

            // ── App menu bar ──────────────────────────────────────────

            // Only show a native menu bar on macOS.
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
                        app,
                        "go_home",
                        "Go to Dashboard",
                        true,
                        Some("CmdOrCtrl+Shift+H"),
                    )?)
                    .separator()
                    .item(&MenuItem::with_id(
                        app,
                        "reload",
                        "Reload",
                        true,
                        Some("CmdOrCtrl+R"),
                    )?)
                    .item(&MenuItem::with_id(
                        app,
                        "hard_reload",
                        "Hard Reload",
                        true,
                        Some("CmdOrCtrl+Shift+R"),
                    )?)
                    .separator()
                    .close_window()
                    .build()?;

                let edit_menu = SubmenuBuilder::new(app, "Edit")
                    .undo()
                    .redo()
                    .separator()
                    .cut()
                    .copy()
                    .paste()
                    .select_all()
                    .build()?;

                #[cfg(debug_assertions)]
                let view_menu = SubmenuBuilder::new(app, "View")
                    .item(&MenuItem::with_id(
                        app,
                        "zoom_in",
                        "Zoom In",
                        true,
                        Some("CmdOrCtrl+="),
                    )?)
                    .item(&MenuItem::with_id(
                        app,
                        "zoom_out",
                        "Zoom Out",
                        true,
                        Some("CmdOrCtrl+-"),
                    )?)
                    .item(&MenuItem::with_id(
                        app,
                        "zoom_reset",
                        "Actual Size",
                        true,
                        Some("CmdOrCtrl+0"),
                    )?)
                    .separator()
                    .fullscreen()
                    .separator()
                    .item(&MenuItem::with_id(
                        app,
                        "dev_tools",
                        "Toggle Developer Tools",
                        true,
                        Some("CmdOrCtrl+Alt+I"),
                    )?)
                    .build()?;

                #[cfg(not(debug_assertions))]
                let view_menu = SubmenuBuilder::new(app, "View")
                    .item(&MenuItem::with_id(
                        app,
                        "zoom_in",
                        "Zoom In",
                        true,
                        Some("CmdOrCtrl+="),
                    )?)
                    .item(&MenuItem::with_id(
                        app,
                        "zoom_out",
                        "Zoom Out",
                        true,
                        Some("CmdOrCtrl+-"),
                    )?)
                    .item(&MenuItem::with_id(
                        app,
                        "zoom_reset",
                        "Actual Size",
                        true,
                        Some("CmdOrCtrl+0"),
                    )?)
                    .separator()
                    .fullscreen()
                    .build()?;

                let help_menu = SubmenuBuilder::new(app, "Help")
                    .item(&MenuItem::with_id(
                        app,
                        "documentation",
                        "Documentation",
                        true,
                        None::<&str>,
                    )?)
                    .item(&MenuItem::with_id(
                        app,
                        "support",
                        "Support",
                        true,
                        None::<&str>,
                    )?)
                    .separator()
                    .item(&MenuItem::with_id(
                        app,
                        "copy_url",
                        "Copy Current URL",
                        true,
                        Some("CmdOrCtrl+L"),
                    )?)
                    .build()?;

                let menu = MenuBuilder::new(app)
                    .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &help_menu])
                    .build()?;

                app.set_menu(menu)?;
            }

            // ── Menu event handler ────────────────────────────────────

            #[cfg(target_os = "macos")]
            app.on_menu_event(move |app_handle, event| {
                let Some(window) = app_handle.get_webview_window("main") else {
                    return;
                };

                match event.id().as_ref() {
                    "go_home" => {
                        let _ = window.eval(&format!("window.location.href = '{HOME_URL}'"));
                    }
                    "reload" => {
                        let _ = window.eval("window.location.reload()");
                    }
                    "hard_reload" => {
                        let _ = window.eval(
                            "caches.keys().then(ks => Promise.all(ks.map(k => caches.delete(k)))).then(() => window.location.reload())"
                        );
                    }
                    "zoom_in" => {
                        let _ = window.eval(
                            "document.body.style.zoom = (parseFloat(document.body.style.zoom || 1) + 0.1).toString()"
                        );
                    }
                    "zoom_out" => {
                        let _ = window.eval(
                            "document.body.style.zoom = Math.max(0.5, parseFloat(document.body.style.zoom || 1) - 0.1).toString()"
                        );
                    }
                    "zoom_reset" => {
                        let _ = window.eval("document.body.style.zoom = '1'");
                    }
                    #[cfg(debug_assertions)]
                    "dev_tools" => {
                        if window.is_devtools_open() {
                            window.close_devtools();
                        } else {
                            window.open_devtools();
                        }
                    }
                    "documentation" => {
                        let _ = open::that(DOCS_URL);
                    }
                    "support" => {
                        let _ = open::that(SUPPORT_URL);
                    }
                    "copy_url" => {
                        let _ = window.eval(
                            "navigator.clipboard.writeText(window.location.href)"
                        );
                    }
                    _ => {}
                }
            });

            // ── System tray ───────────────────────────────────────────

            let tray_quit =
                MenuItem::with_id(app, "tray_quit", "Quit Dodo Payments", true, None::<&str>)?;
            let tray_show =
                MenuItem::with_id(app, "tray_show", "Show Window", true, None::<&str>)?;
            let tray_menu =
                tauri::menu::Menu::with_items(app, &[&tray_show, &tray_quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("Dodo Payments")
                .on_menu_event(|app_handle, event| match event.id.as_ref() {
                    "tray_quit" => {
                        app_handle.exit(0);
                    }
                    "tray_show" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = tray.app_handle();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            thread::spawn(move || {
                let mut was_online = can_reach_app_host();

                if let Some(window) = app_handle.get_webview_window("main") {
                    apply_connectivity_state(&window, was_online);
                }

                loop {
                    thread::sleep(Duration::from_secs(CONNECTIVITY_CHECK_INTERVAL_SECS));

                    let is_online = can_reach_app_host();
                    if is_online != was_online {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            apply_connectivity_state(&window, is_online);
                        }
                        was_online = is_online;
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide to tray instead of closing on macOS
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }

            #[cfg(not(target_os = "macos"))]
            {
                let _ = (window, event);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Dodo Payments");
}
