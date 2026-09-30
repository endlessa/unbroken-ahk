# The geometry kernel: what was decided, and why

A record of the decisions taken in the CSG kernel, the triangulators, the
export funnel and the evaluator — each with the measurement that forced it,
the alternatives that were tried and rejected, and what is still open.

It is written this way on purpose. A decision recorded without its evidence
is a decision nobody can revisit: the next person to look at it can only
take it on trust or tear it out. Every number below was measured on this
machine against the code in this repository, and where something was tried
and failed, the failure is here too.

## How to read this

Each section states the DEFECT first — usually as an exported mesh that was
wrong — then the decision, then the evidence. Measurements are quoted as
they came out, not rounded to flatter. Where a figure was taken on a loaded
machine and is therefore unreliable, it says so.

The validator referred to throughout reports, for an exported mesh: the
triangle count, the signed volume (positive iff the solid is wound the right
way round), `holes` — undirected edges used an odd number of times, `flipped`
— directed edges traversed the same way by both their faces, `split` —
T-junctions, and the number of connected components.

---

## 1. The union

### 1.1 The tolerance that could not work

The union reduction used to keep a pair's CONCATENATION whenever the merged
volume came back within 1e-4 of the sum of the operands, reasoning that a
union which joined nothing has the volume of a union which joined nothing.
Three exports show why that cannot work.

- **Two 40 mm boxes overlapping by six microns** share 9.6 mm³ of 128,000 —
  7.5e-5 of the pair. They exported as two interpenetrating boxes with no
  diagnostic. The shared volume of two solids that genuinely meet has no
  lower bound, so no threshold separates them from solids that never met.

- **The threshold was a fraction of the volume accumulated so far**, which is
  the whole subtree and not the two operands. A full millimetre of overlap
  between the same two boxes — 1,600 mm³ — survived on its own and vanished
  when a 300 mm cube 500 mm away joined the union. It vanished for one
  ordering of the children and not another, which the language reference
  forbids outright: *"Order of children never affects the result."*

- **Two cubes sharing exactly a face** have a union volume identical to their
  sum, so the test passed EXACTLY and the concatenation was kept forever. The
  export carried four triangles on the shared plane — an interior wall in a
  solid the reference says is one solid, its *"interior face dissolves"*.

**Decision: remove the tolerance rather than tune it.** No value of it fixes
any of the three.

### 1.2 What replaced it

A union is now the CONNECTED COMPONENTS of the relation "these two might
share volume", where the relation is answered geometrically:

1. bounding boxes, which settle most pairs outright;
2. a containment ray, which rules out one solid nested wholly inside another
   — the case a surface test cannot see;
3. a separating axis between every pair of triangles in the shared region.

The separating-axis test is exact for triangles (two face normals and nine
edge-edge cross products) and answers in the SUFFICIENT direction only:
"certainly apart" or "cannot say". "Cannot say" costs the boolean that was
going to run anyway, so a wrong answer in that direction is never a
correctness problem — only a cost.

Components are independent, and that is where order-independence comes from:
the set of components does not depend on the order the children were written
in. The pairwise tree this replaced could not manage it, because it merged
neighbours in written order.

### 1.3 Ordering the merges inside a component

Within a component, pieces that do not touch EACH OTHER can still be
concatenated. A girder that four hundred deck planks rest on is four hundred
edges in the touch graph and no edge anywhere else. Merged one at a time,
the growing girder is re-cut at every step: measured on the suspension
bridge, 32 triangles into 18,746, then 24,377, then 30,140, and the export
did not finish in ten minutes.

**Decision: colour the touch graph greedily by descending degree.** No two
adjacent pieces share a colour, so each colour class holds solids that share
no volume and is concatenated outright; only the CLASSES go through a BSP.
A hub becomes two classes and one boolean. Greedy colouring needs at most
one more colour than the largest degree, and on a hub it needs two.

### 1.4 The budget

A BSP plane is infinite, so every polygon straddling one is cut whether the
boolean touches it or not. Past some size that has to be refused.

The budget used to be the WHOLE MODEL's triangle count, gated on whether any
two bounding boxes overlapped. A box test on a model with one long part is
always true, so a bridge of 501 solids that share no volume — and need no
boolean at all — was one triangle from being refused wholesale.

And the refusal was worse than it looked. Two spheres of radius 10 with
centres 12 apart have an exact union of 7,506.31 mm³:

