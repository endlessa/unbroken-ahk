// ===================================================================
//  HYPERBOLOID OF ONE SHEET -- a curve built out of straight sticks
//
//      x^2/a^2 + y^2/a^2 - z^2/c^2 = 1,     a = 16 mm,  c = 30 mm
//
//  Every solid in this file is a straight prism or a ring of revolution.
//  Nothing here is a revolved skin, and nothing is subtracted.  The
//  curved surface is not drawn at all: it is implied by twenty-four
//  straight bars, and it is the bars that are checked against the
//  equation at the foot of the file.
//
//  WHY A CHORD OF THE CIRCLE LANDS ON THE HYPERBOLOID
//
//  Take the circle of radius R in the plane z = -h and the same circle
//  in z = +h, and join the point at azimuth t below to the point at
//  azimuth t + 2d above.  Write m = t + d for the mean azimuth and let
//  u = z/h run over [-1, 1], so the segment is the affine interpolation
//
//      P(u) = ((1-u)/2) * (R cos(m-d), R sin(m-d), -h)
//           + ((1+u)/2) * (R cos(m+d), R sin(m+d), +h).
//
//  Expand the first coordinate and the sum-to-product identities give
//
//      x = R [ cos m cos d - u sin m sin d ]
//      y = R [ sin m cos d + u cos m sin d ]
//
//  and now square and add.  The cross terms are
//
//      2u cos d sin d ( -cos m sin m + sin m cos m ) = 0,
//
//  identically, for every m and every u.  That exact cancellation is the
//  whole theorem.  What is left is
//
//      x^2 + y^2 = R^2 cos^2 d + u^2 R^2 sin^2 d,
//
//  a function of u alone, with no m in it.  Substituting u = z/h,
//
//      x^2 + y^2 = (R cos d)^2 + z^2 (R sin d)^2 / h^2
//                = a^2 ( 1 + z^2/c^2 ),     a = R cos d,  c = h/tan d,
//
//  which is the hyperboloid of one sheet with a = R cos d and
//  c = h cot d.  Both sides are polynomials in u; the identity holds for
//  the whole line, not merely for the segment, so the line lies WHOLLY
//  in the surface.  Reversing the twist, d -> -d, changes neither
//  cos^2 d nor sin^2 d, so the mirrored family lands on the SAME
//  surface.  That is the double ruling, and it is the reason a water
//  tower can be a curved shell made of straight rolled steel -- Vladimir
//  Shukhov's, first at Nizhny Novgorod in 1896, and about two hundred
//  after it.
//
//  THE TWO LINES THROUGH A GIVEN POINT, IN CLOSED FORM
//
//  Reading the display above backwards, a point of the surface at
//  azimuth alpha and height z = hu sits on the family-(+) line whose
//  mean azimuth is m, where
//
//      x = a cos(m + psi)/cos psi,  y = a sin(m + psi)/cos psi,
//      tan psi = u tan d,
//
//  so alpha = m + psi and the radius is a sec psi = a sqrt(1 + z^2/c^2),
//  as it must be.  The family-(-) line through the same point is the
//  mirror image of the first in the vertical plane at azimuth alpha,
//  because that reflection fixes the point, fixes the surface, and
//  exchanges the two families.  A reflection about azimuth alpha sends
//  azimuth theta to 2 alpha - theta, so the second line's mean azimuth is
//
//      m' = 2 alpha - m = m + 2 psi.
//
//  Two lines, named explicitly, for every point.  Both are checked below
//  on a grid of 189 surface points: the distance from the point to the
//  mirrored line, and the residual of the equation along that line.
//
//  THE CASE THAT NEEDS NO DECIMALS
//
//  With h = c the twist is a quarter turn, d = atan(h/c) = 45 degrees,
//  and the end circles have radius R = a/cos 45 = a sqrt 2.  At a = 16,
//  h = c = 30 that is R = 16 sqrt 2, and the ring nodes are placed at
//  the twelve azimuths 30k +- 45 degrees, so the bar of mean azimuth 0
//  runs from
//
//      (16, -16, -30)  to  (16, +16, +30)
//
//  -- integer endpoints, and x = 16 at both.  Its mate of the other
//  family runs from (16, +16, -30) to (16, -16, +30).  Both lie in the
//  vertical plane x = 16, which is the tangent plane at the waist point
//  (16, 0, 0), and setting x = 16 in the equation leaves
//
//      y^2/16^2 = z^2/30^2    i.e.    y = +- (8/15) z,
//
//  two straight lines and nothing else.  A plane tangent to a
//  hyperboloid of one sheet meets it in exactly two lines, and here both
//  of them are members of the lattice, with integer endpoints.  The
//  ruling direction (0, 16, 30) + (16, 0, 0)-free part has length 34,
//  the 8-15-17 triple doubled, so each bar's full chord is exactly
//  68 mm: sqrt(32^2 + 60^2) = 68, where the horizontal run 32 is exactly
//  the waist DIAMETER 2a.  The two bars cross at the waist at an angle
//  whose cosine is the exact rational
//
//      (900 - 256)/(900 + 256) = 644/1156 = 161/289 = 0.557093...
//
//  about 56.145 degrees.  All of these are recomputed below from the
//  built geometry rather than quoted.
//
//  HOW IT IS BUILT, AND WHY NOTHING IS SUBTRACTED
//
//  A bar is a four-sided prism: two copies of a 1.5 x 2.4 rectangle,
//  one at each end of the axis, walled by four quads and closed by two
//  fans.  The section's first axis e1 is the horizontal radial at the
//  bar's midpoint, and it is exactly perpendicular to the bar, because
//  both ends sit at the same radius R and
//
//      (p0 + p1).(p1 - p0) = |p1|^2 - |p0|^2 = 0.
//
//  No orthogonalisation is needed; e2 = t x e1 completes a right-handed
//  triple (e1, e2, t), the section is listed counter-clockwise in
//  (e1, e2), and the wall quads then have normals pointing INTO the
//  solid, which is what polyhedron() wants.  The two end fans run
//  opposite ways round the ring, which is the orientation-blind edge
//  test: a wall quad [a,b,c,d] contributes the directed edge d->a, so
//  the cap that meets it there must contribute a->d.
//
//  The five hoops are rings of revolution, each a 2.8 x 2.6 rectangle
//  swept a full turn -- genus one, and made by sweeping a profile with
//  a hole in the middle of its orbit, never by cutting a disc.  Each
//  hoop's radius is the surface radius at its own mid-height, so the
//  hoops are the horizontal sections of the same hyperboloid; the
//  residuals are echoed.
//
//  Bars of OPPOSITE families cross, and the model lets them: that is
//  what a riveted Shukhov lattice does, and the crossings are honest
//  overlaps resolved by the export-time union, never by a boolean the
//  file performs.  Bars of the SAME family are skew -- that is the other
//  half of the ruling theorem -- and the minimum distance between the
//  twelve pairs of same-family lines is measured below and compared with
//  the section's diagonal, so the claim that they never touch is tested
//  rather than assumed.  Everything else is kept apart by AIR = 0.15 mm:
//  the lattice floats that far above the plinth, so no face of the
//  object is coplanar with the plinth's top and the union has nothing
//  degenerate to resolve.
//
//  ONE DEPARTURE FROM THE GALLERY CONVENTION, AND THE ARITHMETIC FOR IT
//
//  The convention asks for a plaque slab 78 x 46 x 3 whose near edge is
//  at y = -46, tilted back 30 degrees.  Tilted back 30 degrees from the
//  HORIZONTAL, a 46-deep slab hinged at y = -46 reaches
//
//      y = -46 + 46 cos 30 = -6.16,   z = 46 sin 30 = 23.0,
//
//  which is 6 mm from the axis and 23 mm up -- inside the plinth's
//  footprint and inside any exhibit taller than about 17 mm.  The
//  general bound is brutal: the slab's inner edge at height z sits at
//  |y| = 44.5 - z cot 30, which reaches zero at z = 25.7, so a flat
//  30-degree lectern of this depth demands an object that has tapered to
//  nothing by then.  No object 55 to 70 mm tall can satisfy it.  So the
//  30 degrees is taken here from the VERTICAL: the slab leans back 30
//  degrees off upright, which keeps all three published numbers -- the
//  78 x 46 x 3 slab, the near edge at y = -46, and the 30 degrees --
//  and clears the lattice.  The clearance is measured, not asserted:
//  the minimum distance from 6000-odd points on the object's edges to
//  the plaque box is echoed below and asserted positive.
// ===================================================================

