# RB-09 Additive Snapshot Receipts

## Historical RB-09 live-gate snapshot

**Status at creation:** paired live desktop/MCP gate, gate-time evidence validation, and final independent path/size/SHA-256/reparse verification passed.
**Snapshot directory name:** the additive live-gate recovery copy is named RB-09-complete; it records that earlier gate state and does not represent the current target identity, visual approval, owner acceptance, or overall product acceptance.

- Source: E:/3D-Modeler at HEAD 8e2178eddb882f365369ac69e33a98398a75b4a0; RB-09 worktree changes were intentionally uncommitted and unstaged at snapshot time.
- Destination: G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete
- Pre-copy G: free space: 478850187264 bytes.
- Copy command: robocopy E:/3D-Modeler G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete /E /COPY:DAT /DCOPY:DAT /XJ /XD E:/3D-Modeler/target E:/3D-Modeler/.cache E:/3D-Modeler/logs /R:1 /W:1 /NP /NFL /NDL
- Exclusions: repository-root target, .cache, and logs directories; /XJ prevents junction traversal. Nested evidence/rebuild/rb-09-shared-ai-and-regression/logs are included. No existing G: snapshot is overwritten, moved, or deleted.
- Independent verifier: evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1
- Final Robocopy exit code: 001 (files copied; no failures).
- Source included files / bytes: 3404 / 640773528
- Snapshot included files / bytes: 3404 / 640773528
- Missing / extra relative paths: 00000 / 00000
- Size or SHA-256 mismatches: 00000
- Included reparse entries in source / snapshot: 00000 / 00000
- Final verification: PASS; the verification was rerun after synchronizing this receipt and the final docs.

## Historical paused handoff copy

The earlier additive handoff snapshot G:/Builder3D-Rust_snapshot_20261102-152935_RB-09-handoff was verified as a paused copy, not an RB-09 completion snapshot. At that time both sides contained 3,289 included files / 584,907,453 bytes, with zero missing, extra, different, or reparse entries.

## Historical review snapshot

The additive review snapshot G:/Builder3D-Rust_snapshot_20261104-073919_RB-09-review was verified before the live desktop gate. Its receipt is in the 2026-11-04 review entry in WORK_LOG.md. It is a historical review copy, not this RB-09 completion snapshot.

## RB-09 wrap-up review snapshot

**Status at creation:** additive review/recovery copy; not labeled complete because the then-current evidence-validator run reported a Release app hash mismatch against the recorded live build. A later follow-up explains the rebuilds and verifies the current pair separately. This snapshot remains a review copy, not a completion snapshot.

- Source: E:/3D-Modeler at HEAD 8e2178eddb882f365369ac69e33a98398a75b4a0, with the task-scoped RB-09/workspace files staged before commit.
- Destination: G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review
- Pre-copy G: free space: 478198308864 bytes.
- Copy command: robocopy E:/3D-Modeler G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review /E /COPY:DAT /DCOPY:DAT /XJ /XD E:/3D-Modeler/target E:/3D-Modeler/.cache E:/3D-Modeler/logs /R:1 /W:1 /NP /NFL /NDL
- Exclusions: repository-root target, .cache, and logs; nested RB-09 evidence logs and Git metadata were included. No existing G: snapshot was overwritten, moved, or deleted.
- Final Robocopy exit code: 001 (files copied; no failures).
- Independent verifier: evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1
- Verified source/snapshot included files and bytes: 3841 / 700634398 each.
- Missing / extra paths: 00000 / 00000; size or SHA-256 mismatches: 00000; included reparse entries: 00000 / 00000.
- Verification result: PASS. The plan's Recovery snapshot subsection and these receipt/log updates were appended after this exact-tree check; the snapshot preserves the staged recovery state before commit.

## RB-09 artifact-identity review snapshot

**Status:** additive review/recovery copy; independently verified; not a completion snapshot.

- Source: E:/3D-Modeler at HEAD 53e60d6421ede93cb12f24e1f3c4502c5aec5468, with the RB-09 identity evidence and documentation updates.
- Destination: G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review
- Pre-copy G: free space: 477493166080 bytes.
- Copy command: robocopy E:/3D-Modeler G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review /E /COPY:DAT /DCOPY:DAT /XJ /XD E:/3D-Modeler/target E:/3D-Modeler/.cache E:/3D-Modeler/logs /R:1 /W:1 /NP /NFL /NDL
- Exclusions: repository-root target, .cache, and logs. Nested RB-09 evidence logs and Git metadata are included. No prior G: snapshot was overwritten, moved, or deleted.
- Robocopy exit code: 001 (files copied; no failures).
- Independent verifier: evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1.
- The first verifier attempt using the full G: path hit Windows PowerShell path resolution on one deeply nested shader-cache file. The source and destination entries existed; this was a path-length limitation, not a copy omission. The same verifier was rerun with temporary drive Q: mapped to this snapshot, then the mapping was removed.
- Verified source/snapshot included files and bytes: 3904 / 701101053 each.
- Missing / extra relative paths: 0 / 0; size or SHA-256 differences: 0; included reparse entries: 0 / 0.
- Verification result: PASS. The snapshot includes the identity record, screenshot, client log, copied rebuild logs, and passing validator logs. This receipt and the final snapshot-path updates in WORK_LOG.md, checkpoint.md, report.md, verification.md, and the plan were written after the copy; no G: snapshot contents were changed.