| | input triangles | result | volume | holes |
|---|---|---|---|---|
| `$fn` = 110 | 23,800 | union runs | 7,476.12 | 190 |
| `$fn` = 130 | 33,792 | union refused | 8,369.43 | 0 |

An 11.8% volume error crossing a triangle-count threshold, and **the side of
it that is wrong is the side that validates clean**. Nothing downstream can
tell.

**Decision: weigh a connected component, not a model.** Only a component
ever reaches a BSP. A model of any number of disjoint parts now merges
without hesitating because there is nothing to merge — 2,400 cubes, 28,800
triangles, comes out concatenated exactly, and that is pinned by a test. A
component genuinely over the budget is still refused, and still says so.

The threshold itself (25,000) rests on measurements taken before this
rewrite and before list values were shared, and **is known to be stale**. See
§8.

### 1.5 The broad phase

The disjointness grid set its resolution from the TRIANGLE COUNT, which
assumes the triangles are all much the same size and spread evenly. A real
part is neither.

A gear slice put a body of revolution — a few large annular faces — against
a toothed ring of 20,240 small ones. The count asked for 28 cells per axis,
the large faces each landed in hundreds of them, and those cells then
answered every query with hundreds of candidates. The pair test ran past
**107 million triangle pairs** without finishing and gave up. Giving up is
conservative and so still correct, but it cost the model its export:
eighteen solids that share no volume were declared one component and refused
by the merge budget.

**Decision: size the cell to the mean triangle, not the triangle count**, so
a cell holds a few triangles whatever the mix. The mean is SAMPLED over 64
triangles rather than summed over all of them — this test's whole job is to
be cheaper than the boolean, and a bridge asks it 125,250 times; summing
took the bridge from 2.9 seconds to 82.

| | before | after |
|---|---|---|
| gear slice | 98 s, refused | 14 s, all 18 proved disjoint |
| DNA segment | 2.4 s | 0.2 s |
| geodesic | 4.1 s | 0.8 s |

---

## 2. Triangulation

### 2.1 A twisted wall is a ruled patch

`linear_extrude` with a twist builds each wall as a quad between two rotated
rings. The quad is not planar, and a diagonal has to pick a side of it. Both
sides are wrong by the same tetrahedron — the one on the quad's four corners
— and around a ring that error does not cancel, so it lands in the volume
and stays there.

A rotation preserves area at every height, so a 20 mm square twisted 90°
over 10 mm encloses exactly 4,000 mm³ whatever the slice count:

| slices | diagonal | ruled fan |
|---|---|---|
| 4 | 4,408.75 | 3,898.51 |
| 16 | 4,124.27 | 3,993.58 |
| 64 | 4,032.32 | 3,999.60 |
| 256 | 4,008.16 | 3,999.97 |

The diagonal is first order — still 0.2% wrong after 256 slices — and it is
OVER, which no chordal approximation of a convex profile can honestly be.

**Decision: fan the quad through the mean of its four corners.** The ruled
patch's volume is exactly halfway between the two diagonals, so a fan
through the corners' mean straddles it instead of choosing a side. The error
becomes second order and approaches from below.

**Only a wall that is actually bent pays the extra two triangles.** An
untwisted extrusion's walls are planar, and so are a uniformly scaled one's
— those are cone faces through the apex — so both keep two triangles per
quad, which is exact for them. The test is a planarity check on the quad,
not a check of the twist parameter.

A square annulus at the default slice count went from 3,338.45 — 11.3% over
3,000 — to 2,980.79. A letterform went from 0.77% OVER its own predicted
volume to 0.002% under.

### 2.2 `polyhedron`'s faces: fan to ear

`polyhedron` fan-triangulated every face from vertex 0. The reference says
faces with more than three vertices are *"fan/ear triangulated internally"*
and marks the exact choice as one to pin against an oracle — which this
project may not read. So the choice is made on the merits, and the fan loses
on two ordinary faces:

- **A concave face** fanned from a reflex corner covers ground outside
  itself. An L-shaped face is the smallest example.
- **A face with a collinear vertex at the fan apex** makes a zero-area
  triangle. That one is worse, because the mesh looks fine until export: no
  format can write a facet whose three corners are collinear, the export
  funnel drops it, and two edges of a closed mesh go with it.

**Decision: ear-clip in the face's own plane by Newell's normal**, with one
step of lookahead so a flat corner is never stranded as the last three
vertices. The lookahead matters: a plain "largest ear" rule leaves the
collinear triple to the end, where it is emitted with no choice at all.

### 2.3 The weld's projection plane: checked, not argued

