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
export MODELER3D_RELEASE_VERSION="$version"

targets=(aarch64-apple-darwin x86_64-apple-darwin)
rustup target add "${targets[@]}"
for target in "${targets[@]}"; do
    # Output lands in target/<triple>/release, inside target/ as AGENTS.md asks.
    if [[ "$target" == x86_64-apple-darwin ]]; then
        # manifold-csg-sys drives CMake directly and does not pass Cargo's
        # target architecture to it. On Apple Silicon that can leave an ARM64
        # native archive under the Intel target directory. Reconfigure only
        # when the cached archive has the wrong architecture.
        for build in target/"$target"/release/build/manifold-csg-sys-*/out/build; do
            [[ -d "$build" ]] || continue
            wrong_architecture=0
            for archive in \
                "$build/src/libmanifold.a" \
                "$build/bindings/c/libmanifoldc.a" \
                "$build/_deps/clipper2-build/libClipper2.a" \
                "$build"/*release/libtbb.a \
                "$build"/*release/libtbbmalloc.a; do
                [[ -f "$archive" ]] || continue
                if ! lipo -verify_arch x86_64 "$archive" >/dev/null 2>&1; then
                    wrong_architecture=1
                    break
                fi
            done
            if ((wrong_architecture)); then
                rm -rf "${build%/out/build}"
            fi
        done
        CXX="$root/packaging/macos/clang-x86_64.sh" \
            cargo build --release --locked -p modeler3d-app -p modeler3d-mcp --target "$target"
    else
        cargo build --release --locked -p modeler3d-app -p modeler3d-mcp --target "$target"
    fi
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
lipo -create -output "$contents/MacOS/modeler3d-mcp" \
    "target/aarch64-apple-darwin/release/modeler3d-mcp" \
    "target/x86_64-apple-darwin/release/modeler3d-mcp"

# The app looks for assets/ beside its program (artwork::original_asset_root);
# the files live in Resources, as macOS expects, behind a relative link.
cp -R crates/modeler3d-app/assets/original "$contents/Resources/assets/original"
# Release package icons are 128 px; the full-size art stays in the source tree.
"$contents/MacOS/Modeler3D" --shrink-assets "$contents/Resources/assets/original"
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
if [ "${MODELER3D_PUBLIC_RELEASE:-0}" = 1 ]; then "$contents/MacOS/Modeler3D" --require-update-feed; fi

stage="$work/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
mkdir -p "$out_dir"
target_dmg="$out_dir/3D-Modeler-$version-macos-universal.dmg"
hdiutil create -volname "3D Modeler" -srcfolder "$stage" -ov -format UDZO "$target_dmg"
echo "built $target_dmg ($(du -h "$target_dmg" | cut -f1))"
