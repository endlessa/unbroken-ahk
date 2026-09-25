"""Four identities for the modifier characters, none of which needs an oracle.

For a design whose statement B sits under some ancestor transforms:

   *B  (disable)     exports as if B had never been written
   %B  (background)  exports as if B had never been written  (preview-only)
   #B  (debug)       exports exactly as plain B does         (tint-only)
   !B  (root)        exports as B alone, ancestor transforms still applied

The right-hand side of each is a second program built by editing the source,
so the comparison is between two runs of the kernel and needs nothing else.
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
        "sphere(r = %d, $fn = %d)" % (r.randrange(2, 6), r.choice([8, 12, 16])),
        "cylinder(h = %d, r1 = %d, r2 = %d, $fn = %d)"
        % (r.randrange(2, 8), r.randrange(1, 5), r.randrange(0, 5), r.choice([6, 10, 16])),
        "polyhedron(points=[[0,0,0],[4,0,0],[0,4,0],[0,0,4]], "
        "faces=[[0,1,2],[0,3,1],[0,2,3],[1,3,2]])",
        "linear_extrude(%d) square(%d, center = true)" % (r.randrange(1, 5), r.randrange(2, 6)),
    ])

def xform(r):
    return r.choice([
        "translate([%d, %d, %d])" % tuple(r.randrange(-4, 5) for _ in range(3)),
        "rotate([%d, %d, %d])" % tuple(r.choice([0, 30, 45, 90, 180]) for _ in range(3)),
        "scale([%s, %s, %s])" % tuple("%.1f" % r.uniform(0.5, 2.0) for _ in range(3)),
        "mirror([%d, %d, %d])" % (r.randrange(0, 2), r.randrange(0, 2), 1),
    ])

def design(r, mod, keep_only_b, drop_b):
    """One program. `mod` prefixes B; keep_only_b/drop_b build the oracle side.

    The modifier is applied to a transform node, so what it governs is a
    subtree rather than a single leaf, and the group around it is a union, a
    difference or an intersection in turn -- `!` has to beat all three, and
    `*`/`%` have to leave the remaining operands in their original order.
    """
    depth = r.randrange(1, 3)
    pre = " ".join(xform(r) for _ in range(depth))
    a, b, c = shape(r), shape(r), shape(r)
    inner = xform(r)
    group = r.choice(["union", "difference", "intersection"])
    if keep_only_b:
        # `!` roots the subtree it marks: the group vanishes, the ancestor
        # transforms above it do not, and neither does the transform it is
        # attached to.
        return "%s %s %s;\n" % (pre, inner, b)
    body = ["%s;" % a]
    if not drop_b:
        body.append("%s%s %s;" % (mod, inner, b))
    body.append("%s;" % c)
    return "%s %s() {\n  %s\n}\n" % (pre, group, "\n  ".join(body))

def run(src, out):
    open(os.path.join(W, "m.scad"), "w").write(src)
    r = subprocess.run([B, "-o", out, "--export-format", "asciistl", "m.scad"],
                       capture_output=True, text=True, cwd=W, timeout=180)
    if r.returncode != 0:
        return None
    return open(os.path.join(W, out), "rb").read()

CASES = [
    # modifier, how to build the program it must equal
    ("*", dict(drop_b=True,  keep_only_b=False)),
    ("%", dict(drop_b=True,  keep_only_b=False)),
    ("#", dict(drop_b=False, keep_only_b=False)),
    ("!", dict(drop_b=False, keep_only_b=True)),
]

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    for mod, how in CASES:
        r = random.Random(seed)
        got = run(design(r, mod, keep_only_b=False, drop_b=False), "m1.stl")
        r = random.Random(seed)          # same seed => same shapes and transforms
        want = run(design(r, "" if mod == "#" else mod, **how), "m2.stl")
        if got is None or want is None:
            skipped += 1
            continue
        if got != want:
            print("SEED %d: '%s' does not match its equivalent (%d vs %d bytes)"
                  % (seed, mod, len(got), len(want)))
            r = random.Random(seed)
            open(os.path.join(W, "bad_mod_%s_%d.scad" % (mod.replace("*", "star")
                 .replace("%", "pct").replace("#", "hash").replace("!", "bang"), seed)),
                 "w").write(design(r, mod, keep_only_b=False, drop_b=False))
            bad += 1
            continue
        ok += 1
print("mods seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