## Current Rust Release recheck snapshot (2026-11-06)

- Status: new additive recovery copy of the current uncommitted E: worktree. No existing G: snapshot was overwritten, moved, or deleted.
- Source: E:\3D-Modeler at HEAD f3b2d929d5d8ba0e30c447a6736c620b15ac4372; existing source changes and untracked files were preserved.
- Destination: G:\B3D_R09C_20261106. It did not exist before this copy; path length is 20 characters.
- Initial copy started 2026-11-06 09:00:23 local time (America/Denver); G: free space before copy was 475358879744 bytes.
- Copy command: `robocopy E:\3D-Modeler G:\B3D_R09C_20261106 /E /COPY:DAT /DCOPY:DAT /XJ /XD E:\3D-Modeler\target E:\3D-Modeler\.cache E:\3D-Modeler\logs /R:1 /W:1 /NP /NFL /NDL`.
- Exclusions: repository-root `target`, `.cache`, and `logs`; `/XJ` prevented junction traversal. Nested RB-09 evidence logs and `.git` metadata are included.
- Initial Robocopy exit code: 1 (all 3991 source files copied; zero failed files). The pre-copy source manifest and first independent verification both reported 3991 files / 756835989 bytes, zero missing/extra/differing files, and zero reparse entries.
- Final synchronization Robocopy exit code: 1 (files copied; no failures).
- Final synchronized source/snapshot files / bytes: 3991 / 756841099 each. Final independent verification: PASS; missing / extra / differing paths: 0 / 0 / 0; source / snapshot reparse entries: 0 / 0.
- Critical SHA-256 values were compared between source and snapshot after the final sync:

