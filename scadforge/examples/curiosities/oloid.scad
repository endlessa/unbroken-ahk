// ===================================================================
//  THE OLOID  --  Paul Schatz, 1929
//
//  Take two circles of the same radius r.  Put them in perpendicular
//  planes and slide them until each one passes through the centre of
//  the other.  Their centres are then exactly r apart, because the
//  centre of the second circle is a point of the first.  The convex
//  hull of the pair is the oloid.
//
//  Put the first circle in z = 0 about the origin and the second in
//  y = 0 about (r, 0, 0):
//
//      A(s) = ( r cos s,  r sin s,  0 )          x^2 + y^2 = r^2, z = 0
//      B(u) = ( r + r cos u,  0,  r sin u )   (x-r)^2 + z^2 = r^2, y = 0
//
//  A(0) = (r,0,0) is the centre of B.  B(180) = (0,0,0) is the centre
//  of A.  The two planes meet at right angles.  That is the whole
//  definition, and everything below is a consequence of it.
//
//  ---- the rulings, and why they are all the same length -----------
//
//  Nothing here is hulled.  The boundary of the hull between the two
//  circles is a developable surface swept by straight segments, and
//  the segments can be found exactly.
//
//  A plane n.X = d supports the hull when it touches both circles.
//  Write n = (p, q, w).  Over circle A the form n.X peaks at
//  r*sqrt(p^2+q^2); over circle B it peaks at p*r + r*sqrt(p^2+w^2).
//  A common supporting plane needs these equal, so with
//
//      k = sqrt(p^2 + q^2),      m = sqrt(p^2 + w^2)
//
//  the condition is simply   k - m = p.   The touch points are
//
//      cos s = p/k,  sin s = q/k,        cos u = p/m,  sin u = w/m
//
//  and the segment A(s)B(u) is the ruling lying in that plane.
//  Eliminating p gives the relation between the two angles,
//
//      cos u = cos s / (1 - cos s),
//
//  and substituting it into |A(s) - B(u)|^2 collapses everything:
//
//      |AB|^2 = r^2 [ (cos s - 1 - cos u)^2 + sin^2 s + sin^2 u ]
//             = r^2 [ 1 + cos^2 s + cos^2 u + sin^2 s + sin^2 u ]
//             = 3 r^2
//
//  -- the cross terms -2cos s - 2cos s cos u + 2cos u vanish, which is
//  exactly the relation above rearranged.  EVERY ruling has length
//  r*sqrt(3).  Not approximately: identically, for all of them.  The
//  file measures the longest and the shortest and echoes the spread.
//
//  ---- a parameter that is symmetric in the two circles ------------
//
//  Set m = 1 and let k = a.  Then p = a - 1 and
//
//      cos s = (a-1)/a,   sin s = sqrt(2a-1)/a
//      cos u =  a-1,      sin u = sqrt(a(2-a))
//
//  Both square roots stay real only for a in [1/2, 2], and over that
//  interval cos s climbs from -1 to 1/2 while cos u climbs from -1/2
//  to 1.  So the rulings land on a 240-degree arc of each circle and
//  the remaining 120 degrees of each lies strictly inside the hull --
//  the point A(0) = (r,0,0), for instance, is the centre of disc B.
//
//  The construction is symmetric under swapping the two circles: the
//  rotation M(x,y,z) = (r-x, z, y), a half turn about the line
//  x = r/2, y = z, carries A onto B and B onto A.  In the parameter it
//  is a -> 1/a.  Sampling uniformly in log a therefore samples the two
//  circles the same way, and the mesh inherits the symmetry.  The file
//  uses  a = 2^sin(90*t),  t in [-1,1], which is uniform in log a up
//  to a sine easing.  The easing is not cosmetic.  Near a = 2 the arc
//  of B behaves like u ~ sqrt(2 - a): without easing the B samples
//  would spread as a square root and the end quads would shear.  The
//  sine makes 2 - a quadratic in the step, so u comes out linear and
//  the quads stay well shaped at both ends.  M also fixes the
//  centroid, which forces it onto x = r/2, y = z; the separate
//  mirrors y -> -y and z -> -z force y = z = 0.  So the centroid is
//  (r/2, 0, 0) exactly, and the single translate below puts it on the
//  Z axis with no numerical centring needed.
//
//  ---- how the closed surface is assembled -------------------------
//
//  For each a there are two choices of sign for sin s and two for
//  sin u, so four ruled patches.  Each spans 120 degrees of each
//  circle, and they glue along:
//
//      arc A+ (s: 180 -> 60)    shared by patches (+,+) and (+,-)
//      arc A- (s: 180 -> 300)   shared by patches (-,+) and (-,-)
//      arc B+ (u: 120 -> 0)     shared by patches (+,+) and (-,+)
//      arc B- (u: -120 -> 0)    shared by patches (+,-) and (-,-)
//      ruling at a = 1/2, from A(180) to B(+-120)
//      ruling at a = 2,   from A(+-60) to B(0)
//
//  Six vertices, eight edges, four faces: V - E + F = 2.  The surface
//  closes on itself with nothing left over -- no cap, no lid, no
//  subtraction.  Both of the a = 1/2 rulings start at A(180) and both
//  of the a = 2 rulings end at B(0), so those two points are single
//  shared entries in the vertex list; everything else is generated
//  once per patch pair and mirrored by negating one coordinate, which
//  keeps shared coordinates bit-identical.
//
//  One mesh detail worth stating.  Each quad is split by the diagonal
//  a1-b0, never a0-b1.  At the last quad of a patch b1 is the shared
//  vertex B(0) and a0 is shared with the neighbouring patch, so the
//  a0-b1 diagonal would be the SAME edge in two different patches and
//  four faces would meet along it.  The mesh would still close -- the
//  hole count stays zero -- but it would not be a manifold.  The other
//  diagonal can never do this, because a1 is shared only when i+1 = 0
//  and that never happens.
//
//  ---- the hook ----------------------------------------------------
//
//  Rolled on a table the oloid tumbles: the contact wanders along a
//  ruling, hands over to the next, and in the course of the motion
//  every point of the surface touches the table.  And the area of that
//  surface is 4*pi*r^2 -- the area of a sphere of radius r, exactly
//  (Dirnboeck and Stachel, 1997).  The file does not assert this.  It
//  sums the triangle areas of the mesh it exports, sums them again at
//  twice the resolution, and Richardson-extrapolates the O(1/N^2)
//  chord deficit away; the extrapolated figure agrees with 4*pi*r^2 to
//  about one part in 10^9.
//
//  Additive only.  No difference(), no intersection(), no hull().
// ===================================================================

