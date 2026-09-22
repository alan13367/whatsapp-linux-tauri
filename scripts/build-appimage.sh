#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$ROOT_DIR/src-tauri/Cargo.toml"
TARGET_DIR="$ROOT_DIR/src-tauri/target"
BUNDLE_DIR="$TARGET_DIR/release/bundle/appimage"
DIST_DIR="$ROOT_DIR/dist"
EXPECTED_TAURI_CLI="2.11.5"

cd "$ROOT_DIR"

if ! tauri_version="$(cargo tauri --version 2>/dev/null)"; then
  echo "cargo-tauri is required. Install it with:" >&2
  echo "  cargo install tauri-cli --version $EXPECTED_TAURI_CLI --locked" >&2
  exit 1
fi
if [[ "$tauri_version" != *"$EXPECTED_TAURI_CLI"* ]]; then
  echo "Expected cargo-tauri $EXPECTED_TAURI_CLI, found: $tauri_version" >&2
  exit 1
fi

if ! pkg-config --exists ayatana-appindicator3-0.1 \
  && ! pkg-config --exists appindicator3-0.1; then
  echo "An AppIndicator development library is required." >&2
  echo "Install it with: sudo pacman -S --needed libayatana-appindicator" >&2
  exit 1
fi

if ! command -v gst-inspect-1.0 >/dev/null \
  || ! gst-inspect-1.0 --exists autoaudiosink; then
  echo "GStreamer's autoaudiosink plugin is required." >&2
  echo "Install it with: sudo pacman -S --needed gst-plugins-good" >&2
  exit 1
fi

export CARGO_TARGET_DIR="$TARGET_DIR"
# linuxdeploy's bundled strip is older than Arch's RELR ELF format. The release
# binary is already stripped by Cargo, so skip linuxdeploy's incompatible pass.
export NO_STRIP=1

cargo fmt --manifest-path "$MANIFEST" --check
cargo clippy --manifest-path "$MANIFEST" --all-targets --all-features -- -D warnings
cargo test --manifest-path "$MANIFEST"
rm -rf "$BUNDLE_DIR"
cargo tauri build --bundles appimage

artifact="$(find "$BUNDLE_DIR" -maxdepth 1 -type f -name '*.AppImage' -print -quit)"
if [[ -z "$artifact" ]]; then
  echo "No AppImage found in $BUNDLE_DIR" >&2
  exit 1
fi

mkdir -p "$DIST_DIR"
destination="$DIST_DIR/$(basename "$artifact")"
cp "$artifact" "$destination"
chmod +x "$destination"
(
  cd "$DIST_DIR"
  sha256sum "$(basename "$destination")" > "$(basename "$destination").sha256"
)

printf 'AppImage: %s\n' "$destination"
printf 'Checksum: %s\n' "$destination.sha256"
du -h "$destination"
