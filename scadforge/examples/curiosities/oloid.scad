// ===================================================================
//  THE OLOID  --  Paul Schatz, 1929
//
//  Take two circles of the same radius r.  Put them in perpendicular
//  planes and slide them until each one passes through the centre of
//  the other.  Their centres are then exactly r apart, because the
//  centre of the second circle is a point of the first.  The convex
//  hull of the pair is the oloid.
//
//  Put the first circle in Z = 0 about the origin and the second in
//  Y = 0 about (r, 0, 0):
//
//      A(s) = ( r cos s,  r sin s,  0 )          X^2 + Y^2 = r^2, Z = 0
//      B(u) = ( r + r cos u,  0,  r sin u )   (X-r)^2 + Z^2 = r^2, Y = 0
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
//  rotation M(X,Y,Z) = (r-X, Z, Y), a half turn about the line
//  X = r/2, Y = Z, carries A onto B and B onto A.  In the parameter it
//  is a -> 1/a.  Sampling uniformly in log a therefore samples the two
//  circles the same way, and the mesh inherits the symmetry.  The file
//  uses  a = 2^sin(90*t),  t in [-1,1], which is uniform in log a up
//  to a sine easing.  The easing is not cosmetic.  Near a = 2 the arc
//  of B behaves like u ~ sqrt(2 - a): without easing the B samples
//  would spread as a square root and the end quads would shear.  The
//  sine makes 2 - a quadratic in the step, so u comes out linear and
//  the quads stay well shaped at both ends.  M also fixes the
//  centroid, which forces it onto X = r/2, Y = Z; the separate mirrors
//  Y -> -Y and Z -> -Z force Y = Z = 0.  So the centroid is exactly
//  (r/2, 0, 0), and the placement below is a rigid motion with no
//  numerical centring anywhere.
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
//  once per patch and the mirrored copies are made by negating one
//  coordinate, which keeps shared coordinates bit-identical.
//
//  One mesh detail worth stating.  Each quad is split by the diagonal
//  a1-b0, never a0-b1.  At the last quad of a patch b1 is the shared
//  vertex B(0) and a0 is shared with the neighbouring patch, so the
//  a0-b1 diagonal would be the SAME edge in two different patches and
//  four faces would meet along it.  The mesh would still close -- the
//  hole count stays zero -- but it would not be a manifold.  The other
//  diagonal can never do this, because a1 is shared only when i+1 = 0
//  and that never happens.  This cost one wrong draft to find.
//
//  ---- the pose ----------------------------------------------------
//
//  The exhibit is not shown axis-aligned.  An oloid put down on a
//  table rests on a ruling, and the ruling it rests on is a supporting
//  line of the whole solid, so the contact is a straight segment --
//  length r*sqrt(3), like every other ruling.  The middle ruling,
//  a = 1, runs from A(270) = (0,-r,0) to B(270) = (r,0,-r), and its
//  supporting plane has normal -(0,1,1)/sqrt(2).  One rotation of 45
//  degrees about X lays that plane flat, and with K = sqrt(2)/2 the
//  placement is
//
//      x = X - r/2,   y = K(Y - Z),   z = K(Y + Z + r) + AIR
//
//  which sends both ends of that ruling to z = AIR exactly --
//  K(-r+0+r) and K(0-r+r) are both K*0 -- and leaves every other point
//  of the solid above, since Y + Z >= -r is precisely the supporting
//  plane.  The centroid lands at (0, 0, K*r + AIR), on the Z axis.  So
//  the object stands on a line, not a point; the line is r*sqrt(3)
//  long like every other ruling, and its length is measured and
//  echoed.
//
//  AIR is 0.15 mm, and it is there for the exporter, not for the
//  geometry.  A solid that touches the plinth exactly along a line is
//  one connected component to the export-time union, which then runs
//  a boolean on a tangency: on the first try of this that union came
//  back with 16723 triangles in place of 1788 and 21 holes in a mesh
//  that had none.  The hyperboloid, the Enneper surface and the
//  Schwarz lantern in this folder all park their object on the same
//  0.15 mm of air for the same reason, and all float the letters off
//  the slab by it too, so no merge is attempted anywhere and the
//  export is a concatenation of shells that were each already closed.
//  The convention asks for z >= 0 and gets it; the letters still
//  stand proud of the slab, 0.9 mm of them.
//
//  a = 1 is not an arbitrary pick of ruling, either.  The supporting
//  plane of ruling a is n.X = r*a with n = (a-1, sqrt(2a-1),
//  sqrt(a(2-a))) and |n|^2 = 2a, and the centroid C = (r/2, 0, 0) has
//  n.C = r(a-1)/2, so C stands
//
//      H(a) = ( r a - r(a-1)/2 ) / sqrt(2a) = r (a+1) / (2 sqrt(2a))
//
//  above it.  H is least at a = 1, where it is r/sqrt(2) = 0.7071 r,
//  and largest at the two ends of the family, 0.75 r.  So a real
//  oloid put down on a table settles onto the a = 1 ruling, and as it
//  rolls and the contact ruling sweeps the family its centroid rides
//  up and down between those two heights -- 0.0429 r, six per cent of
//  the resting height.  The file evaluates H over the whole ladder
//  and echoes both extremes.  Nothing about the height of the roll
//  goes on the plaque; what does go there is the developed surface,
//  which is Dirnboeck and Stachel's, and its area, which this file
//  recomputes for itself.
//
//  ---- one departure from the gallery convention -------------------
//
//  The convention asks for a plaque slab 78 x 46 x 3 with its near
//  edge at y = -46, tilted back 30 degrees.  Taken from the
//  HORIZONTAL, the back face of that slab runs
//
//      y = -46 + z cot 30 = -46 + 1.7321 z,
//
//  which crosses the axis at z = 26.6 and puts the far arris at
//  y = -6.16, z = 23.0 -- six millimetres off the axis, twenty-three
//  up, over the plinth.  Such a lectern demands an exhibit that has
//  tapered to nothing by 26.6 mm, and the 55-70 mm rule forbids one.
//  For this oloid the thinnest direction is already r*sqrt(2) = 29.7
//  mm, and over the whole family of poses that stand it on the plinth
//  the best case still needs r <= 17.1 -- a 51 mm oloid -- to keep the
//  solid behind that slab's back face.  An earlier draft of this file
//  tilted from the horizontal and paid for it: the solid went 10.1 mm
//  through the back face and burst 7.1 mm clean out of the lettered
//  one, taking a lens 41 mm wide and 10 mm deep out of the top of the
//  plaque, which cleared the top line of text by 0.9 mm.  The
//  hyperboloid, the Enneper surface and the Schwarz lantern in this
//  same folder all hit this and all resolved it the same way, so this
//  file follows them: the 30 degrees is taken from
//  the VERTICAL, the slab leaning 30 degrees off upright.  All three
//  published numbers survive -- 78 x 46 x 3, near edge at y = -46,
//  30 degrees -- and the back face retreats to y = -46 + z cot 60,
//  nearest the axis at its far arris, y = -23.
//
//  The clearance is computed, not eyeballed.  The slab's own depth
//  coordinate is an affine function of a point of the scene, and the
//  oloid is a convex hull of two circles, so the largest value that
//  coordinate takes on the solid is the larger of its two maxima over
//  those circles, each in closed form.  The file evaluates both,
//  echoes the gap, cross-checks it against every vertex of the
//  exported mesh, and asserts it positive.  With that, nothing in the
//  scene touches anything else at all: 119 closed shells, written out
//  one after another, and no boolean anywhere in the file.
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