When a triangle is re-cut (§3), which plane to project in is a choice with
no winner.

- Newell's normal is the better conditioned for choosing ears. Using the
  triangle's own instead tripled a gear's boundary-edge residue, 89 to 329.
- But on a sliver, Newell's area-weighted sum nearly cancels, and a
  triangulation done in a plane that is not really the face's can come out
  overlapping itself and wind a piece backwards. A character model grew two
  inconsistently wound edges that way.

Orienting Newell's normal by the triangle's own does NOT fix the second: the
fault is in the geometry, not the sign — tried, and gnome still came back
with two flipped edges.

Inconsistent winding is much the worse of the two. An edge whose two faces
run the same way round makes the mesh non-orientable, and every boolean on
it silently loses geometry; a boundary edge is merely visible.

**Decision: measure the outcome instead of choosing.** Cut in Newell's
plane, check every piece against the triangle's own normal, and cut again in
that plane only if one disagrees. It costs a cross product per piece on the
faces that are split at all.

| | Newell | own plane | checked |
|---|---|---|---|
| elliptical | 89 / 2 flipped | 329 / 0 | **89 / 0** |
| gnome | 2,214 / 2 | 2,239 / 0 | **2,213 / 0** |
| orc | 2,033 / 0 | 2,062 / 0 | **2,013 / 0** |
| human | 1,459 / 0 | 1,442 / 0 | **1,424 / 0** |
| elf | 1,380 / 0 | 1,399 / 0 | **1,377 / 0** |
| city hall | 102 / 0 | 98 / 0 | **98 / 0** |

Every model at or below both fixed choices.

---

## 3. T-junctions

A BSP cuts one side's polygons against the other side's planes, so a vertex
of one surface comes to rest partway along an edge of the other. The volume
is right and the surface is continuous, but that edge is used once by the
triangle that owns it and twice by the pair opposite, so it reads as a hole.

Two cubes overlapping by a quarter unioned to exactly 5,000 mm³ with twelve
boundary edges. **It meant no union of solids that actually MEET could ever
come out clean**, which is a large part of what the modeller is for.

**Decision: insert the stray vertex into the triangle that owns the edge and
re-cut the triangle around it.** No geometry is added — the point is already
a vertex of the mesh and already lies on the edge — so volume and area are
unchanged.

| | before | after |
|---|---|---|
| heart | 24,289 | 1,130 |
| orc | 19,145 | 2,013 |
| human | 16,557 | 1,424 |
| gnome | 19,801 | 2,213 |
| elf | 16,869 | 1,377 |
| city hall | 6,753 | 98 |
| elliptical | 2,332 | 89 |
| two spheres | 15,919 | 190 |
| two cubes | 12 | 0 (28 triangles instead of 24) |

### The tolerance was swept, not chosen

1e-9 of the model. **Loosening it does not catch more T-junctions, it
MANUFACTURES them:**

| tolerance | human: holes | flipped |
|---|---|---|
| 1e-9 | 1,459 | 0 |
| 1e-7 | 1,500 | 1 |
| 1e-6 | 1,642 | 8 |
| 1e-5 | 2,841 | 18 |
| 1e-4 | 6,565 | 75 |

A vertex inserted into an edge it is not really on MOVES that edge, and the
triangle on the other side — which got no such insertion — stops matching.
The volume moves too.

One other idea was tried and did nothing: excluding split points closer to a
corner than the writer can resolve, on the theory that they make triangles
the export funnel will drop. Measured change: human 1,442 → 1,438, the rest
identical. It was reverted rather than kept as unjustified code.

---

## 4. Export formats

### 4.1 Representability is per format

A triangle whose three corners land on one point in the target file's
coordinate grid cannot be written, and the export funnel drops it. That
guard used to apply f32's answer — binary STL's — to EVERY format.

f32's step at a coordinate of 1e6 is 0.119, so a millimetre-sized face a
kilometre from the origin has all three corners on one f32 point. But OFF,
AMF, 3MF and ASCII STL print `{:.6}` and resolve it a thousand times over.
One guard for all of them lost it from every format, for a limitation only
one of them has.

**Decision: drop only what the CHOSEN format cannot write.**

### 4.2 The six boundary edges that were the format

A letterform carried six boundary edges for a long time. Three repairs were
tried against the triangulator and reverted, because none of them moved the
number. The assumption behind all three was wrong.

