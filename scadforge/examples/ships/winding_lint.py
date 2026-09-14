# Winding audit, done properly.
#
# Reports SIDES and CAPS separately for swept meshes.  A single
# whole-mesh volume sign conflates two different faults: a mesh whose
# side faces are reversed is badly broken and lit wrong everywhere, while
# a mesh with only its end caps reversed is fine except at two faces that
# are usually buried anyway.  Worse, a reversed cap's contribution to the
# signed volume scales with its distance from the origin, so the same cap
# fault flips the total on one mesh and not on another.  Both were true
# at once here and the combination is very hard to read from one number.
# For a CLOSED mesh with outward normals the divergence-theorem volume
#   V = (1/6) * sum over faces of  a . (b x c)
# is positive.  Negative means the mesh is inside-out, and since the viewer
# takes its normals straight from the winding (index.html:208) an inside-out
# mesh shows every visible face at the ambient floor d=0.25: flat, no
# gradient, "not affected by the lighting like the things around it".
# The centroid test I tried first is invalid for thin plates -- on a plate
# the in-plane offset dwarfs the thickness, so it flips sign at random.
import json, math, sys
d = json.load(open(sys.argv[1]))
L = (0.5, 0.35, 0.85); ln = math.sqrt(sum(c*c for c in L)); L = tuple(c/ln for c in L)

NAMED = {}
for line in open(sys.argv[2]):
    t = line.strip()
    if '=' in t and '[' in t and t.split('=')[0].strip().isupper():
        try:
            v = [float(x) for x in t.split('[')[1].split(']')[0].split(',')]
            if len(v) == 3: NAMED[tuple(round(x,4) for x in v)] = t.split('=')[0].strip()
        except Exception: pass

rows = []
for mi, m in enumerate(d['meshes']):
    P, I, col = m['positions'], m['indices'], m['color'][:3]
    if not I: continue
    V = [P[i:i+3] for i in range(0, len(P), 3)]
    vol = 0.0; ds = []; lo = [1e9]*3; hi = [-1e9]*3
    for k in range(0, len(I), 3):
        a, b, c = V[I[k]], V[I[k+1]], V[I[k+2]]
        vol += (a[0]*(b[1]*c[2]-b[2]*c[1])
              - a[1]*(b[0]*c[2]-b[2]*c[0])
              + a[2]*(b[0]*c[1]-b[1]*c[0]))
        ux,uy,uz = b[0]-a[0], b[1]-a[1], b[2]-a[2]
        vx,vy,vz = c[0]-a[0], c[1]-a[1], c[2]-a[2]
        nx,ny,nz = uy*vz-uz*vy, uz*vx-ux*vz, ux*vy-uy*vx
        nl = math.hypot(nx,ny,nz)
        if nl > 1e-12: ds.append(max(0.0,(nx*L[0]+ny*L[1]+nz*L[2])/nl)*0.75+0.25)
        for p in (a,b,c):
            for j in range(3):
                lo[j] = min(lo[j], p[j]); hi[j] = max(hi[j], p[j])
    if not ds: continue
    ctr = [round((lo[j]+hi[j])/2, 1) for j in range(3)]
    rows.append(dict(i=mi, name=NAMED.get(tuple(round(x,4) for x in col), str([round(x,2) for x in col])),
                     tris=len(ds), vol=vol/6.0, ctr=ctr,
                     floor=sum(1 for x in ds if x < 0.2501)/len(ds)))

bad = [r for r in rows if r['vol'] < 0]
print("meshes %d   INSIDE-OUT: %d" % (len(rows), len(bad)))
for r in sorted(bad, key=lambda r: -r['tris']):
    print("  #%-4d %-6s tris=%-5d vol=%+9.2f  centre=%s  at-floor=%.0f%%"
          % (r['i'], r['name'], r['tris'], r['vol'], r['ctr'], 100*r['floor']))

print("\n-- parts near the starboard wing tip (y < -12) --")
for r in rows:
    if r['ctr'][1] < -12:
        print("  #%-4d %-6s tris=%-5d vol=%+8.2f centre=%s" % (r['i'], r['name'], r['tris'], r['vol'], r['ctr']))
print("\n-- parts near the starboard insignia (-9 < y < -4, |x| < 6) --")
for r in rows:
    if -9 < r['ctr'][1] < -4 and abs(r['ctr'][0]) < 6 and r['tris'] < 700:
        print("  #%-4d %-6s tris=%-5d vol=%+8.2f centre=%s" % (r['i'], r['name'], r['tris'], r['vol'], r['ctr']))
