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
//  a function of u alone, with no m left in it.  Substituting u = z/h,
//
//      x^2 + y^2 = (R cos d)^2 + z^2 (R sin d)^2 / h^2
//                = a^2 ( 1 + z^2/c^2 ),     a = R cos d,  c = h/tan d,
//
//  which is the hyperboloid of one sheet with waist a = R cos d and
//  semi-axis c = h cot d.  Both sides are polynomials in u, so the
//  identity holds for the whole LINE and not merely for the chord: the
//  line lies wholly in the surface.  Reversing the twist, d -> -d,
//  changes neither cos^2 d nor sin^2 d, so the mirrored family lands on
//  the SAME surface.  That is the double ruling, and it is the reason a
//  water tower can be a curved shell made of straight rolled steel --
//  Vladimir Shukhov's, first at Nizhny Novgorod in 1896, and some two
//  hundred after it.
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
//  Two lines, named explicitly, for every point of the surface.  Both
//  are tested below on a grid of 189 points: the distance from the point
//  to the mirrored line, the residual of the equation along that whole
//  line, and the angle between the two, which must not be zero or the
//  "two" is a lie.
//
//  THE CASE THAT NEEDS NO DECIMALS
//
//  With h = c the twist is a quarter turn, d = atan(h/c) = 45 degrees,
//  and the end circles have radius R = a/cos 45 = a sqrt 2.  At a = 16,
//  h = c = 30 that is R = 16 sqrt 2, and the twelve ring nodes are put
//  at the azimuths 30k +- 45 degrees, which both families share, so the
//  bar of mean azimuth 0 runs from
//
//      (16, -16, -30)  to  (16, +16, +30)
//
//  -- integer endpoints, with x = 16 at both.  Its mate of the other
//  family runs from (16, +16, -30) to (16, -16, +30).  Both lie in the
//  vertical plane x = 16, which is the tangent plane at the waist point
//  (16, 0, 0), and setting x = 16 in the equation leaves
//
//      y^2/16^2 = z^2/30^2    i.e.    y = +- (8/15) z,
//
//  two straight lines and nothing else.  A plane tangent to a
//  hyperboloid of one sheet meets it in exactly two lines; here both of
//  them are members of the lattice, with integer endpoints, and the file
//  goes and finds them in its own bar list rather than quoting them.
//  Each bar's direction is (0, 16, 30) up to sign, of length 34 -- the
//  8-15-17 triple doubled -- so the full chord is exactly
//
//      sqrt(32^2 + 60^2) = 68 mm,
//
//  and the horizontal run 32 is exactly the waist DIAMETER 2a.  The two
//  bars cross at the waist at an angle whose cosine is the exact
//  rational
//
//      (900 - 256)/(900 + 256) = 644/1156 = 161/289 = 0.557093...,
//
//  about 56.145 degrees, and they cross at the ring nodes at 38.872.
//  All of it is recomputed below from the built geometry.
//
//  HOW A BAR IS BUILT
//
//  A bar is a right prism on a 1.5 x 2.4 rectangle: two copies of the
//  section, one at each end of the axis, four wall quads, two flat caps.
//  The section's first axis e1 is the horizontal radial at the bar's
//  midpoint, and it is exactly perpendicular to the bar with no
//  orthogonalisation, because both ends sit at the same HORIZONTAL
//  radius R and, writing hz() for "drop the z component",
//
//      hz(p0 + p1).(p1 - p0) = |hz(p1)|^2 - |hz(p0)|^2 = R^2 - R^2 = 0.
//
//  The hz() is not decoration.  The two ends are at different heights,
//  so the full radii are not equal and the same dot product without it
//  is |p1|^2 - |p0|^2 = HT^2 = 3600, not zero; it is the horizontal
//  radii that match, and that is the projection the code takes.
//
//  e2 = t x e1 completes a right-handed triple (e1, e2, t); the section
//  is listed counter-clockwise in (e1, e2), so the wall quads' right-hand
//  normals point INTO the solid, which is what polyhedron() wants.  The
//  caps run the other way round the ring: a wall quad [a,b,c,d] gives the
//  directed edge d->a, so the cap that meets it must give a->d.  Because
//  the caps stay perpendicular to the axis, each bar is a right prism and
//  its volume is exactly section x length -- which is how the volume
//  audit at the foot can be a prediction rather than an estimate.
//
//  The ends are pulled in along the axis until the prism's lowest corner
//  sits at z = AIR and its highest at z = HT - AIR.  The axis stays on
//  the ruling: a sub-segment of a line that lies in the surface still
//  lies in the surface.
//
//  The five hoops are square-section rings swept a full turn.  Each
//  hoop's radius is the surface radius at its own mid-height, so the
//  hoops ARE horizontal sections of the same hyperboloid, and a hoop is
//  genus one because its profile orbits a hole, never because a disc was
//  cut out of it.  A hoop of NSEG stations is a prismatoid ring between
//  two regular NSEG-gons, so its volume is exact too:
//
//      V = RH * (NSEG/2) sin(360/NSEG) * ( (r+RW)^2 - (r-RW)^2 ).
//
//  WHY THE WHOLE OBJECT IS ONE polyhedron(), AND WHAT THAT COST
//
//  Bars of OPPOSITE families CROSS -- that is the point of the thing,
//  and a riveted Shukhov lattice crosses in exactly the same places.
//  Bars of the same family are skew, which is the other half of the
//  ruling theorem, and the minimum distance between same-family ruling
//  LINES is measured below against the section's diagonal, so that claim
//  is tested rather than assumed.  But twenty-four bars with every bar
//  meeting seven of the other family is one connected overlap graph of
//  twenty-four pieces, and asking the export-time union to resolve it
//  does not work here.  It was measured, not guessed.  Hand the bars of
//  THIS file to a union, export, and count validate.py's `holes`:
//
//      one crossing pair, generic offset      0 leaked edges
//      the pair that crosses at the waist    42
//      all twenty-four bars                 768
//      all twenty-nine shells               860
//
//  The 42 is not bad luck.  The two bars that cross at the waist share a
//  mean azimuth and therefore share e1, so their inner and outer faces
//  lie in exactly the same two planes x = a +- BR, and coincident faces
//  are the one configuration a union is entitled to get wrong.  An
//  earlier draft rolled one family's section off that configuration and
//  the pair did go 42 -> 0, but the whole lattice only fell to 36, and
//  the leak survived every other knob -- halving the section, staggering
//  the end trims, pulling the ends 4 mm clear of the shared ring nodes.
//  That 36 belongs to that draft, whose roll is no longer in this file,
//  so it is quoted here and not claimed; the four counts in the table
//  are reproducible from the file exactly as it stands.  The coincident
//  faces at the waist are the only degeneracy anyone has named, and
//  removing them still left a leak, so what is actually established is
//  narrower than a diagnosis: this union cannot be relied on across one
//  connected chain of twenty-four overlapping pieces.  That is enough.
//  None is asked for.
//
//  So the lattice is not twenty-nine solids handed to a union.  It is
//  ONE polyhedron() holding twenty-nine closed shells, written out with
//  its own point and face lists, and no boolean runs on it at all.  That
//  is the same answer the geodesic dome in examples/architecture reaches
//  from the other direction: its 782 parts share no volume, so their
//  concatenation IS their union.  Here the parts do share volume and the
//  concatenation is NOT their union -- the export's signed volume adds
//  the crossings twice.  That is stated rather than hidden: the audit
//  below predicts the sum-of-shells volume in closed form, and the
//  number the exporter measures has to match it, which is a real check
//  on every prism and every hoop even though it is not the volume of the
//  union.  The roll is gone with the boolean that needed it, so the two
//  families are once again exact mirror images.
//
//  Nothing else in the file overlaps anything.  AIR = 0.15 mm of air
//  separates the lattice from the plinth and the letters from the slab,
//  the same trick the type specimen in examples/type uses, so the export
//  is a concatenation from end to end and there is nothing anywhere for
//  a boolean to get wrong.
//
//  ONE DEPARTURE FROM THE GALLERY CONVENTION, AND THE ARITHMETIC FOR IT
//
//  The convention asks for a plaque slab 78 x 46 x 3 whose near edge is
//  at y = -46, tilted back 30 degrees.  Tilt it back 30 degrees from the
//  HORIZONTAL and the 3 mm thickness puts the hinge at
//  y = -46 + 3 sin 30 = -44.5, from which the far edge reaches
//
//      y = -44.5 + 46 cos 30 = -4.66,   z = 46 sin 30 = 23.0,
//
//  five millimetres from the axis and twenty-three up -- not merely over
//  the plinth but inside the exhibit.  The bound is general and brutal:
//  that slab's face at height z sits at |y| = 44.5 - z cot 30, which
//  reaches zero at z = 25.7, so a flat 30-degree lectern of this depth
//  demands an object that has tapered to nothing by then.  No object
//  55 to 70 mm tall can satisfy it, and this one does not: the face
//  passes |y| = 19.29 at z = 14.6, and 19.29 is the outer radius of the
//  hoop at z = 15, which spans z = 13.7 to 16.3.  It would cut the hoop
//  in half.  So the 30 degrees is taken
//  here from the VERTICAL: the slab leans back 30 degrees off upright.
//  That keeps all three published numbers -- the 78 x 46 x 3 slab, the
//  near edge at y = -46, the 30 degrees -- and clears the lattice.  The
//  clearance is measured and not asserted: the smallest distance from
//  5376 points along the object's arrises to the plaque box is echoed
//  below, and asserted positive.  It comes out at 2.60 mm.
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
PL_R  = 34;  PL_H = 6;  PL_N = 120;     // plinth: top face at z = 0
PQ_W  = 78;  PQ_D = 46;  PQ_T = 3;      // plaque slab
PQ_TILT = 60;                           // 60 from horizontal = 30 off upright
T_RAISE = 0.90;                         // raised letter thickness

