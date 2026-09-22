mod app;
mod badge;
mod navigation;
mod privacy;

fn main() {
    configure_linux_runtime();
    app::run();
}

#[cfg(target_os = "linux")]
fn configure_linux_runtime() {
    // linuxdeploy bundles WebKit's GStreamer libraries but not the codec plugins.
    // Point the Arch-only AppImage at the documented host plugin packages while
    // preserving an explicit user override.
    if std::env::var_os("GST_PLUGIN_PATH").is_none() {
        std::env::set_var("GST_PLUGIN_PATH", "/usr/lib/gstreamer-1.0");
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_runtime() {}
