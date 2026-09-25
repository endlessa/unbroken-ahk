"""Every 3D exchange format must be an involution, and they must agree.

For each random design and each format F:

  pass 1   design       -> E1
  pass 2   import(E1)   -> E2
  pass 3   import(E2)   -> E3

  E2 == E3            F is a fixed point: reading and writing it again
                      changes nothing, so the reader and the writer agree
                      about what the file means
  tris(E1) == tris(E2) nothing is lost or invented on the way through

Pass 1 is allowed to differ from pass 2 for a format that stores single
precision -- binary STL does -- so the fixed point starts at pass 2, the same
way the 2D vector check in run.py does.

Then the formats are checked against each other: re-importing each of them and
writing ASCII STL must give the same triangle count every time, and volumes
must agree. A format that silently drops a face shows up here even though its
own round trip is a perfectly stable fixed point.
"""
import os, subprocess, sys
try:
    import gen
except ImportError:
    gen = None
B = os.environ.get(
    "SCADFORGE",
    os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(__file__)))), "target", "release", "scadforge"),
)
W = os.path.dirname(os.path.abspath(__file__))

FORMATS = [("asciistl", "stl"), ("binstl", "stl"), ("off", "off"),
           ("3mf", "3mf"), ("amf", "amf")]

def run(src_file, out, fmt):
    try:
        r = subprocess.run([B, "-o", out, "--export-format", fmt, src_file],
                           capture_output=True, text=True, cwd=W, timeout=300)
    except subprocess.TimeoutExpired:
        # A timeout is a finding, not a crash: report it and keep going. The
        # quadratic XML import was found exactly this way.
        return 124, "timed out after 300s"
    return r.returncode, (r.stderr or "")

def write(name, text):
    p = os.path.join(W, name)
    open(p, "w").write(text)
    return name

def rd(name):
    return open(os.path.join(W, name), "rb").read()

def tris_of(ascii_stl):
    return rd(ascii_stl).count(b"facet normal")

def volume_of(ascii_stl):
    v = 0.0
    tri = []
    for line in open(os.path.join(W, ascii_stl)):
        s = line.lstrip()
        if s.startswith("vertex"):
            tri.append([float(x) for x in s.split()[1:4]])
            if len(tri) == 3:
                a, c, d = tri
                v += (a[0]*(c[1]*d[2]-c[2]*d[1]) + a[1]*(c[2]*d[0]-c[0]*d[2])
                      + a[2]*(c[0]*d[1]-c[1]*d[0])) / 6.0
                tri = []
    return abs(v)

lo, hi = int(sys.argv[1]), int(sys.argv[2])
bad = skipped = ok = 0
for seed in range(lo, hi):
    src = gen.program(seed) if gen else "sphere(r=5, $fn=16);"
    write("rt.scad", src)
    if run("rt.scad", "rt_probe.stl", "asciistl")[0] != 0:
        skipped += 1
        continue
    base_tris = tris_of("rt_probe.stl")
    seen = {}
    failed = []
    for fmt, ext in FORMATS:
        e1 = "rt1_%s.%s" % (fmt, ext)
        if run("rt.scad", e1, fmt)[0] != 0:
            failed.append((fmt, "pass 1 export failed"))
            continue
        imp1 = write("rt_i1_%s.scad" % fmt, 'import("%s");\n' % e1)
        e2 = "rt2_%s.%s" % (fmt, ext)
        rc, err = run(imp1, e2, fmt)
        if rc != 0:
            failed.append((fmt, "re-import failed: " + err.strip()[:120]))
            continue
        imp2 = write("rt_i2_%s.scad" % fmt, 'import("%s");\n' % e2)
        e3 = "rt3_%s.%s" % (fmt, ext)
        rc, err = run(imp2, e3, fmt)
        if rc != 0:
            failed.append((fmt, "second re-import failed: " + err.strip()[:120]))
            continue
        if rd(e2) != rd(e3):
            failed.append((fmt, "not a fixed point: pass 2 is %d bytes, pass 3 is %d"
                           % (len(rd(e2)), len(rd(e3)))))
            continue
        # the same mesh, written as ASCII STL, for the cross-format comparison
        flat = "rt_flat_%s.stl" % fmt
        if run(imp1, flat, "asciistl")[0] != 0:
            failed.append((fmt, "could not re-export the re-import as ASCII STL"))
            continue
        n = tris_of(flat)
        if n != base_tris:
            failed.append((fmt, "%d triangles after the round trip, %d before"
                           % (n, base_tris)))
            continue
        seen[fmt] = (n, volume_of(flat))
    if len(seen) > 1:
        vols = [v for _, v in seen.values()]
        # binary STL is single precision, so allow a float32-sized relative gap
        if max(vols) - min(vols) > 1e-5 * max(1.0, max(vols)):
            failed.append(("cross-format", "volumes disagree: "
                           + ", ".join("%s %.6f" % (f, v) for f, (_, v) in seen.items())))
    if failed:
        print("SEED %d:" % seed)
        for fmt, why in failed:
            print("   %-12s %s" % (fmt, why))
        open(os.path.join(W, "bad_rt_%d.scad" % seed), "w").write(src)
        bad += 1
        continue
    ok += 1
print("roundtrip seeds %d..%d  clean %d  skipped %d  BAD %d" % (lo, hi, ok, skipped, bad))
