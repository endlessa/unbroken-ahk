# tools

Checks on exported geometry. Nothing here is part of the modeller — these
read what `scadforge` writes and answer questions the kernel cannot answer
about itself. Python 3, no dependencies.

| | |
|---|---|
| `validate.py OUT.stl` | triangle count, signed volume, `holes`, `flipped`, `split`, components. **This is "the validator" that `docs/GEOMETRY_DECISIONS.md` quotes throughout.** Exit status 1 if the mesh is inside out, leaks, or is inconsistently wound. |
| `overlap.py OUT.stl` | do any two bodies actually share space? Exact edge-crosses-face, plus a ray cast for nesting. Answers what the component count cannot. |
| `bbox.py OUT.stl` | radial, z and x extents — what you check a published envelope against. |
| `shot.py IN.json OUT.png [az el w h]` | software rasteriser with a z-buffer, reading the `/render` route's JSON so it keeps per-body colour. |
| `cutshot.py` | `shot.py` plus a cutaway: a seventh argument of `quarter`, `half` or `none` drops triangles by centroid, opening the model without any boolean. |

The three mesh checks answer three different questions and none implies
another:

- **`validate.py` asks whether one shell is sound.** Closed, consistently
  wound, right way out. It cannot see two shells that interpenetrate: the
  union of two sound shells is still sound by every count it takes.
- **the component count asks how many pieces there are.** Also blind to
  interpenetration — two shells that cross without sharing a vertex are
  still two components. It is a necessary check against a predicted body
  count, never a sufficient one.
- **`overlap.py` asks whether the pieces are disjoint.** This is the one the
  part files under `examples/transmission` raise and then have to leave
  open, because past `csg::MAX_MERGE_TRIS` the export-time union does not
  run. It found a real defect: see section 5 of `docs/TRANSMISSION.md`.

A positive is nearly free and a negative costs everything — `overlap.py`
stops at the first crossing, so finding one took 223 segment-triangle tests
on the equatorial band where proving there were none took 111,768,502. Run
it deliberately, not on every export.

## Getting a JSON for the renderers

`shot.py` and `cutshot.py` want the `/render` route's output, which carries
per-body colour that STL does not:

```sh
scadforge --port 4599 &                      # from the directory the model's `use` paths resolve against
curl -s -X POST --data-binary @model.scad http://127.0.0.1:4599/render -o model.json
python3 scadforge/tools/cutshot.py model.json out.png 52 22 620 1180 quarter
```
