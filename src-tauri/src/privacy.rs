use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager, Runtime};

use crate::app::AppState;

#[cfg(target_os = "linux")]
const PRIVACY_STYLE: &str = include_str!("../scripts/privacy.css");
#[cfg(not(target_os = "linux"))]
pub const INITIALIZATION_SCRIPT: &str = include_str!("../scripts/privacy.js");

pub fn toggle<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let enabled = !state.privacy_enabled.fetch_xor(true, Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("main") {
        apply_to_window(&window, enabled);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "linux")]
pub fn register_local_shortcut<R: Runtime>(app: &AppHandle<R>) {
    use gtk::prelude::*;

    let app_for_shortcut = app.clone();
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.with_webview(move |webview| {
            if let Some(window) = webview
                .inner()
                .toplevel()
                .and_then(|widget| widget.downcast::<gtk::Window>().ok())
            {
                install_local_shortcut(&window, move || toggle(&app_for_shortcut));
            }
        }) {
            eprintln!("window privacy shortcut unavailable: {error}");
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn install_local_shortcut(window: &gtk::Window, toggle: impl Fn() + 'static) {
    use gtk::{gdk, prelude::*};

    let group = gtk::AccelGroup::new();
    group.connect_accel_group(
        *gdk::keys::constants::b,
        gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
        gtk::AccelFlags::VISIBLE,
        move |_, _, _, _| {
            toggle();
            true
        },
    );
    window.add_accel_group(&group);
}

#[cfg(target_os = "linux")]
fn apply_to_window<R: Runtime>(window: &tauri::WebviewWindow<R>, enabled: bool) {
    if let Err(error) = window.with_webview(move |webview| {
        apply_to_webview(&webview.inner(), enabled);
    }) {
        eprintln!("failed to update privacy blur: {error}");
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn apply_to_webview(webview: &webkit2gtk::WebView, enabled: bool) {
    use webkit2gtk::{
        UserContentInjectedFrames, UserContentManagerExt, UserStyleLevel, UserStyleSheet,
        WebViewExt,
    };

    thread_local! {
        static STYLE: UserStyleSheet = UserStyleSheet::new(
            PRIVACY_STYLE,
            UserContentInjectedFrames::TopFrame,
            UserStyleLevel::User,
            &[
                "https://web.whatsapp.com/*",
                "https://flows.whatsapp.net/*",
                "https://webtp.whatsapp.net/*",
            ],
            &[],
        );
    }

    if let Some(manager) = webview.user_content_manager() {
        STYLE.with(|style| {
            // Remove only our stylesheet. Tauri owns the manager's IPC scripts.
            manager.remove_style_sheet(style);
            if enabled {
                manager.add_style_sheet(style);
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_to_window<R: Runtime>(window: &tauri::WebviewWindow<R>, enabled: bool) {
    let script = if enabled {
        "document.documentElement.classList.add('whatsapp-linux-privacy');"
    } else {
        "document.documentElement.classList.remove('whatsapp-linux-privacy');"
    };
    if let Err(error) = window.eval(script) {
        eprintln!("failed to update privacy blur: {error}");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn reapply<R: Runtime>(app: &AppHandle<R>) {
    let enabled = app
        .state::<AppState>()
        .privacy_enabled
        .load(Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("main") {
        apply_to_window(&window, enabled);
    }
}
