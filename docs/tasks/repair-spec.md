# Repair spec: how the Windows repair engine behaves (R1, 2026-11-22)

Phase R1 of [`repair-plan.md`](repair-plan.md). Everything below was measured
by calling the public WinRT API `Printing3DModel.RepairAsync` (the engine
behind 3D Builder) on small models made for the test, and measuring what came
back. Nothing was decompiled and no Microsoft code or tables were looked at or
copied. The rules in the "We implement" lines are our own, written from these
measurements.

Evidence: `evidence/repair-eval-2026-11-22/`

- `scripts/spec_experiments.py`: every experiment below, by section
  (`snap recut weld holes orient tiny time`). How to build the probe is at the
  top of the file.
- `scripts/spec_lab.py`: fixture writer (3MF), probe runner, measurements.
- `scripts/spec_holefill_dp.py`: our own min-area and Liepa hole-fill
  triangulations, used only to compare against the engine's choice.
- `spec-results.txt`: output of one full run of `spec_experiments.py`.

Names used below: **L** = largest side of the whole mesh's bounding box,
**D** = bounding-box diagonal, **s** = snap step (section 1). Windows 11
Enterprise 10.0.26200, `windows` crate 0.62.2, 3MF loaded with
`Printing3D3MFPackage`.

## Summary for the native engine (R4-R6)

The engine is simpler than the plan assumed. In order:

1. Every point is snapped to a grid of step `s = min(0.002 mm, L/2500)`
   (for models up to about 230 mm; larger ones get a coarser grid),
   anchored at the bounding-box minimum. Points that land in the same cell
   merge. That is the only "thin triangle" rule: anything thinner or shorter
   than one step disappears; anything wider stays.
2. Points on open edges within about `5 s` (0.01 mm) of each other weld.
   Points of closed parts never weld, even 0.005 mm apart.
3. Open edges are **not** zipped together across larger gaps. Each open loop
   is filled on its own with the **minimum-total-area triangulation**, no new
   points, no smoothing.
4. Every closed part is then treated as solid, whatever its winding, and the
   result is the **outside surface of their union**: overlaps merge, nested
   parts and **all cavities (even correct ones) are filled**, fins and loose
   sheets go, self-crossing parts are resolved. Output is outward-facing.
5. Closed parts smaller than about 50 grid steps (0.1 mm) are removed.
6. Edges longer than `0.303 D` are bisected (the "re-cut"). It changes no
   shape; we do not copy it.

One owner decision is needed (cavities, end of this file). One risk for R3:
an unwelded "soup" model with gaps over 0.01 mm comes back almost empty, so
the R2 gate must check volume (section 3).

## 1. Point snapping

**Test models.** Spheres (r 1, 15, 40, 100 mm) with random 0.01 mm noise and
odd offsets; ellipsoids of many proportions; boxes of many sizes; a
tetrahedron whose apex x is swept 4.994 to 5.011 mm in 0.0001 mm steps.

**Measured.**

- Every output point lies on a lattice whose origin is the **input bounding-box
  minimum** (not 0,0,0): off-lattice error at most 0.0016 of a cell, which is
  float32 rounding. Output coordinates are float32 values (for example
  109.998 comes back as 109.9980011), so the engine works in float32.
- Step is the same on all three axes: `s = min(0.002, L/2500)` up to about
  230 mm. Measured 0.0004 (L 1), 0.0008 (L 2), 0.0012 (L 3, and a 1x2x3
  box), 0.0016 (L 4, including 4x1x1 and 0.5x4x4), 0.002 from L 5 to 230
  (5x1x1, 10, 30, 50, 100, 200, 230 mm).
- Larger models get a coarser grid: 0.004 (L 250 to 330), 0.006 (360, 400),
  then about `L / 66,900` (0.015 at 1 m, 0.030 at 2 m, 0.060 at 4 m, 0.120 at
  8 m). Most likely a float32 precision limit inside the engine.