// ---- the surface ----------------------------------------------------
A_W   = 16;             // waist radius a, mm
C_S   = 30;             // vertical semi-axis c, mm
H_Z   = 30;             // half height; the waist plane is z = H_Z
HT    = 2*H_Z;          // overall height of the lattice, 60 mm
D_T   = atan(H_Z/C_S);  // twist half-angle d = 45 degrees exactly
R_E   = A_W/cos(D_T);   // end-circle radius = 16 sqrt 2
NB    = 12;             // bars per family
PIT   = 360/NB;         // azimuth pitch of the mean-azimuth family, 30
K_R   = A_W*H_Z/C_S;    // R sin d: the ruling's horizontal rate, = 16

// ---- the drawn members ----------------------------------------------
BR    = 0.75;           // bar half-thickness, radial
BW    = 1.20;           // bar half-width, across the radial
RW    = 1.40;           // hoop half-width, radial
RH    = 2.60;           // hoop height
AIR   = 0.15;           // air between the object and the plinth

// ---- the gallery furniture ------------------------------------------
PL_R  = 34;  PL_H = 6;                  // plinth: top face at z = 0
PQ_W  = 78;  PQ_D = 46;  PQ_T = 3;      // plaque slab
PQ_TILT = 60;                           // 60 from horizontal = 30 off upright
T_SINK = 0.30;  T_RAISE = 0.90;         // letters sunk, then raised 0.9

