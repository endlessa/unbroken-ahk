# scadforge roadmap — the map to 100%

The yardstick is `docs/openscad_language_reference.json` (183 entries,
OpenSCAD 2021.01 semantics). Score entries as **full** (implemented with
edge-case fidelity, pinned by tests), **partial**, or **untouched**.

Standing after the audit and the `$vp*` viewport quartet (2026-09-11):
**155 full / 22 partial / 6 untouched — 85% full, 97% touched.** Every geometry and I/O *format* of 2021.01 is implemented.
PHASE 4 COMPLETE. Phase 5
so far:
modifier characters `* ! # %` (full); STL + OFF import/export; SVG + DXF
export and DXF import (2D vector) → partial; number formatting + value
display forms full+pinned; diagnostic surface class-prefixed → partial;
`render` and `convexity` full; `surface` text heightmap → partial; `text()`
via a from-scratch TrueType parser; `fill`/`roof` correctly rejected as
unknown in 2021.01; `include`/`use`/library-path → partial; io/preproc/font
hardened after security reviews. **The whole Customizer category now lands
(all 5 entries full)**: the comment-based parameter model + widget grammar +
group tabs (`scadforge/src/customizer.rs`), preset JSON (`parameterSets`
sidecar, round-tripped), `-D name=value` overrides, plus a headless CLI
(`scadforge -o out.stl -D w=40 -p sets.json -P Big in.scad`) and a live web
panel (grouped sliders/checkbox/dropdown, in-place value rewrite, preset
save/load). Remaining untouched are the honest tail (PDF/3MF export, SVG
import, `$vp*` viewport vars, DXF-era deprecated functions).

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
String literal escapes are now the full reference set — `\n \t \r \\ \"`
plus `\uXXXX` Unicode code points (four hex digits, surrogates rejected).
Remaining partial here: assert lacks file/line + the TRACE stack.

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
  chamfer (`delta`), both signs. Clean-room; negative offsets use the
  complement identity `erode(P) = B − dilate(B − P)`, so hole-shrink,
  split-into-islands, and silent annihilation on over-inset all fall out
  of the same dilation (`csg2::offset2`).

  **Round dilation was rewritten (2026-09-05) after a severe bug.** The
  original decomposed dilation as an n-ary boolean *union* of region ∪
  edge-slabs ∪ convex-corner caps. That decomposition is *all tangencies*:
  every piece's boundary touches every neighbour's at exactly `r`, never
  crossing. The segment-BSP shredded the shared boundary into a soup with
  no cycles — on one 6-point star only 4 of 171 surviving edges could lie
  on any cycle — and the stitcher then returned an EMPTY region. Measured
  severity of `offset(r=k) offset(delta=-k)`, the standard corner-rounding
  idiom: **15 of 24 star cases failed, and every gear profile at every
  radius.** Simple convex-ish shapes (square, L, plus, ring) always passed,
  which is why it survived the first review pass.

  The fix (`scadforge/src/offset.rs`) computes the offset directly, with no
  boolean at all: orient each contour material-on-left, emit the raw
  (self-intersecting) offset curve — segment translated by `r·n̂`, plus an
  arc fan at each convex corner — split every piece at all pairwise
  intersections, then trim by the defining predicate: a point survives iff
  its distance to the *input* region is ≥ `r`. Cancel opposite directed
  edges, weld, and trace by smallest clockwise turn. Four details carry the
  correctness: arcs are tested at their chord-midpoint *projected back onto
  the generating circle*; endpoints and intersections snap through one
  shared registry; equal/opposite directed edges are cancelled before
  tracing; and every HashMap key that can influence output is sorted first
  (determinism). Pinned by three tests — exact closed-form area for convex
  dilation (`A + P·r + πr²`), the eroded-star regression, and hole-shrink.

  **Miter and chamfer now go through the same kernel (2026-09-05), and the
  old boolean path is deleted.** Round has an exact, cheap membership test —
  the round dilation IS the Minkowski sum with a disc, so a point is inside
  iff its distance to the input is below `r`. Miter and chamfer are not a
  Minkowski sum of anything, so their trim tests membership of the union of
  pieces the corner cap actually produced: the input, one slab per edge, one
  cap per convex corner. Each piece is convex, which turns "strictly inside,
  by a margin" into a few dot products. One `corner_cap` function decides a
  corner's shape for both the raw curve and the trim region, so the two
  cannot disagree about (say) whether a miter tripped its limit and fell
  back to a flat cut — a disagreement would trim away the very segments the
  curve emitted. The miter limit exists only to keep a 180° reversal finite;
  it bites at 179.99°, matching the reference's "set astronomically high so
  it never truncates in practice", because arbitrarily long spikes on acute
  corners are the documented behaviour.

  All three joins are pinned by exact closed forms on a regular n-gon, where
  they differ only in the corner term — `Σ r²·θ/2` (round, = πr²), `Σ
  r²·tan(θ/2)` (miter), `Σ r²·sin(θ)/2` (chamfer) — matched to 1e-6 relative
  across four polygon sizes and three radii; the three constants are far
  enough apart that a join computing the wrong corner cannot pass by
  accident, and a separate test pins miter > round > chamfer in area so a
  silent fallback cannot hide. A grid oracle checks the traced result
  against the piece union on a 24-point star, and the idiom sweep now runs
  all nine shrink-join × grow-join combinations across three profiles and
  three radii.