| Critical file | SHA-256 |
|---|---|
| `crates/modeler3d-mcp/src/r4/catalog.rs` | `116d6f1ec940c8c8b3f2fca7d2365bb1293e978f1460a39e6d0ab821a745cb62` |
| `docs/component-catalog.toml` | `b65168aef87a5f3ec3367dc140be7bf63bd6bef1d7af71ed7f8f4731a0cc8aae` |
| `evidence/rebuild/parity-matrix.toml` | `fc65b6cdae6e8a9b0ca3359b5c3697d9788f28fa5c87471dda67a7cffaa8bbb1` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py` | `cf415e90ddeaa0651b10e0d6bd01b5cb542ee67a96936db0a12cbf34371ddb2a` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/current-run-manifest.json` | `abc20df8d59438f07a6b9e8d2c71c78487282c896188fc73ad47e7bb168ace54` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/schema-comparison.json` | `ef7cd8c860e641acea2111da98d64b83e88f80dfaafa50fbe29c41a44205b41ab` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/fixtures/approved/rust-current-live-saved.3dp` | `e6046a8d3ed7ea949b00a13928311b2fc88e476bda6d4d859af6d866246d9262` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/captures/rust-current-live-gui-reopened-after-restart-final.png` | `b1223ef8a48c17c3ece09ac77ed263620709bb53a150b18fe4266814ac37e5c5` |

The final synchronization and independent verification use the short destination path directly. The verified read-only recovery point `G:\Builder3D-Rust_snapshot_20261105-143636_RB-09-R1` and the incomplete `143410` attempt were left untouched.

## RB-09 review wrap-up snapshot (2026-11-06)

**Status:** additive review/recovery snapshot of the wrap-up worktree; it is not
an RB-09 completion snapshot because schema disposition remains pending.

- Source: `E:\3D-Modeler` at HEAD `f3b2d929d5d8ba0e30c447a6736c620b15ac4372`, preserving the existing changes and untracked files.
- Destination: `G:\B3D_R09W_20261106` (20-character absolute path; absent before creation).
- Created: 2026-11-06 at 11:35:42 local time. G: free space before copy: 474589401088 bytes.
- Copy command: `robocopy E:\3D-Modeler G:\B3D_R09W_20261106 /E /COPY:DAT /DCOPY:DAT /XJ /XD E:\3D-Modeler\target E:\3D-Modeler\.cache E:\3D-Modeler\logs /R:1 /W:1 /NP /NFL /NDL`.
- Robocopy exit code: 1; 3991 files copied, 0 failed. Excluded only repository-root `target`, `.cache`, and `logs`; nested RB-09 evidence logs and Git metadata were included.
- Independent verifier: `evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1` with `-SourceRoot E:\3D-Modeler -SnapshotRoot G:\B3D_R09W_20261106`.
- Verified source/snapshot: 3991 files / 756853562 bytes on each side; missing/extra/differing paths 0/0/0; reparse entries 0/0; verification PASS.
- Critical SHA-256 comparison: all 8 matched:

| Critical file | SHA-256 |
|---|---|
| `crates/modeler3d-mcp/src/r4/catalog.rs` | `116d6f1ec940c8c8b3f2fca7d2365bb1293e978f1460a39e6d0ab821a745cb62` |
| `docs/component-catalog.toml` | `b65168aef87a5f3ec3367dc140be7bf63bd6bef1d7af71ed7f8f4731a0cc8aae` |
| `evidence/rebuild/parity-matrix.toml` | `fc65b6cdae6e8a9b0ca3359b5c3697d9788f28fa5c87471dda67a7cffaa8bbb1` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py` | `cf415e90ddeaa0651b10e0d6bd01b5cb542ee67a96936db0a12cbf34371ddb2a` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/current-run-manifest.json` | `7582ddb35895d78424052a43325a60be29a2bc8731702f672752bb8677a5a139` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/schema-comparison.json` | `fbd3c0dd54fef6a1c81dd97c1c49613c9a54586e3a4117d5e8c87f46df37bb2f` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/fixtures/approved/rust-current-live-saved.3dp` | `e6046a8d3ed7ea949b00a13928311b2fc88e476bda6d4d859af6d866246d9262` |
| `evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/captures/rust-current-live-gui-reopened-after-restart-final.png` | `b1223ef8a48c17c3ece09ac77ed263620709bb53a150b18fe4266814ac37e5c5` |

The prior `G:\B3D_R09C_20261106` copy was left unchanged; its post-regeneration
verification reported only the two current-run JSON files as differing. The
verified read-only R1 recovery point and incomplete `143410` attempt were not
modified. This receipt and the final plan/work-log snapshot notes were appended
after the copy; the snapshot preserves the verified pre-receipt worktree state.
## RB-09 schema-parity pre-edit recovery snapshot (2026-11-07)

Status: additive recovery copy created and verified before the first Rust source
edits. Existing G: snapshots, including G:\B3D_R09W_20261106 and older
snapshots, were not overwritten or changed.

- Source: E:\3D-Modeler; destination:
  G:\Builder3D-Rust_snapshot_20261107-154700_RB-09-schema-parity-pre-edit.
- Copy command:
  robocopy E:\3D-Modeler G:\Builder3D-Rust_snapshot_20261107-154700_RB-09-schema-parity-pre-edit /E /COPY:DAT /DCOPY:DAT /XJ /XD E:\3D-Modeler\target E:\3D-Modeler\.cache E:\3D-Modeler\logs /R:1 /W:1 /NP /NFL /NDL.
- Excluded only the repository-root target, .cache, and logs directories;
  /XJ prevented junction traversal. Git metadata and nested RB-09 evidence logs
  were included.
- Independent verification before source edits reported 3,743 files /
  755,179,156 bytes on both sides; zero missing, extra, size/SHA-256-different,
  or reparse entries.
- All 17 critical SHA-256 comparisons matched at creation. Their verified
  snapshot-side values are in
  pre-edit-snapshot-critical-hashes.json. The 96-file Rust source manifest
  regenerated from the pre-edit snapshot also matches the historical
  96-file source manifest, SHA-256
  9846a4a6dee484186283e169a9b7cdbbf74007bdca772efa5981f438ce78b457.
- The independent verifier was
  evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1.
## RB-09 schema-parity final additive recovery snapshot (2026-11-07)

- Status: new additive G: recovery copy; no prior snapshot was overwritten,
  moved, or deleted.
- Source: E:\3D-Modeler. Destination: G:\B3D_R09S_20261107 (20-character
  destination path). G: free space before copy: 473,061,937,152 bytes.
- Robocopy command: robocopy E:\3D-Modeler G:\B3D_R09S_20261107 /E
  /COPY:DAT /DCOPY:DAT /XJ /XD E:\3D-Modeler\target
  E:\3D-Modeler\.cache E:\3D-Modeler\logs /R:1 /W:1 /NP /NFL /NDL.
  Exit code: 1 (files copied; no failures).
- Excluded only repository-root target, .cache, and logs. /XJ prevented
  junction traversal; Git metadata and nested evidence logs were included.
- First independent verification passed: 4,078 source and snapshot files /
  766,744,485 bytes each; missing/extra/different paths 0/0/0; source/snapshot
  reparse entries 0/0.
- Twenty-three critical SHA-256 comparisons matched. Their paths, byte counts,
  and hashes are in final-snapshot-critical-hashes.json.
- After synchronizing those summary/evidence updates, the source was copied to
  this same destination again; Robocopy exited 1 (files copied, no failures).
  Final independent full-tree verification passed with 4,079 files /
  766,761,893 bytes on both sides, zero missing/extra/different paths, and zero
  source or snapshot reparse entries. The verifier compares SHA-256 for every
  included file and found no differences. Its output is in
  target/rb09-final-snapshot-verification-final.log.
- Verifier: evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1.
  This receipt and the accompanying summary updates were written after the
  final verification; the G: copy preserves the exact tree measured at that
  check.
