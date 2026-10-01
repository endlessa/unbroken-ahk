import json, math, sys, zlib, struct
src, dst = sys.argv[1], sys.argv[2]
AZ = float(sys.argv[3]) if len(sys.argv) > 3 else 55.0
EL = float(sys.argv[4]) if len(sys.argv) > 4 else 28.0
W = int(sys.argv[5]) if len(sys.argv) > 5 else 780
# A separate height, because a bridge two kilometres long and a hundred
# and sixty metres tall is nearly invisible in a square frame.
H = int(sys.argv[6]) if len(sys.argv) > 6 else W
d = json.load(open(src))
az, el = math.radians(AZ), math.radians(EL)
ca, sa, ce, se = math.cos(az), math.sin(az), math.cos(el), math.sin(el)
CAM = (ca*ce, sa*ce, se); RGT = (-sa, ca, 0.0); UP = (-ca*se, -sa*se, ce)
def view(p):
    x, y, z = p
    return (x*RGT[0]+y*RGT[1], x*UP[0]+y*UP[1]+z*UP[2], -(x*CAM[0]+y*CAM[1]+z*CAM[2]))
tris = []
for m in d['meshes']:
    P, I, col = m['positions'], m['indices'], m['color']
    if m.get('background'): col = [0.45, 0.45, 0.48, 1]
    for k in range(0, len(I), 3):
        a, b, c = I[k]*3, I[k+1]*3, I[k+2]*3
        tris.append((P[a:a+3], P[b:b+3], P[c:c+3], col))
if not tris:
    raise SystemExit('no geometry')
pts = [view(v) for t in tris for v in t[:3]]
xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
cx, cy = (min(xs)+max(xs))/2, (min(ys)+max(ys))/2
sx = (max(xs)-min(xs)) or 1.0; sy = (max(ys)-min(ys)) or 1.0
s = 0.90*min(W/sx, H/sy)
zbuf = [1e30]*(W*H)
BG = (14, 18, 26)
img = bytearray(BG*(W*H))
LIGHT = (0.35, 0.45, 0.82)
for a, b, c, col in tris:
    va, vb, vc = view(a), view(b), view(c)
    def px(v): return ((v[0]-cx)*s + W/2, H/2 - (v[1]-cy)*s, v[2])
    pa, pb, pc = px(va), px(vb), px(vc)
    u = [b[i]-a[i] for i in range(3)]; w = [c[i]-a[i] for i in range(3)]
    n = (u[1]*w[2]-u[2]*w[1], u[2]*w[0]-u[0]*w[2], u[0]*w[1]-u[1]*w[0])
    ln = math.sqrt(sum(q*q for q in n)) or 1.0
    n = tuple(q/ln for q in n)
    lam = abs(sum(n[i]*LIGHT[i] for i in range(3)))
    sh = 0.25 + 0.75*lam
    rgb = tuple(max(0, min(255, int(255*col[i]*sh))) for i in range(3))
    minx = max(0, int(min(pa[0], pb[0], pc[0]))); maxx = min(W-1, int(max(pa[0], pb[0], pc[0]))+1)
    miny = max(0, int(min(pa[1], pb[1], pc[1]))); maxy = min(H-1, int(max(pa[1], pb[1], pc[1]))+1)
    d0 = (pb[1]-pc[1])*(pa[0]-pc[0]) + (pc[0]-pb[0])*(pa[1]-pc[1])
    if d0 == 0: continue
    for yy in range(miny, maxy+1):
        for xx in range(minx, maxx+1):
            l1 = ((pb[1]-pc[1])*(xx+0.5-pc[0]) + (pc[0]-pb[0])*(yy+0.5-pc[1]))/d0
            l2 = ((pc[1]-pa[1])*(xx+0.5-pc[0]) + (pa[0]-pc[0])*(yy+0.5-pc[1]))/d0
            l3 = 1-l1-l2
            if l1 < 0 or l2 < 0 or l3 < 0: continue
            z = l1*pa[2] + l2*pb[2] + l3*pc[2]
            at = yy*W+xx
            if z < zbuf[at]:
                zbuf[at] = z
                img[at*3:at*3+3] = bytes(rgb)
raw = b''.join(b'\x00' + bytes(img[y*W*3:(y+1)*W*3]) for y in range(H))
def chunk(t, data):
    c = t + data
    return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
png = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 2, 0, 0, 0))
       + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))
open(dst, 'wb').write(png)
print('wrote', dst)