The plate is a 256-point superellipse with a = 62, and near its rightmost
point three consecutive outline vertices differ in x by less than ONE f32
step, which at a coordinate of 62 is 3.8e-6. The cap triangle between them
is real in f64 — cross product 6e-6 — and writes all three corners at
exactly 62.0 in a binary STL. **Two triangles in 232,458, carrying all six
edges.**

Exporting the same design to OFF and diffing the two settled in one command
what three speculative repairs could not. The OFF export is closed.

**The lesson, recorded because it will recur: when a mesh is closed on the
way out of the modeller and open in the file, suspect the writer before the
geometry.**

### 4.3 Say so

**Decision: when dropping unwritable triangles OPENS a mesh that was closed,
warn**, naming the count, the format's step, and the three ways out —
another format, nearer the origin, or a coarser outline.

This made two silent losses visible immediately: `cycloidal` loses 48
triangles and 62 edges, and `epicyclic` loses 1,988 of 7.9 million faces —
a 170 mm model tessellated finer than binary STL can carry at that size,
which is a format limit rather than a bug.

---

## 5. The evaluator: list values are shared

The evaluator hands values back BY VALUE from every variable read and every
argument bind, and a list value held a plain `Vec`. Each of those deep-copied
the whole list before throwing all but one element away.

| | n = 2,000 | n = 8,000 |
|---|---|---|
| reading a list once per element | 0.06 s | 0.72 s |
| folding one through a function | 0.25 s | 6.90 s |

The fold is the worse and the harder to avoid: a language with no mutation
has no other way to sum a list.

**Decision: `Rc<Vec<Value>>`, so the clone is a refcount bump.** Sharing is
safe because the language has no mutation — a list, once built, is never
written to. The same two measurements become 0.01 s and 0.42 s, and the
spherical gear library — which is nothing but table lookups — went from
1.74 s to 0.25 s for the same 10,268 triangles.

The property is pinned by a test on `Rc::strong_count` and `Rc::ptr_eq`
rather than by a stopwatch, because the property is what matters and a
stopwatch would be flaky.

### A measurement that nearly buried it

The first attempt to reproduce the report ran the test with `-o /dev/null`.
That path has no extension, so the CLI rejected the format and exited
BEFORE evaluating anything. Every size came back at 0.01 s and looked
perfectly linear.

**A measurement that makes a reported problem vanish should be suspected
before the report is.**

---

## 6. What the kernel found in the models

A kernel that proves things finds models that were wrong.

### The bridge's hangers were inside its cable

The suspension bridge promises 20 mm of air everywhere two members meet, and
echoes "no two solids share volume". The new union reported 96 hangers and
one cable as ONE connected component instead of 97 separate solids — twice,
once per cable plane.

The cable's ring is perpendicular to the axis, so where the cable slopes its
bottom VERTEX sits `RCAB·sin θ` along the cable from the panel point, and the
tube's underside directly over the hanger is lower than that vertex by
`RCAB·sin²θ/cos θ`. At the steepest hanger — 29.25° — that is 120 mm. Taking
the relief from the vertex put hanger tops **inside the cable by up to 97 mm**
near the towers, and only at midspan, where the cable is flat and the two
coincide, was the promised 20 mm actually there.

The relief is now read off the cable's drawn underside over the hanger's own
footprint, and the file CHECKS it rather than asserting it: it prints the
worst relief under the drawn cable and under the ideal circular cable —
`RCAB/cos θ` below the axis, independent of everything the relief uses — and
both come out at 20 mm.

The export is 501 solids in 501 components, 18,808 triangles, no boundary
edges, and no boolean at all. Through OFF it measures 80,997.784281 mm³
against the file's predicted 80,997.8: agreement to every digit the echo
prints. Through binary STL it reads 80,997.92, which is 1.7 ppm of f32 — the
file, not the model.

### And the repair was quadratic

That same fix called a function which builds a whole cable ring, once per
panel, inside a scan over every panel. The bridge took **84 seconds to reach
the exporter**, which looked exactly like a union regression until splitting
the export into `.echo` and `.stl` located it in evaluation in one run. The
polyline is built once now: **0.8 seconds**, same 18,808 triangles.

---

## 7. Decisions at a glance

