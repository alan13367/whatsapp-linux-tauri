use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{badge, navigation, privacy};

const MAIN_WINDOW: &str = "main";
const TRAY_ID: &str = "main-tray";
const MENU_TOGGLE: &str = "toggle-window";
const MENU_PRIVACY: &str = "toggle-privacy";
const MENU_QUIT: &str = "quit";
const MAX_UNREAD: u32 = 9_999;
const CHROMIUM_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Default)]
pub struct AppState {
    pub is_quitting: AtomicBool,
    pub privacy_enabled: AtomicBool,
    pub tray_available: AtomicBool,
    pub unread: AtomicU32,
}

pub fn run() {
    let builder = tauri::Builder::default()
        // The single-instance plugin must be registered first.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::default())
        .setup(setup)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if !state.is_quitting.load(Ordering::SeqCst)
                    && state.tray_available.load(Ordering::SeqCst)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        });

    builder
        .run(tauri::generate_context!())
        .expect("failed to run WhatsApp Linux");
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    if !linux_audio_sink_available() {
        show_missing_audio_sink(app.handle().clone());
        return Ok(());
    }

    create_main_window(app)?;
    match create_tray(app) {
        Ok(()) => app
            .state::<AppState>()
            .tray_available
            .store(true, Ordering::SeqCst),
        Err(error) => eprintln!("system tray unavailable; close will exit: {error}"),
    }
    register_privacy_shortcut(app);
    Ok(())
}

fn create_main_window(app: &tauri::App) -> tauri::Result<WebviewWindow> {
    let app_for_new_window = app.handle().clone();
    let app_for_title = app.handle().clone();
    let app_for_load = app.handle().clone();

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW)
        .expect("main window config must exist");

    let builder = WebviewWindowBuilder::from_config(app, config)?
        .initialization_script(privacy::INITIALIZATION_SCRIPT)
        .enable_clipboard_access()
        .devtools(cfg!(debug_assertions))
        .on_new_window(move |url, _features| {
            navigation::open_web_url(&app_for_new_window, &url);
            tauri::webview::NewWindowResponse::Deny
        })
        .on_document_title_changed(move |_window, title| {
            update_unread(&app_for_title, parse_unread_count(&title));
        })
        .on_page_load(move |_window, payload| {
            if navigation::is_allowed_app_navigation(payload.url()) {
                // Apply at navigation start to minimize exposure, then again after load as a fallback.
                privacy::reapply(&app_for_load);
            }
        })
        .on_download(|_webview, event| {
            if let tauri::webview::DownloadEvent::Finished { url, path, success } = event {
                if !success {
                    eprintln!("download failed for {url}; destination: {path:?}");
                }
            }
            true
        });

    let builder = if std::env::args().any(|argument| argument == "--chromium-user-agent") {
        builder.user_agent(CHROMIUM_USER_AGENT)
    } else {
        builder
    };
    let window = builder.build()?;

    configure_linux_webview(&window)?;
    Ok(window)
}

#[cfg(target_os = "linux")]
fn linux_audio_sink_available() -> bool {
    std::process::Command::new("gst-inspect-1.0")
        .args(["--exists", "autoaudiosink"])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "linux")]
fn show_missing_audio_sink(app: AppHandle) {
    use gtk::prelude::*;

    eprintln!(
        "GStreamer autoaudiosink is unavailable; install gst-plugins-good before starting WhatsApp Linux"
    );

    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Close,
        "WhatsApp Linux cannot start safely because GStreamer's autoaudiosink plugin is missing.\n\nInstall it with:\n  sudo pacman -S --needed gst-plugins-good\n\nWithout this plugin, WebKitGTK crashes when WhatsApp plays message sounds.",
    );
    dialog.set_title("Missing media dependency");
    dialog.connect_response(move |dialog, _response| {
        dialog.close();
        app.exit(1);
    });
    dialog.show_all();
}

#[cfg(target_os = "linux")]
fn configure_linux_webview(window: &WebviewWindow) -> tauri::Result<()> {
    let app_for_navigation = window.app_handle().clone();
    window.with_webview(move |platform_webview| {
        use webkit2gtk::{
            glib::prelude::*, DeviceInfoPermissionRequest, NavigationPolicyDecision,
            NavigationPolicyDecisionExt, NotificationPermissionRequest, PermissionRequestExt,
            PolicyDecisionExt, PolicyDecisionType, SettingsExt, URIRequestExt,
            UserMediaPermissionRequest, UserMediaPermissionRequestExt, WebViewExt,
        };

        let webview = platform_webview.inner();
        if let Some(settings) = webview.settings() {
            settings.set_enable_media(true);
            settings.set_enable_media_stream(true);
            settings.set_enable_mediasource(true);
            settings.set_enable_webrtc(true);
            settings.set_enable_write_console_messages_to_stdout(cfg!(debug_assertions));
        }

        webview.connect_decide_policy(move |_view, decision, decision_type| {
            if decision_type != PolicyDecisionType::NavigationAction {
                return false;
            }
            let Some(policy) = decision.downcast_ref::<NavigationPolicyDecision>() else {
                return false;
            };
            let Some(action) = policy.navigation_action() else {
                return false;
            };
            let Some(request) = action.request() else {
                return false;
            };
            let Some(uri) = request.uri() else {
                return false;
            };
            let Ok(url) = url::Url::parse(uri.as_str()) else {
                decision.ignore();
                return true;
            };

            match navigation::decide(&url) {
                navigation::NavigationDecision::AllowInApp => decision.use_(),
                navigation::NavigationDecision::OpenExternal if action.is_user_gesture() => {
                    navigation::open_web_url(&app_for_navigation, &url);
                    decision.ignore();
                }
                navigation::NavigationDecision::OpenExternal
                | navigation::NavigationDecision::Deny => decision.ignore(),
            }
            true
        });

        webview.connect_permission_request(|view, request| {
            let trusted = view
                .uri()
                .and_then(|uri| url::Url::parse(uri.as_str()).ok())
                .is_some_and(|url| navigation::is_trusted_origin(&url));

            if !trusted {
                request.deny();
            } else if request.is::<NotificationPermissionRequest>() {
                request.allow();
            } else if let Some(media) = request.downcast_ref::<UserMediaPermissionRequest>() {
                let description = match (media.is_for_audio_device(), media.is_for_video_device()) {
                    (true, true) => "use your camera and microphone",
                    (true, false) => "use your microphone",
                    (false, true) => "use your camera",
                    (false, false) => "access media devices",
                };
                prompt_for_permission(request.clone(), description);
            } else if request.is::<DeviceInfoPermissionRequest>() {
                prompt_for_permission(request.clone(), "list camera and microphone devices");
            } else {
                request.deny();
            }

            true
        });
    })
}

