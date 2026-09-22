use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager, Runtime};

use crate::app::AppState;

pub const INITIALIZATION_SCRIPT: &str = include_str!("../scripts/privacy.js");
const ENABLE_SCRIPT: &str =
    r#"(() => { document.documentElement.classList.add('whatsapp-linux-privacy'); })();"#;
const DISABLE_SCRIPT: &str =
    r#"(() => { document.documentElement.classList.remove('whatsapp-linux-privacy'); })();"#;

pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let enabled = !state.privacy_enabled.fetch_xor(true, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        apply_to_window(&window, enabled);
    }
}

pub fn reapply<R: Runtime>(app: &AppHandle<R>) {
    let enabled = app
        .state::<AppState>()
        .privacy_enabled
        .load(Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("main") {
        apply_to_window(&window, enabled);
    }
}

fn apply_to_window<R: Runtime>(window: &tauri::WebviewWindow<R>, enabled: bool) {
    let script = if enabled {
        ENABLE_SCRIPT
    } else {
        DISABLE_SCRIPT
    };

    if let Err(error) = window.eval(script) {
        eprintln!("failed to update privacy blur: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_use_only_fixed_namespaced_values() {
        assert!(INITIALIZATION_SCRIPT.contains("whatsapp-linux-privacy"));
        assert!(ENABLE_SCRIPT.contains("classList.add"));
        assert!(DISABLE_SCRIPT.contains("classList.remove"));
        assert!(!ENABLE_SCRIPT.contains("${"));
        assert!(!DISABLE_SCRIPT.contains("${"));
    }
}