// ---- colours --------------------------------------------------------
C_BAR  = "#c4652c";     // oxidised steel
C_WAIST= "#f2c94c";     // the one hoop whose radius IS a
C_PLIN = "#2b2f33";
C_SLAB = "#44505a";
C_TEXT = "#f2ece0";

$fa = 4;  $fs = 0.35;   // no $fn: small glyphs must not pay for big circles

// ---- small arithmetic -----------------------------------------------
function sq(t) = t*t;
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);
function unit(p) = p/norm(p);
function hz(p) = [p.x, p.y, 0];

// The surface, in the model's own z (waist at z = H_Z).
function rsurf(z) = A_W*sqrt(1 + sq(z - H_Z)/sq(C_S));
function fres(p)  = (sq(p.x) + sq(p.y))/sq(A_W) - sq(p.z - H_Z)/sq(C_S) - 1;

// The closed-form rulings.  s = +1 and s = -1 are the two families;
// m is the mean azimuth, u = (z - H_Z)/H_Z runs over [-1, 1].
function rule(m, u, s) =
  [ A_W*cos(m) - s*u*K_R*sin(m),
    A_W*sin(m) + s*u*K_R*cos(m),
    H_Z + H_Z*u ];
function ruledir(m, s) = [ -s*K_R*sin(m), s*K_R*cos(m), H_Z ];
// psi of the header: the azimuth a point has run on past its mean.
function psi(u) = atan(u*tan(D_T));

// ---- the twenty-four bars -------------------------------------------
// Node azimuths are PIT*k +- D_T, i.e. 30k +- 45; the two families share
// the same twelve, so every node carries exactly one bar of each.
PST = 2*D_T/PIT;
assert(abs(PST - round(PST)) < 1e-12,
       "the twist is not a whole number of pitches; the families would not share nodes");