#[cfg(target_os = "linux")]
fn prompt_for_permission(request: webkit2gtk::PermissionRequest, description: &'static str) {
    use gtk::prelude::*;
    use webkit2gtk::PermissionRequestExt;

    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Question,
        gtk::ButtonsType::YesNo,
        format!("Allow WhatsApp Web to {description}?").as_str(),
    );
    dialog.set_title("WhatsApp Linux permission request");
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Yes {
            request.allow();
        } else {
            request.deny();
        }
        dialog.close();
    });
    dialog.show_all();
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_webview(_window: &WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, MENU_TOGGLE, "Show/Hide WhatsApp", true, None::<&str>)?;
    let privacy = MenuItem::with_id(
        app,
        MENU_PRIVACY,
        "Toggle Privacy Blur (Ctrl+Shift+B)",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &privacy, &separator, &quit_item])?;

    let icon = to_tauri_image(badge::render(0).map_err(image_error)?);
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("WhatsApp Linux")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_TOGGLE => toggle_main_window(app),
            MENU_PRIVACY => privacy::toggle(app),
            MENU_QUIT => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn register_privacy_shortcut(app: &tauri::App) {
    if let Err(error) =
        app.global_shortcut()
            .on_shortcut("Ctrl+Shift+B", |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    privacy::toggle(app);
                }
            })
    {
        eprintln!("global privacy shortcut unavailable: {error}");
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_main_window(app);
        }
    }
}

fn quit<R: Runtime>(app: &AppHandle<R>) {
    app.state::<AppState>()
        .is_quitting
        .store(true, Ordering::SeqCst);
    app.exit(0);
}

fn update_unread<R: Runtime>(app: &AppHandle<R>, count: u32) {
    let state = app.state::<AppState>();
    if state.unread.swap(count, Ordering::SeqCst) == count {
        return;
    }

    let title = if count == 0 {
        "WhatsApp Linux".to_owned()
    } else {
        format!("WhatsApp Linux ({count} unread)")
    };
    let tooltip = if count == 0 {
        "WhatsApp Linux".to_owned()
    } else {
        format!("WhatsApp — {count} unread")
    };

    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.set_title(&title);
    }

    let Ok(rendered) = badge::render(count) else {
        return;
    };
    let window_icon = to_tauri_image(rendered.clone());
    let tray_icon = to_tauri_image(rendered);

    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.set_icon(window_icon);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_icon));
        let _ = tray.set_tooltip(Some(&tooltip));
        let _ = tray.set_title((count > 0).then_some(count.to_string()));
    }
}

fn to_tauri_image(image: badge::BadgeImage) -> tauri::image::Image<'static> {
    tauri::image::Image::new_owned(image.rgba, image.width, image.height)
}

fn image_error(error: image::ImageError) -> tauri::Error {
    tauri::Error::InvalidIcon(std::io::Error::other(error))
}

pub fn parse_unread_count(title: &str) -> u32 {
    let Some(after_open) = title.strip_prefix('(') else {
        return 0;
    };
    let Some((digits, remainder)) = after_open.split_once(')') else {
        return 0;
    };
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || !remainder.trim_start().starts_with("WhatsApp")
    {
        return 0;
    }

    digits.parse::<u32>().unwrap_or(MAX_UNREAD).min(MAX_UNREAD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whatsapp_unread_titles() {
        assert_eq!(parse_unread_count("(1) WhatsApp"), 1);
        assert_eq!(parse_unread_count("(42) WhatsApp"), 42);
        assert_eq!(parse_unread_count("(3)   WhatsApp Web"), 3);
    }

    #[test]
    fn resets_for_titles_without_a_valid_count() {
        for title in [
            "WhatsApp",
            "Chat name",
            "() WhatsApp",
            "(-1) WhatsApp",
            "(12 messages) WhatsApp",
            "(12) Not WhatsApp",
            "12) WhatsApp",
        ] {
            assert_eq!(parse_unread_count(title), 0, "{title}");
        }
    }

    #[test]
    fn safely_bounds_excessive_counts() {
        assert_eq!(parse_unread_count("(10000) WhatsApp"), MAX_UNREAD);
        assert_eq!(
            parse_unread_count("(999999999999999999999999999) WhatsApp"),
            MAX_UNREAD
        );
    }
}