- Rounding is **floor** (toward the box minimum) with a small deterministic
  wobble: the same input always gives the same output, but the cut-over point
  sits up to about 0.15 of a cell (r 15 sphere) to 0.4 of a cell (r 40)
  either side of the grid line. Of about 5,960 coordinates per sphere, 92-94%
  land exactly on the floor cell, the rest one cell either way, and only when
  the point was within that wobble of a grid line. Points exactly on a grid
  line at the top of the box usually drop one step: a 10 mm box comes back
  9.998 mm (volume 999.4 instead of 1000).
- **Every point moves**, not only bad ones. Largest move: 0.0035 mm (r 15),
  0.0038 mm (r 40), 0.0050 mm (r 100), 0.0013 mm (r 1, step 0.0008). On the
  book the largest move was 0.027 mm, which is more than snapping alone; it
  comes from the later steps (welding, union).

**We implement.** Same lattice and step (origin at the bounding-box minimum,
`s = min(0.002, L/2500)` at every size; we work in f64, so large models do
not need Windows' coarser grid), but **round to the nearest** grid point instead of
floor-with-wobble. Same welding effect, half the largest move
(`s·√3/2` = 0.0017 mm) and no systematic shrink of the model. This is the
"small shape changes are fine" allowance (owner answer 2) used more gently.
Only on Windows' wobble do we not match point-for-point; that is acceptable
because no user can see a 0.002 mm difference.

## 2. Weld and stitch

**Test models.** A sphere (r 1.5, 15, 150) with one point duplicated at
distance d and half its fan moved to the copy; an unwelded "soup" sphere with
every corner jittered by up to ±j; a box whose top face is detached and lifted
or slid by a gap g; a sphere whose top cap (448 faces) is detached and lifted
by g; a box edge with a T-junction whose middle point sits dz off the edge.

**Measured.**

- **Weld radius ≈ 5 s.** Duplicated point merged up to 0.010 mm and kept
  apart from 0.012 mm on the r 15 and r 150 spheres (s 0.002); one earlier
  r 15 run with the copy along the normal kept it apart at 0.010, so the edge
  is right at 0.01 mm. On the r 1.5 sphere (s 0.0012) merged up to
  0.004 and kept apart from 0.006. The merged point ends up at one of the two
  positions (which one varied), never further away.
- Unmerged pairs leave a thin crack; it is filled like any hole (2 new
  triangles), both points kept.
- Soup sphere: jitter up to ±0.002 (pairs up to about 0.007 apart) comes back
  identical to the welded sphere (1986 points, 3968 faces). ±0.004 is still
  closed with 25 extra points.
  **±0.01 and more is a disaster:** volume 14,080 becomes 1.3 (±0.01), 0.76
  (±0.02), 0.06 (±0.05). Each loose triangle is closed into a flat, empty
  shell and thrown away.
- **No zipping.** Sphere cap lifted 0.02 to 2 mm: the result is **two closed
  parts**, the body with its 64-edge hole filled flat (62 triangles) and the
  cap closed underneath (62 triangles), with the gap kept. Below 0.01 mm the
  rings weld and it is one sphere again. A detached flat box top (any gap
  0.001 to 1 mm, lifted or slid) is dropped and the box's hole is filled flat.
- Closed parts never weld: two cubes 0.005 mm apart stay two parts.
- **T-junction:** a point up to about 0.005 mm off the edge is absorbed (no
  hole); at 0.02 mm and more the 3-edge gap is filled with one triangle.

**We implement.**

1. After snapping, weld **open-edge points only** to the nearest open-edge
   point within `5 s`, using true distance (neighbour cells checked), keeping
   the existing point.
2. Split an open edge where an open-edge point lies within `5 s` of it
   (T-junction), then weld.
3. Never zip loops together across a larger gap: leave that to hole filling
   (section 4). This replaces the "stitch within 0.05 mm" idea in
   `fixrepair.md` Step 3 item 2, which Windows does not do.

## 3. Thin triangles (slivers and needles)

**Test models.** The f10 shape: a 10 mm box whose top has a point
`(5, h, 10)` near one edge, sealed by a sliver of height h
(h 0.00001 to 0.2 mm). A needle fan: a point at distance e from a top corner
(e 0.00001 to 0.2 mm).

**Measured.**

- Sliver height below one step (h < 0.002): the sliver disappears and its tip
  becomes a point **on** the long edge; the neighbouring side face is split
  there. Sealed, no hole. Height 0.002 and above: the sliver **stays**
  (smallest height in the result equals the snapped h).
- Needle: short edge below about one step merges into the corner (e ≤ 0.002).
  From 0.004 up the needle stays. No zero-area faces were left in any result.
- No rule by angle, area or edge ratio was found: a 0.002 mm-high sliver on a
  10 mm edge (ratio 1:5000) is kept.

**We implement.** After snapping, a triangle with two equal corners is
removed (edge collapse with the link check so the surface stays sealed). A
triangle whose corners are distinct but in a line has its middle point
inserted into the neighbour across the long edge (edge split), then is
removed. **Never delete a triangle and leave a hole.** No angle or area
threshold. The one degenerate test (`inspect_mesh` and Repair) is
"zero area after snapping".

## 4. Holes

**Test models.** An open-top cylinder r 10 with a 3, 4, 10, 64, 500, 1000 and
2000-edge hole; the same with a far-away 1 mm cube added so the re-cut
(section 5) does not touch the patch; 12 random non-planar 6-12 edge holes
(open "cups"); a window of 4, 16 and 32 columns by 4 rows cut from a curved
tube wall.

**Measured.**

- Every hole is filled with exactly **n - 2 triangles and no new points**
  (3 → 1, 4 → 2, 10 → 8, 64 → 62, 500 → 498).
- Flat holes are filled exactly flat (spread 0.0000 mm).
- The triangles chosen are the **minimum total area** triangulation: 10 of 12
  random non-planar holes matched our own min-area result triangle for
  triangle; Liepa's rule (smallest worst bend, then area) matched 0 of 12. The
  2 misses were holes where two boundary points snapped together, so our
  point-matching failed, not a different rule.
- It is not a fan (largest point valence 12 on the 64 hole, 14 on 500).
- Curved holes follow their boundary only: in the tube wall the patch bridges
  the top and bottom rows (patch centres at radius 9.73 to 9.99 for a 10 mm
  wall; a flat chord would sit at 0 to 7 mm) but no point is added, so the
  patch is a faceted strip, not a smooth curve.
- Without the far cube, the global re-cut (section 5) then bisects long patch
  edges, which is where the "interior points" in a normal run come from.
- Cost grows with the cube of the hole size: 64 edges 0.05 s, 500 edges
  1.2-1.6 s, 1000 edges 8.9-9.2 s, 2000 edges 70 s (two runs).

**We implement.** For each open loop: the minimum-total-area triangulation
(dynamic programming, O(n³)) up to 300 edges. Above 300 edges, use a cheaper
method with the same goal (for example greedy smallest-area ear clipping);
Windows' cubic time is too slow for an app. No refinement and no smoothing;
this replaces "Liepa-style fill, refine and smooth" in the R5 card.

## 5. Flat faces re-cut (12-face box → 96)

**Test models.** Boxes 10, 1x10x10, 10x1x1, 100x1x1, 10x10x0.5; a tetrahedron;
low-poly spheres (8x4, 6x3); a small cube of side e placed beside a fine sphere
(r 15 and r 30) with e swept in 0.25 mm and 1 mm steps.

**Measured.**

- It is **longest-edge bisection**: each box triangle becomes 8 (split the
  long edge, then again, then again); new points are edge midpoints, so the
  shape is unchanged.
- It has nothing to do with flatness: a 6x3 low-poly sphere also gets split
  (24 → 72), a 8x4 one does not (48 → 48).
- Threshold: an edge is split when it is longer than about **0.303 × D**, the
  diagonal of the whole mesh's bounding box. Measured: kept at 0.3018 D and
  split at 0.3045 D (r 15 scene), kept at 0.3033 D and split at 0.3095 D
  (r 30 scene). All box, tetrahedron and sphere results above fit this.
- So a clean 10 mm box comes back with 96 faces, and a model that already has
  small triangles everywhere (any real scan or the book) is barely touched.
  The likely reason is to keep triangles small enough for the engine's
  internal spatial work; this cannot be seen from outside.

**We implement.** Nothing. It fixes no problem and adds faces. Do not copy
it, and remove `regularize_planar_regions` as planned. Comparisons with
Windows must use geometry (open edges, shells, volume, distance), never face
counts.

## 6. Flipped faces, inside-out parts, several parts: what decides "outside"

**Test models.** Spheres with 20% of faces flipped (f4), fully inside-out
(f9); two separate spheres with one inside-out; nested spheres in every
winding combination; three nested spheres.

**Measured.**

- **The input winding is ignored.** Every closed part is treated as a solid
  and the output is the outward-facing outside surface. f4 and f9 are fixed;
  two separate spheres with one inside-out come back as two outward spheres
  (volumes 13,911.6 and 2,110.4).
- Nested parts: whatever the windings (both outward, both inside-out, outer in
  and inner out, three nested, a ball inside a cavity), the result is **only
  the outer sphere**, volume 13,911.97, 960 faces.

**We implement.** Same: fix winding per part by flood fill, then make each
closed part face outward by its signed volume, then the union of section 7
decides what survives. Exception: cavities, see the owner question below.

## 7. Overlaps ("Wrap"), cavities, fins and non-manifold edges

**Test models.** f6 (two overlapping boxes); two overlapping spheres (one
inside-out as well); a hollow ball (outer outward, inner inward: a correct
cavity); fins on a box edge pointing out, over the top and inside; a loose
internal sheet; a loose double-sided triangle; a flat loose sheet; an open
hemisphere; two cubes sharing an edge, a vertex, a face, and 0.005 / 0.05 mm
apart; a box with one corner pushed through its bottom (one part crossing
itself).

**Measured.**

- Overlaps merge into one solid: f6 volume 16,000 → 14,997.6 (true union
  15,000); two spheres 27,823 → 23,526. Same when one input part is
  inside-out.
- **Cavities are filled, even correct ones.** The hollow ball (volume
  11,801) comes back solid (13,912).
- Fins (all three directions), the loose internal sheet, the loose
  double-sided triangle and loose flat sheets are **removed**. An open but
  curved sheet (hemisphere without its base) is **closed into a solid**
  (volume 260.7), not removed.
- Two cubes sharing an edge or a vertex: one connected part, no
  non-manifold edges by index (volume 1,999.6 and 1,999.4). Face to face: merged
  into one 20x10x10 solid. 0.005 and 0.05 mm apart: two parts, not merged.
- A part crossing itself is resolved to its outside surface (volume
  500 → 536.8, 12 → 122 faces); a sphere with its cap pushed through itself
  likewise (960 → 2,432 faces, one part, closed).

**We implement.** Union all closed parts with Manifold (R6), after the
outward-winding step, so overlaps and nested parts merge and self-crossing
parts resolve. Before the union: remove open fragments that close to zero
volume (fins, loose and internal sheets, double-sided triangles); close a
curved open sheet with the hole fill and keep it. Non-manifold edges are
split into separate sheets before winding repair. Cavities: per the owner's
answer below.

## 8. Tiny parts

**Test models.** A big cube of side B (0.5 to 3,000 mm) with a small cube of
side s one B away, s swept on a 80-step log scale; a 1 x 1 x 0.01 mm plate
beside a 10 mm cube.

**Measured.** Smallest small cube that survives:

| Big cube B (mm) | 0.5 | 1 | 2 | 3 | 5 to 100 | 300 | 1,000 | 3,000 |
|---|---|---|---|---|---|---|---|---|
| Smallest kept (mm) | 0.021 | 0.041 | 0.084 | 0.103 | 0.10-0.12 | 0.159 | 0.57 | 1.57 |

The gap between the two cubes did not matter (B 10 with gap 2, 20 and 200
gave the same 0.115). The 1 x 1 x 0.01 plate survives, so it is about size,
not volume. Measured in grid steps (section 1) the cut-off is about **50 s**
for every scene up to 230 mm (0.021 / 0.0004 = 52, 0.084 / 0.0016 = 52,
0.041 / 0.0008 = 52, 0.10 / 0.002 = 50); for scenes above 230 mm it grows
with the scene but more slowly than the coarser grid there.

**We implement.** Remove a closed part whose largest side is below `50 s`
(0.1 mm for any model of 5 mm or more). Open fragments are handled in
section 7.

## 9. Time against face count (for the R3 progress dialog and time limit)

**Test models.** Spheres r 30 with 4k to 1M faces: clean, with 1% of faces
removed, and as unwelded soup (±0.0003 mm jitter). One flat hole of 500 to
2,000 edges.

**Measured** (this PC; the second run in `spec-results.txt` agrees within
5%, the 1M row was run once):

| Faces | Clean | 1% holes | Soup |
|---|---|---|---|
| 3,968 | 0.10 s | 0.10 s | 0.23 s |
| 16,128 | 0.43 s | 0.43 s | 1.1 s |
| 65,024 | 1.9 s | 1.9 s | 6.0 s |
| 261,120 | 12.1 s | 12.5 s | 32 s |
| 1,046,528 | 128 s | 125 s | 225 s |

The book (334,180 faces) took 21.5 s in the planning session. Time grows
about as `F^1.28`: `t ≈ 0.1 s × (F / 4000)^1.28`, up to 2.6 × for soup.
A single huge hole adds cubic time on top (1,000 edges 8.9 s, 2,000 edges
70 s). Loading the 3MF and copying buffers are not included (under 1 s up to
260k faces).

**We implement (R3).** Show the progress dialog for any model over 20,000
faces (over about 1 s). Allow cancel at any time. Time limit: 3 × the
estimate above, at least 30 s and at most 10 minutes; on time-out cancel,
fall back to the native engine and say so. `RepairAsync` reports no
progress, so the dialog shows elapsed time against the estimate.

## Rules for the R3 Windows route that come out of this

- The engine returns float32 coordinates and moves every point by up to about
  0.005 mm (0.03 mm seen on the book). The R2 gate must allow that.
- The gate must also catch a collapsed result: refuse if the volume drops by
  more than a few percent or the bounding box shrinks by more than about
  0.1 mm, unless the input had overlaps or open sheets that explain it. The
  ±0.01 mm soup test (14,080 mm³ → 1.3 mm³) shows the engine can wipe out a
  model without reporting an error.
- It fills cavities. Until the owner answers below, the Windows route should
  not be used on a model with a correct inner cavity (a closed part fully
  inside another, facing inward). Use the native engine for those.

## Owner question (needed before R6)

**Hollow models.** Windows' repair fills every cavity, including a correct
one, for example a model made hollow with our Hollow tool for printing. After
a Windows repair that model would be solid again. Recommendation: **keep
correct cavities** (an inward-facing closed part fully inside another part),
and only fill inner parts that face the wrong way or overlap. That differs
from Windows on purpose, and only for hollow models.

## Questions from the plan, answered

| Question (plan R1) | Answer | Section |
|---|---|---|
| Point snapping grid? Which points move, by how much? | Grid `min(0.002, L/2500)` from the box minimum (coarser above 230 mm), floor with wobble; every point moves, up to 0.005 mm | 1 |
| Weld / stitch distance, T-junctions | Open-edge points weld within 5 s (0.01 mm); no zipping beyond; T-junctions within about 0.005 mm absorbed, larger filled | 2 |
| Thin triangles: which, by what rule, which way | Only via the snap: below one step they collapse into the long edge or corner; no angle or area rule | 3 |
| Holes 3, 4, 10, 64, 500 edges; flat vs curved; interior points; triangle sizes | Min-area triangulation, n - 2 triangles, no new points, flat stays flat, curved follows the boundary only; size from the re-cut | 4 |
| Flat faces re-cut | Longest-edge bisection of edges over 0.303 D, any surface; not copied | 5 |
| Flipped, inside-out, several parts: what is outside | Input winding ignored; outside surface of the union of all closed parts | 6 |
| Overlaps, cavities, fins, non-manifold | Union; cavities filled; fins and flat loose sheets removed; shared edges kept as one part | 7 |
| Tiny shells cut-off | About 50 grid steps (0.1 mm) largest side | 8 |
| Time vs faces | `0.1 s × (F/4000)^1.28`, soup up to 2.6 ×; 1M faces 128 s | 9 |

Not observable from outside: why the re-cut threshold is 0.303 D and what the
snap wobble is made from. Neither is needed, since we do not copy either.
