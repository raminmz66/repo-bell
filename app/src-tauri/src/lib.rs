pub mod store;

use std::path::PathBuf;

use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{image::Image, Manager, WindowEvent};

#[derive(Serialize)]
struct RepoView {
    path: PathBuf,
    name: String,
    /// Plain-words problem; when set the numbers below mean nothing.
    error: Option<String>,
    behind: u32,
    ahead: u32,
    unsaved: bool,
}

// Async so Git runs off the main thread and the tray stays responsive.
#[tauri::command]
async fn repos() -> Vec<RepoView> {
    let settings = store::load_settings(&store::settings_path());
    repobell_core::discover(&settings.folders, 4)
        .into_iter()
        .filter(|p| !settings.ignored.contains(p))
        .map(|path| {
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            let mut v = RepoView { path, name, error: None, behind: 0, ahead: 0, unsaved: false };
            match repobell_core::status(&v.path) {
                Ok(s) => (v.behind, v.ahead, v.unsaved) = (s.behind, s.ahead, s.unsaved),
                Err(e) => {
                    v.error = Some(repobell_core::classify(&e, &v.path).message().to_string())
                }
            }
            v
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![repos])
        .setup(|app| {
            let headline = MenuItem::with_id(app, "headline", "All quiet", false, None::<&str>)?;
            let menu = MenuBuilder::new(app)
                .item(&headline)
                .separator()
                .text("open", "Open RepoBell")
                .text("quit", "Quit")
                .build()?;
            TrayIconBuilder::new()
                .icon(Image::from_bytes(include_bytes!("../icons/tray/bell.png"))?)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
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
