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

- KDE Wayland blocked the registered `Ctrl+Shift+B` global shortcut in this session. The tray privacy action remains the supported fallback, as documented.
- The Arch runtime must provide `gst-plugins-good`. The application now checks for its `autoaudiosink` element before creating the webview and shows an actionable error instead of allowing WebKitGTK to crash later.

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
