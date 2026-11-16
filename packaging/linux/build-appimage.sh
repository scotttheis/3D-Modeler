#!/usr/bin/env bash
# Builds 3D-Modeler-<version>-x86_64.AppImage from a release build.
# Procedure and context: docs/packaging-linux-mac.md.
#
# Usage (from the repository root, on Linux or WSL):
#   packaging/linux/build-appimage.sh <version> <output-dir>
# Needs: a release build at target/release/modeler3d-app, and appimagetool on
# PATH or in $APPIMAGETOOL (downloaded by the session; not part of the repo).
set -euo pipefail

version=${1:?version, for example 0.1.2}
out_dir=${2:?output directory}
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
exe="$root/target/release/modeler3d-app"
tool=${APPIMAGETOOL:-appimagetool}

[ -x "$exe" ] || { echo "release build missing: $exe" >&2; exit 2; }
"$exe" --smoke

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
app="$work/3D-Modeler.AppDir"
bin="$app/usr/bin"
mkdir -p "$bin" "$app/usr/share/applications" "$app/usr/share/icons/hicolor/512x512/apps"

# Same layout as the Windows package: the program with assets/ and
# licenses/ beside it (artwork::original_asset_root looks there).
install -m 755 "$exe" "$bin/Modeler3D"
mkdir -p "$bin/assets"
cp -r "$root/crates/modeler3d-app/assets/original" "$bin/assets/original"
cp -r "$root/packaging/licenses" "$bin/licenses"

cat > "$app/AppRun" <<'EOF'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
exec "$here/usr/bin/Modeler3D" "$@"
EOF
chmod 755 "$app/AppRun"

desktop="$root/packaging/launchers/modeler3d.desktop"
cp "$desktop" "$app/modeler3d.desktop"
cp "$desktop" "$app/usr/share/applications/modeler3d.desktop"
icon="$root/crates/modeler3d-app/assets/original/Modeler3D.png"
cp "$icon" "$app/modeler3d.png"
cp "$icon" "$app/usr/share/icons/hicolor/512x512/apps/modeler3d.png"
ln -s modeler3d.png "$app/.DirIcon"

mkdir -p "$out_dir"
target="$out_dir/3D-Modeler-$version-x86_64.AppImage"
ARCH=x86_64 VERSION="$version" "$tool" --no-appstream "$app" "$target"
"$target" --appimage-extract-and-run --smoke
echo "built $target ($(du -h "$target" | cut -f1))"