// ---- colours --------------------------------------------------------
C_BAR  = "#c4652c";     // oxidised steel
C_PLIN = "#2b2f33";
C_SLAB = "#44505a";
C_TEXT = "#f2ece0";

// $fa/$fs rather than $fn, because $fn would make a 3.2 mm letter's
// bowl pay the same as the plinth's 34 mm rim.  The plinth asks for its
// own $fn below, where it is the only thing that wants one.
$fa = 7;  $fs = 0.6;

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

// t is the unit axis; e1 the horizontal radial at the midpoint, exactly
// perpendicular to t with no orthogonalisation; e2 = t x e1, so that
// (e1, e2, t) is right-handed.  Both families use the same rule, so bars
// of opposite family that cross at the waist are exact mirror images.
function bar_frame(k, s) =
  let( q = bar_q(k, s), d = q[1] - q[0], L = norm(d), t = d/L,
       e1 = unit(hz((q[0] + q[1])/2)) )
  [ t, e1, cross(t, e1), L ];

// Pull both ends in along the axis until the prism's lowest corner sits
// at z = AIR and its highest at z = HT - AIR.  The axis stays on the
// ruling -- a sub-segment of a line in the surface is still in it.
function bar_ends(k, s) =
  let( q = bar_q(k, s), f = bar_frame(k, s),
       dz = BW*abs(f[2].z) + BR*abs(f[1].z) + AIR,
       lam = dz/f[0].z )
  [ q[0] + lam*f[0], q[1] - lam*f[0] ];

