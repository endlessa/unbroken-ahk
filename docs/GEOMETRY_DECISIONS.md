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
they came out, not rounded to flatter.

**Three kinds of claim appear here and they are not equally strong.**

- *Proved.* Very few: the greedy-colouring bound, the bounding-box shortcut,
  and the two operand-volume bounds on a union.
- *Pinned by a test.* Reproducible by anyone, forever: the twist convergence,
  the disjoint-concatenation budget case, the sharing of list values, the
  T-junction repair, order-independence of the solid.
- *Measured.* Most of the numbers. Those from the CURRENT code can be
  re-run; those in a "before" column cannot, because the code that produced
  them is gone. Every "before" figure below is testimony from the commit
  record, not evidence you can check, and should be read that way.

**Timings are the weakest of all**, and the same model appears at very
different numbers here because it appears at different code states and under
different machine loads. The suspension bridge is quoted at 2.9 s, 82 s,
84 s and 0.8 s; each says which state it belongs to, and the appendix
explains why a fifth figure, 93 s, means nothing at all.

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

**Decision: remove THAT tolerance rather than tune it.** No value of it fixes
any of the three.

### 1.1a The tolerance that remains, and why it is a different thing

Not to overstate this: the union still has a volume tolerance. It is a
different tolerance doing a different job, and the distinction is the point.

`UNION_SLACK = 1e-3` is a BOUNDS CHECK on a merge. A union contains each of
its operands, so it cannot enclose less than the larger; it is covered by
them together, so it cannot enclose more than their sum. Both hold for any
union, need no oracle, and cost one pass over the triangles each. Every
pairwise merge is held to `max(lo) ≤ V ≤ hi_a + hi_b` with a tenth of a
percent of slack for the re-tessellation. A merge that fails is retried with
the operands SWAPPED — a BSP is not symmetric, the first operand supplies
the planes the second is cut by — and if neither order stands, the pair is
concatenated and the user is warned.

The two differ in what they are bounds ON. `UNION_SLACK` bounds the two
operands being merged, and both bounds are theorems. The one that was
removed was a fraction of the whole accumulated subtree, and nothing made it
a bound on anything — it was being used to answer a question it could not
answer, namely "did this merge join anything?"

### 1.1b It still fails, and says so

The bounds exist because the boolean fails on real geometry. Thin shells
that touch or interleave are the case a BSP handles worst, and when it goes
wrong it does not error: a 25,000-triangle assembly of chambers and vessels
once merged to a TENTH of its own volume, silently, and a heart of eleven
shells lost a third of itself — 168,222 mm³ down to 135,556 — when the aorta
was merged in. Those are what the bounds catch.

Catching is not fixing. A pair that fails in both orders is concatenated,
which self-intersects where the operands did, and the export reports how
many such pairs there were. That is the honest floor of the present design,
not a solved problem.

### 1.2 What replaced it

A union is now the CONNECTED COMPONENTS of the relation "these two might
share volume", where the relation is answered geometrically:

1. bounding boxes, which settle most pairs outright;
2. a containment ray, which rules out one solid nested wholly inside another
   — the case a surface test cannot see;
3. a separating axis between every pair of triangles in the shared region.

The separating-axis test uses the axis set that is complete for triangles —
two face normals and nine edge-edge cross products — but the implementation
is SOUND rather than complete: it skips an axis whose direction underflows,
and it demands a strictly positive gap. Both make it conservative. It
answers "certainly apart" or "cannot say", never "certainly together". It is
also not asked of every pair: a grid culls pairs sharing no cell, and two
budgets abandon the test outright and answer "cannot say" (§1.5).

"Cannot say" is not free, and §1.5 is the case where it was not. A
conservative answer feeds the pair into a connected component, and a
component over the merge budget is concatenated rather than merged, which
self-intersects. The direction is safe; the consequence is real. The broad
phase has to be good in practice, not merely sound.

Components are an order-independent SET: which solids end up merged together
does not depend on the order the children were written in. That is the
property the old design broke, and it broke it in the worst way — a 1,600 mm³
overlap was merged under one ordering and lost under another, so the SOLID
changed.