R  = 21;          // circle radius -> bounding box 3R x 2R x 2R = 63 x 42 x 42
N  = 144;         // samples along each patch; 8*N triangles in the oloid

OX = -R/2;        // the centroid sits at (R/2, 0, 0); slide it to the Z axis
OZ =  R;          // lowest point of the solid is B(-90), at z = -R; lift it

C_OLOID = "#c08a3e";   // bronze
C_PLINTH = "#3b4046";  // slate
C_SLAB  = "#243b4a";   // deep blue
C_TEXT  = "#f2eee2";   // bone

// ---- the parameter ladder ----------------------------------------
// a runs 1/2 -> 2 and the two endpoints are pinned to exact binary
// values so that sqrt(2a-1) and sqrt(a(2-a)) vanish exactly there.

function hh(i, M) = (i == 0) ? -1 : ((i == M) ? 1 : sin(90 * (2*i/M - 1)));
function aa(i, M) = pow(2, hh(i, M));

function cos_s(a) = 1 - 1/a;            // cos of the angle on circle A
function sin_s(a) = sqrt(2*a - 1) / a;  // >= 0 branch
function cos_u(a) = a - 1;              // cos of the angle on circle B
function sin_u(a) = sqrt(a * (2 - a));  // >= 0 branch

// Points in the display frame (already translated onto the plinth).
function PA(i, sg) = let (a = aa(i, N))
    [OX + R*cos_s(a), sg * R*sin_s(a), OZ];
function PB(i, tu) = let (a = aa(i, N))
    [OX + R*(1 + cos_u(a)), 0, OZ + tu * R*sin_u(a)];

// ---- vertices ----------------------------------------------------
//   0                  A(180), where both a = 1/2 rulings begin
//   1    .. N          arc A+        (y > 0)
//   N+1  .. 2N         arc A-        (y < 0)
//   2N+1               B(0),  where both a = 2 rulings end
//   2N+2 .. 3N+1       arc B+        (z > OZ)
//   3N+2 .. 4N+1       arc B-        (z < OZ)

pts = concat(
    [ [OX - R, 0, OZ] ],
    [ for (i = [1 : N])     PA(i,  1) ],
    [ for (i = [1 : N])     PA(i, -1) ],
    [ [OX + 2*R, 0, OZ] ],
    [ for (i = [0 : N - 1]) PB(i,  1) ],
    [ for (i = [0 : N - 1]) PB(i, -1) ]
);

function iA(i, sg) = (i == 0)   ? 0        : ((sg > 0) ? i          : N + i);
function iB(i, tu) = (i == N)   ? 2*N + 1  : ((tu > 0) ? 2*N+2 + i  : 3*N+2 + i);

// ---- faces -------------------------------------------------------
// Wound so the right-hand normal points INTO the solid, which is the
// polyhedron() convention.  Mirroring in y or in z reverses handedness,
// so the two patches with sg*tu < 0 take the reversed triangle order.

faces = [ for (sg = [1, -1]) for (tu = [1, -1]) for (i = [0 : N - 1])
            let (a0 = iA(i, sg), a1 = iA(i+1, sg),
                 b0 = iB(i, tu), b1 = iB(i+1, tu))
            each (sg * tu > 0
                    ? [ [a0, a1, b0], [a1, b1, b0] ]
                    : [ [a0, b0, a1], [a1, b0, b1] ]) ];

