#!/usr/bin/env bash
# Stops a release whose tag does not match the source version (OF-75).
# Usage: scripts/check-release-version.sh v1.0.6
# The version lives once, in [workspace.package] in the root Cargo.toml.
set -euo pipefail
tag="${1:?usage: check-release-version.sh <tag>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
source_version="$(awk '/^\[workspace\.package\]/{f=1; next} /^\[/{f=0} f && /^version *=/{gsub(/.*= *"|".*/, ""); print; exit}' "$root/Cargo.toml")"
if [ -z "$source_version" ]; then
  echo "Could not read [workspace.package] version from Cargo.toml." >&2
  exit 1
fi
if [ "${tag#v}" != "$source_version" ]; then
  echo "Tag $tag does not match the source version $source_version in Cargo.toml." >&2
  echo "Set [workspace.package] version to ${tag#v} and commit, or tag v$source_version." >&2
  exit 1
fi
echo "Tag $tag matches the source version $source_version."
