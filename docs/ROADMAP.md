# scadforge roadmap — the map to 100%

The yardstick is `docs/openscad_language_reference.json` (183 entries,
OpenSCAD 2021.01 semantics). Score entries as **full** (implemented with
edge-case fidelity, pinned by tests), **partial**, or **untouched**.

Standing after the Customizer milestone (2026-09-04): **146 full / 24
partial / 13 untouched — 80% full, 93% touched.** PHASE 4 COMPLETE. Phase 5
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
- ◐ Import/export: **STL (ASCII + binary) and OFF done** (`scadforge/src/
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
  via a from-scratch tag scanner, geometry only). `POST
  /export?format=stl|off|amf|svg|dxf`. Remaining: 3MF (needs a from-scratch
  ZIP/deflate), PDF, and SVG import (a path-command parser).
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

~13 entries remain. Some describe the desktop application, not the
language: GUI-viewport PNG export, `$vpr`-family viewport variables,
DXF-era deprecated functions. Others are genuine formats deferred for a
from-scratch codec (3MF needs ZIP/deflate; PDF; SVG import needs a
path-command parser). Each gets a per-entry decision — web-app
equivalent, or documented as intentionally out of scope. 100% of the
*language* is reachable; 100% of all 183 entries goes through those
judgment calls.

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