**What is order-independent now, and what is not — measured.** Four mutually
overlapping spheres, written forwards and backwards:

| | triangles | volume | boundary edges |
|---|---|---|---|
| forwards | 24,509 | 9,299.206724 | 269 |
| backwards | 24,061 | 9,299.206753 | 285 |

The solid agrees to nine significant figures — 3e-9 relative, the BSP's own
arithmetic. The MESH does not agree at all. Within a component the colouring
walks in index order and the classes merge smallest-first with ties broken by
class order, so a BSP sees its operands differently arranged and cuts them
differently.

So the reference's "Order of children never affects the result" is met for
the solid and not for the triangulation. That distinction is pinned by
`a_union_is_order_independent_in_the_solid_not_in_the_mesh`, which asserts
the volumes and deliberately does not assert the triangle counts.

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
| `$fn` = 110 | 24,192 | union runs | 7,476.12 | 190 |
| `$fn` = 130 | 33,792 | union refused | 8,369.43 | 0 |

An 11.5% volume error crossing a triangle-count threshold, and **the side of
it that is wrong is the side that validates clean**. Nothing downstream can
tell.

**Decision: weigh a connected component, not a model.** Only a component
ever reaches a BSP. A model of disjoint parts now merges
without hesitating whatever its TRIANGLE count, because there is nothing to
merge — 2,400 cubes, 28,800 triangles, comes out concatenated exactly, and
that is pinned by a test. It is not free in the number of PARTS: the relation
is built by an exhaustive double loop, so it stays quadratic in the piece
count even when every box test misses. A
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
a cell holds a few triangles whatever the mix. The mean is SAMPLED rather than summed — stride
`len/64`, so 64 to 127 triangles for a mesh above 127 and all of them below
that, taken from the smaller of the two meshes, and the quantity averaged is
a triangle's longest bounding-box axis rather than an area. It only has to be
right to within a factor. Summing instead took the bridge from 2.9 seconds
to 82, because a bridge runs 125,250 box tests and this work sits behind
them.

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

The diagonal is first order — still 0.2% wrong after 256 slices — and for
this profile it is OVER, which no chordal approximation of a CONVEX profile
can honestly be. (The sign argument is about the square. The annulus below is
not convex, and is offered for the size of the error rather than its sign.)

**Decision: fan the quad through the mean of its four corners.** The ruled
patch's volume is exactly halfway between the two diagonals, so a fan
through the corners' mean straddles it instead of choosing a side. The error
becomes second order and approaches from below.

**Only a wall that is actually bent pays the extra two triangles.** An
untwisted extrusion's walls are planar, and so are a uniformly scaled one's
— those are cone faces through the apex — so both keep two triangles per
quad, which is exact for them. The test is a planarity check on the quad,
not a check of the twist parameter.

This is in `linear_extrude` ONLY. `rotate_extrude` emits two triangles per
quad unconditionally, bent or not. It has not been shown to matter there — a
torus converges from below at second order, because the profile's own
chordal error dominates — but the sentence above is not a kernel-wide
invariant and should not be read as one.

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
  triangle. That one is worse, because the mesh looks fine until export: the
  export funnel drops it and two edges of a closed mesh go with it. (Why it
  is dropped is narrower than "no format can write it" — OFF, AMF and 3MF
  store indices and no facet normal, so a collinear triple is nothing to
  them. It goes because STL requires a normal, this kernel's
  `triangle_normal` gives up below |cross| = 1e-12 and writes `0 0 0`, and
  this kernel's own STL reader would then refuse it. A choice made once at
  the funnel so a format never disagrees with itself, not a limitation every
  format has.)

**Decision: ear-clip in the face's own plane by Newell's normal**, with one
step of lookahead that PREFERS a clip leaving no flat corner behind. It
matters: a plain "largest ear" rule leaves the collinear triple to the end,
where it is emitted with no choice at all.

It is a preference, not a guarantee. A degenerate ear ranks last so it loses
to any ear with area — but if every available ear is degenerate one is still
cut, and a face that is not simple falls back to a plain fan. Nothing here
promises a face can always be triangulated without a zero-area piece, and no
test claims it.

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