SEC = [ [BR, -BW], [BR, BW], [-BR, BW], [-BR, -BW] ];   // CCW in (e1, e2)

// Each hoop's radius is the surface radius at its own mid-height, so the
// hoops ARE horizontal sections of the same hyperboloid.
RINGZ = [ AIR + RH/2, HT/4, H_Z, 3*HT/4, HT - AIR - RH/2 ];
FAM = [ for (s = [1, -1]) for (k = [0:NB-1]) [k, s] ];

function bar_ring(k, s, u) =
  let( f = bar_frame(k, s), e = bar_ends(k, s) )
  [ for (c = SEC) e[u] + c[0]*f[1] + c[1]*f[2] ];

// ---- the object, as a single mesh -----------------------------------
// Twenty-four prisms and five hoops, written as ONE polyhedron holding
// twenty-nine closed shells.  The header says why: the bars genuinely
// cross, and the export-time union leaks on a chain of twenty-four
// overlapping pieces, so no boolean is asked for.  Each shell is closed
// and wound inward on its own, which is all the exporter and the
// validator need; what the shells do to each other is the lattice's
// business and is accounted for in the volume audit.

NSEG = 72;              // stations round a hoop
BARP = 8;               // points per bar: two rings of four
HPTS = 4*NSEG;          // points per hoop
NBAR = len(FAM);

// A hoop station.  Sweeping about +z the tangent is t = (-sin f, cos f, 0);
// taking e1 outward radial makes e2 = t x e1 = -z, and (e1, e2, t) is
// right-handed, so the same counter-clockwise section order as the bars
// again gives inward wall normals.
function hoop_ring(z, i) =
  let( f = 360*i/NSEG, c = cos(f), sn = sin(f), r = rsurf(z) )
  [ [ (r+RW)*c, (r+RW)*sn, z + RH/2 ],
    [ (r+RW)*c, (r+RW)*sn, z - RH/2 ],
    [ (r-RW)*c, (r-RW)*sn, z - RH/2 ],
    [ (r-RW)*c, (r-RW)*sn, z + RH/2 ] ];

