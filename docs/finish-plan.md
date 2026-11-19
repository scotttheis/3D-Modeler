# 3D Modeler — finish plan (2026-11-19)

Goal: a finished v1.0 on a public GitHub repository in about three working
sessions. Everything below comes from `STATUS.md`, `product-rebuild-plan.md`
and `github-upload.md` as of 2026-11-19. The delivery plan's rules (gates,
one commit per task, snapshots, no approval inferred) still apply.

## How the work runs (cost)

- **Overseer:** one Claude Opus session at low effort. It writes the task
  cards, starts the workers, reads each diff and the gate output, and commits.
  It does not write the code itself.
- **Workers:** started in the background, **one at a time** in
  `E:\3D-Modeler` (same `target\`, so nothing rebuilds from scratch and no
  extra target folder fills `E:`, which is 87% full). No worktrees.
  - Mechanical work: `codex exec` at low effort, or a Claude Haiku subagent.
  - Feature work: `codex exec` at medium effort, or a Claude Sonnet subagent.
  - Hard work (smoothing algorithm, renderer): Codex at high effort, with a
    Claude review of the diff (cross-family rule, plan 3.2).
- Each worker gets a task card in `docs/tasks/`, runs the Fast gate, and stops.
  The overseer is told when it finishes, reviews the diff and commits.
- The Full gate runs once at the end of each session, not after every task.

## Work by difficulty

### Easy (Haiku / Codex low; minutes each)

| # | Task | Notes |
|---|---|---|
| E1 | Commit the pending `AGENTS.md` edit; take the 2026-11-19 recovery snapshot | Snapshot missing for the claw/grip/resize work |
| E2 | Trim `STATUS.md` back to one screen; move old "Now" entries to `WORK_LOG.md` | Docs only |
| E3 | Tests for the ledger gaps: Center View, Frame, Snap toggle, window-icon load | Ledger says no test covers them |
| E4 | D4: dark-theme viewport a bit darker; before/after captures | Owner decided 2026-11-13, not built |
| E5 | Remove the Windows username and drive letters from files that become public | `github-upload.md`, "Things that become public" |
| E7 | Move Smooth to the right of Subtract on the Edit ribbon | [`tasks/smooth-button.md`](tasks/smooth-button.md) |
| E6 | `README.md` (approved wording in `github-upload.md`) and Apache 2.0 `LICENSE` | Wording already approved |

### Medium (Sonnet / Codex medium; one task each)

| # | Task | Notes |
|---|---|---|
| M1 | D8: Merge of objects that don't touch groups them instead and shows a ~4 s popup; MCP returns the same | Decided, not built |
| M2 | S1-1: put every type size, spacing, radius and control height into `chrome.rs` tokens | Mechanical but wide; captures must match |
| M3 | S1-3: Inspector sections and one field grid | Visible; owner review |
| M4 | S1-4: Ribbon separators, tab type, overflow menu | Visible; owner review |
| M5 | S1-5 / S1-6: one dialog style; Welcome screen grid and empty/missing states | Visible; owner review |
| M6 | S1-8 / S1-9: hover/pressed/focus/disabled states, contrast and keyboard focus | Mostly tests plus small fixes |
| M8 | Ship every action icon at 128 x 128 in the release package | **Owner 2026-11-19:** keep all project artwork at full size in the source tree; only the shipped package uses 128 px (half of 256) for the first release, to judge reception. Shrink at packaging time, not in `assets/`. Check captures for blur at the largest ribbon size and on high-DPI |
| M9 | Update repository setup (`updates.md`) so the public build can find updates | Before the first public build |

### Hard (Codex high + Claude review)

| # | Task | Notes |
|---|---|---|
| H1 | D9: smoothing that behaves like Blender's sculpt Smooth on sparse meshes | Algorithm work; golden tests |
| H2 | GitHub Actions builds for Windows MSI, Linux tarball and Mac DMG | Mac can only be built there or on the owner's Mac |
| H3 | Upload: rewrite the author email, fast-forward `main`, push, first release | Owner signs in; history rewrite only with a `G:` snapshot |

### Left until after v1.0 (each one could come in a later update)

- S3 performance: the 30-million-face control and package/memory budgets.
- Emboss progress (OF-56/57). Owner confirmed 2026-11-19: after v1.0.
- Material texture dialog (OF-46 follow-up). Owner 2026-11-19: after v1.0.
- The faint pale dotted rim at the floor (F9 note). The cause is not known.
- The 119 ledger rows that wait for owner walkthroughs. They are done by
  using the app, not as separate work.

## Sessions

1. **Session 1: cleanup and small features.** E1–E7, M1, D4 captures. Full
   gate. Owner reviews a short pack (D4, D8).
2. **Session 2: polish (S1).** M2–M6.
   Full gate. Owner review pack with captures in both themes.
3. **Session 3: ship.** H1, M8, M9, H2, Ship gate, H3. Owner signs in to
   GitHub; v1.0 goes public.

Model for the overseer session (the owner picks it with `/model` before
pasting the prompt; workers are chosen by the overseer as above):

| Session | Overseer model | Effort | Why |
|---|---|---|---|
| 1 | **Sonnet 5.5** | low | Easy tasks and one medium; reviewing small diffs |
| 2 | **Sonnet 5.5** | medium | Visible polish; more judgement on captures |
| 3 | **Opus 5.5** | medium | Smoothing algorithm, build pipelines, history rewrite and the public push |

Escalate a session to Opus only if a task fails review twice (plan 3.2).
Every session ends by telling the owner, in bold, the model and effort for
the next session and giving the prompt to paste.

Each session starts with the owner saying "carry on with the finish plan".
The overseer reads this file and `STATUS.md` and picks the next unticked task.

## Progress

- [x] Session 1 (2026-11-19: E1-E3, E5-E7 done; E4 and M1 were already built as OF-35 and OF-38)
- [ ] Session 2
- [ ] Session 3

## Owner answers, 2026-11-19

- Approved: Settle, claw cursors, edit tools with no selection, live
  Move/Rotate/Resize numbers, Move/Rotate/Resize arrows and grips.
- Change: Smooth moves to the right of Subtract (E7).
- Emboss and the Material texture dialog: after v1.0.
- Commit email `scotttheisw@gmail.com` is correct (w = work).
- Icons: project files stay full size; the first release ships them at
  128 x 128 ("half of the 256 size on our first release and i'll feel out the
  reception from there").
- Resize floor/object switch: **approved** ("floor/object is awesome. you did
  great. approved!").
