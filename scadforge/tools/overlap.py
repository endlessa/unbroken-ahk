# overlap.py -- do any two bodies of an exported scene actually share space?
#
#   python3 scadforge/tools/overlap.py OUT.stl [max_components]
#
# This answers the question every part file under examples/transmission
# raises and then has to leave open.  Those files draw members that SHARE
# NO VOLUME, and nothing downstream could confirm it: the component count
# cannot, because two shells that interpenetrate without sharing a vertex
# still count as two, and past csg::MAX_MERGE_TRIS the export-time union
# does not run at all.  Each file falls back on its printed clearances,
# and says so.  This is the check itself.
#
# It belongs in the kernel, not here -- "are these two shells disjoint" is
# what the union is for -- and this script is the evidence until it gets
# there.  Results so far, over the transmission parts:
#
#   collar_chain    14 bodies      43,032 tests   ALL DISJOINT
#   polar_cap       40 bodies   4,301,290 tests   ALL DISJOINT
#   pole joint       6 bodies  23,790,749 tests   ALL DISJOINT
#   equator_band     2 bodies         223 tests   OVERLAP FOUND
#     ...after the fix               111,768,502  ALL DISJOINT
#
# The band's crown seat was cut from the same rule as the ring's own back
# cone, so the two surfaces coincided exactly: a contact fit the file had
# declared and printed, invisible to volume, to the component count and to
# the export-time union alike.  Note the two test counts on it -- 223 to
# find a crossing, because the search stops at the first one, against
# 111,768,502 to establish there is none.  A negative costs everything.
#
# Two CLOSED surfaces share space iff an edge of one crosses a face of the
# other, OR one is wholly inside the other with no crossing at all.  The
# first is tested exactly (Moller-Trumbore, no tolerance); the second is
# tested afterwards by a ray cast from one vertex of each non-crossing
# body, so nesting is not missed.
#
# Broad phase is a uniform 3D grid over the pair's common box, and every
# triangle is inserted into EVERY cell its bounding box touches.  (The
# version this grew from bucketed by the azimuth of a triangle's CENTROID
# with a fixed +/-3 degree window, which silently drops a triangle wider
# than the window -- a large annular face is exactly that, and a false
# "disjoint" is the worst answer a checker can give.)
# Die quietly on a closed pipe: `overlap.py x.stl | head` otherwise ends in
# a BrokenPipeError traceback, which reads like a failure of the check.
import signal, sys, struct, math
from collections import defaultdict
try: signal.signal(signal.SIGPIPE, signal.SIG_DFL)
except (AttributeError, ValueError): pass   # not POSIX, or not the main thread

def read_stl(p):
    d = open(p, 'rb').read()
    n = struct.unpack('<I', d[80:84])[0]
    out = []
    for i in range(n):
        f = struct.unpack('<12f', d[84+i*50:84+i*50+48])
        out.append((f[3:6], f[6:9], f[9:12]))
    return out

def components(T):
    par = {}
    def find(x):
        r = x
        while par[r] != r: r = par[r]
        while par[x] != r: par[x], x = r, par[x]
        return r
    def uni(a, b):
        ra, rb = find(a), find(b)
        if ra != rb: par[ra] = rb
    for t in T:
        for v in t: par.setdefault(v, v)
        uni(t[0], t[1]); uni(t[1], t[2])
    g = defaultdict(list)
    for t in T: g[find(t[0])].append(t)
    return sorted(g.values(), key=len, reverse=True)

def box(M):
    lo = [min(v[k] for t in M for v in t) for k in range(3)]
    hi = [max(v[k] for t in M for v in t) for k in range(3)]
    return lo, hi

def tbox(t):
    return ([min(v[k] for v in t) for k in range(3)],
            [max(v[k] for v in t) for k in range(3)])

def seg_tri(p, q, tri):
    v0, v1, v2 = tri
    e1 = tuple(v1[k]-v0[k] for k in range(3))
    e2 = tuple(v2[k]-v0[k] for k in range(3))
    d  = tuple(q[k]-p[k] for k in range(3))
    pv = (d[1]*e2[2]-d[2]*e2[1], d[2]*e2[0]-d[0]*e2[2], d[0]*e2[1]-d[1]*e2[0])
    det = e1[0]*pv[0] + e1[1]*pv[1] + e1[2]*pv[2]
    if -1e-14 < det < 1e-14: return False
    inv = 1.0/det
    tv = tuple(p[k]-v0[k] for k in range(3))
    u = (tv[0]*pv[0] + tv[1]*pv[1] + tv[2]*pv[2]) * inv
    if u < 0.0 or u > 1.0: return False
    qv = (tv[1]*e1[2]-tv[2]*e1[1], tv[2]*e1[0]-tv[0]*e1[2], tv[0]*e1[1]-tv[1]*e1[0])
    v = (d[0]*qv[0] + d[1]*qv[1] + d[2]*qv[2]) * inv
    if v < 0.0 or u+v > 1.0: return False
    s = (e2[0]*qv[0] + e2[1]*qv[1] + e2[2]*qv[2]) * inv
    return 0.0 <= s <= 1.0

