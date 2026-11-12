# R4a continuation checkpoint

- Status: complete.
- Scope: portable Rust `rmcp` stdio server, protocol discovery, official
  client compatibility, adapter-owned state, typed errors, bounded replay and
  operation records.
- Direct probes: `official_client_probe.py` and `raw_discovery_probe.py`.
- Observed official-client protocol: `2025-11-25`.
- Raw server-supported protocol list includes the target `2026-07-28`.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261029-155500_R4a-protocol-final`.
- Target directory is excluded from the snapshot; exact verification is in
  `verification.md`.
- Next authoritative task: R4b catalog/profile/schema parity.
