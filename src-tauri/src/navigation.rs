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

pub fn open_web_url<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    if !matches!(url.scheme(), "http" | "https") {
        return;
    }

    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("failed to open external URL: {error}");
    }
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
}
