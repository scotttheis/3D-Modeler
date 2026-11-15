# Builder3D Rust — delivery plan (v2)

Adopted 2026-11-12. Replaces the v1 plan, which is preserved unchanged at
[`archive/product-rebuild-plan-v1-2026-11-10.md`](archive/product-rebuild-plan-v1-2026-11-10.md).
Live status is in [`STATUS.md`](STATUS.md) and nowhere else. This file changes
only when the plan itself changes.

## 1. Goal

Ship the owner's Builder3D as a dependable native Rust application: the same
layout, artwork, interaction habits, modelling tools, file behaviour and AI
control as the Python original at `L:\Builder3D`, with a finish that reads as
a professional product rather than a port in progress.

Done means: every row of the feature ledger has the evidence its tier requires
(section 6), the owner has accepted the look and the ten task walkthroughs, the
engineering gates pass on the shipping build, and the package meets its size
and performance budgets. The contracts in `AGENTS.md` (`.3dp` format, 60-tool
MCP catalog, millimetres, one undo entry per operation, layering) do not move,
except that the MCP catalog may grow (never change) to reach full desktop
parity (section 3.7, decision D6).

**MCP is 100%.** Anything a person can do to the model or the document in the
desktop app, an AI client can do through MCP, and the two produce the same
result. This is a completion requirement, not a nice-to-have.

## 2. Why v1 was replaced

v1 got the product direction right and the operating model wrong. Measured on
2026-11-11:

| Symptom | Measurement | Cost |
|---|---|---|
| Recovery by full-tree copy | 155 snapshots on `G:\`; recent ones ~774 MB / 4,100 files each, with a full SHA-256 pass and a written receipt | Minutes of agent time and tokens per patch, for something Git does in a second |
| Git not used | 224 uncommitted paths (184 untracked); last commit 2026-11-06 | No diff, no bisect, no cheap rollback; every review re-reads the tree |
| Status written as prose in three places | `AGENTS.md` 14 KB, plan 100 KB, `WORK_LOG.md` 376 KB | Every session pays to read them, and they disagree |
| Tests abandoned | `builder3d-app` tests did not compile; ten consecutive log entries end "Automated tests were not run" | Regressions found by the owner instead of by the machine |
| No way to see the UI | Each visual change needed a 3m42s fat-LTO build, a desktop screenshot, and the owner's eyes; temporary preview hooks were written and deleted repeatedly | The slowest, most expensive loop in the project sat on the most-iterated work |
| Release link runs out of memory | Fat LTO failed with LLVM OOM on 2026-11-11 | Review builds made with hand-typed overrides |
| One 8,900-line UI file | `shell.rs`, hand-painted with literal sizes | Every UI task loads the whole file; no shared control language, hence the home-made look |
| Acceptance never batched | Ledger: 147 features, 81 implemented, 3 owner-accepted, visual review pending on all but 2 | Six slices "done pending review" at once; rework lands late |
| One model at one setting | All work run on the top model at maximum effort | Mechanical edits billed like architecture |

v2 keeps the scope, contracts, ledger and slice history. It changes how work
is executed, verified, reviewed and recovered.

## 3. Operating model

### 3.1 The loop

Every task is one small, verifiable change:

1. **Brief** — a task card (section 3.4) of at most 25 lines. The executor
   reads the card, the files it names, and nothing else.
2. **Change** — the narrowest edit that satisfies the card, with its test.
3. **Verify** — the tier-appropriate gate from section 6, run by script.
4. **Show** — for anything visible, review captures (section 3.5) before and
   after, attached to the card.
5. **Record** — one commit (or one snapshot, see 3.6), one line in
   `STATUS.md`, ledger fields updated. No narrative log entry.

A task that cannot be verified by the machine or shown in a capture is not
ready to start; split it or write the missing check first.

### 3.2 Model routing

Route by the kind of thinking the task needs, not by habit. The names below
are the models available on this machine on 2026-11-12; check current rates
before relying on any cost assumption, and re-route if they change.

| Role | Work | Model | Effort |
|---|---|---|---|
| Architect / reviewer | Plan changes, task cards, design-system decisions, review of every diff before it is recorded, anything touching a contract | Claude Fable 5.2 or Opus 5.5 | High |
| Implementer | A task card against named files: features, refactors, bug fixes with a failing test | Codex GPT-6 Luna, or Claude Sonnet 5.5 | Medium; raise only when a task fails review twice |
| Mechanic | Renames, moving code between modules, applying a decided pattern across call sites, ledger and catalog updates, formatting fallout | Claude Haiku 5.5, or Luna at its lowest effort | Low |
| Second opinion | Independent review of risky changes (renderer, file formats, MCP gateway, undo) by a model that did not write them | Whichever family did not implement it | High, diff only |

Rules that keep this cheap:

- **Maximum effort is the exception.** It is for architecture, a failing
  investigation, or a second failed review — not the default session setting.
- **Reviewers read diffs, not trees.** A review prompt contains the task card,
  the diff, and the gate output.
- **No model self-certifies.** The implementer never marks its own task
  verified; the gate script and the reviewer do.
- **Escalate, do not grind.** Two failed attempts at one tier moves the task
  up a tier with the failure evidence attached; it does not get a third try.
- **Cross-family review on contracts.** If Luna wrote a change to formats, the
  MCP catalog, or undo, Claude reviews it, and the reverse.

### 3.3 Context diet

- `AGENTS.md` holds standing rules only. Status lives in `STATUS.md` (one
  screen). History lives in Git and the archived work log.
- A session reads: `AGENTS.md`, `STATUS.md`, its task card, the named files.
  It does not read this plan, old handoffs, or `WORK_LOG.md` unless the card
  says so.
- `WORK_LOG.md` is frozen as an archive. New history is commit messages.
- Per-slice handoff documents are retired. A slice has a task list in
  `STATUS.md` and evidence under `evidence/rebuild/<slice>/`.
- `shell.rs` may be split as it is touched, when that makes the task at hand
  easier. Module size is not a gate and splitting is not scheduled work
  (owner, 2026-11-12). Likewise, workspace lint enforcement and comment
  density are not work items; the current commenting level is accepted.

### 3.4 Task card

```
ID:        S1-07
Goal:      one sentence, observable
Ledger:    feature ids this moves
Files:     the files to read and edit (and any not to touch)
Do:        3-6 bullets
Verify:    gate tier + the specific test/capture that proves it
Out:       what is explicitly not part of this task
Tier:      Mechanic | Implementer | Architect
```

### 3.5 Seeing the UI: review captures

`crates/builder3d-app/src/review_capture.rs` starts the real application in a
named state, saves the window's own framebuffer to a PNG, and exits. It
simulates no input, does not photograph the desktop, and neither reads nor
writes user settings, so it is safe while the machine is in use.

```powershell
$env:BUILDER3D_REVIEW_CAPTURE = "evidence\rebuild\captures\multi-dark.png"
$env:BUILDER3D_REVIEW_STATE   = "multi"    # welcome | empty | move | rotate | resize | resize-drag | multi | merge | unselected | material | settings | save | export | menu | colors | mirror | merge-apart | repair | paste | align | paint | paint-picker | xray | texture | texture-dialog | grab | crease | crease-sharp | smooth-brush | simplify | subdivide | hollow
$env:BUILDER3D_REVIEW_THEME   = "dark"     # light | dark
$env:BUILDER3D_REVIEW_SIZE    = "normal"   # normal | big
$env:BUILDER3D_REVIEW_TAB     = "Insert"   # any ribbon tab
# Optional:
$env:BUILDER3D_REVIEW_CAMERA  = "45,30,0"  # yaw,pitch (deg),distance (mm; 0 = framed)
$env:BUILDER3D_REVIEW_POINTER = "640,40"   # rest the pointer here to show a hover state
$env:BUILDER3D_REVIEW_VIEW    = "wireframe" # comma list: wireframe, xray
$env:BUILDER3D_REVIEW_INTERACTIVE = "1"    # stay open with isolated settings instead of capturing
cargo run --profile review -p builder3d-app
```

- Use the `review` Cargo profile for iteration (optimised, no fat LTO). Use
  `release` only for shipping builds and measurements.
- Every visible change attaches before/after captures in both themes.
- Add a state to the harness when a task needs one (dialogs, each ribbon tab,
  brush popover, import units, print prep, error and progress states). S1
  extends it to the full v1 capture list so one command regenerates the board.
- A capture proves what was painted. It does not prove pointer behaviour and
  it is never owner approval. `POINTER` shows a hover state only, with no
  button pressed. `resize-drag` holds a handle by setting the shell's state
  directly, not through real input. The full variable table is in the
  module doc of `review_capture.rs`.

### 3.6 Recovery

D1 approved by the owner on 2026-11-12. In force:

- Work happens on the local branch `rebuild/work`. One local commit per
  verified task. No push, no pull request, no rewriting of `main`.
- Git tracks source, manifests, docs, scripts and text evidence. Binary
  captures and mesh fixtures under `evidence/`, `logs/` and `erros/` are
  ignored (see `.gitignore`); files already tracked stay tracked.
- `G:\` receives, per completed slice, a timestamped tree snapshot excluding
  `target\` (count-and-bytes check, no per-file hash pass, no receipt essay).
  That snapshot is what preserves the ignored binary evidence.
- Existing `G:\` snapshots stay untouched.

### 3.7 MCP parity (owner requirement, 2026-11-12)

- **One implementation.** A desktop action and its MCP tool call the same
  core function. Product rules (for example "Merge takes the top-most
  object's color", OF-9) live in `builder3d-core`, never in the app or the
  MCP adapter alone.
- **Same commit.** A task that changes what a desktop action does to the
  model also changes the MCP side and adds an MCP test proving the same
  result. A desktop-only behaviour change is incomplete and fails review.
- **Purely visual work is exempt** (floor labels, hover, shadows, captures),
  because it does not change the model or the document. View toggles that
  MCP already exposes (`set_view`, `view_preset`, `frame_view`) must keep
  matching the desktop.
- **Catalog growth.** The 60 tools captured from the Python build keep their
  names, schemas and behaviour except where the owner has changed the product
  (OF-9). New desktop capabilities that have no tool get a new tool, added
  to the catalog, its schema contract and its tests (D6).
- **Parity check.** The ledger lists, for every desktop action, its MCP tool
  or "visual only". The Full gate's catalog parity check reads that list; an
  action with no tool and no exemption fails it.

Known gaps on 2026-11-12, worked as F12:

| Desktop action | MCP today | Fix |
|---|---|---|
| Merge result color (OF-9) | Kept per-face colors | **Done** 2026-11-12: both use `Scene::apply_product_boolean` |
| Rename an object | No tool | New tool, e.g. `rename_object` |
| Show / hide an object | No tool | New tool, e.g. `set_visibility` |

### 3.8 Evidence, sized to risk

Evidence exists to let someone trust a claim without redoing the work. Keep
what does that and stop producing the rest.

- Keep: gate output, capture pairs, fixture files, the ledger row, the commit.
- Stop: per-patch reports, verification narratives, checkpoint essays,
  snapshot receipts, supersession notices.
- Captures referenced by the ledger live under `evidence/rebuild/captures/`.
  Scratch captures go to `target/` and are never cited.
- Cargo builds only into `target/debug`, `target/review` and
  `target/release`. Never pass `--target-dir` to a new folder; never write
  scratch files into `target/`. Scratch captures go to the session scratchpad.
- `E:` is shared with other large projects. If `target/` exceeds 60 GB, delete
  `target/debug/incremental` (it rebuilds automatically).

## 4. Fix-first list (S0 — nothing else starts until these are closed)

Status of each item is tracked in `STATUS.md`.

| # | Problem | Fix | Proof |
|---|---|---|---|
| F1 | `builder3d-app` tests did not compile (three calls to the removed `Builder3dApp::project_handle`) | Helper restored as test-only | `cargo test --workspace --locked` runs |
| F2 | Tests not run since the gizmo rework | Run the whole suite; fix or quarantine-with-reason every failure | Green suite, or a named list in `STATUS.md` |
| F3 | `cargo clippy --all-targets -- -D warnings` blocked | Follows from F1; clear what it then reports | Clean run |
| F4 | 224 uncommitted paths, 184 untracked | Decide D1; then one baseline commit of the current tree | `git status` clean |
| F5 | Fat-LTO release link runs out of memory | Reproduce with the pinned toolchain; set `codegen-units`/`lto = "thin"` for release if fat cannot link reliably, and re-measure the size budget | Two consecutive clean release links |
| F6 | Feature ledger is stale against the code (e.g. still records a text menu button) | One pass reconciling the 147 rows with the running app | Ledger row count by state in `STATUS.md` |
| F7 | `E:` has 40 GB free and `target/` holds ~60 ad hoc build directories | Delete everything except `debug`, `review`, `release`, `build-tools` after confirming nothing cites them | Free-space figure in `STATUS.md` |
| F8 | No scripted gate | `scripts/Invoke-Gate.ps1 -Tier fast|full|ship` wrapping section 6, including libclang setup | Script used by every task |
| F9 | Mesh rims and the floor fight for depth where primitives rest on Y=0 (serrated lower edge on cylinder, cone, sphere) | Depth bias on the floor/shadow pass or a small epsilon | `multi` capture, both themes |
| F10 | `emboss_text` long-running metadata differs from the reference | Match it or record the owner's decision | Catalog parity check |
| F11 | Status narrative duplicated in `AGENTS.md` | Replaced by a pointer to `STATUS.md` | Done in this revision |
| F12 | MCP is not at full desktop parity (section 3.7) | Merge color shared in core (done); add rename and show/hide tools; ledger column mapping each desktop action to its tool | Parity check passes; an MCP test per closed gap |
| F13 | `cargo deny` and `cargo audit` had not been run | Run both; `cargo-audit` installed 2026-11-12; both now part of the Full gate and of any task that changes `Cargo.lock` | Clean runs recorded in `STATUS.md` |

## 5. Work order

Slices keep their RB ledger ids; the order and grouping change.

| Stage | Contents | Exit |
|---|---|---|
| **S0 Stabilise** | Section 4 | All gates green on a committed (or snapshotted) baseline |
| **S1 Product finish** | The design system and its application across the shell (section 7); capture harness extended to every state on the v1 capture list | Full capture board in both themes; **owner review session 1** |
| **S2 Close parity** | Remaining ledger gaps from RB-03 to RB-09 in ledger order; RB-08 native interaction matrix; scripted versions of the ten walkthroughs wherever they can be scripted, each also driven through MCP | Every ledger row implemented and machine-verified, desktop and MCP; **owner review session 2** (the ten walkthroughs, by hand) |
| **S3 Performance and package** (RB-10) | 30-million-face control and owner model corpus on the Quadro RTX 4000; resource routing on the hardware actually available; clean-machine install; 40 MB installed / 15 MB installer / 120 MB idle | Measured numbers against each budget, simulated profiles labelled as simulated |
| **S4 Acceptance** (RB-11) | Fix what sessions 1–2 and S3 found | **Owner review session 3**: final sign-off on a named build |

Three owner sessions replace the open-ended stream of single-screenshot
reviews. Between sessions the owner's standing feedback list in `STATUS.md`
is the queue; a note there does not interrupt the current task unless it is
marked blocking.

Each owner session gets a prepared packet: the capture board (reference beside
Rust, both themes), a short list of decisions needed, and the build to try. The
owner's answers are recorded verbatim in the ledger's acceptance fields.

## 6. Gates

| Tier | When | Commands | Budget |
|---|---|---|---|
| **Fast** | Every task | `cargo fmt --all -- --check`; `cargo clippy -p <changed crates> --all-targets --locked -- -D warnings`; `cargo test -p <changed crates> --locked`; review captures if visible; MCP test if the task changes model behaviour (3.7); `cargo deny check` and `cargo audit` if the task changes `Cargo.toml` or `Cargo.lock` | Minutes |
| **Full** | End of each day of work and before any owner session | Fast across `--workspace`; `cargo deny check`; `cargo audit`; MCP catalog parity check; `--smoke`; full capture board | Tens of minutes |
| **Ship** | S3, S4 | Full; `cargo build --release --locked`; `cargo bloat`; package staging and measurement; performance run | As long as it takes |

Evidence required per ledger row, by what the feature is:

| Feature kind | Machine evidence | Human evidence |
|---|---|---|
| Geometry, formats, commands, undo | Golden-file or behavioural test against the Python output | None |
| Layout, styling, artwork, states | Capture pair in both themes | Owner, in a batched session |
| Pointer and keyboard interaction | State-machine test of the gesture logic | One hand check per gesture family, recorded once |
| MCP | Catalog parity check, a live client round trip, and for every desktop action a test that the MCP tool gives the same result | None |
| Performance, package | Measurement on the named host | Owner sees the numbers |

A gate that cannot run is reported as not run, with the reason. It is never
replaced by a weaker one.

## 7. Product finish (S1)

The application looked home-made for structural reasons, not for lack of
effort: every control was painted by hand with its own literal sizes, so
nothing quite matched anything else. The fix is a small system, applied
everywhere, inside the owner's existing palette and artwork.

**Landed 2026-11-12** (captures in `evidence/rebuild/captures/2026-11-12-finish-pass/`):

- Viewport: 4x multisampling; wrapped key light plus a weak fill so vertical
  faces have form instead of one flat ambient tone; contact shadows as a
  12-step falloff instead of five visible bands; more room when framing.
- Shell: one control radius and hairline outline for every field and button;
  elevation shadow on floating surfaces; distinct line icons for list
  utilities instead of one repeated placeholder; a drawn visibility toggle in
  place of a glyph the font did not have; full-width selectable item rows;
  axis-coloured, aligned numeric fields; status bar no longer clipped.
- Tooling: the review-capture harness and the `review` build profile.

**Remaining S1 tasks**, in order:

1. `chrome.rs` becomes the single source for type scale (11 / 12.5 / 13 / 15 /
   20), spacing (4 / 8 / 12 / 16 / 24), radii (6 / 10) and control heights
   (26 / 28). Replace every literal in the shell with a token. Mechanic tier.
2. (Optional, owner 2026-11-12) Split part of `shell.rs` only where a task
   needs it; no behaviour change, captures identical before and after.
3. Inspector: replace the default collapsing header and stacked "Apply …"
   buttons with labelled sections and one consistent field grid; fill or
   remove the empty lower half of the right panel.
4. Ribbon: consistent group separators and labels, tab typography on the type
   scale, pin control integrated with the tab row; overflow menu styled as a
   menu, not a combo box.
5. Dialogs and popovers: one dialog frame, one button order, one error style.
6. Welcome: hero, action cards and recent grid on one column grid; designed
   empty, loading and missing-file states.
7. Viewport: depth fight at the floor (F9); selection outline; gizmo handles
   reviewed as a set in one owner pass rather than iteratively; default
   single-object framing.
8. States: hover, pressed, focus, disabled, busy and error audited for every
   control family against the capture board.
9. Accessibility: contrast test extended to every text/background pair in
   use; keyboard order and focus ring on every control.

Standing constraints: the owner's palette, artwork and layout are the design.
A token changes spacing and consistency, never the identity. Anything that
would alter the accepted RB-02 baseline is proposed in the session packet, not
decided by an agent.

## 8. Decisions for the owner

| # | Decision | Recommendation | Until decided |
|---|---|---|---|
| D1 | Allow local commits on a work branch (no push, no PR) as the recovery mechanism | **Approved 2026-11-12** | — |
| D2 | Keep fat LTO for shipping builds if it cannot link reliably | **Approved 2026-11-12:** thin LTO is acceptable if the 40 MB budget holds | — |
| D3 | Delete the ad hoc build directories under `target/` | **Approved and done 2026-11-12** | — |
| D4 | Viewport background in the dark theme | **Decided 2026-11-13:** a bit darker than now, but not full dark mode; before/after captures for owner approval | Unchanged |
| D5 | `emboss_text` long-running metadata | **Decided 2026-11-13:** show progress — keep the Rust "may take a while" flag (approved difference from Python) and show a progress bar, or a Builder3D-styled working spinner when the time can't be estimated, desktop and MCP | — |
| D6 | MCP at 100% desktop parity, growing the catalog beyond the Python 60 where needed | **Approved 2026-11-12** | — |
| D8 | Merge of parts that don't overlap | **Decided 2026-11-13:** group them instead of merging and show a small popup for about 4 s saying the objects were not touching; MCP returns the group with the same message | — |
| D9 | Smoothing on sparse meshes | **Decided 2026-11-13:** match Blender's default sculpt Smooth (the brush Shift switches to); studied with item 13, the Blender brush comparison | — |
| D10 | MCP coverage of look-and-feel controls (theme, recents, ribbon, shortcuts, About, exit …) | **Decided 2026-11-14:** MCP does everything; no "visual only" exemptions (OF-58) | — |
| D7 | Merge color: one color from the top-most object, desktop and MCP alike | **Approved 2026-11-12** (differs from the Python MCP, which kept per-face colors) | — |

## 9. What does not change

`L:\Builder3D` stays read-only and remains the reference. The contracts,
Rust rules, resource-routing requirements and platform seams in `AGENTS.md`
all stand. Nothing here promotes the Rust build over the Python fallback; that
still requires the owner's replacement gate. No owner approval is ever
inferred from silence, a passing gate, or a model's opinion of a screenshot.
