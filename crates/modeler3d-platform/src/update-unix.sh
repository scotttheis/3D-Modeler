#!/bin/sh
# All paths arrive as arguments. Never evaluate release text as shell code.
set -eu
task_pid=$1
installer=$2
executable=$3
work=$4
result="$(dirname "$work")/last-result.txt"
backup=
destination=
mounted=
finish() {
    code=$?
    trap - EXIT
    if [ -n "$mounted" ]; then hdiutil detach "$mounted" -quiet || true; fi
    if [ "$code" -ne 0 ]; then
        if [ -n "$backup" ] && [ -e "$backup" ] && [ ! -e "$destination" ]; then mv "$backup" "$destination"; fi
        printf '%s\n' 'Update failed. The previous installation was kept. See installer.log in the update cache.' > "$result"
    else
        printf '%s\n' '3D Modeler was updated successfully.' > "$result"
    fi
    if [ "$(uname -s)" = Darwin ]; then open "$destination" || true; else "$executable" >/dev/null 2>&1 & fi
    exit "$code"
}
exec >"$work/installer.log" 2>&1
# A timeout never changes installed files.
attempt=0
while kill -0 "$task_pid" 2>/dev/null; do
    attempt=$((attempt + 1))
    if [ "$attempt" -gt 120 ]; then printf '%s\n' 'Update cancelled: application did not close.' > "$result"; exit 1; fi
    sleep 1
done
case "$(uname -s)" in
    Darwin)
        destination=${executable%/Contents/MacOS/*}
        case "$destination" in *.app) ;; *) exit 1 ;; esac
        trap finish EXIT
        mounted="$work/mount"
        mkdir "$mounted"
        hdiutil attach "$installer" -readonly -nobrowse -mountpoint "$mounted" -quiet
        source_app="$mounted/3D Modeler.app"
        test -x "$source_app/Contents/MacOS/Modeler3D"
        codesign --verify --deep "$source_app"
        replacement="${destination}.update-$$"
        ditto "$source_app" "$replacement"
        backup="${destination}.previous-$$"
        mv "$destination" "$backup"
        mv "$replacement" "$destination"
        ;;
    Linux)
        destination=$(dirname "$executable")
        test "$(basename "$executable")" = Modeler3D
        test -d "$destination/assets/original"
        trap finish EXIT
        mkdir "$work/unpacked"
        tar -xzf "$installer" -C "$work/unpacked"
        archive_name=$(basename "$installer" .tar.gz)
        source_dir="$work/unpacked/$archive_name"
        test -x "$source_dir/Modeler3D"
        test -d "$source_dir/assets/original"
        replacement="${destination}.update-$$"
        cp -R "$source_dir" "$replacement"
        backup="${destination}.previous-$$"
        mv "$destination" "$backup"
        mv "$replacement" "$destination"
        ;;
    *) exit 1 ;;
esac
