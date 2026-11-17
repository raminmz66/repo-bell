pub mod actions;
pub mod autostart;
pub mod editors;
pub mod news;
pub mod notify;
pub mod store;
pub mod updates;
pub mod watcher;

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};

use watcher::SharedEngine;

/// Shows the window at the top right of its screen, then raises and focuses it. The position is
/// set after `show` because some window managers ignore a position given to a hidden window.
/// Emits `window-shown` at the end if the window was hidden.
fn show_and_position_window(window: &tauri::WebviewWindow) {
    let was_hidden = !window.is_visible().unwrap_or(true);
    let _ = window.show();
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) {
        let scale = monitor.scale_factor();
        let origin = monitor.position();
        let x = origin.x + monitor.size().width as i32 - size.width as i32 - (16.0 * scale) as i32;
        let y = origin.y + (36.0 * scale) as i32;
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: x.max(origin.x),
            y,
        }));
    }
    let _ = window.unminimize();
    let _ = window.set_focus();
    if was_hidden {
        let _ = window.emit("window-shown", ());
    }
}

/// A fresh folder for the tray icon files on each launch. The tray names its files
/// `tray-icon-<id>-<counter>.png` with a counter that restarts at zero, and GNOME's AppIndicator
/// extension caches images by path, so reusing a folder shows the previous run's icons.
fn tray_icon_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("repobell-tray");
    // Icons from earlier launches are never read again.
    let _ = std::fs::remove_dir_all(&base);
    let launch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    base.join(launch.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine: SharedEngine = watcher::Shared::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(engine.clone())
        .invoke_handler(tauri::generate_handler![
            actions::get_app_state,
            actions::check_now,
            actions::mark_all_seen,
            actions::save_settings,
            actions::remove_folder,
            actions::preview_update,
            actions::update_repo,
            actions::undo_repo,
            actions::refresh_old_branches,
            actions::delete_old_branches,
            actions::undo_cleanup,
            actions::update_main,
            actions::open_editor,
            actions::pick_folder,
            actions::detect_editors,
            actions::discover_suggested_folders,
            actions::scan_folder,
            actions::check_tray,
            actions::hide_window
        ])
        .setup({
            let engine = engine.clone();
            move |app| {
                let handle = app.handle().clone();

                if let Some(w) = app.get_webview_window("main") {
                    show_and_position_window(&w);
                }

                // The first tray update replaces this menu with the real one.
                let headline =
                    MenuItem::with_id(app, "headline", "All quiet", false, None::<&str>)?;
                let menu = MenuBuilder::new(app)
                    .item(&headline)
                    .separator()
                    .text("open", "Open RepoBell")
                    .text("check_now", "Check now")
                    .text("mark_seen", "Mark all as seen")
                    .separator()
                    .text("quit", "Quit")
                    .build()?;

                let icon = Image::from_bytes(include_bytes!("../icons/tray/bell.png"))?;

                TrayIconBuilder::with_id("main-tray")
                    .icon(icon)
                    .temp_dir_path(tray_icon_dir())
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event({
                        let engine = engine.clone();
                        move |app, event| {
                            let id = event.id().as_ref();
                            match id {
                                "open" => {
                                    if let Some(w) = app.get_webview_window("main") {
                                        show_and_position_window(&w);
                                    }
                                }
                                "quit" => app.exit(0),
                                "check_now" => {
                                    let (engine, app) = (engine.clone(), app.clone());
                                    tauri::async_runtime::spawn(async move {
                                        watcher::run_check(&app, &engine).await;
                                    });
                                }
                                "mark_seen" => {
                                    let (engine, app) = (engine.clone(), app.clone());
                                    tauri::async_runtime::spawn(async move {
                                        let _ = watcher::mark_all_seen(&app, &engine).await;
                                    });
                                }
                                _ => {
                                    if let Some(repo_path) = id.strip_prefix("repo:") {
                                        if let Some(w) = app.get_webview_window("main") {
                                            show_and_position_window(&w);
                                            let _ = w.emit("focus-repo", repo_path);
                                        }
                                    }
                                }
                            }
                        }
                    })
                    .build(app)?;

                engine.state.blocking_lock().update = updates::UpdateState::for_bundle(
                    app.package_info().version.to_string(),
                    tauri::utils::platform::bundle_type(),
                );

                // The first check (and every later one) runs in the background.
                tauri::async_runtime::spawn(watcher::scheduler(handle, engine.clone()));
                Ok(())
            }
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running RepoBell");
}
