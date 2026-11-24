# Application updates

The desktop checks GitHub once per normal startup, in the background. A newer
stable release with an installer for this computer shows the app icon, current
and available versions, What's new, Later, and Update and Restart. Later skips
the offer for this run. No installer downloads before acceptance. Offline,
rate-limited and unreleased repositories leave the app usable and log the check
failure. Review captures, tests and command-line modes do not check GitHub.

Acceptance downloads the installer to the app cache, verifies its byte count
and GitHub's SHA-256 asset digest, and requests the normal close flow. Unsaved
projects still offer Save, Discard and Cancel. Cancel keeps the app open with
an Update ready button. Later during downloading defers the restart. Install
helpers wait for the app process to exit, then install and restart it; the next
start reports success or failure. Mac and Linux keep a sibling `.previous-*`
installation for recovery; settings and projects remain in their own folders.

Windows updates the existing per-user MSI installation with its unchanged
UpgradeCode. Mac replaces the installed `.app` from the universal DMG. Linux
replaces the extracted tarball installation. Cargo builds, Mac apps running
from `/Volumes`, and unsupported installation layouts offer a release download.
Windows builds do not need a signing certificate for this implementation;
release assets must come from the configured public repository over HTTPS.

MCP uses the same updater: `get_update_status` is available in Observe and
higher profiles; `respond_to_update` is Full only and takes `response`
(`install` or `later`) plus the exact `available_version` as `version`.
The version is the update identity, separate from the scene revision. Repeated
acceptance never launches a second download. A stale version is a conflict,
unknown fields are rejected, and responding while a model job is active is
busy. Progress is polled with `get_update_status`; Later defers restart while
an accepted download finishes. Installation preserves the desktop save prompt.
No client-provided URL, path, command or repository can be executed.
The catalog was 76 tools, version `2026-11-17.1`, when updates were added. Existing tool order and the
pinned rmcp 3.9.0 transport are preserved. The current official MCP tools
specification and Rust SDK release notes were reviewed; this adds tools to
the established transport without changing its negotiated protocol support.

## Before the first public build

The repository does not exist yet. `packaging/update-feed.json` deliberately
has a null repository, so these development builds make no release requests.
At launch, configure the **actual** chosen repository once:

```powershell
packaging/Configure-UpdateFeed.ps1 -Repository OWNER/REPOSITORY
```

Commit that configuration and rebuild **every platform**, including the first
public version. A first public build without this address cannot discover a
later release. No API key or user login is required for public releases.

Build Windows using `Build-Msi.ps1 -Version <version> -Build -PublicRelease`
(add the usual `-Name` and `-Verify` when making an installer candidate).
For Mac/Linux, set `MODELER3D_PUBLIC_RELEASE=1` while running their packaging
scripts. These guards call the built binary's `--require-update-feed` mode;
an unconfigured binary fails. `--version` reports its embedded release version.
The packaging scripts stamp `MODELER3D_RELEASE_VERSION`; MSI staging also
refuses a stale executable whose version differs from the MSI version.

Publish a GitHub **Release**, with tag `v<version>` and these asset names:

| Platform | Asset |
|---|---|
| Windows x64 | `3D-Modeler-<version>-windows-x86_64.msi` |
| Mac Intel + Apple Silicon | `3D-Modeler-<version>-macos-universal.dmg` |
| Linux x64 | `3D-Modeler-<version>-linux-x86_64.tar.gz` |

The MSI script emits the canonical update filename as well as its RC filename.
The Mac/Linux scripts emit the canonical filename. GitHub supplies the asset
`digest` field; verify it is populated before publishing. Missing installers
are not offered to that platform. Drafts, prereleases, equal versions and
downgrades are never offered. Increment all three-part release/package versions
together. Keep the repository address stable after launch.

Before public launch, exercise one installed-version upgrade to a higher
version on **each OS**, including Save, Cancel, Later, offline startup and an
installer failure. This cannot be measured against our live feed until the
repository and two releases exist. The Mac helper must be exercised on a Mac.
Do not describe unit tests or review captures as that end-to-end result.

The Linux helper can be checked independently with
`sh scripts/Test-UpdateHelpers.sh` (on Windows, run it in WSL). It uses a
temporary fixture and real tarballs to check success, an invalid payload and
rollback after a failed rename, including paths containing spaces. It never
installs into the user's app directory.

`py -3.13 scripts/Test-UpdateMcp.py` uses freshly built debug `modeler3d-app`
and `r4_mcp` binaries to exercise the real stdio gateway against an isolated
desktop profile: discovery, update state, refusal of an unavailable install,
and a normal application close. It keeps the bridge secret in its descriptor
and prints only a compact result. Its generated files stay under `logs/`.

## Reference

The reference is the **individual Craft application** update prompt, confirmed
in installed CADCraft on 2026-11-17: 0.17.1 offered 0.20.1. Its window framebuffer
is `evidence/rebuild/captures/2026-11-17-startup-updates/cadcraft-startup.png`.
No update was installed in CADCraft. Its public integration is
[CADCraft updates_ui.rs](https://github.com/storytold/cadcraft/blob/main/crates/ui-egui/src/updates_ui.rs),
using the shared [per-app startup card](https://github.com/storytold/craft-libs/blob/main/crates/update/src/dialog.rs)
and [updater](https://github.com/storytold/craft-libs/blob/main/crates/update/src/updater.rs).
This implementation independently mirrors that flow using 3D Modeler's own
artwork, existing close confirmation and package identities. ArtCraft's current
Windows installer additionally requires Authenticode; our unsigned test-build
policy uses the configured GitHub source and verified digest.
