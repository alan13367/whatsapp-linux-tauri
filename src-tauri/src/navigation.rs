use tauri::{AppHandle, Runtime};
use tauri_plugin_opener::OpenerExt;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDecision {
    AllowInApp,
    OpenExternal,
    Deny,
}

const APP_NAVIGATION_HOSTS: &[&str] = &[
    "web.whatsapp.com",
    // WhatsApp loads these exact auxiliary documents in child frames.
    "flows.whatsapp.net",
    "webtp.whatsapp.net",
];

fn has_secure_origin(url: &Url, allowed_hosts: &[&str]) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| allowed_hosts.contains(&host))
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

pub fn is_trusted_origin(url: &Url) -> bool {
    has_secure_origin(url, &["web.whatsapp.com"])
}

pub fn is_allowed_app_navigation(url: &Url) -> bool {
    has_secure_origin(url, APP_NAVIGATION_HOSTS)
}

pub fn decide(url: &Url) -> NavigationDecision {
    if is_allowed_app_navigation(url) {
        NavigationDecision::AllowInApp
    } else if matches!(url.scheme(), "http" | "https") {
        NavigationDecision::OpenExternal
    } else {
        NavigationDecision::Deny
    }
}

fn is_trusted_blob(url: &Url) -> bool {
    url.scheme() == "blob" && Url::parse(url.path()).is_ok_and(|origin| is_trusted_origin(&origin))
}

pub fn open_web_url<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    if !matches!(url.scheme(), "http" | "https") {
        return;
    }

    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open external URL: {error}");
    }
}

#[cfg(target_os = "linux")]
pub fn configure_webview(webview: &webkit2gtk::WebView, open_external: impl Fn(&Url) + 'static) {
    use std::rc::Rc;

    use webkit2gtk::{
        glib::prelude::*, NavigationPolicyDecision, NavigationPolicyDecisionExt, PolicyDecisionExt,
        PolicyDecisionType, URIRequestExt, WebViewExt,
    };

    let open_external = Rc::new(open_external);
    let open_popup = open_external.clone();
    webview.connect_create(move |_, action| {
        if action.is_user_gesture() {
            if let Some(url) = action
                .request()
                .and_then(|request| request.uri())
                .and_then(|uri| Url::parse(uri.as_str()).ok())
                .filter(|url| matches!(url.scheme(), "http" | "https"))
            {
                open_popup(&url);
            }
        }
        None
    });

    webview.connect_decide_policy(move |view, decision, decision_type| {
        if !matches!(
            decision_type,
            PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction
        ) {
            return false;
        }
        let Some(policy) = decision.downcast_ref::<NavigationPolicyDecision>() else {
            decision.ignore();
            return true;
        };
        let Some(action) = policy.navigation_action() else {
            decision.ignore();
            return true;
        };
        let Some(request) = action.request() else {
            decision.ignore();
            return true;
        };
        let Some(uri) = request.uri() else {
            decision.ignore();
            return true;
        };
        let Ok(url) = Url::parse(uri.as_str()) else {
            decision.ignore();
            return true;
        };

        if decision_type == PolicyDecisionType::NewWindowAction {
            if action.is_user_gesture() && matches!(url.scheme(), "http" | "https") {
                open_external(&url);
            }
            decision.ignore();
            return true;
        }

        if is_trusted_blob(&url) {
            let trusted_page = view
                .uri()
                .and_then(|uri| Url::parse(uri.as_str()).ok())
                .is_some_and(|url| is_trusted_origin(&url));
            if trusted_page {
                // Decrypted attachments are local blobs. Download without navigating away.
                decision.download();
            } else {
                decision.ignore();
            }
            return true;
        }

        match decide(&url) {
            NavigationDecision::AllowInApp => decision.use_(),
            NavigationDecision::OpenExternal if action.is_user_gesture() => {
                open_external(&url);
                decision.ignore();
            }
            NavigationDecision::OpenExternal | NavigationDecision::Deny => decision.ignore(),
        }
        true
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(url: &str) -> Url {
        Url::parse(url).expect("test URL should parse")
    }

    #[test]
    fn allows_exact_whatsapp_origins_with_paths_and_queries() {
        for url in [
            "https://web.whatsapp.com/",
            "https://web.whatsapp.com/app",
            "https://web.whatsapp.com/?version=1#chat",
            "https://WEB.WHATSAPP.COM:443/",
            "https://flows.whatsapp.net/flows/cache_management/",
            "https://webtp.whatsapp.net/pdf-viewer/?locale=en_US",
        ] {
            assert_eq!(decide(&parsed(url)), NavigationDecision::AllowInApp);
        }
    }

    #[test]
    fn grants_sensitive_permissions_only_to_primary_origin() {
        assert!(is_trusted_origin(&parsed("https://web.whatsapp.com/")));
        assert!(!is_trusted_origin(&parsed(
            "https://flows.whatsapp.net/flows/cache_management/"
        )));
        assert!(!is_trusted_origin(&parsed(
            "https://webtp.whatsapp.net/pdf-viewer/"
        )));
    }

    #[test]
    fn sends_foreign_web_links_external() {
        for url in [
            "https://example.com/",
            "http://example.com/",
            "https://whatsapp.com/",
            "https://web.whatsapp.com.evil.test/",
            "https://sub.web.whatsapp.com/",
            "https://evilflows.whatsapp.net/",
            "https://webtp.whatsapp.net.evil.test/",
            "http://web.whatsapp.com/",
            "https://web.whatsapp.com:444/",
            "https://user@web.whatsapp.com/",
        ] {
            assert_eq!(decide(&parsed(url)), NavigationDecision::OpenExternal);
        }
    }

    #[test]
    fn rejects_non_web_schemes() {
        for url in [
            "file:///tmp/message",
            "javascript:alert(1)",
            "data:text/html,hello",
            "mailto:test@example.com",
            "whatsapp://send?text=hello",
        ] {
            assert_eq!(decide(&parsed(url)), NavigationDecision::Deny);
        }
    }

    #[test]
    fn malformed_urls_do_not_parse() {
        for url in ["not a url", "://missing", "https://[invalid"] {
            assert!(Url::parse(url).is_err());
        }
    }

    #[test]
    fn only_primary_origin_blobs_are_download_candidates() {
        assert!(is_trusted_blob(&parsed(
            "blob:https://web.whatsapp.com/attachment"
        )));
        for url in [
            "blob:https://web.whatsapp.com.evil.test/attachment",
            "blob:https://web.whatsapp.com:444/attachment",
            "blob:https://user@web.whatsapp.com/attachment",
            "blob:https://flows.whatsapp.net/attachment",
            "blob:http://web.whatsapp.com/attachment",
            "blob:null/attachment",
            "data:text/plain,attachment",
        ] {
            assert!(!is_trusted_blob(&parsed(url)), "{url}");
            assert_eq!(decide(&parsed(url)), NavigationDecision::Deny);
        }
    }
}
