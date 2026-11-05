#!/usr/bin/env sh
set -eu
install_root=${BUILDER3D_INSTALL_ROOT:-"$HOME/.local/opt/builder3d"}
expected_root="$HOME/.local/opt/builder3d"
if [ "$install_root" != "$expected_root" ]; then
    printf '%s\n' 'Refusing to remove a path other than the private Builder3D install root.' >&2
    exit 2
fi
if [ -d "$install_root" ]; then
    rm -rf -- "$install_root"
fi
printf 'Builder3D removed from %s\n' "$install_root"
