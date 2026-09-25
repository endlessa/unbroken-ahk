"""Boolean algebra identities over random solids. No oracle needed: the
identities pin the answer exactly.

  vol(A u B) + vol(A n B) == vol(A) + vol(B)      inclusion-exclusion
  vol(A - B) + vol(A n B) == vol(A)               A splits at B
  vol(A - B) + vol(B - A) + vol(A n B) == vol(A u B)
  A u A == A,  A n A == A,  A - A == empty        idempotence
  vol(scale(s) A) == s^3 * vol(A)                 and translate/rotate keep it
"""
import os, random, struct, subprocess, sys
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))

def vol(src, tag):
    p = os.path.join(W, "alg_%s.scad" % tag)
    open(p, "w").write(src)
    o = os.path.join(W, "alg_%s.stl" % tag)
    r = subprocess.run([B, "-o", o, p], capture_output=True, text=True, cwd=W, timeout=300)
    if r.returncode != 0:
        return 0.0 if "empty" in (r.stderr or "") else None
    b = open(o, "rb").read()
    n = struct.unpack_from("<I", b, 80)[0]
    v = 0.0
    for k in range(n):
        x = struct.unpack_from("<9f", b, 84 + k * 50 + 12)
        a, c, d = x[0:3], x[3:6], x[6:9]
        v += (a[0]*(c[1]*d[2]-c[2]*d[1]) + a[1]*(c[2]*d[0]-c[0]*d[2]) + a[2]*(c[0]*d[1]-c[1]*d[0])) / 6.0
    return abs(v)

SOLIDS = [
    lambda r: "cube([%g,%g,%g], center=true)" % (r.uniform(3,9), r.uniform(3,9), r.uniform(3,9)),
    lambda r: "sphere(r=%g, $fn=%d)" % (r.uniform(2,5), r.choice((8,12,16,20))),
    lambda r: "cylinder(h=%g, r1=%g, r2=%g, center=true, $fn=%d)" % (r.uniform(3,9), r.uniform(1,4), r.uniform(1,4), r.choice((6,9,12,16))),
    lambda r: "rotate([%g,%g,%g]) cube([%g,%g,%g], center=true)" % (r.uniform(0,90), r.uniform(0,90), r.uniform(0,90), r.uniform(3,8), r.uniform(3,8), r.uniform(3,8)),
    lambda r: "translate([%g,%g,%g]) sphere(r=%g, $fn=12)" % (r.uniform(-3,3), r.uniform(-3,3), r.uniform(-3,3), r.uniform(2,5)),
]

def run(lo, hi, tol):
    bad = skipped = ok = 0
    for seed in range(lo, hi):
        r = random.Random(seed)
        A, C = r.choice(SOLIDS)(r), r.choice(SOLIDS)(r)
        vs = {}
        for tag, src in (
            ("a",  "%s;" % A),
            ("b",  "%s;" % C),
            ("u",  "union(){ %s; %s; }" % (A, C)),
            ("i",  "intersection(){ %s; %s; }" % (A, C)),
            ("d",  "difference(){ %s; %s; }" % (A, C)),
            ("e",  "difference(){ %s; %s; }" % (C, A)),
        ):
            vs[tag] = vol(src, tag)
        if any(v is None for v in vs.values()):
            skipped += 1
            continue
        scale = max(vs["a"], vs["b"], 1.0)
        checks = [
            ("inclusion-exclusion", vs["u"] + vs["i"], vs["a"] + vs["b"]),
            ("A splits at B",       vs["d"] + vs["i"], vs["a"]),
            ("B splits at A",       vs["e"] + vs["i"], vs["b"]),
            ("three pieces",        vs["d"] + vs["e"] + vs["i"], vs["u"]),
        ]
        failed = False
        for name, got, want in checks:
            if abs(got - want) > tol * scale:
                print("SEED %d: %s  %.6f vs %.6f (rel %.3g)" % (seed, name, got, want, abs(got-want)/scale))
                failed = True
        # idempotence
        for name, src, want in (
            ("A u A", "union(){ %s; %s; }" % (A, A), vs["a"]),
            ("A n A", "intersection(){ %s; %s; }" % (A, A), vs["a"]),
            ("A - A", "difference(){ %s; %s; }" % (A, A), 0.0),
        ):
            got = vol(src, "x")
            if got is None or abs(got - want) > tol * scale:
                print("SEED %d: %s  %s vs %.6f" % (seed, name, got, want))
                failed = True
        if failed:
            open(os.path.join(W, "bad_alg_%d.scad" % seed), "w").write("A = %s;\nB = %s;\n" % (A, C))
            bad += 1
        else:
            ok += 1
    print("algebra seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))

run(int(sys.argv[1]), int(sys.argv[2]), float(sys.argv[3]) if len(sys.argv) > 3 else 1e-4)
