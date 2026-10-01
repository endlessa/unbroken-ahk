#!/usr/bin/env python3
"""Cylindrical and axis-aligned extents of an exported binary STL.

  python3 bbox.py FILE.stl [FILE.stl ...]

Prints, per file, the radial range about the z axis, the z range and the x
range.  Written for the transmission parts, where almost every clearance is
stated as a radius or a z station, so the mesh's own extents are what you
check a published envelope against.
"""
# Die quietly on a closed pipe: `validate.py x.stl | head` otherwise
# ends in a BrokenPipeError traceback, which looks like the mesh is
# broken when it is only the reader that went away.
import signal
try: signal.signal(signal.SIGPIPE, signal.SIG_DFL)
except (AttributeError, ValueError): pass   # not POSIX, or not the main thread
import struct, sys, math
for f in sys.argv[1:]:
    d = open(f,'rb').read()
    n = struct.unpack('<I', d[80:84])[0]
    lo = [1e30]*3; hi = [-1e30]*3; rmax = 0.0; rmin = 1e30
    for k in range(n):
        o = 84 + k*50 + 12
        for v in range(3):
            x,y,z = struct.unpack('<3f', d[o+v*12:o+v*12+12])
            for a,c in enumerate((x,y,z)):
                lo[a] = min(lo[a], c); hi[a] = max(hi[a], c)
            r = math.hypot(x,y); rmax = max(rmax,r); rmin = min(rmin,r)
    print("%-14s r %8.3f..%8.3f   z %9.3f..%9.3f   x %8.3f..%8.3f" %
          (f.split('/')[-1], rmin, rmax, lo[2], hi[2], lo[0], hi[0]))
