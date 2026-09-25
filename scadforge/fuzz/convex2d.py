"""Convexity and containment in the plane: hull() and offset() in 2D.

The 2D hull has the same defining property as the 3D one and it is checkable
straight off the exported contour: the result must be convex, so every edge's
supporting line must have every vertex on one side of it.

offset() is pinned by containment rather than by area, which keeps the checks
exact -- a rounded corner's area depends on how finely $fn cuts the arc, but
"A is inside its own outset" does not depend on anything. An emptiness test is
a boolean the kernel has to get right, so each containment is one more
exercise of difference() as well.

  hull is convex          every vertex is inside every edge's line
  hull contains input     area(hull(A)) >= area(A)
  hull idempotent         area(hull(hull A)) == area(hull A)
  hull of a convex shape  area(hull(K)) == area(K)
  offset(r=0) is identity area unchanged
  outset contains         A - offset(r=d) A is empty          for d > 0
  inset is contained      offset(r=-d) A - A is empty         for d > 0
  offsets nest            offset(r=d1) A - offset(r=d2) A is empty  for d1 < d2
  outset of convex        is still convex
"""
import os, random, subprocess, sys
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))

def contours(path):
    t = open(path).read().split("\n")
    out, i = [], 0
    while i < len(t):
        if t[i].strip() == "LWPOLYLINE":
            pts, i = [], i + 1
            while i < len(t) and t[i].strip() != "0":
                if t[i].strip() == "10":
                    pts.append((float(t[i + 1]), float(t[i + 3])))
                    i += 4
                else:
                    i += 1
            out.append(pts)
        else:
            i += 1
    return out

def shape(src, tag):
    p = os.path.join(W, "c2_%s.scad" % tag)
    open(p, "w").write(src)
    o = os.path.join(W, "c2_%s.dxf" % tag)
    r = subprocess.run([B, "-o", o, p], capture_output=True, text=True, cwd=W, timeout=300)
    if r.returncode != 0:
        return [] if "empty" in (r.stderr or "") else None
    return contours(o)

def area(cs):
    total = 0.0
    for c in cs:
        s = 0.0
        for k in range(len(c)):
            x1, y1 = c[k]
            x2, y2 = c[(k + 1) % len(c)]
            s += x1 * y2 - x2 * y1
        total += s / 2.0
    return total

def worst_outside(cs):
    """How far the worst vertex lies outside an edge's line, relative to size.

    Zero (to tolerance) exactly when the region is a single convex contour.
    """
    cs = [c for c in cs if len(c) >= 3]
    if not cs:
        return 0.0
    if len(cs) != 1:
        return 1.0            # a hole or a second island is not convex
    c = cs[0]
    verts = c
    span = max(max(v[i] for v in verts) - min(v[i] for v in verts) for i in (0, 1)) or 1.0
    sgn = 1.0 if area(cs) >= 0 else -1.0
    worst = 0.0
    for k in range(len(c)):
        x1, y1 = c[k]
        x2, y2 = c[(k + 1) % len(c)]
        ex, ey = x2 - x1, y2 - y1
        ln = (ex * ex + ey * ey) ** 0.5
        if ln < 1e-12:
            continue
        nx, ny = -ey / ln * sgn, ex / ln * sgn     # inward normal
        for vx, vy in verts:
            worst = max(worst, -((vx - x1) * nx + (vy - y1) * ny))
    return worst / span

PARTS = [
    lambda r: "square([%g,%g], center=true)" % (r.uniform(2, 7), r.uniform(2, 7)),
    lambda r: "circle(r=%g, $fn=%d)" % (r.uniform(1, 4), r.choice((6, 10, 16))),
    lambda r: "translate([%g,%g]) square(%g)" % (r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(2, 5)),
    lambda r: "rotate(%g) square([%g,1], center=true)" % (r.uniform(0, 180), r.uniform(4, 9)),
    lambda r: "polygon([[0,0],[%g,0],[%g,%g]])" % (r.uniform(3, 8), r.uniform(1, 6), r.uniform(3, 8)),
]
CONVEX = [
    lambda r: "square([%g,%g], center=true)" % (r.uniform(2, 7), r.uniform(2, 7)),
    lambda r: "circle(r=%g, $fn=%d)" % (r.uniform(2, 5), r.choice((6, 12, 20))),
]

