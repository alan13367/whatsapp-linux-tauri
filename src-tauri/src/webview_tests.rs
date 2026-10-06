use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use gtk::prelude::*;
use webkit2gtk::{
    gio, glib, DownloadExt, NavigationPolicyDecision, NavigationPolicyDecisionExt,
    PolicyDecisionType, SettingsExt, URIRequestExt, WebContext, WebContextExt, WebView, WebViewExt,
};

use crate::navigation;
use crate::privacy::{apply_to_webview, install_local_shortcut};

fn wait_until(description: &str, mut ready: impl FnMut() -> bool) {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "WebKit regression check timed out: {description}"
        );
        context.iteration(false);
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn check(webview: &WebView, script: &str) {
    let script = format!("if (!({script})) {{ throw new Error('webview assertion failed'); }}");
    let result = Rc::new(RefCell::new(None));
    let result_for_callback = result.clone();
    webview.evaluate_javascript(
        &script,
        None,
        None,
        None::<&gio::Cancellable>,
        move |value| {
            *result_for_callback.borrow_mut() = Some(value.map(|_| ()));
        },
    );
    wait_until("JavaScript result", || result.borrow().is_some());
    result.borrow_mut().take().unwrap().expect(script.as_str());
}

fn load_document(webview: &WebView, origin: &str) {
    webview.load_html(
        "<!doctype html><html><head><style>body { filter: none !important; }</style></head><body>Privacy regression check</body></html>",
        Some(origin),
    );
    wait_until("document load", || !webview.is_loading());
}

