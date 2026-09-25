"""Properties of hull() and minkowski() that need no oracle.

hull()'s defining property is checkable directly on the exported mesh: the
result must be convex, so every face plane must have every vertex of the mesh
on its inner side. That is a much stronger statement than any volume
comparison and it catches a hull that has merely grown rather than closed.

  hull is convex               every vertex is behind every face plane
  hull contains its input      vol(hull(A)) >= vol(A)
  hull is idempotent           vol(hull(hull(A, B))) == vol(hull(A, B))
  hull of a convex body        vol(hull(A)) == vol(A) for a cube or a sphere
  minkowski commutes           vol(A (+) B) == vol(B (+) A)
  minkowski grows              vol(A (+) B) >= vol(A)
"""
import os, random, struct, subprocess, sys
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))

def mesh(src, tag):
    p = os.path.join(W, "cx_%s.scad" % tag)
    open(p, "w").write(src)
    o = os.path.join(W, "cx_%s.stl" % tag)
    r = subprocess.run([B, "-o", o, p], capture_output=True, text=True, cwd=W, timeout=300)
    if r.returncode != 0:
        return [] if "empty" in (r.stderr or "") else None
    b = open(o, "rb").read()
    n = struct.unpack_from("<I", b, 80)[0]
    out = []
    for k in range(n):
        x = struct.unpack_from("<9f", b, 84 + k * 50 + 12)
        out.append((x[0:3], x[3:6], x[6:9]))
    return out

def volume(tris):
    v = 0.0
    for a, c, d in tris:
        v += (a[0]*(c[1]*d[2]-c[2]*d[1]) + a[1]*(c[2]*d[0]-c[0]*d[2])
              + a[2]*(c[0]*d[1]-c[1]*d[0])) / 6.0
    return abs(v)

def worst_outside(tris):
    """Largest distance any vertex sits on the OUTER side of any face plane.

    Zero (to tolerance) for a convex body. Scaled by the model's own size so
    the number is comparable across shapes.
    """
    if not tris:
        return 0.0
    verts = [v for t in tris for v in t]
    lo = [min(v[i] for v in verts) for i in range(3)]
    hi = [max(v[i] for v in verts) for i in range(3)]
    span = max(hi[i] - lo[i] for i in range(3)) or 1.0
    worst = 0.0
    for a, c, d in tris:
        u = [c[i]-a[i] for i in range(3)]
        w = [d[i]-a[i] for i in range(3)]
        n = (u[1]*w[2]-u[2]*w[1], u[2]*w[0]-u[0]*w[2], u[0]*w[1]-u[1]*w[0])
        ln = (n[0]**2 + n[1]**2 + n[2]**2) ** 0.5
        if ln < 1e-12:
            continue
        n = tuple(q/ln for q in n)
        off = sum(n[i]*a[i] for i in range(3))
        for v in verts:
            worst = max(worst, sum(n[i]*v[i] for i in range(3)) - off)
    return worst / span

PARTS = [
    lambda r: "cube([%g,%g,%g], center=true)" % (r.uniform(2, 7), r.uniform(2, 7), r.uniform(2, 7)),
    lambda r: "sphere(r=%g, $fn=%d)" % (r.uniform(1, 4), r.choice((8, 12, 16))),
    lambda r: "cylinder(h=%g, r1=%g, r2=%g, $fn=%d)" % (r.uniform(2, 7), r.uniform(1, 3), r.uniform(0, 3), r.choice((6, 10, 16))),
    lambda r: "translate([%g,%g,%g]) cube(%g, center=true)" % (r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(2, 5)),
    lambda r: "translate([%g,%g,%g]) sphere(r=%g, $fn=10)" % (r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(1, 3)),
    lambda r: "rotate([%g,%g,%g]) cube([%g,1,%g], center=true)" % (r.uniform(0, 90), r.uniform(0, 90), r.uniform(0, 90), r.uniform(3, 8), r.uniform(3, 8)),
]
CONVEX = [
    lambda r: "cube([%g,%g,%g], center=true)" % (r.uniform(2, 7), r.uniform(2, 7), r.uniform(2, 7)),
    lambda r: "sphere(r=%g, $fn=%d)" % (r.uniform(2, 5), r.choice((8, 12, 16))),
]

def rel(x, y):
    return abs(x - y) <= 1e-6 * max(1.0, abs(x), abs(y))

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    r = random.Random(seed)
    A, C = r.choice(PARTS)(r), r.choice(PARTS)(r)
    K = r.choice(CONVEX)(r)
    m = {}
    for tag, src in (
        ("a",   "%s;" % A),
        ("h",   "hull(){ %s; %s; }" % (A, C)),
        ("hh",  "hull() hull(){ %s; %s; }" % (A, C)),
        ("k",   "%s;" % K),
        ("hk",  "hull() %s;" % K),
        ("mab", "minkowski(){ %s; sphere(r=1, $fn=6); }" % A),
        ("mba", "minkowski(){ sphere(r=1, $fn=6); %s; }" % A),
    ):
        m[tag] = mesh(src, tag)
    if any(v is None for v in m.values()):
        skipped += 1
        continue
    v = dict((k, volume(t)) for k, t in m.items())
    out = worst_outside(m["h"])
    checks = [
        ("hull is convex",      out <= 1e-6,
         "worst vertex sits %.3g of the model's size outside a face plane" % out),
        ("hull contains input", v["h"] >= v["a"] - 1e-6 * max(1.0, v["a"]),
         "vol(hull) %.9f < vol(A) %.9f" % (v["h"], v["a"])),
        ("hull idempotent",     rel(v["hh"], v["h"]),
         "vol(hull(hull)) %.9f != vol(hull) %.9f" % (v["hh"], v["h"])),
        ("hull of convex == it", rel(v["hk"], v["k"]),
         "vol(hull(K)) %.9f != vol(K) %.9f" % (v["hk"], v["k"])),
        ("minkowski commutes",  rel(v["mab"], v["mba"]),
         "vol(A(+)B) %.9f != vol(B(+)A) %.9f" % (v["mab"], v["mba"])),
        ("minkowski grows",     v["mab"] >= v["a"] - 1e-6 * max(1.0, v["a"]),
         "vol(A(+)B) %.9f < vol(A) %.9f" % (v["mab"], v["a"])),
    ]
    failed = [(n, why) for n, good, why in checks if not good]
    if failed:
        print("SEED %d: A = %s" % (seed, A))
        print("        C = %s" % C)
        for n, why in failed:
            print("   %-22s %s" % (n, why))
        bad += 1
        continue
    ok += 1
print("convex seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