R   = 21;     // circle radius.  Long axis of the solid is 3R = 63 mm.
N   = 144;    // samples per patch edge;  8*N triangles in the oloid.
K   = sqrt(2)/2;
AIR = 0.15;   // air under the solid.  See the note on contact above.

C_OLOID  = "#c08a3e";   // bronze
C_PLINTH = "#3b4046";   // slate
C_SLAB   = "#243b4a";   // deep blue
C_TEXT   = "#f2eee2";   // bone

// ---- the parameter ladder ----------------------------------------
// a runs 1/2 -> 2.  Both endpoints and the midpoint are pinned to
// exact binary values, so sqrt(2a-1), sqrt(a(2-a)) and the contact
// ruling at a = 1 come out exact rather than nearly exact.

function hh(i, M) = (i == 0) ? -1 : ((i == M) ? 1 : sin(90 * (2*i/M - 1)));
function aa(i, M) = pow(2, hh(i, M));

function cos_s(a) = 1 - 1/a;            // angle on circle A
function sin_s(a) = sqrt(2*a - 1) / a;  // non-negative branch
function cos_u(a) = a - 1;              // angle on circle B
function sin_u(a) = sqrt(a * (2 - a));  // non-negative branch

// Points, already carried into the display pose by the rigid motion
//     x = X - R/2,  y = K(Y - Z),  z = K(Y + Z + R) + AIR.
function PA(i, sg) = let (a = aa(i, N), ss = sg * sin_s(a))
    [ R*cos_s(a) - R/2,  K*R*ss,  K*R*(ss + 1) + AIR ];
