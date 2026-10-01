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
save/load).

This paragraph used to close by naming what was left untouched: PDF and 3MF
export, SVG import, the `$vp*` viewport variables, and the DXF-era
deprecated functions. Every one of those has since landed. `write_3mf` and
`read_3mf` round-trip real deflate-compressed OPC files; `pdf` and `svg`
are export tags and `svg::read_svg` imports; the `$vp*` quartet is wired
through the dynamic environment; `dxf_linear_extrude`, `dxf_rotate_extrude`,
`import_dxf`, `import_stl` and `import_off` are all handled. `dxf_cross`
and `dxf_dim` are the two that genuinely are not.

Which means the 155/22/6 score above, from the audit of 2026-09-11,
UNDERSTATES where the project stands. It is left as written rather than
guessed upwards: a number in this file has to come from a pass over all 183
entries, and that pass has not been run since.

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
split-plane chooser (reviewed + hardened). `union`/`group` are preview
concatenation during evaluation — which is the F5 behaviour, and what
keeps per-shape colour and modifier flags alive for the viewer — and the
EXPORT merges them for real, because the reference is explicit that
"export always operates on fully rendered (F6-equivalent) geometry" and
concatenated overlapping shells make a self-intersecting file. Disjoint
parts cost nothing (a pair whose bounding boxes miss is concatenated),
so only genuinely overlapping solids pay. Above
`eval::MAX_UNION_EXPORT_TRIS` the merge is skipped with a console line:
a BSP plane is infinite, so every polygon straddling one is cut whether
the boolean touches it or not, and the cost turns vertical with how
deeply the parts interpenetrate — measured on this corpus, 16k triangles
merge in seconds while 34k of a densely crossing hull took 299s and came
out 33x larger. Reducing that inflation is the work that would raise the
budget, and the same work would lift a second limit: the splitter is
chosen from the operand's own FACE planes, and on a CONVEX solid every
face plane is a supporting plane, so no candidate can halve the set and
the tree runs one node deep per facet. `sphere(r = 10, $fn = 91)` —
8,368 facets — reaches `MAX_BSP_DEPTH` and comes back approximated with
a warning, and the identical sphere at r = 0.01 reaches it in exactly the
same place, so this is about facet count and not coordinate magnitude.
Admitting non-face splitters (an axis-aligned median split is the
obvious one) would fix the depth, but it is not a local change: a
first attempt corrupted six boolean results, because a cube is convex
too and the fallback then fires almost everywhere. `hull` is a
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

**`use` now works more than one level deep (2026-10-01).** A used file's
top-level constants are evaluated so its functions can see them, and all the
definitions land in one flat table the evaluator walks IN ORDER — so that
table has to be a topological order of the use graph. It was breadth-first:
dependents first, dependencies after. One level deep there is nothing to
order, which is why it stood from the day `use` was written. Two levels deep
it broke everything: a part file opening with `M = sg_m();`, reading the
contract it uses, was evaluated before the contract's own constants were
assigned, and every number it took from that contract came back `undef` with
a warning naming a private spelling the author never wrote — `Ignoring
unknown variable '__use1__MODULE_MM'`. The walk is now depth-first and
post-order; `used_seen` is inserted before recursing so a cycle terminates
and a diamond is read once, and the depth check comes first so a file refused
for depth on one route can still arrive by a shorter one. Found by trying to
assemble a model out of six part files, each of which used a seventh.

**The transmission stack is assembled (2026-10-01).** Seven files in
`examples/transmission/` now describe one machine rather than six parts and a
library: `stack.scad` puts four spherical-bevel planetary rows on one polar
axis with the equatorial band as the bottom row's ring, a swappable collar on
its register and a polar cap at each end — 1,624,244 triangles, 69 components
against 69 predicted bodies, 0 holes, 0 flipped edges, 0 T-junctions. The
assembly found four defects that no part file could see, all of the same
shape: a number two files have to agree on was *published* by one and
*copied* by the other, and nothing ever compared the two. `docs/TRANSMISSION.md`
is the record. The one worth repeating here is that the placement rule the
design states in words — each row's ring pitch circle on the fundamental
sphere at its own latitude — is not buildable, and the file proves it rather
than asserting it: each row's sun has a solid hub band, two of them sharing a
point of the (r, z) half-plane is a collision outright, and 109 of 169 sampled
points of row 1's sun land inside row 3's.

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

