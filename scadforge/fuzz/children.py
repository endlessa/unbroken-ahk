"""Identities for children(), $children and the modules that forward them.

A module whose whole body is `children();` is the identity function on
geometry, and every way of spelling "all of my children, in order" has to
agree with it exactly -- byte for byte, since order is part of the answer.
The identities are checked through one and two levels of forwarding, and
`$children` is checked to be the count the caller actually passed.
"""
import os, random, subprocess, sys
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))

def shape(r):
    return r.choice([
        "cube(%d, center = %s)" % (r.randrange(2, 9), r.choice(["true", "false"])),
        "sphere(r = %d, $fn = %d)" % (r.randrange(2, 6), r.choice([8, 12])),
        "cylinder(h = %d, r1 = %d, r2 = %d, $fn = %d)"
        % (r.randrange(2, 8), r.randrange(1, 5), r.randrange(0, 5), r.choice([6, 12])),
        "linear_extrude(%d) circle(%d, $fn = %d)"
        % (r.randrange(1, 5), r.randrange(1, 4), r.choice([6, 10])),
    ])

def xform(r):
    return r.choice([
        "translate([%d, %d, %d])" % tuple(r.randrange(-5, 6) for _ in range(3)),
        "rotate([%d, %d, %d])" % tuple(r.choice([0, 45, 90, 180]) for _ in range(3)),
        "scale([%s, %s, %s])" % tuple("%.1f" % r.uniform(0.6, 1.8) for _ in range(3)),
        "mirror([1, 0, 0])",
    ])

# Every body below must behave as `children();` does.
BODIES = {
    "plain":          "children();",
    "range":          "children([0 : $children - 1]);",
    "loop":           "for (i = [0 : $children - 1]) children(i);",
    "explicit list":  "children([for (i = [0 : $children - 1]) i]);",
    "empty guard":    "if ($children > 0) children(); else cube(99);",
    "let around":     "let (n = $children) children([0 : n - 1]);",
    "two levels":     "inner() children();",
    "double mirror":  "mirror([1, 0, 0]) mirror([1, 0, 0]) children();",
    "indices backwards":
        "for (i = [$children - 1 : -1 : 0]) children($children - 1 - i);",
    "block":          "{ children(); }",
    "explicit union": "union() children();",
    "for of one":     "for (j = [0 : 0]) children();",
    "translate zero": "translate([0, 0, 0]) children();",
}

def program(r, body, n):
    kids = "\n  ".join("%s %s;" % (xform(r), shape(r)) for _ in range(n))
    return (
        "module inner() { children(); }\n"
        "module m() { %s }\n"
        "%s m() {\n  %s\n}\n" % (body, xform(r), kids)
    )

def run(src, out):
    open(os.path.join(W, "k.scad"), "w").write(src)
    p = subprocess.run([B, "-o", out, "--export-format", "asciistl", "k.scad"],
                       capture_output=True, text=True, cwd=W, timeout=180)
    if p.returncode != 0:
        return None, p.stderr or ""
    return open(os.path.join(W, out), "rb").read(), p.stderr or ""

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    n = 1 + seed % 4
    r = random.Random(seed)
    want, _ = run(program(r, BODIES["plain"], n), "k1.stl")
    if want is None:
        skipped += 1
        continue
    for name, body in BODIES.items():
        if name == "plain":
            continue
        r = random.Random(seed)              # same seed => same children
        got, err = run(program(r, body, n), "k2.stl")
        if got is None:
            print("SEED %d: '%s' failed to export: %s" % (seed, name, err.strip()[:160]))
            bad += 1
            continue
        if got != want:
            print("SEED %d: '%s' is not children() (%d vs %d bytes, %d children)"
                  % (seed, name, len(got), len(want), n))
            r = random.Random(seed)
            open(os.path.join(W, "bad_children_%d_%s.scad"
                              % (seed, name.replace(" ", "_"))), "w").write(
                program(r, body, n))
            bad += 1
            continue
        ok += 1
    # $children must be the count the caller passed.
    r = random.Random(seed)
    src = program(r, "echo($children); children();", n)
    _, err = run(src, "k3.stl")
    if ("ECHO: %d" % n) not in err:
        print("SEED %d: $children wrong for %d children: %s"
              % (seed, n, err.strip()[:160]))
        bad += 1
    else:
        ok += 1
print("children seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
