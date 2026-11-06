#!/bin/sh
# Tạo VnKey-<version>.dmg (kéo VnKey vào Applications). Chạy trên macOS.
set -e
cd "$(dirname "$0")/.."

./bundle/build-app.sh

VERSION=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" bundle/Info.plist)
DMG="target/VnKey-$VERSION.dmg"
STAGING=target/dmg-staging

rm -rf "$STAGING" "$DMG"
mkdir -p "$STAGING"
cp -R target/VnKey.app "$STAGING/"
ln -s /Applications "$STAGING/Applications"

hdiutil create \
  -volname "VnKey $VERSION" \
  -srcfolder "$STAGING" \
  -fs HFS+ \
  -format UDZO \
  -imagekey zlib-level=9 \
  -ov "$DMG"

rm -rf "$STAGING"
du -h "$DMG"
echo "Xong: $DMG"
