#!/usr/bin/env bash
set -euo pipefail

binary="${1:?Provide the path to the Cargo binary}"
dist="${2:?Provide an output directory}"
version="${3:?Provide a release version}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$dist"
dist="$(cd "$dist" && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
app="$scratch/bundle/termian.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
install -m 755 "$binary" "$app/Contents/MacOS/termian"
cp "$root/assets/app/icon.icns" "$app/Contents/Resources/"
cp "$root/assets/app/Info.plist" "$app/Contents/Info.plist"
plutil -insert CFBundleShortVersionString -string "$version" "$app/Contents/Info.plist"
plutil -insert CFBundleVersion -string "$version" "$app/Contents/Info.plist"
plutil -insert LSMinimumSystemVersion -string "${MACOSX_DEPLOYMENT_TARGET:-11.0}" "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist"

identity="${APPLE_SIGNING_IDENTITY:--}"
if [[ "$identity" != "-" ]]; then
  for name in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
    [[ -n "${!name:-}" ]] || { echo "Missing $name for notarization"; exit 1; }
  done
  sign=(--options runtime --timestamp)
  if [[ -n "${APPLE_KEYCHAIN:-}" ]]; then sign+=(--keychain "$APPLE_KEYCHAIN"); fi
  codesign --force --sign "$identity" "${sign[@]}" "$app"
  ditto -c -k --keepParent "$app" "$scratch/termian.zip"
  xcrun notarytool submit "$scratch/termian.zip" --wait --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID"
  xcrun stapler staple "$app"
  spctl --assess --type execute --verbose "$app"
else
  codesign --force --sign - "$app"
fi
codesign --verify --deep --strict --verbose=2 "$app"

base="termian-$version-mac-arm64"
image="$scratch/$base.dmg"
bundle_kib="$(du -Ask "$scratch/bundle" | cut -f1)"
image_kib="$((bundle_kib + 32768))"
for attempt in 1 2 3; do
  if hdiutil create -ov -volname Termian -srcfolder "$scratch/bundle" -size "${image_kib}k" -format UDZO "$image" >"$scratch/dmg.log" 2>&1; then break; fi
  cat "$scratch/dmg.log"
  if [[ "$attempt" == "3" ]] || ! grep -qi 'resource busy' "$scratch/dmg.log"; then exit 1; fi
  sleep "$((attempt * 5))"
done
if [[ "$identity" != "-" ]]; then
  codesign --force --sign "$identity" "${sign[@]}" "$image"
  xcrun notarytool submit "$image" --wait --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID"
  xcrun stapler staple "$image"
fi
hdiutil verify "$image"
COPYFILE_DISABLE=1 tar -czf "$scratch/$base.app.tar.gz" -C "$scratch/bundle" termian.app
mv "$image" "$scratch/$base.app.tar.gz" "$dist/"