| decision | rejected alternative | the evidence |
|---|---|---|
| Union by disjointness proof | a volume tolerance | overlaps of 9.6 mm³ and 1,600 mm³ lost silently; order-dependence |
| Components, then graph colouring | pairwise reduction | 32 tris into 18,746 into 24,377 into 30,140; ten minutes, unfinished |
| Budget per component | budget per model | 501 disjoint solids one triangle from refusal; 11.8% error on the clean-looking side |
| Grid sized by triangle size | sized by triangle count | 107 million pair tests, then a give-up |
| Ruled fan on bent walls | a diagonal | 4,408.75 against an exact 4,000, first order, and over |
| Ear clip in `polyhedron` | fan from vertex 0 | concave faces covered; zero-area triangles dropped at export |
| Weld plane chosen by checking | either plane fixed | 89 vs 329 boundary edges one way, 2 flipped edges the other |
| Weld tolerance 1e-9 | anything looser | 1e-4 gives 6,565 holes and 75 flipped, from 1,459 and 0 |
| Per-format representability | one guard for all | a face OFF resolves a thousand times over, lost from OFF |
| Warn when a drop opens a mesh | silence | six boundary edges chased for days that were the file format |
| `Rc<Vec<Value>>` | deep copy | 6.90 s to 0.42 s; gear library 1.74 s to 0.25 s |

---

## 8. What is open

Written down rather than guessed at.

### 8.1 The residue after welding is not T-junctions

On the elliptical gear's 89 remaining open edges, 39 of the first 40 have
another vertex in their interior — offset from the edge by 8.9e-6 to 4.9e-2
of the model extent, median 2.1e-4. Those are not points ON an edge. They
are points the BSP MEANT to put on an edge and missed, which makes them
cracks rather than junctions.

The weld cannot reach them and must not try (§3). The fix belongs where the
points are computed — the plane-segment intersection in the BSP split — and
probably means snapping a new vertex to an existing one when it lands within
the plane tolerance of it, rather than emitting a fresh point.

### 8.2 Passing a list to a function still costs O(|V|^0.7) per call

Sharing list values took the fold from 6.90 s to 0.42 s at n = 8,000. It is
not linear yet, and the residue has been narrowed without being found.

What is ruled out, by counting rather than reasoning:

- **Not indexing.** Reading by index is linear now — 0.01, 0.03, 0.05 s at
  n = 8,000, 16,000, 32,000 — and a fold that never reads the list costs the
  same as one that does (0.67 vs 0.64 s).
- **Not the clone, and not a rebuild.** A probe on the clone and on list
  construction says the list is built exactly ONCE (128,012 elements for
  |V| = 128,000, the surplus being other lists) and cloned 8,004 times for
  4,000 calls — two per call, every one an `Rc` bump.
- **It is the passing.** A list merely VISIBLE to a recursive function costs
  nothing: 0.04, 0.05, 0.12 s for |V| from 1,000 to 128,000. The same list
  PASSED costs 0.07, 0.55, 1.90.

Per-call and growing sub-linearly with the length of a list that is never
touched: the signature of memory rather than arithmetic. The next step is a
profiler, not another hypothesis.

*(The probe had to run INSIDE the evaluator thread to see anything: the
evaluator runs on its own 256 MB stack, so a thread-local counter read from
`main` reports zero and looks like proof of the opposite.)*

### 8.3 The merge budget's basis is stale

25,000 triangles per merged component came from measurements taken before
the union rewrite and before list values were shared — "models up to about
16,000 merge in three to seven seconds". The gear library alone went from
1.74 s to 0.25 s on the second of those changes. It wants re-measuring on a
quiet machine and probably raising; until then two spheres at `$fn` = 130
are one component of 33,792 triangles and are refused, which is reported but
is a worse answer than the merge would be.

### 8.4 The completeness score is stale in the other direction

`ROADMAP.md` carries 155 full / 22 partial / 6 untouched from an audit of
2026-09-11, and names PDF/3MF export, SVG import, the `$vp*` viewport
variables and the DXF-era deprecated functions as the remaining tail. All of
those have since landed except `dxf_cross` and `dxf_dim`. The score
UNDERSTATES the project, and is left as written because a number in that
file has to come from a pass over all 183 reference entries.

---

## Appendix: measurement hygiene

Three ways a measurement lied during this work, all of them cheap to repeat:

1. **`-o /dev/null` does not evaluate anything.** The path has no extension,
   the CLI rejects the format, and the process exits before the model runs.
   Every timing comes back at 0.01 s.
2. **A loaded machine makes everything look like a regression.** The bridge
   measured 93 s at load average 6.5 on four cores and 2.9 s quiet. Check
   `uptime` before believing a slowdown.
3. **A thread-local counter read from the wrong thread reads zero**, which
   looks exactly like proof that the thing you are counting never happens.
   The evaluator runs on its own thread for the stack space.