- ✅ `projection` (3D→2D): `cut = false` silhouette (union of every
  non-vertical facet's XY shadow, Z ignored) and `cut = true` planar
  section at z=0 (triangle-plane crossings stitched into contours), with
  the flatten→re-extrude idiom round-tripping (`csg2::project`).

- ✅ `text()`: real glyph geometry from a from-scratch TrueType parser
  (`scadforge/src/font.rs`: sfnt table directory, `head`/`maxp`/`hhea`,
  `hmtx` advances, `cmap` formats 4 and 12, `loca`, and `glyf` simple +
  composite outlines with quadratic Béziers flattened by $fn/$fa/$fs). The
  bundled default face is Instrument Sans (SIL OFL, `fonts/`). `text_region`
  lays glyphs out with size, spacing, and halign/valign (all four vertical
  anchors via ascent/descent metrics); the run is one even-odd region, so
  counters (O/e/a) are holes and it extrudes/booleans/offsets like any 2D
  shape. Full alignment + spacing; PARTIAL on font selection (only the
  bundled face — no fontconfig match or `use <font.ttf>` loading) and on
  direction/shaping (ltr only, no kerning/ligatures/bidi). `textmetrics`/
  `fontmetrics` are snapshot-only (absent in 2021.01).

### Phase 5 — I/O + polish (~30 entries)

- ✅ Modifier characters `*` `!` `#` `%` — parser-level prefixes on one
  instantiation, stacking (`*!x`). `*` disables (short-circuits
  instantiation, so echo/assert inside never fire); `!` roots (prunes
  siblings, keeps ancestor transforms, so a `!` child of a boolean shows
  the raw child); `#` highlights (geometric no-op + a translucent pink
  ghost overlay, so a `#` cutter both cuts and shows where); `%`
  backgrounds (excluded from every boolean/export, shown as a gray ghost,
  so a `%` cutter ghosts without cutting and a `%` first child promotes the
  next to minuend). `!`/`%` bypass CSG via passthrough extraction; the
  viewer draws highlight/background from per-mesh flags in the render JSON.
- ✅ Import/export: **STL (ASCII + binary) and OFF done** (`scadforge/src/
  io.rs`): STL read autodetects ASCII vs binary by the size sniff (so a
  binary file whose header starts with "solid" still loads), welds vertices
  by exact position, drops degenerate triangles, and ignores stored
  normals; OFF read skips comments and fan-triangulates n-gons. Writers for
  ASCII/binary STL and OFF recompute facet normals. `import()` (+ deprecated
  `import_stl`/`import_off`) routes by extension, warns+empty on a missing
  file, errors on an unsupported format, and refuses absolute/`..` paths;
  `%` background is excluded from exports and `#` included. A `POST /export`
  route + Export-STL button download the scene. **SVG + DXF export and DXF
  import now land too**: SVG export writes one Y-flipped even-odd filled
  path; DXF export writes closed LWPOLYLINE entities; DXF import reads
  LINE/LWPOLYLINE/POLYLINE/CIRCLE/ARC (curves tessellated by $fn/$fa/$fs),
  stitches loose segments into even-odd loops, and warns on unsupported
  entities. **AMF** read/write also lands (write_amf / read_amf — XML mesh
  via a from-scratch tag scanner, geometry only). **SVG import** now lands too
  (`scadforge/src/svg.rs`): a from-scratch reader for the 2021.01 element
  subset — `<path>` (M/L/H/V/C/S/Q/T/A/Z, absolute + relative, béziers and
  endpoint-arcs flattened by $fn/$fa/$fs), `<rect>` (rounded corners),
  `<circle>`, `<ellipse>`, `<line>`, `<polyline>`, `<polygon>`, with `<g>`
  transforms (translate/scale/rotate/matrix/skew) — honoring the `dpi`
  argument (px/unitless user units → mm) and mm/cm/in/pt/pc physical units,
  with the SVG Y axis flipped. It is the exact inverse of the SVG writer, so a
  region survives an export→import round-trip (pinned). `<text>` is ignored
  with a warning; only fill geometry imports (strokes not expanded), and
  self-intersecting nonzero-fill resolution is the one approximation (shared
  with the whole even-odd 2D kernel). **PDF export** also lands
  (`io::write_pdf`): a minimal, uncompressed, ASCII-safe one-page vector PDF
  — the geometry filled even-odd and stroked, the page sized to its bbox in
  points, with a hand-built xref table whose byte offsets and stream
  `/Length` are pinned by tests (so the file actually opens). It rides the
  text export path, so `POST /export?format=stl|off|amf|svg|dxf|pdf` and the
  CLI (`-o out.pdf`) both produce it, and the web UI now has a format
  selector for the whole export surface. **3MF read + write also land** — the
  last mesh format — via a from-scratch ZIP + DEFLATE stack: `deflate.rs`
  inflates RFC-1951 streams (stored / fixed / dynamic Huffman + LZ77, pinned
  against Python-generated blobs), `zip.rs` writes STORED archives and reads
  both stored and deflated entries (CRC-32 from scratch), and `io::write_3mf`
  / `read_3mf` wrap/parse the OPC parts (`[Content_Types].xml`, `_rels/.rels`,
  `3D/3dmodel.model`). 3MF export is binary, so it goes through the CLI's
  bytes path (`-o out.3mf`, `eval::render_export_bytes`) rather than the
  String HTTP route (which returns a clear 415 for `format=3mf`); `import()`
  reads `.3mf` like any mesh. Verified against real Python-authored,
  deflate-compressed `.3mf` files (export re-opened by Python; a Python zip
  imported and re-exported). This closes the STL/OFF/AMF/3MF import AND export
  entries. Remaining 2D: none.
- ◐ `include` / `use` / library-path (`scadforge/src/preproc.rs`): a
  source-resolution pass run before eval. `include <path>` textually inlines
  the referenced file (recursively, cycle-guarded); its geometry runs and its
  defs/vars join the scope, so a later main-file assignment wins
  (override-after-include). `use <path>` exposes only the file's
  module/function definitions (geometry and top-level vars not run); a local
  same-name def shadows it. Paths resolve relative to the including file and
  are sandboxed (no absolute, no `..`); missing files warn with the
  format-specific text ("Can't open include file" vs "... library") and are
  non-fatal. Remaining: the 2019.05 override-before-include and used-file
  private-var scope, and the OPENSCADPATH/user/bundled search tiers.
- ✅ Customizer (`scadforge/src/customizer.rs`): a comment-based parameter
  model scanned from the main file — a typed literal RHS (number/bool/string/
  vector) before the first `module`/`function`, with a `// [widget]` comment
  selecting the widget (slider `[min:max]`/`[min:step:max]`/`[max]`, dropdown
  `[a, b]`/`[v:label]`, checkbox, textbox, spinbox), a `// label` line above
  it, and `/* [Group] */` opening a section (`[Hidden]` hides). Overrides are
  applied by rewriting each parameter's declaration line IN PLACE (RHS
  replaced, widget comment kept) — no duplicate assignment, no "reassigned"
  warning — and are validated against the model (right name, a single literal
  of the right kind), so a hostile value like `"a"; cube(9); //"` can never
  smuggle a statement into the source (pinned by a regression test). Preset
  sets round-trip through the reference `{parameterSets}` JSON. Exposed three
  ways: the web `/render` JSON carries the parameter model and takes `p=`
  overrides; the CLI runs headless (`scadforge -o out.stl -D name=value
  -p file.json -P SetName in.scad`, `-D` winning over the preset); and the web
  panel renders grouped live widgets with preset save/load (localStorage, the
  same sidecar schema).
