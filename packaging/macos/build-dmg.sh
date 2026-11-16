#!/usr/bin/env bash
# Builds 3D-Modeler-<version>.dmg holding a Universal (Apple Silicon + Intel)
# "3D Modeler.app". Run on a Mac. Procedure and context:
# docs/packaging-linux-mac.md ("Mac package from the USB").
#
# Usage (from the repository root):
#   packaging/macos/build-dmg.sh <version> <output-dir>
# Needs: Xcode Command Line Tools (clang, lipo, sips, iconutil, hdiutil,
# codesign) and rustup. Written on Windows and not yet run on a Mac: expect
# the first Mac session to fix it, one commit per fix.
set -euo pipefail

version=${1:?version, for example 0.1.2}
out_dir=${2:?output directory}
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"

targets=(aarch64-apple-darwin x86_64-apple-darwin)
rustup target add "${targets[@]}"
for target in "${targets[@]}"; do
    # Output lands in target/<triple>/release, inside target/ as AGENTS.md asks.
    cargo build --release --locked -p modeler3d-app --target "$target"
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
app="$work/3D Modeler.app"
contents="$app/Contents"
mkdir -p "$contents/MacOS" "$contents/Resources/assets"

lipo -create -output "$contents/MacOS/Modeler3D" \
    "target/aarch64-apple-darwin/release/modeler3d-app" \
    "target/x86_64-apple-darwin/release/modeler3d-app"
lipo -info "$contents/MacOS/Modeler3D"

# The app looks for assets/ beside its program (artwork::original_asset_root);
# the files live in Resources, as macOS expects, behind a relative link.
cp -R crates/modeler3d-app/assets/original "$contents/Resources/assets/original"
cp -R packaging/licenses "$contents/Resources/licenses"
ln -s ../Resources/assets "$contents/MacOS/assets"
ln -s ../Resources/licenses "$contents/MacOS/licenses"

iconset="$work/Modeler3D.iconset"
mkdir -p "$iconset"
png=crates/modeler3d-app/assets/original/Modeler3D.png
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$contents/Resources/Modeler3D.icns"

cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>3D Modeler</string>
    <key>CFBundleDisplayName</key><string>3D Modeler</string>
    <key>CFBundleIdentifier</key><string>org.modeler3d.Modeler3D</string>
    <key>CFBundleExecutable</key><string>Modeler3D</string>
    <key>CFBundleIconFile</key><string>Modeler3D</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Not signed with a developer ID (owner has not decided on the $99 account);
# an ad-hoc signature keeps Apple Silicon willing to run it.
codesign --force --deep --sign - "$app"
"$contents/MacOS/Modeler3D" --smoke

stage="$work/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
mkdir -p "$out_dir"
target_dmg="$out_dir/3D-Modeler-$version.dmg"
hdiutil create -volname "3D Modeler" -srcfolder "$stage" -ov -format UDZO "$target_dmg"
echo "built $target_dmg ($(du -h "$target_dmg" | cut -f1))"
