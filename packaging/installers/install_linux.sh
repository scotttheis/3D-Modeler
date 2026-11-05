#!/usr/bin/env sh
set -eu
source_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
install_root=${BUILDER3D_INSTALL_ROOT:-"$HOME/.local/opt/builder3d"}
case "$install_root" in
    ''|/|/usr|/usr/bin|/opt) printf '%s\n' 'Refusing an unsafe install root.' >&2; exit 2 ;;
esac
if [ ! -x "$source_root/Builder3D" ] && [ ! -f "$source_root/Builder3D.exe" ]; then
    printf '%s\n' 'The native Builder3D executable is missing from the package root.' >&2
    exit 2
fi
mkdir -p -- "$install_root"
cp -R -- "$source_root/." "$install_root/"
chmod +x "$install_root/Builder3D" "$install_root"/*.sh "$install_root"/*.command 2>/dev/null || true
printf 'Builder3D installed to %s\n' "$install_root"