- `$t`, `$preview`, `$children` done (phase 2); viewport `$vp*` variables
  are desktop-camera state — a documented web judgment call, still open.
- ◐ echo/assert output formatting: the 6-significant-digit number formatter
  (fixed for 1e-5..1e6, `1e+6`/`1.23457e+6` scientific, `-0`→`0`) and the
  value display forms (spaced-colon ranges, container-quoted strings vs bare
  str() top level, nested vectors, function-literal rendering) are full and
  pinned. The diagnostic surface now class-prefixes every line
  (WARNING/DEPRECATED/ERROR); file/line suffixes, the TRACE stack, and
  --hardwarnings remain.

## The honest tail

~6 entries remain, and they are the true tail — desktop-application surface
rather than the modeling language: GUI-viewport PNG export and DXF-era
deprecated metadata functions (`dxf_dim`/`dxf_cross`). Each gets a per-entry
decision — web-app equivalent, or documented as intentionally out of scope.

**The `$vp*` quartet now lands (2026-09-11).** `$vpr`/`$vpt`/`$vpd`/`$vpf`
carry the reference's defaults ([55, 0, 25], origin, 140, 22.5), and the
read/assign model is the interesting part: the viewport writes its live
camera into the variables BEFORE evaluation, so a script that reads `$vpr`
sees where the user actually is; a TOP-LEVEL assignment is reported back so
the camera moves once the compile finishes — which is what makes the
reference's camera-animation idiom (`$vpr` driven from `$t`) work. An
assignment below top level does NOT move the camera, and a wrong-shaped
assignment leaves the camera alone while the variable still holds what the
script wrote; both are pinned. The CLI gains `--camera
tx,ty,tz,rx,ry,rz,dist`, which populates the quartet; the alternate
eye/center spelling is refused by name rather than guessed at.

