#!/usr/bin/env sh
set -eu
install_root=${MODELER3D_INSTALL_ROOT:-"$HOME/Applications/Modeler3D"}
expected_root="$HOME/Applications/Modeler3D"
if [ "$install_root" != "$expected_root" ]; then
    printf '%s\n' 'Refusing to remove a path other than the private 3D Modeler install root.' >&2
    exit 2
fi
if [ -d "$install_root" ]; then
    rm -rf -- "$install_root"
fi
printf '3D Modeler removed from %s\n' "$install_root"