**The offset lens (2026-09-11) — the fourth instance, and a test of mine
with a hole in exactly the wrong place.** Erosion is the only offset path
that touches the segment BSP, whose `EPS` and `SNAP` are ABSOLUTE. So a
negative offset lost its holes below ~1e-5 units and returned EMPTY below
~1e-6, while the positive offset of the identical shape was bit-identical
from 1e-8 to 1e6. The same constant made a FINER `$fn` produce a WORSE
result: once the eroded corner's arc chord `|d|*2*pi/$fn` fell under `SNAP`
the arc welded shut and the hole vanished, so raising quality destroyed the
model at `$fn` = 700.

The shipped `offset_is_scale_invariant` test swept 10^-6..10^6 and passed
the whole time — because every call in it used a POSITIVE distance. The one
code path where scale invariance actually broke was the one path the
scale-invariance test never exercised. That is a sharper version of the
testing lesson above: it is not enough to pin the right PROPERTY, the sweep
has to reach the code that can violate it.

Fixed by normalizing the erosion to unit scale before it enters the BSP and
scaling the answer back — offsetting commutes with uniform scaling, and a
POWER-OF-TWO factor makes both scalings exact in binary floating point, so a
region already near unit scale gets s = 1 and is bit-identical to before.

**Still open from the audit:** `offset` cost is driven by the number of
self-intersections of the raw curve, which grows with (offset distance /
feature spacing)², not with vertex count — so `OFFSET_MAX_VERTS` bounds V
but not work. A 4000-vertex, 2000-spike profile at r=5 produces 1.29M
sub-segments and takes ~25s (down from 44s after indexing the input edges
and short-circuiting the containment test). Either the split pass needs a
spatial index too, or the cap needs to bound work rather than vertices.

**Even-odd resolution of the offset INPUT is missing**, and it is one root
cause behind three separate reproduced failures. The reference is explicit
(`offset`, semantics + edge_cases[9]): children are unioned and
self-intersecting outlines are "resolved by the clipping kernel (even-odd/
nonzero resolution as per polygon() semantics) BEFORE offsetting". Nothing
resolves them. Measured:

- A hole whose vertices all lie ON the outer contour casts zero votes in
  `nesting_depths`, fails the strict-majority test, and is wound as an
  OUTER — so the offset GROWS the hole instead of shrinking it. 63% area
  error at r=0.5, and the traced graph is perfectly balanced, so it returns
  at the first ladder rung with `degree_mismatch = 0`. Reachable from an
  ordinary SVG import with two filled subpaths.
- Crossing contours are offset as the UNION, not the even-odd region: the
  overlap hole is filled (up to 52% error on a negative offset).
- A self-intersecting contour with zero signed area (any balanced bowtie)
  is silently DELETED by `normalize`'s area filter — `offset()` returns
  nothing where the bare `polygon()` renders two triangles. With non-zero
  signed area it survives but is oriented by WINDING rather than even-odd
  (17-59% error).

The fix is a resolve pass, not three patches: split the input at all mutual
and self intersections, keep the edges whose two sides disagree under an
even-odd crossing count, and re-trace — the same pipeline shape the offset
itself already uses. It should run only when a cheap detector says the input
needs it, so well-formed input keeps the current fast path bit-identically.

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

- **One assembly still will not merge, and the reason is not found.** The
  gnome's head sub-assembly reduces to a node of 2,851 triangles enclosing
  88 working-frame units, and every union with that node loses geometry --
  in either operand order, against three different partners, by different
  amounts each time. Its bounds held when it was made, so it is accepted;
  its own merge output carries two edges whose faces run the same way
  round, which is two triangles' worth of volume and far inside the slack.
  A postcondition rejecting a merge that produced such an edge was written
  and BACKED OUT: a boolean's output legitimately carries T-junctions, so
  the test needs the vertices welded, and welding on exact bits (the only
  non-arbitrary rule available) does not bring those two faces together.
  Making it fire needs a quantised weld whose threshold nothing justifies.
  Recorded rather than guessed at.

