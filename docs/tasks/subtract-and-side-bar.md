# Subtract still hangs; side-bar status and item rows

Status (2026-11-21): **parts 2-4 done and owner-approved, shipped in v1.0.2.**
Part 1 (the UI-thread freeze, 5-12 s slow frames on `Secret Book A`) is **still
open**. Subtract itself now works for the owner: with one object selected it
cuts everything it touches and disappears (no pick step); the earlier "never
finishes" was the invisible pick prompt. Also done this session at the owner's
request: Home tiles open on click (no Open button), Save writes back an opened
STL/3MF, boolean error logging. Left: full gate, deny, audit, G: snapshot.

## 1. Subtract never finishes on the owner's file (top priority)

File: `C:\Users\<owner>\Desktop\Secret Book A.3mf` (in the owner's recent
files). Its objects: `6` = book, 334,180 faces; `7` = a scaled box, 12 faces;
`8` = 169,762 faces. The owner selects a box (`7` or `7 (copy)`) and presses
Subtract. Owner: "I've been typing a while and still no subtraction."

What is known:

- **First cause, fixed in `7641f7a`.** The book is sealed but has 1 zero-area
  triangle, so the boolean ran the full Extended repair first. That repair
  opened 87 holes and the boolean failed after about 2 s, with only status-bar
  text. Now a sealed, consistently wound operand whose only flaws are
  zero-area or duplicate triangles goes straight to the kernel
  (`is_kernel_ready` / `kernel_operand` in
  `crates/modeler3d-core/src/mesh_ops.rs`). A temporary probe that called the
  core boolean on the loaded file (Release) finished in 0.22 to 0.68 s for
  every pair. That proves the **core** is fast. It does not prove the app
  path is.
- **The app still never finishes.** Release app pid 16264, started 10:00:
  the log (`%LOCALAPPDATA%\Modeler3D\logs\modeler3d.log`) has no
  `boolean timing` line and no `desktop job failed` line (failures are now
  logged), only one `slow UI frame: UI update took 6041 ms`. After about 40
  minutes the **main thread had used 4 min 49 s of CPU**, while the other
  threads used 14 s or less. The time is going into the UI thread, not the
  boolean.
- In the earlier session (pid 12276) the main thread also kept using CPU in
  bursts with no boolean logged.

Suspects to check, in this order:

1. Is the job even started? With one object selected and more than one
   other visible object, Subtract enters a **pick mode** ("click the target
   object", `start_boolean_pick` / `handle_boolean_pick` in `shell.rs`), and
   that prompt shows only in the status text. With 4 objects in the scene
   (see `erros/abortstatus.png`), the owner may be in pick mode, waiting.
2. Per-frame UI-thread work on ~500k faces while a job is running or a pick
   is pending: `poll_desktop_job` repaints every 50 ms; check
   `refresh_preview`, `RenderScene::from_core_reusing`, the side-bar health
   marks and icons, and `pick_scene` / `object_under_pointer` on hover (CPU
   ray picking over 500k triangles every frame).
3. `commit_boolean` and the work after it on the UI thread (display rebuild,
   health check of the result).

How to find it: reproduce with the owner's file in the Release app, then
take a stack sample of the main thread (or add timing logs around the frame
phases). Add an app-level test that loads a 500k-face scene and runs Subtract
through `dispatch_boolean` and `poll_desktop_job` to completion, with a time
limit.

Goal (owner): "make it work like 3D Builder... super fast and smart." A
subtract on this file should finish in about a second, with the window
responsive the whole time.

## 2. Left click works during a cut or other long task

While a cut runs, right-drag orbits the view but **left-drag does nothing**.
Left click/drag must work as normal (select, orbit, move) during a cut or any
other heavy background task. Look at the `desktop_job.is_some()` guards in
`shell.rs` (around lines 1952, 1982, 2129) and the viewport input handling.
Edits that would change the scene during a job must still be refused or
handled safely (`commit_boolean` already refuses a changed scene).

## 3. Status gets its own strip at the bottom of the right bar

Reference: `erros/abortstatus.png` (red marks at the bottom of the right
bar).

- At the bottom of the right bar, add a separator bar styled like the other
  side-bar separators, with its own space below it.
- In that space: the **status text left-aligned**, and the **Abort button at
  the far right** (only while something can be aborted).
- Current code: `render_operation_progress` and `abort_operation` in
  `shell.rs`.

## 4. Item rows in the right bar ("Items" list)

Current look is in `erros/abortstatus.png`. Code: the item list around
`shell.rs` lines 4100-4400 (`select_item_row`, `paint_object_icon`).

1. **Hover highlights the whole row.** Today only the name is highlighted,
   and nothing to its right.
2. **Fixed order, right-aligned:** `[object icon] [name] ...... [warning] [eye]`.
   The eye goes at the far right of the row so every eye lines up. The
   warning (error) icon sits just left of the eye. A row without a warning
   leaves that space empty, so the eyes still line up.
3. **Warning tooltip:** hovering the warning icon shows the message that
   exists today (the one with the count). The count does **not** appear in
   the row itself.
4. **Keep the "(copy)" naming** (owner likes it). Clicking anywhere on the
   row, including on the name text, behaves exactly like clicking the object
   (select it); the name must not act like a text field.

## Done when

- Owner subtracts on `Secret Book A` in the Release app and it finishes fast.
- Parts 2-4 are tried by the owner in the Release app and approved.
- Run `scripts/Update-CurrentBuild.ps1`, commit, take a G: snapshot, and
  update `docs/STATUS.md` (see `AGENTS.md`).