module oloid() { polyhedron(points = pts, faces = faces); }

// ---- plinth and plaque -------------------------------------------

module plinth() { translate([0, 0, -6]) cylinder(h = 6, r = 34, $fn = 160); }

module tilted() { translate([0, -46, 0]) rotate([30, 0, 0]) children(); }

module slab() { tilted() translate([-39, 0, 0]) cube([78, 46, 3]); }

module plaque_text() {
    tilted() translate([0, 0, 3]) linear_extrude(height = 0.9) {
        translate([-33, 34.5]) text("THE OLOID", size = 6);
        translate([-33, 25.5]) text("Paul Schatz, 1929", size = 3.2);
        translate([-33, 18.0]) text("hull: x^2+y^2=r^2,z=0 & (x-r)^2+z^2=r^2,y=0", size = 3.2);
        translate([-33, 10.5]) text("rolls on all of itself; area = 4 pi r^2", size = 3.2);
    }
}

$fn = 8;
color(C_OLOID)  oloid();
color(C_PLINTH) plinth();
color(C_SLAB)   slab();
color(C_TEXT)   plaque_text();

// ===================================================================
//  measurement
// ===================================================================

// Binary sum: depth log2(n), so a few thousand terms cost ~12 frames.
function psum(v, lo, hi) =
      (hi - lo <= 0) ? 0
    : (hi - lo == 1) ? v[lo]
    : psum(v, lo, floor((lo+hi)/2)) + psum(v, floor((lo+hi)/2), hi);

// -- bounding box of the object as placed --
XS = [for (p = pts) p[0]];  YS = [for (p = pts) p[1]];  ZS = [for (p = pts) p[2]];
BB = [max(XS) - min(XS), max(YS) - min(YS), max(ZS) - min(ZS)];

// -- ruling lengths, measured on the exported mesh --
RL   = [ for (i = [0 : N]) norm(PB(i, 1) - PA(i, 1)) ];
RLo  = R * sqrt(3);

// -- surface area.  One patch, times four: the other three are exact
//    mirror images, and reflection preserves area.
function qA(i, M) = let (a = aa(i, M)) [R*cos_s(a), R*sin_s(a), 0];
function qB(i, M) = let (a = aa(i, M)) [R*(1 + cos_u(a)), 0, R*sin_u(a)];
function quadA(i, M) =
    let (a0 = qA(i,M), a1 = qA(i+1,M), b0 = qB(i,M), b1 = qB(i+1,M))
        0.5*norm(cross(a1 - a0, b0 - a0)) + 0.5*norm(cross(b1 - a1, b0 - a1));
function areaOf(M) = 4 * psum([for (i = [0 : M-1]) quadA(i, M)], 0, M);

AREA_N   = areaOf(N);        // area of the mesh actually exported
AREA_2N  = areaOf(2*N);      // same surface, twice the samples
AREA_RICH = (4*AREA_2N - AREA_N) / 3;   // chord deficit is O(1/M^2)
AREA_EXACT = 4 * PI * R * R;

// -- volume, by the divergence theorem over the same faces --
function tdet(f) = let (A = pts[f[0]], B = pts[f[1]], C = pts[f[2]])
    (A[0]*(B[1]*C[2] - B[2]*C[1])
   + A[1]*(B[2]*C[0] - B[0]*C[2])
   + A[2]*(B[0]*C[1] - B[1]*C[0])) / 6;
VOL = -psum([for (f = faces) tdet(f)], 0, len(faces));  // sign: inward winding

echo(str("oloid  r = ", R, " mm"));
echo(str("bounding box  ", BB[0], " x ", BB[1], " x ", BB[2],
         " mm   (3r x 2r x 2r);  rests at z = ", min(ZS),
         ", centroid on the Z axis"));
echo(str("triangles  oloid = ", len(faces), "  (4 patches x ", N, " quads x 2)",
         "   vertices = ", len(pts)));
echo(str("ruling length  min = ", min(RL), "  max = ", max(RL),
         "   r*sqrt(3) = ", RLo, "   spread = ", max(RL) - min(RL)));
echo(str("surface area   mesh(N=", N, ")  = ", AREA_N,
         "   diff from 4*pi*r^2 = ", AREA_N - AREA_EXACT));
echo(str("surface area   mesh(N=", 2*N, ") = ", AREA_2N,
         "   diff from 4*pi*r^2 = ", AREA_2N - AREA_EXACT));
echo(str("surface area   Richardson   = ", AREA_RICH,
         "   diff from 4*pi*r^2 = ", AREA_RICH - AREA_EXACT));
echo(str("surface area   4*pi*r^2     = ", AREA_EXACT, " mm^2"));
echo(str("volume  = ", VOL, " mm^3   = ", VOL/(R*R*R), " r^3",
         "   (literature 3.05242 r^3)"));
