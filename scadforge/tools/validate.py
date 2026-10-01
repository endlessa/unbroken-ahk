#!/usr/bin/env python3
"""Check an exported STL the way the kernel cannot check itself.

  python3 validate.py FILE.stl

Reports, in order:
  tris          triangle count
  volume        signed volume by the divergence theorem; NEGATIVE means the
                solid is inside out, which renders identically and ruins
                every boolean it touches
  holes         undirected edges used an odd number of times -- a leak
  flipped       directed edges used twice with no reverse -- two faces that
                traverse a shared edge the SAME way round.  Closed by parity,
                no consistent inside.  This is the one that hides.
  split         directed edges with no reverse at all -- T-junctions.  A
                boolean's output has these legitimately; an authored mesh
                that never went through a boolean should have none.
  components    connected pieces, each with its own volume

Exit status is 1 if the mesh is inside out, leaks, or is inconsistently wound.
"""
import struct, sys
from collections import defaultdict

def read(p):
    d = open(p, 'rb').read()
    if d[:5].lower() == b'solid' and b'facet' in d[:2000]:
        out, cur = [], []
        for line in d.decode('utf8', 'replace').splitlines():
            s = line.lstrip()
            if s.startswith('vertex'):
                cur.append(tuple(float(x) for x in s.split()[1:4]))
                if len(cur) == 3:
                    out.append(tuple(cur)); cur = []
        return out
    n = struct.unpack('<I', d[80:84])[0]
    return [ (lambda f: (f[3:6], f[6:9], f[9:12]))(
                struct.unpack('<12f', d[84+i*50:84+i*50+48])) for i in range(n) ]

def main(path):
    T = read(path)
    if not T:
        print('%s: EMPTY' % path); return 1
    # Key on the EXACT coordinates as written, with no tolerance. A binary
    # STL holds f32, and two vertices the model shares are written from the
    # same f64 and so land on the same f32 -- they match exactly or they were
    # never the same point. Rounding instead was a real mistake here: at 4
    # decimals, on a bridge 2 km long with 0.15 m details, distinct vertices
    # collapsed together and the merged edge counts reported 50,166 holes in
    # a mesh that has none.
    k = lambda v: (v[0] + 0.0, v[1] + 0.0, v[2] + 0.0)
    det = lambda a, b, c: (a[0]*(b[1]*c[2]-b[2]*c[1]) + a[1]*(b[2]*c[0]-b[0]*c[2])
                           + a[2]*(b[0]*c[1]-b[1]*c[0])) / 6.0
    und, dir_ = defaultdict(int), defaultdict(int)
    par = {}
    def find(x):
        r = x
        while par[r] != r: r = par[r]
        while par[x] != r: par[x], x = r, par[x]
        return r
    def uni(a, b):
        ra, rb = find(a), find(b)
        if ra != rb: par[ra] = rb
    vol = 0.0
    for t in T:
        a, b, c = (k(v) for v in t)
        vol += det(*t)
        for x in (a, b, c): par.setdefault(x, x)
        uni(a, b); uni(b, c)
        for x, y in ((a, b), (b, c), (c, a)):
            und[tuple(sorted((x, y)))] += 1
            dir_[(x, y)] += 1
    holes   = sum(1 for e, n in und.items() if n % 2)
    flipped = sum(1 for e, n in dir_.items() if n >= 2 and not dir_.get((e[1], e[0])))
    split   = sum(1 for e, n in dir_.items() if not dir_.get((e[1], e[0])))
    comp = defaultdict(lambda: [0, 0.0])
    for t in T:
        r = find(k(t[0])); comp[r][0] += 1; comp[r][1] += det(*t)
    print('file        %s' % path)
    print('tris        %d' % len(T))
    print('volume      %+.6f  %s' % (vol, 'INSIDE OUT' if vol < 0 else 'ok'))
    print('holes       %d' % holes)
    print('flipped     %d' % flipped)
    print('split       %d  (T-junctions; expected only after a boolean)' % split)
    print('components  %d' % len(comp))
    for i, (_, (n, v)) in enumerate(sorted(comp.items(), key=lambda kv: -abs(kv[1][1]))[:8]):
        print('   %2d  %7d tris  %+.6f' % (i, n, v))
    return 1 if (vol < 0 or holes or flipped) else 0

if __name__ == '__main__':
    sys.exit(main(sys.argv[1]))