Capturing the assignment took two tries: `set_var` writes a `$`-name into
the CURRENT dynamic layer, and `exec_scope` drops that layer on the way out,
so reading the camera after evaluation returned the input every time. It has
to be read on exit from the outermost scope, while the layer is still
alive — which is also exactly where "top level" is decidable. Every
geometry and I/O format of 2021.01 is now implemented; 100% of the
*language* is reachable; 100% of all 183 entries goes through those
judgment calls.

**The debug/text export entry now lands in full (2026-09-05).** `.echo` is
the console stream — every `ECHO:` line plus the class-prefixed
diagnostics — exported by `?format=echo` / `-o out.echo`, captured even
when a script fatally errors. `.csg` is the evaluated instantiation tree
(`scadforge/src/csgfmt.rs` for the node model and spellings, the recorder
in `eval.rs`): every variable, loop, function call and user module
resolved away, leaving canonical built-ins — every affine transform
collapsed to `multmatrix`, `d` resolved to `r`, `color("tomato")` to its
RGBA, user modules and control flow to `group()`. `.nef3`/`.nefdbg` are
refused by name as CGAL internals (the reference's own recommendation),
and `.ast`/`.term` say plainly that they are not implemented rather than
reading as a typo.

Two properties make `.csg` the oracle the reference says it is, and both
are pinned by tests:

1. **The export is valid OpenSCAD that reproduces the design.** Numbers
   print in the shortest round-tripping form (so 1/3 keeps all 16 digits —
   "more digits than echo's %g", and exact on re-parse). A test evaluates a
   script, exports `.csg`, re-evaluates the export, and compares the STL,
   over one scene per recording path.
2. **The child-grouping structure survives.** A `for` that emits three
   cubes is ONE operand of a surrounding `difference()`, not three, so
   every statement records exactly one node — a `group()` wrapper when it
   has no more specific head. Flattening would silently change what the
   re-import subtracts.

**That oracle found a real bug within an hour of existing.** Sweeping 46
scenes through export-and-re-import turned up four mismatches, which
bisected to `hull()` — not to the export. `convex_hull` read its horizon
edges back out of a `HashSet`, and Rust seeds `HashSet`'s hasher randomly
*per process*: the cap faces were appended in a different order every run,
so `hull()` emitted a different mesh — different triangles *and* different
vertex numbering — each time the same file was exported. Five runs of one
two-sphere hull gave five different STLs. Fixed by walking the visible
faces in their own order and keeping only ordered containers on any path
that reaches output (`csg.rs`, the same discipline `offset.rs` already
followed). Pinned by an exact emission snapshot; note that the
set-comparison test alongside it does *not* catch this class of bug,
because it sorts away the very ordering that breaks — worth remembering
when writing the next determinism test.

## The audit, and what it says about testing here

An adversarial sweep (2026-09-06..10) found **fourteen reproduced bugs**,
all in code that had passing tests. Every one was reproduced locally
before and after its fix. They fall into four groups, and the groups are
more instructive than the individual defects:

1. **A shipped invariant that was simply false.** The `.csg` export
   claimed "the export re-imports to the same geometry" and broke eight
   ways: the recorder SPLICED an unnamed frame's nodes into its parent, so
   `children()` forwarding two shapes into an `intersection()` became
   three operands (a 1.1 MB STL re-imported as 61 KB); statements that
   drew nothing kept no operand slot at all, so a `difference()` lost its
   minuend; `linear_extrude` recorded `slices = 1` whenever the real count
   came from `$fa`; `surface()` dropped `invert` and `import()` dropped
   `dpi`; `inf`/`nan` were spelled as identifiers the lexer cannot read;
   and `cube`/`square` heads coerced sizes the renderer rejects.
2. **Tangency, twice.** The straight-join trim asked "is this point
   strictly inside SOME piece?" — but a point can be interior to the UNION
   while lying on the shared boundary of two pieces, inside neither. That
   is the *same* blind spot that sank the original boolean dilation,
   reintroduced one layer down, and it shattered `offset(delta=6)` on a
   plus-sign into four slivers. If a construction's pieces meet only in
   tangencies, assume it is broken until a sweep says otherwise.
3. **Crashes from plausible input.** Both BSP builders recursed without a
   bound; `difference() { cube(1e9, center=true); sphere(0.6e9); }` aborted
   the process with SIGABRT. A model measured in microns reaches 1e9
   without trying.
4. **Hidden scale dependence.** An absolute `1e-9` area floor annihilated
   `offset(r=1e-6) square(1e-5)`; a bounding-box-relative cleanup tolerance
   let one distant contour dissolve its neighbour.

**The lesson about tests is sharper than the bug list.** In three separate
cases the test written alongside the code passed on the broken version:

- `hull`'s set-comparison test sorted away the very ordering the
  nondeterminism perturbed.
- The offset closed-form test only exercises CONVEX polygons, which have
  no tangencies to trip over.
- The piece-union raster oracle used the same flawed predicate as its
  ground truth — it was checking the bug against itself.

What worked instead was pinning the *property the bug violates*, not the
output it produces: emission reproducibility (an exact snapshot), area
continuity in the offset distance (dA/dd is the perimeter, so a collapse
is a discontinuity), and scale invariance (10^k in, 10^2k of area out).
Each of those fails loudly on the old code and is indifferent to how the
right answer is computed.

**The one-lens-at-a-time rerun (2026-09-10..11).** Three attempts to run the
audit as a seven-lens fan-out died on token limits without returning
anything. Run one lens per batch instead, they complete — two lenses, two
clean reports, fourteen more reproduced bugs. Two of them were defects in
fixes made EARLIER in the same audit, which is the argument for an
independent pass rather than more self-review:

- The non-finite `$fn` guard tested only `is_finite`, so a merely large
  finite `$fn` (40000 — a plausible "max quality" value) still asked for
  19 GB and aborted.
- The BSP no-progress guard traded a crash for a silently non-watertight
  mesh with up to 17% volume error and nothing on the console.

The semantics lens also turned up `$preview` hard-coded true — so
`$fn = $preview ? 24 : 120;` exported the COARSE mesh, shipping
preview-resolution geometry to a printer with no trace in the file — and a
console stream that was not interleaved, which quietly disqualified `.echo`
from the oracle role the reference assigns it.

The 3D lens found the scale-dependence bug's twin: `Plane::from_points`
rejected triangles by an ABSOLUTE area floor, so the same model in
millimetres and in metres gave different booleans, and 8 of 22,496 facets of
`sphere(r=1,$fn=150)` were dropped at ordinary scale. Third instance of the
same root cause (after offset's area floor and its cleanup tolerance): **a
geometry kernel has no intrinsic unit, so every threshold in it must be
relative to something in the input.** The fix is a sliver test on the sine of
the angle between edges, which is scale-free by construction.

**The parser lens found nothing, and that is the result (2026-09-11).** Run
by hand after the agent pass hit a limit. The path sandbox holds: with
GEOMETRY as the oracle — a readable heightmap yields facets, a blocked one
yields none — all nine escape routes are refused (parent-dir, absolute,
nested `..`, symlinked file and symlinked directory, for both `import()` and
`surface()`), while the in-sandbox controls read normally. A 6,152-case
mutation sweep over STL/OFF/AMF/3MF/SVG/DXF/heightmap — truncation at every
length for small files and 400 sampled lengths for large, plus byte flips and
spliced 2^32-1 counts — produced zero panics, zero aborts, zero timeouts.
Hand-built structural attacks that random mutation is too blunt to reach were
all refused gracefully in under 0.3s: a binary STL claiming 2^32-1 triangles
in 84 bytes, an OFF header claiming four billion vertices, 200k-deep SVG and
AMF nesting, a 2M-column heightmap, an include/use cycle, a 1 GiB zip bomb, a
20,000-entry archive, patched uncompressed-size fields, a corrupted CRC, and a
300 MiB payload against the 256 MiB budget.

The thing being defended against is specific and worth naming: in Rust an
allocation failure is an ABORT, not a catchable panic — so a declared-count
attack does not merely fail, it kills the process and takes the `.echo`
diagnostic stream down with it. That is exactly how the `$fn` bugs presented.
Both classes are now pinned by tests.

**Still open from the audit:** `offset` cost is driven by the number of
self-intersections of the raw curve, which grows with (offset distance /
feature spacing)², not with vertex count — so `OFFSET_MAX_VERTS` bounds V
but not work. A 4000-vertex, 2000-spike profile at r=5 produces 1.29M
sub-segments and takes ~25s (down from 44s after indexing the input edges
and short-circuiting the containment test). Either the split pass needs a
spatial index too, or the cap needs to bound work rather than vertices.

Two more, both from the 3D lens and both left deliberately:

- **Sub-EPS features are amplified, not lost.** `split_polygon`'s coplanar
  test uses an absolute 1e-7. When a solid's opposing faces are closer than
  2*EPS both classify coplanar with the same plane, so the slab degenerates
  into a HALF-SPACE and deletes half the other operand — measured: a 1e-7
  slab turned an intersection of 3.4e-7 into 4.0, and left the difference
  genuinely open. The correct degradation is to lose the feature. The real
  fix is a scale-relative epsilon threaded through the BSP, which is the
  same medicine as `Plane::from_points` but a much larger change; it is
  NOT a one-line constant edit, because the tolerance has to follow the
  operand pair, not the module.
- **BSP output carries T-junctions.** A split polygon's new vertex is not
  inserted into the neighbour sharing that edge, so after welding, a long
  edge faces two short ones: on `sphere - cylinder`, 1344 of 2619 welded
  edges have valence 1. The mesh is watertight in the geometric sense
  (leak 8e-17, volumes exact, zero valence>2, zero inverted normals) and
  every viewer renders it correctly, but a consumer demanding an
  edge-matched 2-manifold will reject it. Fixing it means propagating
  T-vertices during the split.

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
7. For kernel work, sweep a *parameter space*, not a handful of shapes.
   The `offset` bug hid behind squares and rings for weeks and only showed
   up under a 24-case star × radius sweep. A geometry test that exercises
   one nice shape is a smoke test, not a proof — and any construction whose
   pieces meet only in tangencies should be assumed to break the BSP until
   a sweep says otherwise.