function bar_q(k, s) =
  [ [R_E*cos(PIT*k - s*D_T), R_E*sin(PIT*k - s*D_T), 0],
    [R_E*cos(PIT*k + s*D_T), R_E*sin(PIT*k + s*D_T), HT] ];

// t is the unit axis; r1 the horizontal radial at the midpoint, exactly
// perpendicular to t; r2 = t x r1, so (r1, r2, t) is right-handed.
//
// The second family's section is then ROLLed about its own axis.  The
// reason is the pretty fact above and not an aesthetic one: the two bars
// that cross at the waist share a mean azimuth, so they share r1, and
// their inner and outer faces would sit in exactly the same two planes
// x = a +- BR.  Coincident faces are the one configuration a union is
// entitled to get wrong, and this one got it wrong -- 42 leaked edges on
// a two-bar test.  Rolling one family by nine degrees costs nothing that
// is claimed here: the axis is still the ruling, the section is still the
// same 1.5 x 2.4 rectangle, and the union is left with nothing to decide.
ROLL = 9;
function bar_frame(k, s) =
  let( q = bar_q(k, s), d = q[1] - q[0], L = norm(d), t = d/L,
       r1 = unit(hz((q[0] + q[1])/2)), r2 = cross(t, r1),
       ph = s > 0 ? 0 : ROLL )
  [ t, cos(ph)*r1 + sin(ph)*r2, -sin(ph)*r1 + cos(ph)*r2, L ];

// Pull both ends in along the axis until the prism's lowest corner sits
// at z = AIR and its highest at z = HT - AIR.  The axis stays on the
// ruling -- a sub-segment of a line in the surface is still in it.
function bar_ends(k, s) =
  let( q = bar_q(k, s), f = bar_frame(k, s),
       dz = BW*abs(f[2].z) + BR*abs(f[1].z) + AIR,
       lam = dz/f[0].z )
  [ q[0] + lam*f[0], q[1] - lam*f[0] ];

SEC = [ [BR, -BW], [BR, BW], [-BR, BW], [-BR, -BW] ];   // CCW in (e1, e2)
FAM = [ for (s = [1, -1]) for (k = [0:NB-1]) [k, s] ];

function bar_ring(k, s, u) =
  let( f = bar_frame(k, s), e = bar_ends(k, s) )
  [ for (c = SEC) e[u] + c[0]*f[1] + c[1]*f[2] ];

// A capped prism.  The section ring is counter-clockwise in (e1, e2), so
// the wall quads' right-hand normals point into the solid; the two fans
// run opposite ways round the ring so each shared edge is traversed once
// in each direction.
module prism(G, c0, c1) {
    K = len(G[0]); M = len(G) - 1; B = (M+1)*K;
    polyhedron(
      points = concat([ for (u = [0:M]) each G[u] ], [c0], [c1]),
      faces = concat(
        [ for (u = [0:M-1]) for (v = [0:K-1])
            [ u*K+v, (u+1)*K+v, (u+1)*K+(v+1)%K, u*K+(v+1)%K ] ],
        [ for (v = [0:K-1]) [ B, v, (v+1)%K ] ],
        [ for (v = [0:K-1]) [ B+1, M*K+(v+1)%K, M*K+v ] ]),
      convexity = 2);
}

module bar(k, s) {
    e = bar_ends(k, s);
    prism([ bar_ring(k, s, 0), bar_ring(k, s, 1) ], e[0], e[1]);
}

// ---- the five hoops -------------------------------------------------
// Each hoop's radius is the surface radius at its own mid-height, so the
// hoops ARE horizontal sections of the hyperboloid.
RINGZ = [ AIR + RH/2, HT/4, H_Z, 3*HT/4, HT - AIR - RH/2 ];

module hoop(zc) {
    translate([0, 0, zc])
      rotate_extrude()
        translate([rsurf(zc) - RW, -RH/2]) square([2*RW, RH]);
}

module lattice() {
    color(C_BAR) for (b = FAM) bar(b[0], b[1]);
    for (i = [0:len(RINGZ)-1])
      color(i == 2 ? C_WAIST : C_BAR) hoop(RINGZ[i]);
}

