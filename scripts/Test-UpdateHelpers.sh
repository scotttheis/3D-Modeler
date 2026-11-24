#!/bin/sh
# Linux integration checks: real tarballs, installed folders and rollback;
# no network, release feed, actual application, or user installation.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture=$(mktemp -d /tmp/modeler3d-update-test.XXXXXXXX)
trap 'rm -rf "$fixture"' EXIT
helper="$root/crates/modeler3d-platform/src/update-unix.sh"
name=3D-Modeler-9.9.9-linux-x86_64
mkdir -p "$fixture/source/$name/assets/original"
printf '#!/bin/sh\n# new version\nexit 0\n' > "$fixture/source/$name/Modeler3D"
chmod +x "$fixture/source/$name/Modeler3D"
for scenario in success missing-executable rollback; do
    case_dir="$fixture/$scenario"
    install="$case_dir/old install with spaces"
    work="$case_dir/cache/session"
    mkdir -p "$install/assets/original" "$work"
    printf '#!/bin/sh\n# old version\nexit 0\n' > "$install/Modeler3D"
    chmod +x "$install/Modeler3D"
    cp "$install/Modeler3D" "$case_dir/original"
    tar -czf "$work/$name.tar.gz" -C "$fixture/source" "$name"
    if [ "$scenario" = missing-executable ]; then
        mkdir -p "$case_dir/broken/$name"
        tar -czf "$work/$name.tar.gz" -C "$case_dir/broken" "$name"
    fi
    if [ "$scenario" = rollback ]; then
        mkdir "$case_dir/bin"
        cat > "$case_dir/bin/mv" <<'SH'
#!/bin/sh
case "$1" in *.update-*) exit 1 ;; esac
exec /bin/mv "$@"
SH
        chmod +x "$case_dir/bin/mv"
        if PATH="$case_dir/bin:$PATH" sh "$helper" 999999 "$work/$name.tar.gz" "$install/Modeler3D" "$work"; then
            echo 'rollback scenario unexpectedly succeeded' >&2; exit 1
        fi
    elif [ "$scenario" = missing-executable ]; then
        if sh "$helper" 999999 "$work/$name.tar.gz" "$install/Modeler3D" "$work"; then
            echo 'invalid package unexpectedly succeeded' >&2; exit 1
        fi
    else
        sh "$helper" 999999 "$work/$name.tar.gz" "$install/Modeler3D" "$work"
    fi
    if [ "$scenario" = success ]; then
        cmp "$install/Modeler3D" "$fixture/source/$name/Modeler3D"
        grep -q 'successfully' "$case_dir/cache/last-result.txt"
    else
        cmp "$install/Modeler3D" "$case_dir/original"
        grep -q 'failed' "$case_dir/cache/last-result.txt"
    fi
    echo "PASS: $scenario"
done