function PB(i, tu) = let (a = aa(i, N), su = tu * sin_u(a))
    [ R*(1 + cos_u(a)) - R/2,  -K*R*su,  K*R*(su + 1) + AIR ];

// ---- vertices ----------------------------------------------------
//   0                  A(180), where both a = 1/2 rulings begin
//   1    .. N          arc A+        (sin s > 0)
//   N+1  .. 2N         arc A-        (sin s < 0)
//   2N+1               B(0),  where both a = 2 rulings end
//   2N+2 .. 3N+1       arc B+        (sin u > 0)
//   3N+2 .. 4N+1       arc B-        (sin u < 0)

pts = concat(
    [ [-1.5*R, 0, K*R + AIR] ],
    [ for (i = [1 : N])     PA(i,  1) ],
    [ for (i = [1 : N])     PA(i, -1) ],
    [ [ 1.5*R, 0, K*R + AIR] ],
    [ for (i = [0 : N - 1]) PB(i,  1) ],
    [ for (i = [0 : N - 1]) PB(i, -1) ]
);

function iA(i, sg) = (i == 0) ? 0       : ((sg > 0) ? i         : N + i);
function iB(i, tu) = (i == N) ? 2*N + 1 : ((tu > 0) ? 2*N+2 + i : 3*N+2 + i);

// ---- faces -------------------------------------------------------
// Wound so the right-hand normal points INTO the solid, which is the
// polyhedron() convention.  Mirroring in Y or in Z reverses handedness,
// so the two patches with sg*tu < 0 take the reversed triangle order.
// The 45-degree placement is a rotation, so it changes nothing here.

faces = [ for (sg = [1, -1]) for (tu = [1, -1]) for (i = [0 : N - 1])
            let (a0 = iA(i, sg), a1 = iA(i+1, sg),
                 b0 = iB(i, tu), b1 = iB(i+1, tu))
            each (sg * tu > 0
                    ? [ [a0, a1, b0], [a1, b1, b0] ]
                    : [ [a0, b0, a1], [a1, b0, b1] ]) ];

module oloid() { polyhedron(points = pts, faces = faces); }

// ---- plinth and plaque -------------------------------------------

PL_R = 34;  PL_H = 6;                   // plinth: top face at z = 0
PQ_W = 78;  PQ_D = 46;  PQ_T = 3;       // plaque slab
PQ_Y = -46;                             // its near edge
PQ_TILT = 60;                           // 60 from horizontal = 30 off upright

