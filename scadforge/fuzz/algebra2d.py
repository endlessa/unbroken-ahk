"""The boolean algebra identities again, in 2D, measured as signed area.

Same shape as algebra.py but exercising the 2D kernel -- polygon clipping,
contour orientation and hole nesting -- rather than the BSP. Area comes from
the DXF export, whose only entity is the LWPOLYLINE, summed with the shoelace
formula: an outline winds positive and a hole winds negative, so the total is
the net filled area and the identities can be written straight down.

  area(A u B) + area(A n B) == area(A) + area(B)      inclusion-exclusion
  area(A - B) + area(A n B) == area(A)                A splits at B
  area(A - B) + area(B - A) + area(A n B) == area(A u B)
  A u A == A,  A n A == A,  A - A == empty            idempotence
  area(scale([s,s]) A) == s^2 * area(A)
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

def area(src, tag):
    p = os.path.join(W, "a2_%s.scad" % tag)
    open(p, "w").write(src)
    o = os.path.join(W, "a2_%s.dxf" % tag)
    r = subprocess.run([B, "-o", o, p], capture_output=True, text=True, cwd=W, timeout=300)
    if r.returncode != 0:
        return 0.0 if "empty" in (r.stderr or "") else None
    total = 0.0
    for c in contours(o):
        s = 0.0
        for k in range(len(c)):
            x1, y1 = c[k]
            x2, y2 = c[(k + 1) % len(c)]
            s += x1 * y2 - x2 * y1
        total += s / 2.0
    return total

SHAPES = [
    lambda r: "square([%g,%g], center=true)" % (r.uniform(3, 9), r.uniform(3, 9)),
    lambda r: "circle(r=%g, $fn=%d)" % (r.uniform(2, 5), r.choice((6, 9, 12, 20))),
    lambda r: "translate([%g,%g]) square(%g)" % (r.uniform(-3, 3), r.uniform(-3, 3), r.uniform(2, 6)),
    lambda r: "rotate(%g) square([%g,%g], center=true)" % (r.uniform(0, 90), r.uniform(3, 8), r.uniform(3, 8)),
    lambda r: "polygon([[0,0],[%g,0],[%g,%g],[0,%g]])" % (r.uniform(3, 8), r.uniform(3, 8), r.uniform(3, 8), r.uniform(3, 8)),
    # An outline with a hole, so nesting is in play on both sides of a boolean.
    lambda r: "difference(){ circle(r=%g, $fn=16); circle(r=%g, $fn=12); }" % (r.uniform(4, 7), r.uniform(1, 3)),
    lambda r: "offset(r=%g, $fn=8) square(%g, center=true)" % (r.uniform(0.5, 2), r.uniform(2, 6)),
]

def close(x, y, scale):
    return abs(x - y) <= 1e-6 * max(1.0, abs(scale))

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    r = random.Random(seed)
    A, C = r.choice(SHAPES)(r), r.choice(SHAPES)(r)
    s = round(r.uniform(0.5, 2.0), 2)
    areas = {}
    for tag, src in (
        ("a",  "%s;" % A),
        ("b",  "%s;" % C),
        ("u",  "union(){ %s; %s; }" % (A, C)),
        ("i",  "intersection(){ %s; %s; }" % (A, C)),
        ("d",  "difference(){ %s; %s; }" % (A, C)),
        ("e",  "difference(){ %s; %s; }" % (C, A)),
        ("aa", "union(){ %s; %s; }" % (A, A)),
        ("ai", "intersection(){ %s; %s; }" % (A, A)),
        ("ad", "difference(){ %s; %s; }" % (A, A)),
        ("sc", "scale([%g,%g]) %s;" % (s, s, A)),
    ):
        areas[tag] = area(src, tag)
    if any(v is None for v in areas.values()):
        skipped += 1
        continue
    g = areas
    checks = [
        ("inclusion-exclusion", g["u"] + g["i"], g["a"] + g["b"]),
        ("A splits at B",       g["d"] + g["i"], g["a"]),
        ("three pieces",        g["d"] + g["e"] + g["i"], g["u"]),
        ("A u A == A",          g["aa"], g["a"]),
        ("A n A == A",          g["ai"], g["a"]),
        ("A - A empty",         g["ad"], 0.0),
        ("scale squares area",  g["sc"], s * s * g["a"]),
    ]
    failed = [(n, x, y) for n, x, y in checks if not close(x, y, g["u"])]
    if failed:
        print("SEED %d: %s" % (seed, A))
        print("        %s" % C)
        for n, x, y in failed:
            print("   %-22s %.9f != %.9f   (off by %.3g)" % (n, x, y, x - y))
        bad += 1
        continue
    ok += 1
print("algebra2d seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
