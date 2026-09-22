# WhatsApp Linux

An unofficial, lightweight WhatsApp Web desktop wrapper for current Arch Linux systems. It uses Tauri 2 and the system WebKitGTK engine instead of bundling Chromium.

> This project is not affiliated with, authorized by, endorsed by, or otherwise connected to WhatsApp LLC or Meta Platforms, Inc. WhatsApp is a trademark of its respective owner.

## Features

- Persistent WhatsApp linked-device session
- System tray with show/hide and explicit quit actions
- Close-to-tray behavior and single-instance activation
- Native WebKit notifications
- Unread count in the window title and dynamically rendered tray/window badge
- Privacy blur from the tray or `Ctrl+Shift+B`
- Attachment upload, drag/drop, clipboard, and downloads
- Validated external links opened in the default browser
- Camera and microphone permission handling restricted to WhatsApp Web with explicit native consent prompts
- No Node.js runtime, frontend framework, or remote Tauri API capabilities

## Support scope

The initial release supports **current Arch Linux/Manjaro x86_64 only**. The AppImage is built locally on rolling Arch and may require a similarly current glibc. It is not promised to run on Ubuntu, Debian, older Arch snapshots, or other architectures.

Tauri uses WebKitGTK rather than Chromium. WhatsApp can change browser requirements at any time. Verify the compatibility checklist below before treating a build as ready for daily use.

## Arch prerequisites

Install Rust through `rustup`, then install the native build/runtime packages:

```bash
sudo pacman -Syu --needed \
  base-devel webkit2gtk-4.1 openssl curl wget file \
  libayatana-appindicator librsvg xdotool \
  gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad \
  gst-plugins-ugly gst-libav libnotify fuse2

cargo install tauri-cli --version 2.11.5 --locked
```

A running desktop notification daemon is required. GNOME users normally need `gnome-shell-extension-appindicator` for tray icons. Other desktops must provide a StatusNotifier/AppIndicator host.

## Development

There is no npm or frontend build step.

```bash
cargo tauri dev
```

The first launch should show WhatsApp's QR code. Linking state is kept in WebKit's persistent application data directory associated with `io.github.alan13367.whatsapp-linux-tauri`. Replacing or rebuilding the AppImage does not intentionally clear this profile.

### Compatibility gate

Before relying on the application, verify on the target Arch machine:

1. The QR code renders and an account can be linked.
2. Text messages send and receive, including after network reconnect.
3. The linked session survives a complete restart and rebuild.
4. Audio/video, voice notes, attachment upload, and downloads work.
5. Camera and microphone requests are limited to WhatsApp and work if the installed WebKitGTK build supports WebRTC.
6. Notifications appear while the window is visible and hidden.
7. Close hides the window, the tray restores/quits it, and a second launch focuses it.
8. The privacy shortcut/tray action survives a page reload, and unread badges increment and reset.
9. Foreign links and popups open in the browser, while non-web schemes remain blocked.
10. Camera/microphone prompts are denied after any attempted navigation away from the trusted origin.

If WhatsApp reports an unsupported browser, first update the whole system. To test the centralized Chromium-style user-agent fallback, launch with `--chromium-user-agent` (for example, `./WhatsApp-Linux.AppImage --chromium-user-agent`). This may bypass simple browser detection, but it cannot provide missing Chromium APIs; retest every item above. If ordinary login or messaging remains unreliable, a pure Tauri/WebKitGTK architecture is not viable.

## Local AppImage build

```bash
./scripts/build-appimage.sh
```

The script runs formatting, Clippy, tests, and the AppImage build. It copies the artifact and a portable SHA-256 checksum to `dist/`. It disables linuxdeploy's second stripping pass because that tool's bundled binutils cannot read Arch's RELR ELF sections; Cargo still strips the release binary. There is no CI, updater, signing, or automated publishing.

Run the result with:

```bash
chmod +x dist/*.AppImage
./dist/*.AppImage
```

If FUSE mounting is unavailable:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./dist/*.AppImage
```

## Desktop behavior

- Closing the window hides it when the tray was created; if tray setup fails, closing exits normally.
- If a desktop hides an otherwise-created tray icon, launching the AppImage again restores the single running window; terminate it from the system process manager if needed.
- Left-clicking the tray icon toggles the main window where the tray host supports click events.
- A second launch shows and focuses the existing process.
- `Ctrl+Shift+B` toggles a 10px privacy blur and shows the window if hidden.
- Global shortcuts may be blocked by some Wayland compositors. The tray privacy action remains available.
- Linux tray tooltips and dynamic window icons are not honored by every desktop shell.

## Downloads and media

Downloads use WebKitGTK's normal destination handling. WhatsApp remote content receives no generic filesystem command or shell access. File inputs, HTML drag/drop, and clipboard access remain handled by the webview.

The AppImage intentionally does not bundle Tauri's GStreamer media framework because that option is documented as fully supported only on Ubuntu build systems. The Arch GStreamer packages listed above are runtime dependencies.

WebKitGTK camera/microphone permissions are considered only for the exact `https://web.whatsapp.com` top-level origin and require confirmation in a native dialog. Notifications are allowed for that origin without an extra wrapper prompt. Clipboard access remains subject to WebKit's normal page and user-gesture rules and is enabled to support copy/paste attachments. Actual calls or recording can still fail if Arch's WebKitGTK package lacks working WebRTC support, particularly under some Wayland setups.

## Troubleshooting

- **Unsupported browser:** fully update Arch and WebKitGTK; browser spoofing cannot fix missing web APIs.
- **Missing media dependency at startup:** install `gst-plugins-good` and restart the app. Its `autoaudiosink` element is required because WebKitGTK 2.52 aborts its web process when WhatsApp plays a message sound without an available automatic audio sink.
- **No sound/video:** install all listed GStreamer plugin groups and restart the app.
- **No notifications:** ensure a notification daemon is running and notifications are enabled for the application.
- **No tray icon:** install/enable an AppIndicator or StatusNotifier host for the desktop environment.
- **Shortcut unavailable:** use the tray privacy item; restrictive Wayland sessions may reject global shortcut registration.
- **AppImage will not mount:** install `fuse2` or use `APPIMAGE_EXTRACT_AND_RUN=1`.
- **Reset login state:** quit the application, back up anything needed, then remove `${XDG_DATA_HOME:-$HOME/.local/share}/io.github.alan13367.whatsapp-linux-tauri`. This permanently logs out the wrapper.

## Security

Only the primary WhatsApp Web HTTPS origin and the exact auxiliary WhatsApp Flows/PDF origins required by its child frames may remain in the application webview. Sensitive permissions remain restricted to the primary origin. User-initiated foreign HTTP(S) links open externally; background foreign navigations and non-web schemes are denied. Remote content has no Tauri capabilities or custom native commands. See [SECURITY.md](SECURITY.md) for the trust boundary and private reporting process, and [docs/compatibility.md](docs/compatibility.md) for current Arch smoke-test results.


## License

Copyright (c) 2026 Alan Beltran Pozo. This repository is licensed under the [MIT License](LICENSE).
