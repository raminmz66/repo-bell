pub mod actions;
pub mod autostart;
pub mod notify;
pub mod store;
pub mod watcher;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tauri::async_runtime::Mutex;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};

use watcher::{EngineState, SharedEngine};

fn show_and_position_window(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let screen_size = monitor.size();
        let scale = monitor.scale_factor();
        let win_width = (420.0 * scale) as i32;
        let x = (screen_size.width as i32) - win_width - (16.0 * scale) as i32;
        let y = (36.0 * scale) as i32;
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: x.max(0),
            y,
        }));
    }
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine: SharedEngine = Arc::new(Mutex::new(EngineState::new()));

    tauri::Builder::default()
        .manage(engine.clone())
        .invoke_handler(tauri::generate_handler![
            actions::get_app_state,
            actions::check_now,
            actions::mark_all_seen,
            actions::save_settings,
            actions::update_repo,
            actions::undo_repo,
            actions::open_editor,
            actions::pick_folder,
            actions::detect_editors,
            actions::discover_suggested_folders,
            actions::hide_window
        ])
        .setup({
            let engine = engine.clone();
            move |app| {
                let handle = app.handle().clone();

                if let Some(w) = app.get_webview_window("main") {
                    show_and_position_window(&w);
                }

                // Initial tray setup
                let headline = MenuItem::with_id(app, "headline", "All quiet", false, None::<&str>)?;
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

                let _tray = TrayIconBuilder::with_id("main-tray")
                    .icon(icon)
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event({
                        let engine = engine.clone();
                        let _handle = handle.clone();
                        move |app, event| {
                            let id_str = event.id().as_ref();
                            if id_str == "open" {
                                if let Some(w) = app.get_webview_window("main") {
                                    show_and_position_window(&w);
                                }
                            } else if id_str == "quit" {
                                app.exit(0);
                            } else if id_str == "check_now" {
                                let engine = engine.clone();
                                let app = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    let (settings, seen) = {
                                        let state = engine.lock().await;
                                        (state.settings.clone(), state.seen.clone())
                                    };
                                    let (mut repos, offline) = watcher::check_repositories(&settings, &seen);
                                    let mut state = engine.lock().await;
                                    watcher::apply_settings_to_repos(&mut repos, &state.settings);
                                    state.repos = repos;
                                    state.offline = offline;
                                    state.last_checked = Some(watcher::now_seconds());
                                    if let Some(tray) = app.tray_by_id("main-tray") {
                                        let _ = watcher::update_tray(&app, &tray, &state);
                                    }
                                    let _ = app.emit("state-updated", state.to_view());
                                });
                            } else if id_str == "mark_seen" {
                                let engine = engine.clone();
                                let app = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    let mut state = engine.lock().await;
                                    let mut to_insert = Vec::new();
                                    for r in &state.repos {
                                        if let Ok(sha) = repobell_core::remote_main_sha(&r.path, &r.main_branch) {
                                            to_insert.push((r.path.clone(), sha));
                                        }
                                    }
                                    for (p, s) in to_insert {
                                        state.seen.insert(p, s);
                                    }
                                    let _ = store::save_seen(&store::seen_path(), &state.seen);
                                    for r in &mut state.repos {
                                        r.feed = None;
                                    }
                                    if let Some(tray) = app.tray_by_id("main-tray") {
                                        let _ = watcher::update_tray(&app, &tray, &state);
                                    }
                                    let _ = app.emit("state-updated", state.to_view());
                                });
                            } else if let Some(repo_path) = id_str.strip_prefix("repo:") {
                                if let Some(w) = app.get_webview_window("main") {
                                    show_and_position_window(&w);
                                    let _ = w.emit("focus-repo", repo_path);
                                }
                            }
                        }
                    })
                    .build(app)?;

                // Background check loop
                let bg_engine = engine.clone();
                let bg_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let mut known_notified_shas: HashSet<String> = HashSet::new();

                    // Initial check on launch
                    {
                        let (settings, seen) = {
                            let state = bg_engine.lock().await;
                            (state.settings.clone(), state.seen.clone())
                        };

                        // Populate initially known shas
                        for sha in seen.values() {
                            known_notified_shas.insert(sha.clone());
                        }

                        let (mut repos, offline) = watcher::check_repositories(&settings, &seen);
                        notify::notify_news(&repos, &known_notified_shas);

                        for r in &repos {
                            if let Some(ref feed) = r.feed {
                                for c in &feed.changes {
                                    known_notified_shas.insert(c.sha.clone());
                                }
                            }
                        }

                        let mut state = bg_engine.lock().await;
                        watcher::apply_settings_to_repos(&mut repos, &state.settings);
                        state.repos = repos;
                        state.offline = offline;
                        state.last_checked = Some(watcher::now_seconds());

                        if let Some(tray) = bg_handle.tray_by_id("main-tray") {
                            let _ = watcher::update_tray(&bg_handle, &tray, &state);
                        }
                        let _ = bg_handle.emit("state-updated", state.to_view());
                    }

                    loop {
                        let minutes = {
                            let state = bg_engine.lock().await;
                            state.settings.check_minutes.max(1)
                        };
                        tokio::time::sleep(Duration::from_secs(minutes as u64 * 60)).await;

                        let (settings, seen) = {
                            let state = bg_engine.lock().await;
                            (state.settings.clone(), state.seen.clone())
                        };

                        let (mut repos, offline) = watcher::check_repositories(&settings, &seen);
                        notify::notify_news(&repos, &known_notified_shas);

                        for r in &repos {
                            if let Some(ref feed) = r.feed {
                                for c in &feed.changes {
                                    known_notified_shas.insert(c.sha.clone());
                                }
                            }
                        }

                        let mut state = bg_engine.lock().await;
                        watcher::apply_settings_to_repos(&mut repos, &state.settings);
                        state.repos = repos;
                        state.offline = offline;
                        state.last_checked = Some(watcher::now_seconds());

                        if let Some(tray) = bg_handle.tray_by_id("main-tray") {
                            let _ = watcher::update_tray(&bg_handle, &tray, &state);
                        }
                        let _ = bg_handle.emit("state-updated", state.to_view());
                    }
                });

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
