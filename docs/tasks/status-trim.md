# Trim STATUS.md to one screen (finish plan E2)

- Goal: `docs/STATUS.md` (533 lines) becomes about 60-80 lines. Keep: the title
  and plan link, the "Updated" line (shortened), the Test releases paragraph,
  the Finish plan paragraph, and under "Now" only the three newest entries
  (claw/Rotate reach/Resize handles, OF-70, Resize Settle and claws) plus the
  "Known work, in order" list and anything an open item still depends on
  (owner decisions that are not yet built).
- Move every other removed block, verbatim and in the same order, to the end of
  `WORK_LOG.md` under a heading `## 2026-11-19 - STATUS.md entries moved out`.
  Lose nothing: line count removed from STATUS must equal line count appended
  (plus the heading).
- Docs only. Do not touch code. Do not commit; the overseer commits.
- Check: `git diff --stat` shows only the two files; no text was altered.
- Tier: mechanic (Haiku).