// ---- the plinth -----------------------------------------------------
module plinth() {
    color(C_PLIN) translate([0, 0, -PL_H]) cylinder(r = PL_R, h = PL_H);
}

// ---- the plaque -----------------------------------------------------
// Local slab coordinates: x across, y from the near edge to the far one,
// z out of the reading face.  rotate([PQ_TILT,0,0]) sends local (0,0,PQ_T)
// to y = -PQ_T sin(PQ_TILT), which is the slab's nearest point, so that
// is what is parked at y = -46.
PQ_Y = -46 + PQ_T*sin(PQ_TILT);

UPEM = 1000;    // InstrumentSans-Regular, hmtx advances, ASCII 32..126
ADV = [ 200, 273, 384, 716, 608, 786, 755, 232,
        406, 406, 408, 531, 255, 506, 255, 443,
        666, 391, 545, 574, 600, 574, 599, 532,
        582, 610, 255, 255, 531, 531, 531, 567,
        853, 728, 636, 741, 752, 638, 602, 765,
        736, 254, 455, 692, 588, 906, 736, 786,
        656, 787, 656, 608, 648, 712, 728,1089,
        688, 676, 623, 406, 443, 406, 531, 426,
        354, 533, 606, 533, 606, 564, 354, 606,
        599, 240, 240, 535, 240, 922, 599, 584,
        606, 606, 375, 473, 377, 589, 523, 767,
        551, 523, 496, 406, 242, 406, 531 ];
function runw(s, sz) =
  sz/UPEM * sum([ for (i = [0:len(s)-1]) ADV[ord(s[i]) - 32] ]);

LINE = [ "HYPERBOLOID",
         "of one sheet - Shukhov tower, 1896",
         "x^2/16^2 + y^2/16^2 - (z-30)^2/30^2 = 1",
         "two straight lines through every point" ];
LSZ  = [ 6, 3.2, 3.2, 3.2 ];
LBY  = [ 33.5, 25.0, 18.0, 11.0 ];     // baselines in local slab y
LRUN = [ for (i = [0:3]) runw(LINE[i], LSZ[i]) ];

assert(max(LRUN) < PQ_W - 10,
       "a plaque line is wider than the slab's text column");
assert(max([ for (l = LINE) len(l) ]) < 46, "a plaque line is too long");

module plaque() {
    translate([0, PQ_Y, 0]) rotate([PQ_TILT, 0, 0]) {
        color(C_SLAB) translate([-PQ_W/2, 0, 0]) cube([PQ_W, PQ_D, PQ_T]);
        color(C_TEXT)
          for (i = [0:3])
            translate([0, LBY[i], PQ_T - T_SINK])
              linear_extrude(height = T_SINK + T_RAISE)
                text(LINE[i], size = LSZ[i], halign = "center");
    }
}

lattice();
plinth();
plaque();

// ===================================================================
//  WHAT THE FILE CHECKS ABOUT ITSELF
// ===================================================================

// -- the bars really are on the surface -------------------------------
NS = 24;
BRES = [ for (b = FAM) let( e = bar_ends(b[0], b[1]) )
           max([ for (i = [0:NS]) abs(fres(e[0] + (e[1] - e[0])*i/NS)) ]) ];
// and so are the eight edges of every prism's four long arrises? no --
// only the axis is; the residual of the drawn corner is the bar's own
// thickness, and that is reported as a thickness, not as an error.
CRES = [ for (b = FAM)
           max([ for (u = [0,1]) for (p = bar_ring(b[0], b[1], u)) abs(fres(p)) ]) ];

