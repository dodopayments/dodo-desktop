use tauri::{
    menu::{AboutMetadataBuilder, MenuBuilder, MenuItem, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

const DOCS_URL: &str = "https://docs.dodopayments.com";
const SUPPORT_URL: &str = "https://dodopayments.com/support";
const HOME_URL: &str = "https://app.dodopayments.com";


pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // ── App menu bar ──────────────────────────────────────────

            // macOS: first submenu becomes the app menu ("Dodo Payments")
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

            // File menu
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

            // Edit menu
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            // View menu
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

            // Help menu
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

            // ── Menu event handler ────────────────────────────────────

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

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide to tray instead of closing on macOS
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(target_os = "macos")]
                {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Dodo Payments");
}
