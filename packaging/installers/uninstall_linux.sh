#!/usr/bin/env sh
set -eu
install_root=${MODELER3D_INSTALL_ROOT:-"$HOME/.local/opt/modeler3d"}
expected_root="$HOME/.local/opt/modeler3d"
if [ "$install_root" != "$expected_root" ]; then
    printf '%s\n' 'Refusing to remove a path other than the private 3D Modeler install root.' >&2
    exit 2
fi
if [ -d "$install_root" ]; then
    rm -rf -- "$install_root"
fi
printf '3D Modeler removed from %s\n' "$install_root"
