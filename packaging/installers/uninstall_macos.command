#!/usr/bin/env sh
set -eu
install_root=${BUILDER3D_INSTALL_ROOT:-"$HOME/Applications/Builder3D"}
expected_root="$HOME/Applications/Builder3D"
if [ "$install_root" != "$expected_root" ]; then
    printf '%s\n' 'Refusing to remove a path other than the private Builder3D install root.' >&2
    exit 2
fi
if [ -d "$install_root" ]; then
    rm -rf -- "$install_root"
fi
printf 'Builder3D removed from %s\n' "$install_root"
