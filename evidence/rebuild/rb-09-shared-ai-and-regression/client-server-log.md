# RB-09 MCP Client/Server Log

## Live clients

Client implementation: official Python MCP SDK 2.2.0, run from the existing L:/Builder3D Python environment. It connected to the Python app’s live MCP bridge and to the Rust r4_mcp stdio gateway while each desktop app was open. Both catalogs listed the fixed 60 tools.

| Log | Session |
|---|---|
| logs/rb09-python-client.jsonl | Python main sequence: shared project creation/read/edit, save/New/Open, file policy, replay, history, operation progress/cancel/failure/stale result, and final save/reopen. |
| logs/rb09-python-client-2.jsonl | Python competing-client revision race and cancel follow-up. |
| logs/rb09-python-optional-client.jsonl | Python optional MCP startup and closed-desktop response. |
| logs/rb09-rust-client.jsonl | Rust first live run, including the Welcome presentation defect and initial shared-document checks. |
| logs/rb09-rust-client-2.jsonl | Repaired Rust run: file failures, replay, revision conflict, competing client, long operations, and stale-result discard. |
| logs/rb09-rust-client-final-pair.jsonl | Final Release app shared GUI/MCP undo/redo and final paired project identity/read. |
| logs/rb09-rust-optional-client.jsonl | Rust optional startup failure, full catalog, and unavailable response. |

Logs contain tool inputs/results and no bridge bearer secret. Runtime descriptor secrets were not printed.

## Rust final live server identity

The final paired live Rust session used target/release/r4_mcp.exe SHA-256 a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b. After the paired sequence, the locked Release build command passed again and produced the current executable SHA-256 9b8817015ab5c8195013ee788fb095e9393848be74c7da0221ebd33342fc99d3. Both are tied to the 96-file source manifest in build-manifest.json. The post-run binary was separately checked by the official SDK probe below.

The 2026-11-04 wrap-up later ran `cargo bloat`; after that measurement/rebuild,
the current `target/release/r4_mcp.exe` SHA-256 was
`aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. It
matches neither the live gateway hash nor the separately recorded post-run
hash above. The current app hash also differs from the live hash. The recorded
hashes above continue to identify the processes used in the live run; the later
outputs are not claimed as live-tested binaries.

## Closed desktop and optional startup

The Rust Release gateway initialized over stdio and listed all 60 tools. With no runtime descriptor, get_status returned structured retryable desktop_unavailable. The post-run official-client result is rust_stdio_client_probe-result.json, with stdout/stderr in logs/rust-stdio-client-probe.*.

For optional startup, the Rust app was launched with BUILDER3D_RUNTIME_FILE pointing under a regular file, so its parent directory could not be created. MCP status reported unavailable while the normal desktop stayed usable. The official SDK still discovered 60 tools; get_status returned desktop_unavailable with retryable=true. On normal shutdown and failed-startup shutdown, owned runtime descriptor and listener were absent, app/gateway/client processes exited, and a client request after close returned desktop_unavailable.

The Python app’s optional startup failure and closed-client response are logged in rb09-python-optional-client.jsonl. It remained usable and shut down cleanly.

## Credential and file policy

No bearer token, bridge secret, or credential was written to evidence. Approved paths were under fixtures/approved. Denied-root and overwrite attempts were rejected; the overwrite sentinel remained byte-identical. The exact request outcomes and fixture hashes are in the JSONL logs and fixture-manifest.json.

## Current Rust Release session (2026-11-06)

Run directory: `currpair-20261106-072716/`. The app SHA-256 was `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`; the Rust stdio gateway SHA-256 was `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. Client logs are `logs/rust-current-live.jsonl`, `logs/rust-current-live-reopen-client.jsonl`, and `logs/rust-current-optional-failure-client.jsonl` under that run.

The official Python MCP SDK 2.2.0 was used only as an MCP client to the Rust gateway; the Python Builder3D product was not launched. The client found 60 ordered tools, read and edited the live Rust document, saved it, and read the same project after the Rust GUI reopened it. The optional-startup failure session returned retryable `desktop_unavailable` while leaving the GUI usable. App PIDs 28536 and 5400 exited normally. No token or credential was recorded.

The schema report compares current Rust live catalog data with saved Python catalog snapshots. Exact input/output schema matches were zero in each profile; schema compatibility remains unresolved. The comparison does not refresh the Python reference or perform full nested output-schema validation.
