# Arch compatibility results

Test environment: current Arch Linux x86_64, KDE Plasma Wayland, WebKitGTK 4.1.

## Verified

- The release AppImage starts normally through FUSE from outside the repository.
- The default WebKit user agent is accepted; no Chromium user-agent override is required.
- The WhatsApp QR code renders and account linking completes.
- The linked account survives application termination and restart.
- Text messages send and receive after allowing WhatsApp's exact auxiliary frame origins.
- Sending a message no longer freezes the UI when GStreamer's `autoaudiosink` is available; the apparent freeze was a WebKit web-process abort while creating the message-sound audio sink.
- WhatsApp's internal Flows cache document and PDF viewer stay inside the webview instead of opening unsolicited browser tabs.
- A second launch focuses the existing process instead of creating another instance.
- The KDE tray icon is visible.
- The generated AppImage checksum verifies successfully.

## Environment-specific results

- The initial KDE Wayland build did not receive its X11 global shortcut. Wayland now uses a native window shortcut for `Ctrl+Shift+B`; use the tray privacy action while the window is hidden.
- The Arch runtime must provide `gst-plugins-good`. The application now checks for its `autoaudiosink` element before creating the webview and shows an actionable error instead of allowing WebKitGTK to crash later.

## Automated regression checks, October 6, 2026

The native WebKit check uses local HTML, an ephemeral browser profile, and a temporary attachment directory. It passed on the same Arch Wayland environment and verified:

- The window privacy shortcut activates once, rejects incomplete modifier combinations, and survives navigation.
- Privacy blur overrides page CSS, applies immediately, and remains active before the first script on a reloaded page runs.
- Toggling privacy repeatedly removes the blur, and the stylesheet covers the allowed auxiliary origins while excluding foreign documents.
- A local WhatsApp attachment blob downloads with its original filename and contents. The previous navigation filter blocked this test before a download could start.
- Background foreign navigation and popup requests stay blocked, while deliberate external links and popups reach the external opener once.

The release badge benchmark rendered 500 icons in 1.85 seconds with repeated PNG decoding and resizing, versus 0.59 milliseconds with the cached base image. This measures icon generation only. Counts above 99 also skip unchanged native icons, avoiding repeated tray PNG writes while keeping the exact unread title and tray label current.

The native check and benchmark commands are documented in the README. These checks do not exercise a linked account.

## Still requiring interactive verification

- Notifications while visible and hidden, including click-to-focus behavior.
- Unread badge increment and reset against incoming messages.
- Tray show/hide and Quit actions.
- Privacy blur through the tray and persistence across page reload.
- Attachment picker, drag/drop, clipboard image/text paste, and downloads.
- Audio/video and voice-note playback after installing all documented GStreamer plugin groups.
- Camera/microphone consent prompts and the installed WebKitGTK build's actual WebRTC behavior.
- Offline recovery and reconnect.
- An X11 desktop session.

These remaining items require user interaction, incoming messages, local files/devices, or another desktop session. They must be completed before calling the build a fully qualified release.