- **The last six boundary edges were the FORMAT, not the triangulator.**
  With the walls taken from the cap triangulation's boundary, an extruded
  region with holes closes exactly -- a letter went from 44 boundary edges to
  none, a sheet of type from 38,618 to six. The six were chased for a long
  time on the assumption that the triangulator was dropping a vertex, and
  three repairs aimed at that were tried and reverted because none of them
  moved the number. The assumption was wrong, and so was the note that used
  to stand here.

  What is actually happening: the plate is a 256-point superellipse with
  a = 62, and near its rightmost point three consecutive outline vertices
  differ in x by less than ONE f32 STEP, which at a coordinate of 62 is
  3.8e-06. The cap triangle between them is real in f64 -- its cross product
  is 6e-06 -- and writes all three corners at exactly 62.0 in a binary STL.
  No binary STL can carry it, the export funnel drops it, and with it go
  three edges of each cap. Six boundary edges, from two triangles, in
  232,458.

  It is not a modelling error and there is nothing to repair in the
  triangulator. It is settled instead by telling the truth about it in three
  places. The representability guard is now per-format, so OFF, AMF, 3MF and
  ASCII STL -- which print `{:.6}` and resolve 2e-06 five hundred times over
  -- keep the triangle and export the plate CLOSED. Binary STL still drops
  it, because it must. And the export now warns when a drop opens a mesh
  that was closed, naming the count, the step, and the three ways out
  (another format, nearer the origin, a coarser outline).

  The lesson for the next one of these: when a mesh is closed on the way out
  of the modeller and open in the file, suspect the writer before the
  geometry. Exporting the same design to OFF and diffing the two settled in
  one command what three speculative repairs could not.

- **What a boolean still gets wrong, after the T-junction weld.**
  Inserting a stray vertex into the edge it lies on closed most of them --
  a heart went from 24,289 boundary edges to 1,177, an orc from 19,145 to
  2,033, two overlapping spheres from 15,919 to 190 -- but not all, and the
  residue is a different defect wearing the same clothes.

  Measured on the elliptical gear's 89 remaining open edges: 39 of the
  first 40 have another vertex somewhere in their interior, offset from the
  edge by 8.9e-06 to 4.9e-02 of the model extent, median 2.1e-04. Those are
  not points on an edge. They are points the BSP MEANT to put on an edge
  and missed, which makes them cracks rather than T-junctions.

  The weld cannot reach them and must not try. Sweeping its tolerance
  makes things worse, not better: at 1e-07 a character model went from
  1,459 open edges to 1,500 and grew its first inconsistently wound edge,
  and at 1e-04 to 6,565 open and 75 flipped. A vertex inserted into an edge
  it is not really on MOVES that edge, and the triangle on the other side,
  which got no such insertion, stops matching. The fix belongs where the
  points are computed -- the plane-segment intersection in the BSP split --
  and probably means snapping a new vertex to an existing one when it lands
  within the plane tolerance of it, rather than emitting a fresh point.

- **The union budget's basis is now stale.** 25,000 triangles per merged
  component came from measurements taken before the union was rewritten and
  before list values were shared: "models up to about 16,000 merge in three
  to seven seconds". The gear library alone went from 1.74 s to 0.25 s on
  the second of those changes. The threshold should be re-measured on a
  quiet machine and probably raised; until then two spheres at $fn = 130
  are one component of 33,792 triangles and are refused, which is reported
  but is a worse answer than the merge would be.

- **Passing a list to a function still costs something proportional to its
  length, and it is NOT the copy.** Sharing list values (`Rc<Vec<Value>>`)
  took the recursive fold -- the only way to reduce a list in a language
  without mutation -- from 6.90 s to 0.42 s at n = 8,000. It is not linear
  yet, and what is left has been narrowed but not found.

  What is measured. Reading a list by index IS linear now: 0.01 s at
  n = 8,000, 0.03 at 16,000, 0.05 at 32,000. The agent that filed this
  attributed it to indexing, and that is wrong: a fold that never reads the
  list costs the same as one that does (0.67 vs 0.64 s at n = 8,000). And a
  list that is merely VISIBLE to a recursive function costs nothing --
  0.04, 0.05, 0.12 s for |V| of 1,000 to 128,000 -- while the same list
  PASSED as an argument costs 0.07, 0.55, 1.90. Mapped in both dimensions,
  the cost is about k * |V|^0.7.

  What is ruled out, by counting rather than reasoning. A probe on
  `Value::clone` and on list construction says the list is built exactly
  ONCE (128,012 elements for |V| = 128,000, the surplus being other lists)
  and cloned 8,004 times for 4,000 calls -- two per call, and every one of
  them an `Rc` bump, not a copy. So neither the value clone nor a rebuild
  is doing it, which is what the obvious two hypotheses were.

  (The probe had to run INSIDE the evaluator thread to see anything: the
  evaluator runs on its own 256 MB stack, so a thread-local counter read
  from `main` reports zero and looks like proof of the opposite.)

  What is left is per-call and grows sub-linearly with the length of a list
  that is never touched -- the signature of memory rather than arithmetic,
  and the next step is a profiler rather than another hypothesis.

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
