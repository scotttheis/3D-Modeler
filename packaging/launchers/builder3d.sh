#!/usr/bin/env sh
set -eu
app_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
app_exe="$app_dir/Builder3D"
if [ ! -x "$app_exe" ]; then
    printf '%s\n' 'Builder3D was not found or is not executable beside this launcher.' >&2
    exit 2
fi
exec "$app_exe" "$@"