Two 20 × 20 × 10 boxes offset by 5 mm along one axis — so they share three
quarters of their length — union to exactly 5,000 mm³ (2 × 4,000 − 3,000)
with twelve boundary edges. **It meant no union of solids that actually MEET could ever
come out clean**, which is a large part of what the modeller is for.

**Decision: insert the stray vertex into the triangle that owns the edge and
re-cut the triangle around it.** No geometry is added: the point is already a
vertex of the mesh, and it lies on the edge to within the tolerance below —
1e-9 of the model extent — so volume and area move by at most that order.
"Unchanged" would be too strong, for the same reason a looser tolerance fails
two paragraphs down.

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
| the two boxes above | 12 | 0 (28 triangles instead of 24) |

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

### The weld has a give-up of its own

An edge whose bounding cells span more than 8,192 of the vertex grid is
skipped rather than queried. Giving up leaves that one edge unwelded, which
is the safe direction, but it means a long edge across a large model is a
SECOND source of surviving T-junctions, unrelated to the BSP cracks of §8.1.
The residue analysis there does not separate the two.

One other idea was tried and did nothing: excluding split points closer to a
corner than the writer can resolve, on the theory that they make triangles
the export funnel will drop. It moved one model by four boundary edges and
left the others identical, and was reverted rather than kept as unjustified
code. (That measurement was taken against the own-plane variant of §2.3,
which is not the code that shipped, so the four is indicative and not a
figure to carry forward.)

---

## 4. Export formats

### 4.1 Representability is per format

A triangle whose three corners come out COLLINEAR once moved onto the target
file's coordinate grid cannot be given a normal, and the export funnel drops
it. (Collinear, not coincident — the §4.2 case has three distinct y
coordinates and only its x collapses.) That guard used to apply f32's answer
— binary STL's — to EVERY format.

f32's step at a coordinate of 1e6 is 0.119, so a millimetre-sized face a
kilometre from the origin has all three corners on one f32 point. But OFF,
AMF, 3MF and ASCII STL print `{:.6}` — an absolute grid of 1e-6 against
f32's 0.119 out there, finer by a factor of about 119,000. One guard for all
of them lost the face from every format, for a limitation only one of them
has.

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

This made real losses visible immediately: `epicyclic` loses 1,988 of 7.9
million faces — a 170 mm model tessellated finer than binary STL can carry
at that size, which is a format limit rather than a bug.

**And it made the warning itself wrong twice, which an audit caught.** It
hard-coded "at a coordinate of 62 is 3.8e-06" — 62 being the letterform's
semi-axis, where the case was found — so every other model was told a
resolution that was not its own. And it counted boundary edges in the f64
mesh left after the drop, which is not the file: the same rounding that
collapses a triangle also brings its neighbours' corners together and can
close the gap the collapse opened. A cycloidal gear drops 48 triangles and
the f64 remainder has 62 boundary edges, while the binary STL it writes
validates CLOSED. It was warning about a file that was fine.

The step is now computed from the model's own largest coordinate, and the
edge count is taken from the mesh AS WRITTEN — coordinates on the format's
grid, coincident points treated as one, and every triangle kept, because a
facet whose corners coincide is still a facet in the file. Cycloidal no
longer warns. The letterform still does, and its six edges are still there.

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
edges, and no boolean at all. Through OFF it measures 80,997.784281 **m³** — the
model works in metres — against the file's predicted 80,997.8: agreement to
every digit the echo prints. Through binary STL it reads 80,997.92, which is
1.7 ppm of f32 — the file, not the model.

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
| Budget per component | budget per model | 501 disjoint solids refused wholesale; 11.5% error on the clean-looking side |
| Grid sized by triangle size | sized by triangle count | 107 million pair tests, then a give-up |
| Ruled fan on bent walls | a diagonal | 4,408.75 against an exact 4,000, first order, and over |
| Ear clip in `polyhedron` | fan from vertex 0 | concave faces covered; zero-area triangles dropped at export |
| Weld plane chosen by checking | either plane fixed | 89 vs 329 boundary edges one way, 2 flipped edges the other |
| Weld tolerance 1e-9 | anything looser | 1e-4 gives 6,565 holes and 75 flipped, from 1,459 and 0 |
| Per-format representability | one guard for all | a face OFF resolves 119,000× over, lost from OFF |
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