module plinth() { translate([0, 0, -PL_H]) cylinder(h = PL_H, r = PL_R, $fn = 160); }

module tilted() { translate([0, PQ_Y, 0]) rotate([PQ_TILT, 0, 0]) children(); }

module slab() { tilted() translate([-PQ_W/2, 0, 0]) cube([PQ_W, PQ_D, PQ_T]); }

module plaque_text() {
    tilted() translate([0, 0, PQ_T + AIR]) linear_extrude(height = 0.9) {
        translate([-33, 32.5]) text("THE OLOID", size = 6);
        translate([-33, 24.0]) text("Paul Schatz, 1929", size = 3.2);
        translate([-33, 16.5]) text("hull: x^2+y^2=r^2,z=0 & (x-r)^2+z^2=r^2,y=0", size = 3.2);
        translate([-33,  9.0]) text("rolls on all of itself; area = 4 pi r^2", size = 3.2);
    }
}

$fn = 8;
color(C_OLOID)  oloid();
color(C_PLINTH) plinth();
color(C_SLAB)   slab();
color(C_TEXT)   plaque_text();

// ===================================================================
//  measurement -- every number the plaque claims, recomputed here
// ===================================================================

// Binary sum: depth log2(n), so a few thousand terms cost ~12 frames.
function psum(v, lo, hi) =
      (hi - lo <= 0) ? 0
    : (hi - lo == 1) ? v[lo]
    : psum(v, lo, floor((lo+hi)/2)) + psum(v, floor((lo+hi)/2), hi);

XS = [for (p = pts) p[0]];  YS = [for (p = pts) p[1]];  ZS = [for (p = pts) p[2]];
BB = [max(XS) - min(XS), max(YS) - min(YS), max(ZS) - min(ZS)];
FOOT = max([for (p = pts) sqrt(p[0]*p[0] + p[1]*p[1])]);   // vs plinth r = 34

// -- ruling lengths, measured on the exported mesh --
RL   = [ for (i = [0 : N]) norm(PB(i, 1) - PA(i, 1)) ];
RLo  = R * sqrt(3);
// -- the ruling the solid stands on: a = 1, both signs negative --
CONTACT = norm(PB(N/2, -1) - PA(N/2, -1));

// -- surface area.  One patch, times four: the other three are exact
//    mirror images, and reflection preserves area.
function qA(i, M) = let (a = aa(i, M)) [R*cos_s(a), R*sin_s(a), 0];
function qB(i, M) = let (a = aa(i, M)) [R*(1 + cos_u(a)), 0, R*sin_u(a)];
function quadA(i, M) =
    let (a0 = qA(i,M), a1 = qA(i+1,M), b0 = qB(i,M), b1 = qB(i+1,M))
        0.5*norm(cross(a1 - a0, b0 - a0)) + 0.5*norm(cross(b1 - a1, b0 - a1));
function areaOf(M) = 4 * psum([for (i = [0 : M-1]) quadA(i, M)], 0, M);

AREA_N     = areaOf(N);                  // the mesh actually exported
AREA_2N    = areaOf(2*N);                // same surface, twice the samples
AREA_RICH  = (4*AREA_2N - AREA_N) / 3;   // chord deficit is O(1/M^2)
AREA_EXACT = 4 * PI * R * R;

// -- volume, by the divergence theorem over the same faces --
function tdet(f) = let (A = pts[f[0]], B = pts[f[1]], C = pts[f[2]])
    (A[0]*(B[1]*C[2] - B[2]*C[1])
   + A[1]*(B[2]*C[0] - B[0]*C[2])
   + A[2]*(B[0]*C[1] - B[1]*C[0])) / 6;
VOL = -psum([for (f = faces) tdet(f)], 0, len(faces));  // sign: inward winding

