#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//! Simorgh (سیمرغ) — real-time bidirectional speech translation.

mod commands;
mod state;
mod windows;

use state::AppState;
use tauri::Manager;
use tracing_subscriber::prelude::*;

fn main() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::get_config,
            commands::set_config,
            commands::start_pipeline,
            commands::stop_pipeline,
            commands::pipeline_status,
            commands::latency_report,
            commands::meter_levels,
            commands::set_api_key,
            commands::ensure_virtual_mic,
            commands::list_models,
            commands::download_model,
            commands::delete_model,
            commands::export_history,
            commands::toggle_overlay,
            commands::set_overlay_click_through,
            commands::get_logs
        ])
        .setup(|app| {
            // Overlay is created lazily when a pipeline starts (see windows::show_overlay).
            windows::register_hotkeys(app.handle())?;
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                // Closing the dashboard quits the whole app (tears down the overlay too).
                tauri::WindowEvent::CloseRequested { .. } if window.label() == "main" => {
                    app.exit(0);
                }
                // Persist overlay position/size on move/resize.
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                    if window.label() == windows::OVERLAY_LABEL =>
                {
                    app.state::<AppState>().persist_overlay_bounds(window);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Simorgh");
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Show Simorgh", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .expect("default window icon");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Simorgh")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

fn init_logging() {
    let file_appender = tracing_appender::rolling::daily(log_dir(), "simorgh.log");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,simorgh=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_appender),
        )
        .init();
}

pub fn log_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("app", "simorgh", "simorgh")
        .map(|d| d.data_dir().join("logs"))
        .unwrap_or_else(|| "logs".into())
}
