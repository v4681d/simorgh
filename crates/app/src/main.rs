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
            commands::set_overlay_click_through
        ])
        .setup(|app| {
            windows::create_overlay(app.handle())?;
            windows::register_hotkeys(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Persist overlay position/size on move/resize.
            if window.label() == windows::OVERLAY_LABEL {
                if let tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) = event {
                    let state = window.app_handle().state::<AppState>();
                    state.persist_overlay_bounds(window);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Simorgh");
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

fn log_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("app", "simorgh", "simorgh")
        .map(|d| d.data_dir().join("logs"))
        .unwrap_or_else(|| "logs".into())
}
