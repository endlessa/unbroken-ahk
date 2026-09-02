# scadforge roadmap — the map to 100%

The yardstick is `docs/openscad_language_reference.json` (183 entries,
OpenSCAD 2021.01 semantics). Score entries as **full** (implemented with
edge-case fidelity, pinned by tests), **partial**, or **untouched**.

Standing after phase 2 (2026-09-02): **113 full / 10 partial / 60
untouched — 62% full, 67% touched.**

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

### Phase 3 — CSG booleans (few entries, hardest geometry)

- Real mesh `union` (today it is pass-through grouping)
- `difference`, `intersection`, `intersection_for`
- `hull`, `minkowski` at the edge of this phase
- From-scratch mesh boolean algorithms (clean-room; no CGAL)

### Phase 4 — 2D + extrusions (~15 entries)

- `square`, `circle`, `polygon`, the z=0 2D geometry model
- `linear_extrude`, `rotate_extrude`, `offset`, `projection`
- Remaining transforms: `mirror`, `multmatrix`, `resize`
- `polyhedron`
- `text()` and its 5 sub-entries need a from-scratch font rasterizer —
  likely the very last item in the project

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
