"""Random OpenSCAD programs, both 3D- and 2D-rooted."""
import random

LEAF3 = [
    lambda r: "cube([%g, %g, %g], center = %s);" % (r.uniform(1,8), r.uniform(1,8), r.uniform(1,8), r.choice(("true","false"))),
    lambda r: "sphere(r = %g, $fn = %d);" % (r.uniform(1,6), r.randint(6,20)),
    lambda r: "cylinder(h = %g, r1 = %g, r2 = %g, $fn = %d);" % (r.uniform(1,9), r.uniform(0,5), r.uniform(0,5), r.randint(5,17)),
    lambda r: "polyhedron(points = [[0,0,0],[%g,0,0],[0,%g,0],[0,0,%g]], faces = [[0,2,1],[0,1,3],[1,2,3],[0,3,2]]);" % (r.uniform(1,6), r.uniform(1,6), r.uniform(1,6)),
]
LEAF2 = [
    lambda r: "square([%g, %g], center = %s);" % (r.uniform(1,8), r.uniform(1,8), r.choice(("true","false"))),
    lambda r: "circle(r = %g, $fn = %d);" % (r.uniform(1,6), r.randint(5,18)),
    lambda r: "polygon(points = [[0,0],[%g,0],[%g,%g],[0,%g]]);" % (r.uniform(1,7), r.uniform(1,7), r.uniform(1,7), r.uniform(1,7)),
    lambda r: 'text("%s", size = %g);' % (r.choice(("A","gW","o8","Mi")), r.uniform(4,12)),
    lambda r: "polygon(points = [[0,0],[%g,0],[%g,%g],[%g,%g],[0,%g]], paths = [[0,1,2,3,4]]);" % (
        r.uniform(3,8), r.uniform(3,8), r.uniform(1,3), r.uniform(1,3), r.uniform(3,8), r.uniform(3,8)),
]
XF = [
    lambda r: "translate([%g, %g, %g])" % (r.uniform(-8,8), r.uniform(-8,8), r.uniform(-4,4)),
    lambda r: "rotate([%g, %g, %g])" % (r.uniform(0,360), r.uniform(0,360), r.uniform(0,360)),
    lambda r: "rotate(%g, [%g, %g, %g])" % (r.uniform(0,360), r.uniform(-1,1), r.uniform(-1,1), r.uniform(0.1,1)),
    lambda r: "scale([%g, %g, %g])" % (r.uniform(0.3,2), r.uniform(0.3,2), r.uniform(0.3,2)),
    lambda r: "mirror([%g, %g, %g])" % (r.choice((0,1)), r.choice((0,1)), 1),
    lambda r: 'color("%s")' % r.choice(("red","SteelBlue","#3fa","rgba(10,20,30,0.5)")),
    lambda r: "multmatrix([[1,%g,0,%g],[0,1,%g,%g],[%g,0,1,%g]])" % tuple(r.uniform(-0.4,0.4) for _ in range(6)),
    lambda r: "resize([%g, 0, 0])" % r.uniform(2,12),
    lambda r: "render()",
]
XF2 = [
    lambda r: "translate([%g, %g])" % (r.uniform(-6,6), r.uniform(-6,6)),
    lambda r: "rotate([0, 0, %g])" % r.uniform(0,360),
    lambda r: "scale([%g, %g])" % (r.uniform(0.4,2), r.uniform(0.4,2)),
    lambda r: "mirror([%d, %d, 0])" % (r.choice((0,1)), 1),
    lambda r: 'color("%s")' % r.choice(("red","SteelBlue")),
    lambda r: "resize([%g, 0])" % r.uniform(2,10),
]
BOOL = ["union()", "intersection()", "difference()"]

MODULES = """
module wrap_all() { union() { children(); } }
module wrap_first() { if ($children > 0) children(0); }
module wrap_last() { if ($children > 0) children($children - 1); }
module wrap_range() { if ($children > 1) children([0 : $children - 2]); }
module wrap_pick() { for (k = [0 : $children - 1]) translate([0, 0, k * 0.5]) children(k); }
module ring(n = 3, rad = 6) { for (k = [0 : n - 1]) rotate([0, 0, k * 360 / n]) translate([rad, 0, 0]) children(); }
module cut(d = 2) { difference() { children(0); translate([0, 0, -1]) cylinder(h = 30, r = d, $fn = 12); } }
module ring2(n = 3, rad = 6) { for (k = [0 : n - 1]) rotate([0, 0, k * 360 / n]) translate([rad, 0]) children(); }
function lift(k) = [0, 0, k * 1.5];
"""
WRAP = ["wrap_all()", "wrap_first()", "wrap_last()", "wrap_range()", "wrap_pick()",
        "ring(n = 3, rad = 7)", "ring(n = 2, rad = 4)", "cut(d = 1.5)"]

def indent(s):
    return "\n".join("    " + l for l in s.split("\n"))