def ray_inside(p, M):
    """Is point p inside closed surface M?  Parity of +x crossings."""
    hits = 0
    for v0, v1, v2 in M:
        if max(v0[0], v1[0], v2[0]) < p[0]: continue
        e1 = tuple(v1[k]-v0[k] for k in range(3))
        e2 = tuple(v2[k]-v0[k] for k in range(3))
        pv = (0.0*e2[2]-0.0*e2[1], 0.0*e2[0]-1.0*e2[2], 1.0*e2[1]-0.0*e2[0])
        det = e1[0]*pv[0] + e1[1]*pv[1] + e1[2]*pv[2]
        if -1e-14 < det < 1e-14: continue
        inv = 1.0/det
        tv = tuple(p[k]-v0[k] for k in range(3))
        u = (tv[0]*pv[0] + tv[1]*pv[1] + tv[2]*pv[2]) * inv
        if u < 0.0 or u > 1.0: continue
        qv = (tv[1]*e1[2]-tv[2]*e1[1], tv[2]*e1[0]-tv[0]*e1[2], tv[0]*e1[1]-tv[1]*e1[0])
        v = (1.0*qv[0]) * inv
        if v < 0.0 or u+v > 1.0: continue
        s = (e2[0]*qv[0] + e2[1]*qv[1] + e2[2]*qv[2]) * inv
        if s > 0.0: hits += 1
    return hits % 2 == 1

def main(path, nmax=None):
    T = read_stl(path)
    C = components(T)
    if nmax: C = C[:nmax]
    BX = [box(c) for c in C]
    print("%s: %d triangles, %d components" % (path.split('/')[-1], len(T), len(C)))
    pairs = bad = tests = 0
    nested = []
    for i in range(len(C)):
        for j in range(i+1, len(C)):
            (al, ah), (bl, bh) = BX[i], BX[j]
            if not all(al[k] <= bh[k] and bl[k] <= ah[k] for k in range(3)):
                continue
            lo = [max(al[k], bl[k]) for k in range(3)]
            hi = [min(ah[k], bh[k]) for k in range(3)]
            A = [t for t in C[i] if _hits(tbox(t), lo, hi)]
            B = [t for t in C[j] if _hits(tbox(t), lo, hi)]
            if not A or not B: continue
            pairs += 1
            n = max(1, min(48, int(round((len(A)+len(B))**(1/3.0)))))
            step = [max((hi[k]-lo[k])/n, 1e-12) for k in range(3)]
            def cell(v, k): return min(n-1, max(0, int((v-lo[k])/step[k])))
            grid = defaultdict(list)
            for t in B:
                tl, th = tbox(t)
                for x in range(cell(tl[0],0), cell(th[0],0)+1):
                    for y in range(cell(tl[1],1), cell(th[1],1)+1):
                        for z in range(cell(tl[2],2), cell(th[2],2)+1):
                            grid[(x,y,z)].append(t)
            hit = 0
            for t in A:
                for (p, q) in ((t[0],t[1]), (t[1],t[2]), (t[2],t[0])):
                    el = [min(p[k],q[k]) for k in range(3)]
                    eh = [max(p[k],q[k]) for k in range(3)]
                    seen = set()
                    for x in range(cell(el[0],0), cell(eh[0],0)+1):
                        for y in range(cell(el[1],1), cell(eh[1],1)+1):
                            for z in range(cell(el[2],2), cell(eh[2],2)+1):
                                for tri in grid.get((x,y,z), ()):
                                    if id(tri) in seen: continue
                                    seen.add(id(tri)); tests += 1
                                    if seg_tri(p, q, tri):
                                        hit += 1
                                        break
                                if hit: break
                            if hit: break
                        if hit: break
                    if hit: break
                if hit: break
            if hit:
                bad += 1
                print("  INTERSECT  body %d (%d tris) x body %d (%d tris)"
                      % (i, len(C[i]), j, len(C[j])))
            else:
                # no surface crossing: one could still be wholly inside the
                # other, which shares space just as much.
                if all(al[k] >= bl[k] and ah[k] <= bh[k] for k in range(3)):
                    if ray_inside(C[i][0][0], C[j]): nested.append((i, j))
                elif all(bl[k] >= al[k] and bh[k] <= ah[k] for k in range(3)):
                    if ray_inside(C[j][0][0], C[i]): nested.append((j, i))
    for (a, b) in nested:
        print("  NESTED     body %d lies wholly inside body %d" % (a, b))
    print("bbox-overlapping pairs tested %d   intersecting %d   nested %d   "
          "segment-triangle tests %d" % (pairs, bad, len(nested), tests))
    print("VERDICT:", "ALL DISJOINT" if bad == 0 and not nested else "OVERLAP FOUND")

def _hits(tb, lo, hi):
    tl, th = tb
    return all(tl[k] <= hi[k] and lo[k] <= th[k] for k in range(3))

main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else None)