#[test]
#[ignore = "requires a Linux graphical session; run with --ignored --test-threads=1"]
fn native_webview_regressions() {
    gtk::init().expect("a graphical session is required");
    let webview = WebView::with_context(&WebContext::new_ephemeral());
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("WhatsApp Linux regression check");
    window.set_default_size(320, 240);
    window.add(&webview);
    window.show_all();

    let activations = Rc::new(Cell::new(0));
    let activations_for_callback = activations.clone();
    install_local_shortcut(&window, move || {
        activations_for_callback.set(activations_for_callback.get() + 1);
    });
    let control = gtk::gdk::ModifierType::CONTROL_MASK;
    let shift = gtk::gdk::ModifierType::SHIFT_MASK;
    assert!(!gtk::accel_groups_activate(
        &window,
        *gtk::gdk::keys::constants::b,
        control
    ));
    assert!(!gtk::accel_groups_activate(
        &window,
        *gtk::gdk::keys::constants::b,
        shift
    ));
    assert!(gtk::accel_groups_activate(
        &window,
        *gtk::gdk::keys::constants::b,
        control | shift
    ));
    assert_eq!(activations.get(), 1);

    load_document(&webview, "https://web.whatsapp.com/");
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'none'",
    );
    apply_to_webview(&webview, true);
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'blur(10px)'",
    );
    check(&webview, "document.querySelectorAll('style').length === 1");

    // Check the first page script, before DOMContentLoaded or load callbacks.
    webview.load_html(
        "<!doctype html><html><body>Reloaded<script>window.initiallyBlurred = getComputedStyle(document.body).filter === 'blur(10px)';</script></body></html>",
        Some("https://web.whatsapp.com/"),
    );
    wait_until("privacy reload", || !webview.is_loading());
    check(&webview, "window.initiallyBlurred === true");
    apply_to_webview(&webview, false);
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'none'",
    );

    apply_to_webview(&webview, true);
    apply_to_webview(&webview, true);
    apply_to_webview(&webview, false);
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'none'",
    );

    apply_to_webview(&webview, true);
    load_document(&webview, "https://example.com/");
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'none'",
    );
    for origin in ["https://flows.whatsapp.net/", "https://webtp.whatsapp.net/"] {
        load_document(&webview, origin);
        check(
            &webview,
            "getComputedStyle(document.body).filter === 'blur(10px)'",
        );
    }
    load_document(&webview, "https://web.whatsapp.com/");
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'blur(10px)'",
    );
    assert!(gtk::accel_groups_activate(
        &window,
        *gtk::gdk::keys::constants::b,
        control | shift
    ));
    assert_eq!(activations.get(), 2);

    let downloads = Rc::new(Cell::new(0));
    let downloads_for_callback = downloads.clone();
    let completed = Rc::new(Cell::new(false));
    let completed_for_callback = completed.clone();
    let download_error = Rc::new(RefCell::new(None));
    let download_error_for_callback = download_error.clone();
    let suggested_name = Rc::new(RefCell::new(String::new()));
    let suggested_name_for_callback = suggested_name.clone();
    let directory = tempfile::tempdir().unwrap();
    let attachment = directory.path().join("attachment.txt");
    let destination = url::Url::from_file_path(&attachment).unwrap().to_string();
    webview
        .context()
        .unwrap()
        .connect_download_started(move |_, download| {
            downloads_for_callback.set(downloads_for_callback.get() + 1);
            let destination = destination.clone();
            let suggested_name = suggested_name_for_callback.clone();
            download.connect_decide_destination(move |download, filename| {
                *suggested_name.borrow_mut() = filename.to_owned();
                download.set_destination(&destination);
                true
            });
            let download_error = download_error_for_callback.clone();
            download.connect_failed(move |_, error| {
                *download_error.borrow_mut() = Some(error.to_string());
            });
            let completed = completed_for_callback.clone();
            download.connect_finished(move |_| completed.set(true));
        });
    let foreign_navigations = Rc::new(Cell::new(0));
    let foreign_navigations_for_callback = foreign_navigations.clone();
    webview.connect_decide_policy(move |_, decision, decision_type| {
        if decision_type == PolicyDecisionType::NavigationAction {
            if let Some(uri) = decision
                .downcast_ref::<NavigationPolicyDecision>()
                .and_then(|policy| policy.navigation_action())
                .and_then(|action| action.request())
                .and_then(|request| request.uri())
            {
                if uri.starts_with("https://example.com/") {
                    foreign_navigations_for_callback
                        .set(foreign_navigations_for_callback.get() + 1);
                }
            }
        }
        false
    });
    let created_windows = Rc::new(Cell::new(0));
    let created_windows_for_callback = created_windows.clone();
    webview.connect_create(move |_, _| {
        created_windows_for_callback.set(created_windows_for_callback.get() + 1);
        None
    });
    let external_urls = Rc::new(RefCell::new(Vec::new()));
    let external_urls_for_callback = external_urls.clone();
    navigation::configure_webview(&webview, move |url| {
        external_urls_for_callback.borrow_mut().push(url.clone());
    });
    check(&webview, "(() => { const link = document.createElement('a'); link.download = 'test.txt'; link.href = URL.createObjectURL(new Blob(['test attachment'], { type: 'text/plain' })); document.body.appendChild(link); link.click(); return true; })()");
    wait_until("blob download completion", || completed.get());
    assert_eq!(downloads.get(), 1);
    assert!(
        download_error.borrow().is_none(),
        "{:?}",
        download_error.borrow()
    );
    assert_eq!(suggested_name.borrow().as_str(), "test.txt");
    assert_eq!(
        std::fs::read_to_string(&attachment).unwrap(),
        "test attachment"
    );
    WebViewExt::settings(&webview)
        .unwrap()
        .set_javascript_can_open_windows_automatically(true);
    // WebKit treats native evaluate_javascript calls as user gestures. Page scripts
    // exercise background behavior without inheriting that native activation.
    webview.load_html(
        "<!doctype html><html><body><script>window.open('https://example.com/popup', '_blank'); location.href = 'https://example.com/background';</script></body></html>",
        Some("https://web.whatsapp.com/"),
    );
    wait_until("popup request", || created_windows.get() == 1);
    wait_until("foreign navigation policy", || {
        foreign_navigations.get() == 1
    });
    assert!(external_urls.borrow().is_empty());
    check(&webview, "location.origin === 'https://web.whatsapp.com'");
    check(
        &webview,
        "getComputedStyle(document.body).filter === 'blur(10px)'",
    );

    // Native evaluation provides the activation needed for a deliberate link click.
    check(
        &webview,
        "(() => { location.href = 'https://example.com/user-link'; return true; })()",
    );
    wait_until("external link", || external_urls.borrow().len() == 1);
    assert_eq!(
        external_urls.borrow()[0].as_str(),
        "https://example.com/user-link"
    );
    check(&webview, "location.origin === 'https://web.whatsapp.com'");
    check(
        &webview,
        "(() => { window.open('https://example.com/user-popup', '_blank'); return true; })()",
    );
    wait_until("external popup", || external_urls.borrow().len() == 2);
    assert_eq!(created_windows.get(), 2);
    assert_eq!(
        external_urls.borrow()[1].as_str(),
        "https://example.com/user-popup"
    );
    window.close();
}
