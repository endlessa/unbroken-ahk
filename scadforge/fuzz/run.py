"""Three invariants over random designs:
   A  determinism      -- the same source exported twice gives the same bytes
   B  csg round trip   -- source -> .csg -> STL equals source -> STL
   C  2D round trip    -- source -> DXF/SVG -> re-import -> DXF is a fixed point
"""
import os, subprocess, sys, gen
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))
lo, hi = int(sys.argv[1]), int(sys.argv[2])
MODE = sys.argv[3] if len(sys.argv) > 3 else "3d"

def run(out, src, fmt=None):
    cmd = [B, "-o", out] + (["--export-format", fmt] if fmt else []) + [src]
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=W, timeout=240)
    return r.returncode, (r.stderr or "")

def rd(p):
    return open(os.path.join(W, p), "rb").read()

bad = skipped = ok = 0
for seed in range(lo, hi):
    src = gen.program(seed) if MODE == "3d" else gen.program2d(seed)
    open(os.path.join(W, "f.scad"), "w").write(src)
    geom_fmt, tree_ext = ("stl", "csg") if MODE == "3d" else ("dxf", "csg")
    if run("f1." + geom_fmt, "f.scad")[0] != 0:
        skipped += 1
        continue
    # A: determinism
    run("f2." + geom_fmt, "f.scad")
    if rd("f1." + geom_fmt) != rd("f2." + geom_fmt):
        print("SEED %d: NOT DETERMINISTIC" % seed); bad += 1
        open(os.path.join(W, "bad_det_%d.scad" % seed), "w").write(src); continue
    # B/C: through the evaluated tree
    rc, err = run("f." + tree_ext, "f.scad")
    if rc != 0:
        print("SEED %d: .csg export failed: %s" % (seed, err.strip()[:150])); bad += 1; continue
    rc, err = run("f3." + geom_fmt, "f." + tree_ext)
    if rc != 0:
        print("SEED %d: re-import of .csg failed: %s" % (seed, err.strip()[:200])); bad += 1; continue
    if rd("f1." + geom_fmt) != rd("f3." + geom_fmt):
        print("SEED %d: CSG ROUND TRIP CHANGED GEOMETRY (%d -> %d bytes)"
              % (seed, len(rd("f1." + geom_fmt)), len(rd("f3." + geom_fmt))))
        open(os.path.join(W, "bad_%d.scad" % seed), "w").write(src); bad += 1; continue
    # 2D only: the vector export must re-import to itself
    if MODE == "2d":
        stable = True
        for fmt, ext in (("dxf", "dxf"), ("svg", "svg")):
            if run("v." + ext, "f.scad", fmt)[0] != 0:
                continue
            open(os.path.join(W, "v.scad"), "w").write('import("v.%s");\n' % ext)
            if run("v2.dxf", "v.scad")[0] != 0:
                print("SEED %d %s: re-import failed" % (seed, fmt)); bad += 1; stable = False; break
            # The direct export writes the region's own contours; a
            # re-import canonicalizes any that touch. That is a change of
            # representation, not of geometry, so the invariant is that the
            # WRITER is a fixed point: write, read, write again, and the
            # second and third files agree.
            open(os.path.join(W, "v3.scad"), "w").write('import("v2.dxf");\n')
            if run("v4.dxf", "v3.scad")[0] != 0:
                print("SEED %d %s: second re-import failed" % (seed, fmt)); bad += 1; stable = False; break
            if rd("v2.dxf") != rd("v4.dxf"):
                print("SEED %d %s: export is not a fixed point (%d vs %d bytes)"
                      % (seed, fmt, len(rd("v2.dxf")), len(rd("v4.dxf"))))
                open(os.path.join(W, "bad_%s_%d.scad" % (fmt, seed)), "w").write(src)
                bad += 1; stable = False; break
        if not stable:
            continue
    ok += 1
print("%s seeds %d..%d  clean %d  skipped %d  BAD %d" % (MODE, lo, hi, ok, skipped, bad))