def node3(r, d):
    if d <= 0 or r.random() < 0.28:
        return r.choice(LEAF3)(r)
    k = r.random()
    if k < 0.30:
        return "%s {\n%s\n}" % (r.choice(BOOL), indent("\n".join(node3(r, d-1) for _ in range(r.randint(2,3)))))
    if k < 0.40:
        return "hull() {\n%s\n}" % indent("\n".join(node3(r, 0) for _ in range(r.randint(2,3))))
    if k < 0.47:
        return "minkowski() {\n%s\n}" % indent("\n".join([r.choice(LEAF3)(r), "sphere(r = %g, $fn = 8);" % r.uniform(0.3,1.5)]))
    if k < 0.58:
        return "linear_extrude(height = %g, twist = %g, slices = %d, scale = %g)\n%s" % (
            r.uniform(1,8), r.choice((0, 0, 90, -120)), r.randint(1,6), r.uniform(0.3,1.6), indent(node2(r, d-1)))
    if k < 0.66:
        return "rotate_extrude(angle = %g, $fn = %d)\n%s" % (
            r.choice((90, 180, 360, 360)), r.randint(8,24), indent(shifted2(r, d-1)))
    if k < 0.72:
        return "for (i = [0:%d])\n%s" % (r.randint(1,3), indent("rotate([0,0,i*%g])\n" % r.uniform(20,120) + node3(r, d-1)))
    if k < 0.77:
        return "intersection_for (i = [0:%d])\n%s" % (r.randint(1,2), indent("rotate([0,0,i*%g])\n" % r.uniform(20,90) + node3(r, d-1)))
    if k < 0.82:
        return "if (%s) {\n%s\n} else {\n%s\n}" % (r.choice(("true","false")), indent(node3(r, d-1)), indent(node3(r, d-1)))
    if k < 0.87:
        return "let (q = %g)\n%s" % (r.uniform(1,3), indent(node3(r, d-1)))
    if k < 0.91:
        return "%s%s" % (r.choice(("#", "%", "!", "")), node3(r, d-1))
    return "%s\n%s" % (r.choice(XF)(r), indent(node3(r, d-1)))

def node2(r, d):
    if d <= 0 or r.random() < 0.4:
        return r.choice(LEAF2)(r)
    k = r.random()
    if k < 0.32:
        return "%s {\n%s\n}" % (r.choice(BOOL), indent("\n".join(node2(r, d-1) for _ in range(r.randint(2,3)))))
    if k < 0.46:
        return "offset(%s)\n%s" % (r.choice(("r = 1", "r = -0.5", "delta = 0.8", "delta = 1, chamfer = true")), indent(node2(r, d-1)))
    if k < 0.56:
        return "hull() {\n%s\n}" % indent("\n".join(r.choice(LEAF2)(r) for _ in range(2)))
    if k < 0.63:
        return "minkowski() {\n%s\n}" % indent("\n".join([r.choice(LEAF2)(r), "circle(r = %g, $fn = 8);" % r.uniform(0.3,1.2)]))
    if k < 0.70:
        return "projection(cut = %s)\n%s" % (r.choice(("true","false")), indent(node3(r, 0)))
    if k < 0.76:
        return "for (i = [0:%d])\n%s" % (r.randint(1,2), indent("rotate([0,0,i*%g])\n" % r.uniform(30,120) + node2(r, d-1)))
    if k < 0.81:
        return "intersection_for (i = [0:%d])\n%s" % (r.randint(1,2), indent("rotate([0,0,i*%g])\n" % r.uniform(20,60) + node2(r, d-1)))
    if k < 0.86:
        return "ring2(n = %d, rad = %g) {\n%s\n}" % (r.randint(2,4), r.uniform(3,7), indent(r.choice(LEAF2)(r)))
    if k < 0.91:
        return "%s%s" % (r.choice(("#", "%", "")), node2(r, d-1))
    return "%s\n%s" % (r.choice(XF2)(r), indent(node2(r, d-1)))

def shifted2(r, d):
    """A profile entirely at x > 0, so rotate_extrude accepts it."""
    return "translate([%g, 0])\n%s" % (r.uniform(4,10), indent(r.choice(LEAF2)(r)))

def wrapped(r, d):
    n = r.randint(1, 3)
    body = "\n".join(node3(r, d - 1) for _ in range(n))
    return "%s {\n%s\n}" % (r.choice(WRAP), indent(body))

def statement(r, d):
    k = r.random()
    if k < 0.20:
        return wrapped(r, d)
    if k < 0.26:
        return "assign(z = %g)\n%s" % (r.uniform(1, 3), indent(node3(r, d - 1)))
    if k < 0.33:
        return "for (v = [%s])\n%s" % (", ".join("%g" % r.uniform(-5, 5) for _ in range(r.randint(2, 3))),
                                       indent("translate([v, 0, 0])\n" + indent(node3(r, d - 1))))
    if k < 0.39:
        return "for (p = [for (j = [0 : %d]) lift(j)])\n%s" % (r.randint(1, 3),
                                                               indent("translate(p)\n" + indent(node3(r, d - 1))))
    return node3(r, d)

def program(seed):
    r = random.Random(seed)
    return MODULES + "\n".join(statement(r, r.randint(2, 4)) for _ in range(r.randint(1, 4))) + "\n"

def program2d(seed):
    """A purely 2D design, so the SVG/DXF export path is the one under test."""
    r = random.Random(seed ^ 0x5eed)
    return MODULES + "\n".join(node2(r, r.randint(2, 4)) for _ in range(r.randint(1, 3))) + "\n"