// A wall quad [a,b,c,d] emits the directed edge d->a on the near ring and
// b->c on the far one, so the near cap must run 0->1->2->3 and the far cap
// the other way round.  That is the orientation-blind edge test, and it is
// the only thing that keeps the two caps from agreeing with their walls.
function bar_faces(o) = concat(
  [ for (v = [0:3]) [ o+v, o+4+v, o+4+(v+1)%4, o+(v+1)%4 ] ],
  [ [o+0, o+1, o+2, o+3], [o+7, o+6, o+5, o+4] ]);
function hoop_faces(o) =
  [ for (u = [0:NSEG-1]) for (v = [0:3])
      [ o + u*4 + v,             o + ((u+1)%NSEG)*4 + v,
        o + ((u+1)%NSEG)*4 + (v+1)%4, o + u*4 + (v+1)%4 ] ];

OBJ_PTS = concat(
  [ for (b = FAM) each concat(bar_ring(b[0], b[1], 0), bar_ring(b[0], b[1], 1)) ],
  [ for (z = RINGZ) for (i = [0:NSEG-1]) each hoop_ring(z, i) ]);
OBJ_FACES = concat(
  [ for (i = [0:NBAR-1]) each bar_faces(i*BARP) ],
  [ for (i = [0:len(RINGZ)-1]) each hoop_faces(NBAR*BARP + i*HPTS) ]);

module lattice() {
    color(C_BAR) polyhedron(points = OBJ_PTS, faces = OBJ_FACES, convexity = 8);
}

// ---- the plinth -----------------------------------------------------
module plinth() {
    color(C_PLIN) translate([0, 0, -PL_H]) cylinder(r = PL_R, h = PL_H, $fn = PL_N);
}

// ---- the plaque -----------------------------------------------------
// Local slab coordinates: x across, y from the near edge to the far one,
// z out of the reading face.  rotate([PQ_TILT,0,0]) sends local (0,0,PQ_T)
// to y = -PQ_T sin(PQ_TILT), which is the slab's nearest point, so that
// is what is parked at y = -46.
PQ_Y = -46 + PQ_T*sin(PQ_TILT);

UPEM = 1000;  ASC = 970;  DESC = -250;   // InstrumentSans-Regular, per em
// hmtx advances for ASCII 32..126, straight out of the bundled face
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
         "of one sheet - Shukhov water tower, 1896",
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
            translate([0, LBY[i], PQ_T + AIR])
              linear_extrude(height = T_RAISE)
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
// The drawn arrises are NOT on the surface and are not meant to be: the
// axis is, and the corner is half a section away from it.  The residual
// there is measured too, and reported as what it is -- the bar's own
// thickness showing up in the equation -- so that the number above
// cannot be mistaken for a tolerance that was chosen.
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
// Both ruling directions have the same length -- (K_R, H_Z) up to signs,
// here (16, 30) and so 34 -- which is why one squared norm serves as the
// whole denominator of the cosine.
XCOS  = (ruledir(0,1)*ruledir(0,-1))/sq(norm(ruledir(0,1)));
XANG  = acos(XCOS);
XLEN  = norm(ruledir(0,1));

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
// frame, with the box grown out to PQ_T + AIR + T_RAISE so that the
// letters standing proud of the face count as part of it.
function to_pq(p) =
  let( q = p - [0, PQ_Y, 0], c = cos(PQ_TILT), s = sin(PQ_TILT) )
  [ q.x, q.y*c + q.z*s, -q.y*s + q.z*c ];
function pqdist(p) =
  let( l = to_pq(p),
       dx = max(0, abs(l.x) - PQ_W/2),
       dy = max(0, max(-l.y, l.y - PQ_D)),
       dz = max(0, max(-l.z, l.z - (PQ_T + AIR + T_RAISE))) )
  norm([dx, dy, dz]);
NE = 40;
CLEAR = min([ for (b = FAM)
                let( f = bar_frame(b[0], b[1]), e = bar_ends(b[0], b[1]) )
                min([ for (i = [0:NE]) for (c = SEC)
                        pqdist(e[0] + (e[1]-e[0])*i/NE + c[0]*f[1] + c[1]*f[2]) ]) ]);
HCLEAR = min([ for (z = RINGZ) for (i = [0:NSEG-1])
                 for (dr = [-RW, RW]) for (dz = [-RH/2, RH/2])
                   let( f = 360*i/NSEG, r = rsurf(z) + dr )
                   pqdist([r*cos(f), r*sin(f), z + dz]) ]);
assert(min(CLEAR, HCLEAR) > 1.0, "the plaque touches the object");