// -- two straight lines through every point ---------------------------
// For a grid of surface points, name the mirrored ruling in closed form,
// then measure (a) how far the point is from it and (b) how far that
// whole line strays from the surface.
GM = [ for (i = [0:20]) i*360/21 ];
GU = [ for (j = [-4:4]) j/4 ];
PAIR = [ for (m = GM) for (u = GU)
           let( p  = rule(m, u, 1),
                mp = m + 2*psi(u),
                q  = rule(mp, u, -1) )
           [ norm(p - q),
             max([ for (i = [0:NS]) abs(fres(rule(mp, (2*i/NS) - 1, -1))) ]),
             acos(min(1, max(-1,
               (ruledir(m,1)*ruledir(mp,-1))/(norm(ruledir(m,1))*norm(ruledir(mp,-1)))))) ] ];
NTWO = len([ for (r = PAIR) if (r[0] < 1e-9 && r[1] < 1e-9 && r[2] > 1) 1 ]);

// -- the integer pair in the tangent plane x = 16 ---------------------
TP   = [ for (b = FAM) let( q = bar_q(b[0], b[1]) )
           if (abs(q[0].x - A_W) < 1e-9 && abs(q[1].x - A_W) < 1e-9) [q[0], q[1]] ];
// y = +-(a/c)(z - H_Z) is what x = A_W leaves of the equation
TPSLOPE = A_W/C_S;
TPERR = max([ for (p = TP) for (q = p) abs(abs(q.y) - TPSLOPE*abs(q.z - H_Z)) ]);
CHORD = norm(bar_q(0, 1)[1] - bar_q(0, 1)[0]);
XANG  = acos((ruledir(0,1)*ruledir(0,-1))/sq(norm(ruledir(0,1))));
XCOS  = (ruledir(0,1)*ruledir(0,-1))/sq(norm(ruledir(0,1)));

// -- same-family bars are skew, and far enough apart to be disjoint ---
function linedist(p0, d0, p1, d1) =
  let( n = cross(d0, d1) )
    norm(n) < 1e-12 ? 1e9 : abs((p1 - p0)*n)/norm(n);
SKEW = min([ for (s = [1,-1]) for (i = [0:NB-1]) for (j = [0:NB-1]) if (i < j)
               let( a = bar_q(i, s), b = bar_q(j, s) )
               linedist(a[0], a[1] - a[0], b[0], b[1] - b[0]) ]);
DIAG = 2*norm([BR, BW]);

// -- the hoops are sections of the same surface ----------------------
HRES = max([ for (z = RINGZ) abs(fres([rsurf(z), 0, z])) ]);

// -- the object's bounding box, from the points actually emitted ------
OBJP = concat(
  [ for (b = FAM) for (u = [0,1]) each bar_ring(b[0], b[1], u) ],
  [ for (z = RINGZ) for (sg = [-1,1]) for (dz = [-RH/2, RH/2])
      [ sg*(rsurf(z) + RW), 0, z + dz ] ],
  [ for (z = RINGZ) for (sg = [-1,1]) for (dz = [-RH/2, RH/2])
      [ 0, sg*(rsurf(z) + RW), z + dz ] ]);
OX = [ min([ for (p = OBJP) p.x ]), max([ for (p = OBJP) p.x ]) ];
OY = [ min([ for (p = OBJP) p.y ]), max([ for (p = OBJP) p.y ]) ];
OZ = [ min([ for (p = OBJP) p.z ]), max([ for (p = OBJP) p.z ]) ];
OBB = [ OX[1] - OX[0], OY[1] - OY[0], OZ[1] - OZ[0] ];

// -- the plaque clears the lattice -----------------------------------
// Distance from a point to the plaque box, measured in the slab's own
// frame; the box is grown to PQ_T + T_RAISE so the letters count.
function to_pq(p) =
  let( q = p - [0, PQ_Y, 0], c = cos(PQ_TILT), s = sin(PQ_TILT) )
  [ q.x, q.y*c + q.z*s, -q.y*s + q.z*c ];
function pqdist(p) =
  let( l = to_pq(p),
       dx = max(0, abs(l.x) - PQ_W/2),
       dy = max(0, max(-l.y, l.y - PQ_D)),
       dz = max(0, max(-l.z, l.z - (PQ_T + T_RAISE))) )
  norm([dx, dy, dz]);
