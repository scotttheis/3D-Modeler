#!/usr/bin/env bash
# Builds 3D-Modeler-<version>-linux-x86_64.tar.gz from a release build.
# No downloaded tools needed (the AppImage needs appimagetool; see
# build-appimage.sh). Procedure and context: docs/packaging-linux-mac.md.
#
# Usage (from the repository root, on Linux or WSL):
#   packaging/linux/build-tarball.sh <version> <output-dir>
set -euo pipefail

version=${1:?version, for example 0.1.2}
out_dir=${2:?output directory}
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
exe="$root/target/release/modeler3d-app"
cd "$root"
MODELER3D_RELEASE_VERSION="$version" cargo build --release --locked -p modeler3d-app
if [ "${MODELER3D_PUBLIC_RELEASE:-0}" = 1 ]; then "$exe" --require-update-feed; fi

[ -x "$exe" ] || { echo "release build missing: $exe" >&2; exit 2; }
"$exe" --smoke

name="3D-Modeler-$version-linux-x86_64"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
pkg="$work/$name"
mkdir -p "$pkg/assets" "$pkg/installers"

# Same layout as the Windows package: the program with assets/ and
# licenses/ beside it (artwork::original_asset_root looks there).
install -m 755 "$exe" "$pkg/Modeler3D"
cp -r "$root/crates/modeler3d-app/assets/original" "$pkg/assets/original"
# Release package icons are 128 px; the full-size art stays in the source tree.
"$pkg/Modeler3D" --shrink-assets "$pkg/assets/original"
cp -r "$root/packaging/licenses" "$pkg/licenses"
install -m 755 "$root/packaging/launchers/modeler3d.sh" "$pkg/modeler3d.sh"
cp "$root/packaging/launchers/modeler3d.desktop" "$pkg/modeler3d.desktop"
cp "$root/crates/modeler3d-app/assets/original/Modeler3D.png" "$pkg/modeler3d.png"
install -m 755 "$root/packaging/installers/install_linux.sh" "$pkg/installers/"
install -m 755 "$root/packaging/installers/uninstall_linux.sh" "$pkg/installers/"
cat > "$pkg/README.txt" <<TXT
3D Modeler $version for Linux (x86_64), test build.

Run it:      double-click Modeler3D, or in a terminal:  ./Modeler3D
Install it:  ./installers/install_linux.sh   (copies to ~/.local/opt/modeler3d)
Remove it:   ./installers/uninstall_linux.sh
Settings:    ~/.config/Modeler3D   Logs: ~/.local/state/Modeler3D

Needs a 2024-or-newer Linux (glibc 2.39: Ubuntu 24.04, Fedora 40, Debian 13
or later) with X11 or Wayland. Licence notices are in licenses/.
TXT

"$pkg/Modeler3D" --smoke
mkdir -p "$out_dir"
target="$out_dir/$name.tar.gz"
tar -C "$work" -czf "$target" "$name"
echo "built $target ($(du -h "$target" | cut -f1))"
