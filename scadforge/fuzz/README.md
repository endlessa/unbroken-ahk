# Oracle-free fuzzing

Nothing in here compares scadforge against another modeller. Every check is
an identity the kernel has to satisfy against *itself*, so a failure is
always a real defect and never a difference of opinion about a corner of the
language. That property is what makes these runnable in bulk: a red line is
worth reading every time.

Between them they have found defects that several rounds of reading the same
code had missed — an `intersection_for` that dropped its passthrough
geometry, a signed zero that reached the STL writer, and the architectural
gap where `union()` concatenated shells instead of merging them.

## Running

Each script takes a half-open seed range and prints one summary line.

```sh
cargo build --release -p scadforge      # the fuzzers shell out to the binary

cd scadforge/fuzz
python3 run.py     1000 1200            # 3D designs
python3 run.py     1000 1200 2d         # 2D designs
python3 algebra.py  100  160
python3 algebra2d.py 100 160
python3 expr.py    1000 1600
python3 mods.py     100  200
python3 children.py 100  300
```

`SCADFORGE=/path/to/scadforge` overrides the binary; the default is
`target/release/scadforge` relative to the workspace root. Each script writes
its probe programs and exports into this directory (ignored by git) and
leaves a `bad_*.scad` behind for every seed that failed, so a reported seed
is reproducible by hand immediately.

Nothing here is wired into `cargo test`. These runs take minutes to hours,
which is the wrong shape for a test suite; the regressions they turn up get
added to the Rust tests as fixed cases instead.

## What each one checks

### `run.py` — three whole-pipeline invariants

- **Determinism.** The same source exported twice must give byte-identical
  output. This is tier 1 of the project's compatibility contract: no hash
  iteration order, no thread scheduling, no wall clock may leak into a mesh.
- **`.csg` round trip.** `source -> STL` must equal `source -> .csg -> STL`.
  The `.csg` file is the flattened instantiation tree, so this catches any
  construct the tree writer drops, mis-quotes, or cannot express.
- **2D vector fixed point** (`2d` mode). Writing a DXF, reading it back and
  writing it again must reach a fixed point: passes 2 and 3 must agree. Pass
  1 is allowed to differ because a re-import canonicalises touching contours,
  which is representation rather than geometry.

`gen.py` is the program generator behind it — random solids, transforms,
booleans, extrusions, comprehensions and user modules exercising `children()`
and `$children`.

### `algebra.py` — boolean identities

Volume-level identities that hold for any two solids, checked on random
pairs:

- inclusion–exclusion: `|A ∪ B| == |A| + |B| − |A ∩ B|`
- `(A − B) ∪ (A ∩ B) == A`
- the three-piece decomposition `A − B`, `B − A`, `A ∩ B` partitions `A ∪ B`
- idempotence: `A ∪ A == A`, `A ∩ A == A`, and `A − A` is empty

### `algebra2d.py` — the same identities, in 2D

The same list again, measured as signed area instead of volume, so it
exercises the 2D kernel — polygon clipping, contour orientation and hole
nesting — rather than the BSP. Area comes from the DXF export summed with the
shoelace formula: an outline winds positive and a hole winds negative, so the
total is the net filled area. Shapes with holes and `offset()` results are in
the generator, so nesting is in play on both sides of every boolean.

### `expr.py` — a value must not depend on its route

Each random expression is echoed eight ways — written out directly, returned
from a function, bound by `let()`, stored in a one-element vector and indexed
back out, passed as a function parameter, assigned at top level, passed as a
module parameter, and bound by a `for` loop. All eight `ECHO` lines must be
character-identical. A difference is a scoping, copying or evaluation-order
bug.

### `children.py` — every spelling of "my children" is the same geometry

A module whose whole body is `children();` is the identity on geometry, so
every other way of writing "all of my children, in order" must export
byte-identically to it: `children([0 : $children - 1])`, a `for` loop over the
indices, an explicitly built index list, the same list walked backwards,
forwarding through a second module, and a few no-op wrappers (`{ }`,
`union()`, `translate([0,0,0])`, a double `mirror`). Byte equality rather than
volume, because order is part of the answer. `$children` is separately checked
to be the count the caller actually passed.

### `mods.py` — the modifier characters have exact equivalents

For a statement `B` under some ancestor transforms:

| written | must export as |
| --- | --- |
| `*B` | the same design with `B` deleted |
| `%B` | the same design with `B` deleted (background is preview-only) |
| `#B` | the same design with plain `B` (the tint is preview-only) |
| `!B` | `B` alone, with its ancestor transforms still applied |

The modifier is attached to a transform node rather than a leaf, so what it
governs is a subtree, and the group around it is a `union`, a `difference`
and an `intersection` in turn — `!` has to beat all three, and `*`/`%` have
to leave the remaining operands in their original order.
