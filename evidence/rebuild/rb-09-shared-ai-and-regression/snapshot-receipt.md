# RB-09 Additive Snapshot Receipts

## Historical RB-09 live-gate snapshot

**Status at creation:** paired live desktop/MCP gate, gate-time evidence validation, and final independent path/size/SHA-256/reparse verification passed.
**Snapshot directory name:** the additive live-gate recovery copy is named RB-09-complete; it records that earlier gate state and does not represent the current target identity, visual approval, owner acceptance, or overall product acceptance.

- Source: E:/Builder3D-Rust at HEAD 8e2178eddb882f365369ac69e33a98398a75b4a0; RB-09 worktree changes were intentionally uncommitted and unstaged at snapshot time.
- Destination: G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete
- Pre-copy G: free space: 478850187264 bytes.
- Copy command: robocopy E:/Builder3D-Rust G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete /E /COPY:DAT /DCOPY:DAT /XJ /XD E:/Builder3D-Rust/target E:/Builder3D-Rust/.cache E:/Builder3D-Rust/logs /R:1 /W:1 /NP /NFL /NDL
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

- Source: E:/Builder3D-Rust at HEAD 8e2178eddb882f365369ac69e33a98398a75b4a0, with the task-scoped RB-09/workspace files staged before commit.
- Destination: G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review
- Pre-copy G: free space: 478198308864 bytes.
- Copy command: robocopy E:/Builder3D-Rust G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review /E /COPY:DAT /DCOPY:DAT /XJ /XD E:/Builder3D-Rust/target E:/Builder3D-Rust/.cache E:/Builder3D-Rust/logs /R:1 /W:1 /NP /NFL /NDL
- Exclusions: repository-root target, .cache, and logs; nested RB-09 evidence logs and Git metadata were included. No existing G: snapshot was overwritten, moved, or deleted.
- Final Robocopy exit code: 001 (files copied; no failures).
- Independent verifier: evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1
- Verified source/snapshot included files and bytes: 3841 / 700634398 each.
- Missing / extra paths: 00000 / 00000; size or SHA-256 mismatches: 00000; included reparse entries: 00000 / 00000.
- Verification result: PASS. The plan's Recovery snapshot subsection and these receipt/log updates were appended after this exact-tree check; the snapshot preserves the staged recovery state before commit.

## RB-09 artifact-identity review snapshot

**Status:** additive review/recovery copy; independently verified; not a completion snapshot.

- Source: E:/Builder3D-Rust at HEAD 53e60d6421ede93cb12f24e1f3c4502c5aec5468, with the RB-09 identity evidence and documentation updates.
- Destination: G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review
- Pre-copy G: free space: 477493166080 bytes.
- Copy command: robocopy E:/Builder3D-Rust G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review /E /COPY:DAT /DCOPY:DAT /XJ /XD E:/Builder3D-Rust/target E:/Builder3D-Rust/.cache E:/Builder3D-Rust/logs /R:1 /W:1 /NP /NFL /NDL
- Exclusions: repository-root target, .cache, and logs. Nested RB-09 evidence logs and Git metadata are included. No prior G: snapshot was overwritten, moved, or deleted.
- Robocopy exit code: 001 (files copied; no failures).
- Independent verifier: evidence/rebuild/rb-09-shared-ai-and-regression/verify_rb09_snapshot.ps1.
- The first verifier attempt using the full G: path hit Windows PowerShell path resolution on one deeply nested shader-cache file. The source and destination entries existed; this was a path-length limitation, not a copy omission. The same verifier was rerun with temporary drive Q: mapped to this snapshot, then the mapping was removed.
- Verified source/snapshot included files and bytes: 3904 / 701101053 each.
- Missing / extra relative paths: 0 / 0; size or SHA-256 differences: 0; included reparse entries: 0 / 0.
- Verification result: PASS. The snapshot includes the identity record, screenshot, client log, copied rebuild logs, and passing validator logs. This receipt and the final snapshot-path updates in WORK_LOG.md, checkpoint.md, report.md, verification.md, and the plan were written after the copy; no G: snapshot contents were changed.
