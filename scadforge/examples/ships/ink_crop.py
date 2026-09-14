# Crop a PNG to the bounding box of everything that is not the viewer's
# background, with a margin.  No PIL in this container, so the PNG is
# decoded by hand: IHDR for the header, the IDAT chunks concatenated and
# inflated, then the five PNG row filters undone in place.
import sys, zlib, struct

def load(path):
    d = open(path, 'rb').read()
    assert d[:8] == b'\x89PNG\r\n\x1a\n', path
    pos, idat, hdr = 8, [], None
    while pos < len(d):
        (ln,) = struct.unpack('>I', d[pos:pos+4]); typ = d[pos+4:pos+8]
        body = d[pos+8:pos+8+ln]
        if typ == b'IHDR': hdr = struct.unpack('>IIBBBBB', body)
        elif typ == b'IDAT': idat.append(body)
        pos += 12 + ln
    w, h, depth, ctype, comp, filt, inter = hdr
    assert depth == 8 and inter == 0, (depth, inter)
    ch = {0:1, 2:3, 3:1, 4:2, 6:4}[ctype]
    raw = zlib.decompress(b''.join(idat))
    stride = w*ch
    out = bytearray(h*stride)
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        f = raw[p]; p += 1
        line = bytearray(raw[p:p+stride]); p += stride
        if f == 1:
            for i in range(ch, stride): line[i] = (line[i] + line[i-ch]) & 255
        elif f == 2:
            for i in range(stride): line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i-ch] if i >= ch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i-ch] if i >= ch else 0
                b = prev[i]; c = prev[i-ch] if i >= ch else 0
                pa, pb, pc = abs(b-c), abs(a-c), abs(a+b-2*c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        out[y*stride:(y+1)*stride] = line
        prev = line
    return w, h, ch, out

def save(path, w, h, ch, buf):
    ctype = {1:0, 2:4, 3:2, 4:6}[ch]
    stride = w*ch
    raw = bytearray()
    for y in range(h):
        raw.append(0); raw += buf[y*stride:(y+1)*stride]
    def chunk(t, b):
        return struct.pack('>I', len(b)) + t + b + struct.pack('>I', zlib.crc32(t+b) & 0xffffffff)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n'
        + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, ctype, 0, 0, 0))
        + chunk(b'IDAT', zlib.compress(bytes(raw), 6)) + chunk(b'IEND', b''))

def crop(src, dst, tol=10, pad=0.035, ui=0.045):
    # The viewer paints "drag to orbit - wheel to zoom" into the bottom-left
    # of its own canvas, and that text is ink like any other: leave it in and
    # every crop is anchored to it instead of to the ship.
    w, h, ch, buf = load(src)
    h_scan = int(h*(1.0-ui))
    bg = buf[0:3]
    x0, y0, x1, y1 = w, h, -1, -1
    for y in range(h_scan):
        row = y*w*ch
        for x in range(w):
            i = row + x*ch
            if (abs(buf[i]-bg[0]) > tol or abs(buf[i+1]-bg[1]) > tol or abs(buf[i+2]-bg[2]) > tol):
                if x < x0: x0 = x
                if x > x1: x1 = x
                if y < y0: y0 = y
                if y > y1: y1 = y
    if x1 < 0: raise SystemExit('all background: ' + src)
    m = int(pad*max(x1-x0, y1-y0))
    x0 = max(0, x0-m); y0 = max(0, y0-m); x1 = min(w-1, x1+m); y1 = min(h-1, y1+m)
    cw, cw_h = x1-x0+1, y1-y0+1
    nb = bytearray(cw*cw_h*ch)
    for y in range(cw_h):
        s = ((y+y0)*w + x0)*ch
        nb[y*cw*ch:(y+1)*cw*ch] = buf[s:s+cw*ch]
    save(dst, cw, cw_h, ch, nb)
    print('%-18s %dx%d -> %dx%d' % (dst.split('/')[-1], w, h, cw, cw_h))

for name in sys.argv[1:]:
    crop('demo/%s.png' % name, 'demo/%s_c.png' % name)