(A source comment in `weld_tjunctions` still gives the range as 1e-5 to
1e-4, which was the first estimate. The distribution above supersedes it and
the comment is stale.)

### 8.2 Passing a list to a function still costs O(|V|^0.7) per call

Sharing list values took the fold from 6.90 s to 0.42 s at n = 8,000. It is
not linear yet, and the residue has been narrowed without being found.

What is ruled out, by counting rather than reasoning:

- **Not indexing.** Reading by index is at worst very weakly superlinear now
  — 0.01, 0.03, 0.05 s at n = 8,000, 16,000, 32,000, which is three points at
  the stopwatch's own 10 ms resolution and fixes an order of magnitude rather
  than an asymptote. And a fold that never reads the list costs within 5% of
  one that does (0.67 vs 0.64 s), which is the part that matters: whatever
  the cost is, reading is not it.
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
quiet machine; until then two spheres at `$fn` = 130 are one component of
33,792 triangles and are refused, which is reported but is a worse answer
than the merge would be.

**Raising it is not obviously the fix, and the record says why.** The cost
is not a function of size alone but of how deeply the parts interpenetrate,
and it turns vertical: 34,288 triangles of a hull with many crossing parts
took 299 seconds and came out at 1,122,789 — a 33× explosion — while 69,656
triangles of a gear took 206. A threshold that is not really a function of
triangle count cannot be repaired by choosing a different triangle count.
Whatever replaces it probably has to measure the interpenetration, which is
what the BSP was going to compute anyway.

The corpus makes the urgency plain: **thirteen models trip this budget**,
with components from 28,872 to 252,304 triangles. It is not a corner case.

### 8.4 The completeness score is stale in the other direction

`ROADMAP.md` carries 155 full / 22 partial / 6 untouched from an audit of
2026-09-11, and names PDF/3MF export, SVG import, the `$vp*` viewport
variables and the DXF-era deprecated functions as the remaining tail. All of
those have since landed except `dxf_cross` and `dxf_dim`. The score
UNDERSTATES the project, and is left as written because a number in that
file has to come from a pass over all 183 reference entries.

---

## 9. This document was audited, and it was wrong in six places

It was checked by an independent reader whose brief was to refute it. Worth
recording what that found, because the failures are of a kind that will
recur.

- **Two figures were simply wrong.** Two spheres at `$fn` = 110 are 24,192
  triangles, not 23,800 — a sphere at that `$fn` is 12,096, and the adjacent
  row (`$fn` = 130 → 33,792 = 2 × 16,896) was exact, which should have made
  the inconsistency obvious. And the volume error across the budget
  threshold is 11.5%, not 11.8%; 11.8 had been copied in from a report and
  matched neither of the two numbers printed beside it.
- **A unit was wrong by 1e9.** The bridge's 80,997.784281 is m³, not mm³.
- **A claim about the code was wrong, and the code was wrong with it.** The
  warning does not name "the format's step" — it named 62 for everything.
  Fixed in the code, not just here.
- **A description did not match its model.** "Two cubes overlapping by a
  quarter" are two 20 × 20 × 10 boxes offset by a quarter of their length,
  which means they SHARE three quarters of it. The 5,000 mm³ was right; the
  words were not, and an auditor checking the arithmetic of the words found
  them unreachable.
- **The central claim was overstated.** Order-independence held for the
  solid and not for the mesh, and the document did not distinguish them.
  That is now measured, stated, and pinned by a test.
- **The largest omission was not a number**: the union still carries
  `UNION_SLACK`, an operand-swap retry and a concatenate-and-warn fallback,
  and real models still defeat it. A document opening with "remove the
  tolerance" has to say which tolerance. §1.1a and §1.1b now do.

The audit also confirmed about two thirds of the numbers by re-running them,
several to every digit printed. That is the useful part of the result and
the reason to keep doing it: a document that has been attacked and mostly
survived is worth more than one that has not been attacked.

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
