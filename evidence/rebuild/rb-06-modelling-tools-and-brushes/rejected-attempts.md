# RB-06 rejected attempts

Rejected intermediate work is not used to claim completion. The final accepted
records were regenerated after each evidence-harness correction.

1. The first native replay used a stale debug executable, so its executable
   identity did not match the completed Rust source/build. Its captures were
   overwritten and are not in the accepted result set.
2. The first expanded Boolean replay used Items/All while three objects were
   present. Merge, Intersect, and Subtract therefore exercised the app's typed
   exact-two error rather than a valid operation. The harness was corrected to
   deselect the third row.
3. A second corrected replay still deselected the newly added Subtract operand;
   the cylinder fixture produced the valid typed empty-result error. The final
   harness selects the target plus the newly added overlapping Cube. The final
   Light/Dark frames report `Subtract: 10496 face(s)` and are accepted.
4. A first `cargo bloat` invocation was rejected because a new PowerShell
   process lacked `LIBCLANG_PATH`. It was rerun with the pinned build-only
   libclang directory and passed.
5. The new viewport preview regression initially asserted a nonexistent AABB
   `extents()` helper. The assertion was corrected to compare minimum/maximum
   coordinates; the full locked workspace test then passed.
6. Temporary debug captures from the rejected harness runs were removed from
   the RB-06 capture directory. No rejected PNG is referenced by the accepted
   journal or snapshot claim.

The Python modal reference continuation was not accepted as a full operation
replay because it stalled after its baseline sequence. The six baseline images
and the complete 272-test result remain valid and are documented as such.