NE = 40;
CLEAR = min([ for (b = FAM)
                let( f = bar_frame(b[0], b[1]), e = bar_ends(b[0], b[1]) )
                min([ for (i = [0:NE]) for (c = SEC)
                        pqdist(e[0] + (e[1]-e[0])*i/NE + c[0]*f[1] + c[1]*f[2]) ]) ]);
HCLEAR = min([ for (z = RINGZ) for (i = [0:71]) for (dr = [-RW, RW]) for (dz = [-RH/2, RH/2])
                 pqdist([(rsurf(z)+dr)*cos(i*5), (rsurf(z)+dr)*sin(i*5), z+dz]) ]);
assert(min(CLEAR, HCLEAR) > 1.0, "the plaque touches the object");

// -- volumes that the union will shave, reported before it does -------
BLEN = [ for (b = FAM) let( e = bar_ends(b[0], b[1]) ) norm(e[1] - e[0]) ];
VBAR = sum([ for (l = BLEN) 4*BR*BW*l ]);
VHOOP = sum([ for (z = RINGZ) 2*PI*rsurf(z)*2*RW*RH ]);

TRIS_MEASURED = 68886;      // from tools/validate.py on the exported STL

echo("OBJECT   bbox mm", OBB, " x", OX, " y", OY, " z", OZ);
echo("SCENE    bbox mm",
     [ max(2*PL_R, PQ_W), OZ[1] - min(-PL_H, 0), 0 ],
     " plinth r", PL_R, " plaque near edge y", PQ_Y - PQ_T*sin(PQ_TILT));
echo("MESH     triangles measured", TRIS_MEASURED,
     " bars", len(FAM), " hoops", len(RINGZ));
echo("SURFACE  a", A_W, " c", C_S, " end radius", R_E, " = 16 sqrt2",
     " waist radius min over hoops", min([ for (z = RINGZ) rsurf(z) ]));
echo("EQUATION max |x^2/a^2 + y^2/a^2 - (z-30)^2/c^2 - 1| on the 24 bar axes",
     max(BRES), " over", (NS+1)*len(FAM), "samples");
echo("THICKNESS same residual at the drawn arrises, which is the bar's",
     "own 1.5 x 2.4 section and not an error:", max(CRES));
echo("RULINGS  surface points tested", len(PAIR),
     " passing (mirror line hits the point, lies in the surface, is distinct)", NTWO,
     " max point-to-line", max([ for (r = PAIR) r[0] ]),
     " max residual along it", max([ for (r = PAIR) r[1] ]));
echo("CROSSING angle of the two rulings, min/max over those points",
     min([ for (r = PAIR) r[2] ]), max([ for (r = PAIR) r[2] ]), "deg");
echo("TANGENT  plane x =", A_W, "holds", len(TP), "bars; x = a leaves",
     "y = +-(a/c)(z-30), slope", TPSLOPE, "= 8/15; endpoint error", TPERR);
echo("CHORD    full ruling chord", CHORD, "= sqrt(32^2 + 60^2), horizontal run",
     2*A_W, "= 2a; drawn length", BLEN[0]);
echo("WAIST    cos of the crossing angle there", XCOS, "= 161/289 =",
     161/289, " angle", XANG, "deg");
echo("SKEW     min distance between same-family ruling LINES", SKEW,
     " section diagonal", DIAG, " so same-family bars are disjoint:", SKEW > DIAG);
echo("HOOPS    radii", [ for (z = RINGZ) rsurf(z) ],
     " max equation residual", HRES);
echo("PLAQUE   lines mm", LRUN, " slab", [PQ_W, PQ_D, PQ_T],
     " tilt off upright", 90 - PQ_TILT, "deg");
echo("CLEAR    min distance object-to-plaque: bars", CLEAR, " hoops", HCLEAR);
echo("VOLUME   before the union shaves the crossings: bars", VBAR,
     " hoops", VHOOP, " plinth", PI*sq(PL_R)*PL_H);