// -- where the solid comes to rest.  H(a) = r(a+1)/(2 sqrt(2a)) is the
//    height of the centroid over the supporting plane of ruling a.
function Hc(a) = R*(a + 1) / (2*sqrt(2*a));
HLAD = [ for (i = [0 : N]) Hc(aa(i, N)) ];

// -- clearance to the plaque slab -----------------------------------
// lz(P) = (P - (0, PQ_Y, 0)) . (0, -sin t, cos t) is the slab's own
// depth coordinate: the slab occupies 0 <= lz <= PQ_T, so lz <= 0 over
// the whole solid means the solid is behind its back face, and -max lz
// is the perpendicular gap -- a lower bound on the distance to the
// slab itself, which is finite.  lz is affine and the oloid is the
// convex hull of two circles, so max lz is the larger of the two
// closed-form maxima below; the mesh is polled as a cross-check.
TS = sin(PQ_TILT);  TC = cos(PQ_TILT);
function lzOf(p) = -(p[1] - PQ_Y)*TS + p[2]*TC;
LZ_A  = K*R*abs(TC - TS) + (K*R + AIR)*TC + PQ_Y*TS;   // over circle A
LZ_B  = K*R*(TS + TC)    + (K*R + AIR)*TC + PQ_Y*TS;   // over circle B
GAP   = -max(LZ_A, LZ_B);
GAP_M = -max([ for (p = pts) lzOf(p) ]);       // same thing, off the mesh
assert(GAP > 0, "the oloid runs into the plaque slab");

echo(str("OLOID   r = ", R, " mm,  resting on the a = 1 ruling"));
echo(str("bounding box   ", BB[0], " x ", BB[1], " x ", BB[2], " mm",
         "   (3r x r*sqrt2 x r*sqrt2);  lowest point z = ", min(ZS),
         ";  footprint radius ", FOOT, " <= plinth 34"));
echo(str("triangles  oloid = ", len(faces), "  (4 patches x ", N,
         " quads x 2),  vertices = ", len(pts)));
echo(str("ruling length  min = ", min(RL), "  max = ", max(RL),
         "   r*sqrt(3) = ", RLo, "   spread = ", max(RL) - min(RL)));
echo(str("contact ruling, ", AIR, " mm over the plinth = ", CONTACT,
         "  vs r*sqrt(3) = ", RLo, "   (it stands on a line, not a point)"));
echo(str("surface area   mesh(N=", N, ")  = ", AREA_N,
         "   minus 4*pi*r^2 = ", AREA_N - AREA_EXACT));
echo(str("surface area   mesh(N=", 2*N, ") = ", AREA_2N,
         "   minus 4*pi*r^2 = ", AREA_2N - AREA_EXACT));
echo(str("surface area   Richardson   = ", AREA_RICH,
         "   minus 4*pi*r^2 = ", AREA_RICH - AREA_EXACT));
echo(str("surface area   4*pi*r^2     = ", AREA_EXACT, " mm^2  <- the claim"));
echo(str("volume = ", VOL, " mm^3 = ", VOL/(R*R*R),
         " r^3   (Dirnboeck-Stachel 3.05242 r^3)"));
echo(str("resting ruling  H(a) = r(a+1)/(2 sqrt(2a)):  min = ", min(HLAD),
         " = r/sqrt(2) = ", R/sqrt(2), " at a = 1;   max = ", max(HLAD),
         " = 0.75 r = ", 0.75*R, " at a = 1/2 and 2"));
echo(str("plaque: slab ", PQ_W, " x ", PQ_D, " x ", PQ_T, ", near edge y = ",
         PQ_Y, ", tilted ", PQ_TILT, " from horizontal = ", 90 - PQ_TILT,
         " off upright;  back face nearest the axis at y = ",
         PQ_Y + PQ_D*cos(PQ_TILT)));
echo(str("clearance, solid to the slab's back plane = ", GAP,
         " mm   (closed form over the two circles; off the mesh ", GAP_M, ")"));
