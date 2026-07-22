//! Window management: the frameless always-on-top subtitle overlay and
//! global hotkeys. Overlay position/size/opacity are persisted via AppState.

use crate::state::AppState;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const OVERLAY_LABEL: &str = "overlay";

pub fn create_overlay(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().expect("config lock").clone();

    let mut builder = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("index.html#/overlay".into()))
        .title("Simorgh Subtitles")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .inner_size(cfg.subtitle.width.unwrap_or(760) as f64, 132.0);
    if let (Some(x), Some(y)) = (cfg.subtitle.x, cfg.subtitle.y) {
        builder = builder.position(x as f64, y as f64);
    }
    let overlay = builder.build()?;
    // Click-through by default; toggleable from the main window.
    overlay.set_ignore_cursor_events(cfg.subtitle.click_through)?;
    Ok(())
}

pub fn toggle_overlay(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(OVERLAY_LABEL) {
        if w.is_visible()? { w.hide()?; } else { w.show()?; }
    }
    Ok(())
}

pub fn set_click_through(app: &AppHandle, enabled: bool) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(OVERLAY_LABEL) {
        w.set_ignore_cursor_events(enabled)?;
    }
    if let Some(state) = app.try_state::<AppState>() {
        let mut cfg = state.config.lock().expect("config lock");
        cfg.subtitle.click_through = enabled;
        let _ = cfg.save();
    }
    Ok(())
}

/// Global hotkey: toggle both pipelines. Push-to-talk is handled by the
/// frontend (keydown/keyup) while the window is focused, and via this global
/// shortcut otherwise.
pub fn register_hotkeys(app: &AppHandle) -> tauri::Result<()> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    let hotkey = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().expect("config lock");
        cfg.general.global_hotkey_toggle.clone()
    };
    let handle = app.clone();
    app.global_shortcut().on_shortcut(hotkey.as_str(), move |_app, _sc, event| {
        if event.state() == ShortcutState::Pressed {
            use tauri::Emitter;
            let _ = handle.emit("hotkey-toggle-pipelines", ());
        }
    }).map_err(|e| tauri::Error::Anyhow(e.into()))?;
    Ok(())
}