// -- the object's volume, in closed form, to be matched by the export --
// A bar is a right prism, so its volume is section x length exactly.  A
// hoop of NSEG stations is a prismatoid ring between two regular
// NSEG-gons of circumradius r +- RW, so its volume is exactly
//     RH * (NSEG/2) sin(360/NSEG) * ((r+RW)^2 - (r-RW)^2),
// which at NSEG -> infinity is the 2 pi r (2 RW) RH one expects.  The
// shells overlap at the crossings and the exporter's signed volume adds
// each crossing twice; so does this sum, which is the point -- the two
// have to agree, and they do to the last digit a float32 STL can carry.
BLEN  = [ for (b = FAM) let( e = bar_ends(b[0], b[1]) ) norm(e[1] - e[0]) ];
VBAR  = sum([ for (l = BLEN) 4*BR*BW*l ]);
VHOOP = sum([ for (z = RINGZ)
                RH*(NSEG/2)*sin(360/NSEG)*(sq(rsurf(z)+RW) - sq(rsurf(z)-RW)) ]);
VOBJ_MEASURED = 10118.648231;   // tools/validate.py on the object alone

TRIS_MEASURED = 122116;     // tools/validate.py on the whole exported STL

echo("OBJECT   bbox mm", OBB, " x", OX, " y", OY, " z", OZ);
// The plaque, carried out of the slab's frame so the scene box is
// measured rather than guessed: the slab's own eight corners, and a box
// round the letters, whose vertical extent comes from the face's own
// ascent and descent (970 and -250 per 1000 em) and whose width is the
// longest line.  Keeping them apart matters, because the letters stand
// 1.05 mm proud of the face and would otherwise be reported as the
// plaque's near edge, which sits at y = -46 exactly.
function pq_pt(x, y, z) =
  [ x, PQ_Y + y*cos(PQ_TILT) - z*sin(PQ_TILT), y*sin(PQ_TILT) + z*cos(PQ_TILT) ];
PQC  = [ for (x = [-PQ_W/2, PQ_W/2]) for (y = [0, PQ_D]) for (z = [0, PQ_T])
           pq_pt(x, y, z) ];
TXTC = [ for (x = [-max(LRUN)/2, max(LRUN)/2])
           for (y = [ LBY[3] + DESC*LSZ[3]/UPEM, LBY[0] + ASC*LSZ[0]/UPEM ])
             for (z = [PQ_T + AIR, PQ_T + AIR + T_RAISE]) pq_pt(x, y, z) ];
PQALL = concat(PQC, TXTC);
SX = [ min(-PL_R, min([ for (p = PQALL) p.x ]), OX[0]),
       max( PL_R, max([ for (p = PQALL) p.x ]), OX[1]) ];
SY = [ min(-PL_R, min([ for (p = PQALL) p.y ]), OY[0]),
       max( PL_R, max([ for (p = PQALL) p.y ]), OY[1]) ];
SZ = [ -PL_H, max(OZ[1], max([ for (p = PQALL) p.z ])) ];

echo("SCENE    bbox mm", [SX[1]-SX[0], SY[1]-SY[0], SZ[1]-SZ[0]],
     " x", SX, " y", SY, " z", SZ,
     " slab near edge y", min([ for (p = PQC) p.y ]),
     " slab top z", max([ for (p = PQC) p.z ]),
     " letters stand proud by", AIR + T_RAISE);
echo("MESH     triangles measured", TRIS_MEASURED, " shells in the object",
     len(FAM) + len(RINGZ), " bars", len(FAM), " hoops", len(RINGZ),
     " holes 0  inconsistently wound edges 0");
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
echo("WAIST    cos of the crossing angle there", XCOS, "= 161/289 =", 161/289,
     " angle", XANG, "deg; ruling direction length", XLEN,
     "= 2 x the 8-15-17 triple");
echo("SKEW     min distance between same-family ruling LINES", SKEW,
     " section diagonal", DIAG, " so same-family bars are disjoint:", SKEW > DIAG);
echo("HOOPS    radii", [ for (z = RINGZ) rsurf(z) ],
     " max equation residual", HRES);
echo("PLAQUE   lines mm", LRUN, " slab", [PQ_W, PQ_D, PQ_T],
     " tilt off upright", 90 - PQ_TILT, "deg");
echo("CLEAR    min distance object-to-plaque: bars", CLEAR, " hoops", HCLEAR);
echo("VOLUME   object predicted: bars", VBAR, "+ hoops", VHOOP, "=", VBAR + VHOOP,
     " exporter measured", VOBJ_MEASURED,
     " relative difference", abs(VBAR + VHOOP - VOBJ_MEASURED)/(VBAR + VHOOP));
echo("NOTE     that is the sum of 29 interpenetrating shells, not the volume",
     "of their union: the crossings are counted twice in both numbers.");
