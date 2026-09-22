# Security Policy

## Trust boundary

WhatsApp Linux loads `https://web.whatsapp.com/` in the system WebKitGTK webview. WhatsApp Web is remote, changeable content and must be treated as untrusted by the native wrapper.

The remote page receives no Tauri capabilities, custom commands, filesystem API, shell API, or global `window.__TAURI__` object. Native code restricts navigation to the primary WhatsApp Web origin plus exact WhatsApp-owned Flows and PDF-viewer origins needed by child frames. Only user-initiated foreign HTTP(S) navigation is sent to the system browser; background foreign navigation and other schemes are denied.

The only page modifications are fixed, embedded privacy-blur scripts. No page-derived value is interpolated into executable JavaScript. Notification requests are accepted only while the top-level page is the trusted WhatsApp origin. Camera, microphone, and device-enumeration requests additionally require confirmation in a native dialog; unrelated WebKit permission classes are denied.

The security of message content and end-to-end encryption remains the responsibility of WhatsApp Web. The wrapper also depends on the security of Tauri, WebKitGTK, GStreamer, and the host operating system.

## Supported versions

Only the latest release on a current, fully updated Arch Linux x86_64 system is supported. Builds made on rolling Arch are not guaranteed to run on older Arch snapshots or other distributions.

## Reporting a vulnerability

Do not open a public issue for an unpatched vulnerability. Use the repository's private GitHub Security Advisory form:

<https://github.com/alan13367/whatsapp-linux-tauri/security/advisories/new>

Include reproduction steps, affected version/commit, expected impact, and a proof of concept when possible. Vulnerabilities in WhatsApp Web itself should be reported to Meta through its official vulnerability reporting program.
