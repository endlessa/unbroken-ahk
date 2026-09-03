# scadforge roadmap — the map to 100%

The yardstick is `docs/openscad_language_reference.json` (183 entries,
OpenSCAD 2021.01 semantics). Score entries as **full** (implemented with
edge-case fidelity, pinned by tests), **partial**, or **untouched**.

Standing after phase 4e offset + projection (2026-09-03): **130 full / 10
partial / 43 untouched — 71% full, 76% touched.** (offset and projection
were the last two untouched 2D entries; phase 4d's 2D booleans deepened
the already-full `difference`/`intersection`/`hull`/`minkowski` entries to
full 2D+3D fidelity. Only `text()` + its 5 sub-entries remain in phase 4.)

Ground rules (from the project owner, non-negotiable):

- No IP-encumbering licenses; clean-room implementation only — never
  read OpenSCAD/CGAL source. Semantics come from the reference JSON.
- Rust, zero third-party crates. `std` and our own sibling crates only.
  The web UI is likewise dependency-free.

## The five phases

### ✅ Phase 1 — Expression engine (DONE, commits 6554ae8..38a7b6c)

All 21 operator/lexical entries, all 22 math builtins (degree-exact
trig), type-test builtins, ranges (incl. legacy reversed-range swap),
string indexing/comparison semantics, vector arithmetic with full `*`
shape dispatch, five-value truthiness, lazy `?:`, short-circuit
`&&`/`||`. Viewer: per-pixel z-buffer software renderer + WebGL path,
verified pixel-identical, two-pass transparency.

### ✅ Phase 2 — Language core (DONE)

Everything on the list landed: module/function definitions with
hoisting, three namespaces, the two-phase slot rule (first-slot
position, last-write-wins), lazy `children()` with the caller-lexical /
callee-dynamic split, the dynamic `$`-environment, `let`/`assign`,
`if/else`, all six comprehension clause forms (C-style `for` with
simultaneous updates), function literals with closures and
value-calling conventions, tail-call elimination (through ternary, let,
echo, assert wrappers; 200k-frame contract pinned) with named recursion
errors, and the builtins: `str` (shared 6-significant-digit formatter),
`chr`, `ord`, `echo` (statement + expression), `assert` (halting, with
serialized condition text), `search`, `lookup`, `rands`,
`version`/`version_num`, `parent_module`/`$parent_modules`.
Remaining partials here: assert lacks file/line + TRACE stack; string
literal escapes are the basic four.

### ✅ Phase 3 — CSG booleans (DONE)

`difference`, `intersection`, `intersection_for` compute real mesh
geometry via a from-scratch BSP-tree-merging kernel (`scadforge/src/
csg.rs`; Thibault & Naylor 1987 — clean-room, no CGAL), with a balancing
split-plane chooser (reviewed + hardened). `union`/`group` stay preview
concatenation (faithful to the reference's F5 preview), with the exact
union used internally to combine multi-shape operands. `hull` is a
from-scratch incremental 3D convex hull. `minkowski` is exact for convex
operands (hull of pairwise vertex sums, the dominant rounding use),
over-approximates concave ones with a warning, and caps the pairwise
product so a public preview can't hang — scored PARTIAL until real
convex decomposition lands.

Low-priority remnants (observable-surface, deferrable): `render()`,
`convexity`, the 2-manifold export warning.

### Phase 4 — 2D + extrusions (~15 entries, in progress)

Landed (2026-09-03):

- ✅ `square`, `circle`, `polygon` + the z=0 2D geometry model
  (`scadforge/src/poly2.rs`: even-odd contours, ear-clip triangulation
  with hole bridging, nesting-depth re-winding). 2D shapes flow through
  the pipeline via `Shape.outline: Option<Poly2>`; transforms reduce to
  2D (drop z row/col, rewind on det2<0).
- ✅ `linear_extrude` (height/center/twist/slices/scale) and
  `rotate_extrude` (angle/$fn, X-sign straddle error) — clean-room
  contour sweeps in `poly2.rs`, pinned by volume tests.
- ✅ Remaining transforms: `mirror` (Householder), `multmatrix`
  (non-finite subtree drop), `resize` (bbox measure + auto factors).
- ✅ `polyhedron` (winding fix, fan-triangulation, index bounds check).
- ✅ 2D booleans: `difference` / `intersection` / `union` / `hull` /
  `minkowski` on 2D operands now compute real geometry via a from-scratch
  2D segment-BSP kernel (`scadforge/src/csg2.rs`), the exact planar
  analogue of `csg.rs` (lines split segments instead of planes splitting
  polygons); the surviving segment soup is stitched back into even-odd
  contours. `union` is applied when an extrusion collects multiple 2D
  children (so crossing squares extrude to a plus, not a plus with a hole).
  Mixing 2D and 3D children in one boolean warns.

- ✅ `offset` (2D): round (`r`, arc corners from $fn/$fa/$fs), miter and
  chamfer (`delta`), both signs. Clean-room via Minkowski dilation
  (region ∪ edge-slabs ∪ convex-corner caps) for positive and the
  complement identity `erode(P) = B − dilate(B − P)` for negative — so
  hole-shrink, split-into-islands, and silent annihilation on over-inset
  all fall out of the union2/difference2 kernel (`csg2::offset2`).
- ✅ `projection` (3D→2D): `cut = false` silhouette (union of every
  non-vertical facet's XY shadow, Z ignored) and `cut = true` planar
  section at z=0 (triangle-plane crossings stitched into contours), with
  the flatten→re-extrude idiom round-tripping (`csg2::project`).

Remaining:

- `text()` and its 5 sub-entries need a from-scratch font rasterizer —
  likely the very last item in the project.

### Phase 5 — I/O + polish (~30 entries)

- Import/export: STL, OFF, AMF, 3MF, DXF, SVG
- `include` / `use`, library path resolution
- Modifier characters `*` `!` `#` `%`
- `$t`, `$preview`, viewport variables
- echo/assert output formatting (6-significant-digit numbers, value
  display forms, WARNING/ERROR/DEPRECATED/TRACE surface)
- Customizer parameter model

## The honest tail

15–20 entries describe the desktop application, not the language: CLI
switches, GUI-viewport PNG export, `$vpr`-family viewport variables,
DXF-era deprecated functions, Customizer preset JSON. Each gets a
per-entry decision — web-app equivalent, or documented as intentionally
out of scope. 100% of the *language* is reachable; 100% of all 183
entries goes through those judgment calls.

## Working method (established, keep using it)

1. Extract exact semantics for the phase's entries from the reference
   JSON before writing code.
2. Implement; pin every reference quirk with tests (workspace suite must
   stay green, zero warnings).
3. Update the web demo to exercise the new features; verify with a
   headless screenshot of the running app.
4. Re-score completeness against all 183 entries.
5. Adversarial review panel over the new code; fix confirmed findings.
6. Commit + push each milestone.
