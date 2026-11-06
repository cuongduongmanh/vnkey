#!/bin/sh
# Build VnKey.app (universal: Apple Silicon + Intel). Chạy trên macOS.
set -e
cd "$(dirname "$0")/.."

rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
cargo build --release -p vnkey --target aarch64-apple-darwin
cargo build --release -p vnkey --target x86_64-apple-darwin

APP=target/VnKey.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp bundle/Info.plist "$APP/Contents/Info.plist"
lipo -create \
  target/aarch64-apple-darwin/release/vnkey \
  target/x86_64-apple-darwin/release/vnkey \
  -output "$APP/Contents/MacOS/vnkey"

# Ký ad-hoc để macOS nhớ quyền Accessibility giữa các lần chạy
codesign --force --deep --sign - "$APP"

du -sh "$APP"
echo "Xong: $APP  (copy vào /Applications rồi mở)"