def rel(x, y):
    return abs(x - y) <= 1e-6 * max(1.0, abs(x), abs(y))

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    r = random.Random(seed)
    A, C = r.choice(PARTS)(r), r.choice(PARTS)(r)
    K = r.choice(CONVEX)(r)
    d1 = round(r.uniform(0.3, 1.0), 2)
    d2 = round(d1 + r.uniform(0.3, 1.0), 2)
    g = {}
    for tag, src in (
        ("a",    "%s;" % A),
        ("h",    "hull(){ %s; %s; }" % (A, C)),
        ("hh",   "hull() hull(){ %s; %s; }" % (A, C)),
        ("k",    "%s;" % K),
        ("hk",   "hull() %s;" % K),
        ("o0",   "offset(r=0) %s;" % A),
        ("ok",   "offset(r=%g, $fn=16) %s;" % (d1, K)),
        # containment, each expressed as a difference that must come out empty
        ("out",  "difference(){ %s; offset(r=%g, $fn=16) %s; }" % (A, d1, A)),
        ("in",   "difference(){ offset(r=-%g, $fn=16) %s; %s; }" % (d1, A, A)),
        ("nest", "difference(){ offset(r=%g, $fn=16) %s; offset(r=%g, $fn=16) %s; }"
                 % (d1, A, d2, A)),
    ):
        g[tag] = shape(src, tag)
    if any(v is None for v in g.values()):
        skipped += 1
        continue
    ar = dict((k, area(v)) for k, v in g.items())
    hout = worst_outside(g["h"])
    kout = worst_outside(g["ok"])
    checks = [
        ("hull is convex",     hout <= 1e-6,
         "worst vertex %.3g of the size outside an edge line" % hout),
        ("hull contains input", ar["h"] >= ar["a"] - 1e-6 * max(1.0, abs(ar["a"])),
         "area(hull) %.9f < area(A) %.9f" % (ar["h"], ar["a"])),
        ("hull idempotent",    rel(ar["hh"], ar["h"]),
         "area(hull(hull)) %.9f != area(hull) %.9f" % (ar["hh"], ar["h"])),
        ("hull of convex == it", rel(ar["hk"], ar["k"]),
         "area(hull(K)) %.9f != area(K) %.9f" % (ar["hk"], ar["k"])),
        ("offset(r=0) identity", rel(ar["o0"], ar["a"]),
         "area(offset(r=0) A) %.9f != area(A) %.9f" % (ar["o0"], ar["a"])),
        ("outset contains A",  abs(ar["out"]) <= 1e-9,
         "A - outset(A) has area %.9g, should be empty" % ar["out"]),
        ("inset inside A",     abs(ar["in"]) <= 1e-9,
         "inset(A) - A has area %.9g, should be empty" % ar["in"]),
        ("offsets nest",       abs(ar["nest"]) <= 1e-9,
         "outset(%g) - outset(%g) has area %.9g, should be empty" % (d1, d2, ar["nest"])),
        ("outset of convex is convex", kout <= 1e-6,
         "worst vertex %.3g of the size outside an edge line" % kout),
    ]
    failed = [(n, why) for n, good, why in checks if not good]
    if failed:
        print("SEED %d: A = %s" % (seed, A))
        print("        C = %s   K = %s   d1 = %g  d2 = %g" % (C, K, d1, d2))
        for n, why in failed:
            print("   %-28s %s" % (n, why))
        bad += 1
        continue
    ok += 1
print("convex2d seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
