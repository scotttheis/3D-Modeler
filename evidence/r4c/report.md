# R4c — behavior, security, replay, and bounded operations

R4c completes the live adapter behavior behind the R4 contract. Mutations use
the portable core scene/command boundary, while MCP-only state remains in the
adapter. The implementation now covers:

- typed validation and structured errors for invalid input, conflicts, busy
  operations, unsupported capabilities, path policy, replay conflicts, core
  failures, and slicer failures;
- stable project/object/scene revision context checks and canonical
  idempotency replay with a 256-entry/600-second bound;
- one active asynchronous operation with bounded 128-record/3600-second
  retention, cooperative cancellation flags, progress, terminal status, and
  bounded argument/result payloads;
- approved-root canonical reads/writes, overwrite refusal, symlink/reparse
  component rejection, bounded PPM image sampling, clipboard count/byte
  limits, and bounded slicer staging retention;
- real grouped transforms/colors, mesh operation dispatch, selection/history,
  project lifecycle, image-color application, platform slicer discovery,
  visible-scene staging, and known-slicer process handoff;
- explicit typed limitations for Rust-core features not yet ported in this
  stage (`emboss_text` and `hollow`) rather than silent mutation or an
  unrestricted fallback.

Focused Rust tests cover replay conflict/replay success, revision conflicts,
async completion, grouped actions, clipboard, bounded PPM sampling, approved
root reads/writes, slicer discovery, profile filtering, and strict schemas.
The full workspace matrix and official probes are rerun for the final R4
checkpoint.

The next and final ordered checkpoint is R4d: full parity evidence, policy and
security scans, final verification, and the additive R4 backup.
