// ===================================================================
//  THE SCHWARZ LANTERN  --  Hermann Amandus Schwarz, 1880
//
//  The shape that broke a definition.
//
//  Arc length is the limit of inscribed polygons.  Take any curve,
//  mark points along it, join them with chords, refine: the chord
//  total converges to the length, and it does so however the points
//  are chosen, so long as the gaps between them shrink to zero.  No
//  other condition is needed.  For a century it was assumed -- taught,
//  it was printed in textbooks -- that surface area worked the same
//  way: inscribe a polyhedron in a surface, refine the triangulation,
//  and the polyhedral area converges to the area of the surface.
//
//  It does not.  Schwarz built this.  Every vertex of it lies exactly
//  ON the cylinder.  The polyhedron converges to the cylinder in the
//  strongest geometric sense -- no point of its wall is further
//  than r(1 - cos(pi/n)) from the cylinder, and that goes to zero
//  as n grows, uniformly, whatever m is doing.  And yet its
//  area can be made to converge to anything at all from 2 pi r h
//  upwards, including infinity, by choosing how fast m grows with n.
//  The limit of the areas is not a property of the limit shape.
//
//  ---- the construction -------------------------------------------
//
//  Cut the cylinder of radius r and height h into m bands by m+1
//  level circles z_j = j h / m.  On each circle mark n points, and
//  rotate the odd-numbered circles by HALF a step:
//
//      P(j, k) = ( r cos(t), r sin(t), j h / m ),
//                t = 2 pi k / n  +  (j mod 2) * pi / n
//
//  Between consecutive circles lay 2n triangles: n pointing up (base
//  a chord of circle j, apex the lone vertex of circle j+1 that sits
//  over the chord's midpoint) and n pointing down (base a chord of
//  circle j+1, apex on circle j).  The half-step is what makes each
//  apex land over a midpoint; without it the bands would be flat
//  quadrilateral strips and nothing interesting would happen.
//
//  2mn triangles in the wall.  Cap both ends with a fan of n and the
//  thing is a closed polyhedron: 2n(m+1) faces on n(m+1)+2 vertices.
//
//  ---- why the area is wrong --------------------------------------
//
//  All 2mn wall triangles are congruent, so the wall area is 2mn
//  times one of them, and one of them is elementary.
//
//  Its base is a chord subtending 2 pi / n, so
//
//      base = 2 r sin(pi / n).
//
//  Its apex sits on the next circle, half a step round, so the apex
//  is radially OUTSIDE the chord's midpoint by the chord's sagitta
//
//      d = r - r cos(pi / n) = r (1 - cos(pi / n)),
//
//  and vertically above it by one band, h / m.  Base and the segment
//  from the chord midpoint to the apex are perpendicular -- the chord
//  runs across the axial plane that contains both the midpoint and
//  the apex -- so that segment IS the triangle's height:
//
//      height = sqrt( (h/m)^2 + d^2 ).
//
//  Therefore, exactly, with no approximation anywhere:
//
//      A(m, n) = 2 m n r sin(pi/n) sqrt( (h/m)^2 + r^2 (1-cos(pi/n))^2 )
//
//  Divide by the cylinder's own lateral area 2 pi r h and the m's and
//  the h's fall out into one dimensionless statement:
//
//      A / (2 pi r h) = ( n sin(pi/n) / pi ) * sqrt( 1 + (m d / h)^2 )
//                                             ^^^^^^^^^^^^^^^^^^^^^^^
//                       Archimedes' inscribed        THE DAMAGE
//                       polygon -- harmless,
//                       rises to 1 as n grows
//
//  The first factor is the perimeter ratio of an inscribed n-gon.  It
//  is the only factor arc length has, which is exactly why arc length
//  behaves.  The second factor is what a surface has and a curve does
//  not: the triangles are TILTED off the surface.  The chord lies in
//  the tangent plane, and the triangle's height leaves it with a
//  radial component d against an axial component h/m, so the facet's
//  plane meets the cylinder's tangent plane at exactly
//
//      T = atan( m d / h ),
//
//  and sqrt(1 + (m d/h)^2) is nothing but sec T.  The whole statement
//  is therefore
//
//      A / (2 pi r h)  =  (inscribed n-gon perimeter ratio) * sec T,
//
//  a tilted triangle covering sec T times the area of its shadow.
//  Nothing stops T from climbing to 90 degrees, and sec 90 is where
//  the counterexample lives.  The file checks the identity at the
//  foot: sec T against sqrt(1 + (m d/h)^2), to 1e-12.
//
//  Since d = r(1 - cos(pi/n)) -> r pi^2 / (2 n^2),
//
//      m d / h  ->  (pi^2 r / 2h) * (m / n^2),
//
//  so everything is decided by the single ratio m / n^2:
//
//      m / n^2 -> 0        A -> 2 pi r h          the expected answer
//      m / n^2 -> c        A -> 2 pi r h sqrt(1 + (pi^2 r c / 2h)^2)
//                                                 any value above it
//      m / n^2 -> infinity A -> infinity          Schwarz's boot
//
//  The file prints all three columns at the foot, computed, for
//  n = 4, 8, 16, 32, 64, 128 with m = n, m = n^2 and m = n^3.  The
//  middle column walks up to sqrt(1 + (pi^2 r / 2h)^2) = 1.758435...
//  for this exhibit's r and h -- a perfectly finite, perfectly wrong
//  answer, which is the detail that makes the counterexample lethal.
//  You cannot patch the definition by demanding convergence; the
//  areas converge beautifully, to the wrong number.
//
//  ---- the exhibited specimen -------------------------------------
//
//  r = 17, h = 58, n = 8, m = 44.  m was picked to put m d / h
//  within a hair of 1, which stands every wall facet at T = 44.47
//  degrees to the cylinder.  That is the 1 : 1 pleat -- the radial
//  depth d = 1.294 mm and the vertical pitch h/m = 1.318 mm are the
//  same size -- and it is the aspect ratio at which the pleating is
//  unmistakable: flatter and the lantern reads as a slightly rough
//  tube, steeper and the bands close up into a fringe too fine to
//  resolve.  The surface really does swing between radius 17 and
//  17 cos(22.5) = 15.706 once per band, forty-four times over
//  fifty-eight millimetres.
//
//  The measured wall area is 1.3656 times the cylinder it is
//  inscribed in.  The file measures it off the exported face list
//  rather than trusting the formula, then checks the two against
//  each other and asserts they agree to 1e-9.
//
//  ---- winding and watertightness ---------------------------------
//
//  polyhedron() here wants the right-hand normal pointing INTO the
//  solid.  In cylindrical coordinates e_theta x e_z = e_r, outward,
//  so an up-triangle listed base-then-apex in the +theta sense faces
//  out; it is listed [base0, apex, base1] instead.  A down-triangle
//  listed [base0, base1, apex] has e_theta x (-e_z) = -e_r and is
//  already inward.  The caps follow from the ring edges: the wall
//  traverses the bottom ring the way the bottom fan must traverse it
//  backwards, and likewise at the top.  Every interior edge is used
//  by exactly two faces in opposite directions -- the orientation
//  blind test -- so the export has no holes and nothing flipped.
//
//  Nothing is subtracted.  There is no difference(), no
//  intersection(), no hull().  The lantern is one polyhedron() call,
//  the plinth a cylinder, the plaque a cube and four extruded texts.
//
//  ---- nothing touches anything -----------------------------------
//
//  AIR = 0.15 mm separates the lantern from the plinth and the
//  letters from the slab, the same gap examples/curiosities/enneper
//  and hyperboloid use and for the same reason.  It is not cosmetic.
//  Resting the lantern's base cap flat on the plinth's top face puts
//  two discs in the plane z = 0 sharing the point (0,0,0), and the
//  export-time union then has two sound shells in contact to merge:
//  it returned one body of 3664 triangles with 12 holes and 12
//  T-junctions, from 720 + 636 triangles that were each perfect
//  alone.  With the gap there is no contact, no merge is attempted,
//  and the export is a concatenation of shells that were already
//  right.  The lantern is 58 mm tall and its lowest point is at
//  z = 0.15, so it still sits at z >= 0 as the convention asks.
//
//  ---- one departure from the gallery convention ------------------
//
//  The convention asks for the 78 x 46 x 3 plaque tilted back 30
//  degrees with its near edge at y = -46.  Taken from the HORIZONTAL
//  that slab is parked at y = -46 + 3 sin30 = -44.5 and its far top
//  arris lands at
//
//      y = -44.5 + 46 cos30 = -4.66,   z = 46 sin30 = 23.0,
//
//  four and a half millimetres from the axis, twenty-three up, which
//  is inside any exhibit tall enough to meet the 55-70 mm rule.  The
//  hyperboloid and the Enneper surface in this same folder both hit
//  this and both resolved it the same way, so this file follows them:
//  the 30 degrees is taken from the VERTICAL instead.  All three
//  published numbers survive -- 78 x 46 x 3, near edge at y = -46,
//  30 degrees -- and the slab's closest approach to the axis becomes
//
//      y = -46 + 3 sin60 + 46 cos60 = -20.402,
//
//  clearing the lantern's 17 mm radius by 3.402 mm.  That clearance
//  is computed and asserted below, not eyeballed.
// ===================================================================

// ---- the cylinder the lantern is inscribed in -----------------------
R  = 17;        // radius, mm
H  = 58;        // height, mm   -- 58 is the exhibit's largest dimension

// ---- the triangulation ----------------------------------------------
N  = 8;         // points per level circle;  2N triangles per band
M  = 44;        // bands.  M d / H = 0.9817, so the facets sit at 44.47
                // degrees and the pleating is at its most legible.

// ---- the gallery furniture -------------------------------------------
PL_R = 34;  PL_H = 6;  PL_N = 160;      // plinth: top face at z = 0
PQ_W = 78;  PQ_D = 46;  PQ_T = 3;       // plaque slab
PQ_TILT = 60;                           // 60 off horizontal = 30 off upright
T_RAISE = 0.90;                         // raised letter thickness
AIR     = 0.15;                         // see "nothing touches anything" above

// ---- colour ----------------------------------------------------------
C_LANT = "#e3a62f";     // lamplight on folded paper
C_PLIN = "#2a2d31";     // graphite
C_SLAB = "#2f4854";     // slate blue
C_TEXT = "#f4efe2";     // bone

// =====================================================================
//  THE LANTERN
// =====================================================================

// Derived quantities the whole file leans on.
STEP = 360 / N;                 // azimuth between neighbours on a circle
HALF = STEP / 2;                // the offset that makes the thing work
BAND = H / M;                   // vertical pitch
SAG  = R * (1 - cos(HALF));     // chord sagitta d: the radial pleat depth
CHORD= 2 * R * sin(HALF);       // chord length, the triangle's base

function par(j)  = j - 2*floor(j/2);        // j mod 2, for the half-step
function wrap(k) = (k >= N) ? k - N : k;    // k only ever reaches N

// vertex (j, k) of the lantern wall; then the two cap centres
function VI(j, k) = j*N + wrap(k);
CB = (M + 1) * N;               // bottom cap centre, at the origin
CT = CB + 1;                    // top cap centre

PTS = concat(
    [ for (j = [0 : M]) for (k = [0 : N-1])
        let (t = STEP*k + par(j)*HALF)
          [ R*cos(t), R*sin(t), AIR + BAND*j ] ],
    [ [0, 0, AIR], [0, 0, AIR + H] ]
);

// --- the wall: m bands of 2n triangles -------------------------------
// s = j mod 2.  Circle j+1 is offset from circle j by +HALF when s = 0
// and by -HALF when s = 1, so the apex over the midpoint of the chord
// (j,k)-(j,k+1) is vertex k+s of circle j+1, and the apex under the
// midpoint of (j+1,k)-(j+1,k+1) is vertex k+1-s of circle j.
//
// Winding is inward (right-hand normal into the solid):
//   up    [base0, apex, base1]   -- reversed, because +theta then +z
//                                   would point the normal outward
//   down  [base0, base1, apex]   -- +theta then -z already points in
WALL = [ for (j = [0 : M-1]) for (k = [0 : N-1]) let (s = par(j))
           each [ [ VI(j, k),   VI(j+1, k+s), VI(j, k+1)     ],
                  [ VI(j+1, k), VI(j+1, k+1), VI(j, k+1-s)   ] ] ];

// --- the caps ---------------------------------------------------------
// The wall traverses the bottom ring as (0,k+1)->(0,k), so the fan must
// traverse it as (0,k)->(0,k+1); at the top the wall runs (M,k)->(M,k+1)
// so the fan runs backwards.  Both then have inward normals: +z at the
// bottom of the solid, -z at the top.
CAPS = concat(
    [ for (k = [0 : N-1]) [ CB, VI(0, k),   VI(0, k+1) ] ],
    [ for (k = [0 : N-1]) [ CT, VI(M, k+1), VI(M, k)   ] ]
);

FACES = concat(WALL, CAPS);

module lantern() {
    color(C_LANT) polyhedron(points = PTS, faces = FACES, convexity = 6);
}

// =====================================================================
//  PLINTH AND PLAQUE
// =====================================================================

module plinth() {
    color(C_PLIN) translate([0, 0, -PL_H]) cylinder(r = PL_R, h = PL_H, $fn = PL_N);
}

// Local slab coordinates: x across, y from the near edge to the far one,
// z out of the reading face.  rotate([PQ_TILT,0,0]) sends local (0,0,PQ_T)
// to y = -PQ_T sin(PQ_TILT), the slab's nearest point, so that is what is
// parked at y = -46.
PQ_Y = -46 + PQ_T*sin(PQ_TILT);

// --- measuring the text so no line can overrun the slab ---------------
// InstrumentSans-Regular, the one face scadforge bundles.  hmtx advances
// for ASCII 32..126, per 1000 units of em.  No font= argument anywhere:
// a named lookup would only warn and fall back to this same face.
UPEM = 1000;
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

function psum(v, a, b) =
      (b - a <= 0) ? 0
    : (b - a == 1) ? v[a]
    : let (c = floor((a + b)/2)) psum(v, a, c) + psum(v, c, b);

function runw(s, sz) =
    sz/UPEM * psum([ for (i = [0 : len(s)-1]) ADV[ord(s[i]) - 32] ], 0, len(s));

// --- the measured numbers, computed before the plaque is written ------
// Areas are summed off the exported face list, so the plaque quotes the
// mesh and not the algebra.  The algebra is then checked against it.
function tri_area(f) =
    0.5 * norm(cross(PTS[f[1]] - PTS[f[0]], PTS[f[2]] - PTS[f[0]]));

A_WALL  = psum([ for (f = WALL) tri_area(f) ], 0, len(WALL));
A_CAPS  = psum([ for (f = CAPS) tri_area(f) ], 0, len(CAPS));
A_CYL   = 2 * PI * R * H;               // what it is inscribed in
RATIO   = A_WALL / A_CYL;               // <- the number on the plaque

// closed form, for the cross-check
function lantern_area(m, n) =
    let (hf = 180/n, sg = R*(1 - cos(hf)))
        2*m*n*R*sin(hf) * sqrt((H/m)*(H/m) + sg*sg);
function lantern_ratio(m, n) = lantern_area(m, n) / A_CYL;

// Four decimals, built from integers so the plaque cannot pick up a
// float's tail.  str(1.3656) is fine; str(round(x*1e4)/1e4) is a gamble.
function pad4(i) = str(i < 10 ? "000" : i < 100 ? "00" : i < 1000 ? "0" : "", i);
function f4(x) = let (k = round(x*10000), w = floor(k/10000))
                     str(w, ".", pad4(k - w*10000));

// Line 3 states only what this file measures and asserts: the ratio, off
// the exported face list, and that every vertex is on the cylinder
// (ONCYL = 0, asserted below).  It earlier read "A ~ m/n^2", which is
// not true as written -- A is not proportional to m/n^2.  m/n^2 decides
// the LIMIT as n -> infinity, and only in the regime m/n^2 -> infinity
// does A grow in proportion to it; at this specimen's m/n^2 = 0.6875 the
// ratio is 1.3656, and for m/n^2 -> 0 the area is asymptotically CONSTANT.
// The REGIMES table at the foot is where that statement belongs, stated
// as the limit it actually is.
LINE = [ "SCHWARZ LANTERN",
         "Hermann Amandus Schwarz, 1880",
         str("m=", M, " n=", N, ":  A = ", f4(RATIO),
             " x 2 pi r h, inscribed"),
         "converges to the cylinder; the area does not" ];
LSZ  = [ 6, 3.2, 3.2, 3.2 ];
LBY  = [ 33.5, 25.0, 18.0, 11.0 ];      // baselines in local slab y
LRUN = [ for (i = [0:3]) runw(LINE[i], LSZ[i]) ];

assert(max(LRUN) < PQ_W - 10,  "a plaque line is wider than the text column");
assert(max([ for (l = LINE) len(l) ]) < 46, "a plaque line is over 46 characters");

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

$fa = 6;  $fs = 0.5;

lantern();
plinth();
plaque();

// =====================================================================
//  WHAT THE FILE CHECKS ABOUT ITSELF
// =====================================================================

XS = [ for (p = PTS) p[0] ];
YS = [ for (p = PTS) p[1] ];
ZS = [ for (p = PTS) p[2] ];
OBB  = [ max(XS)-min(XS), max(YS)-min(YS), max(ZS)-min(ZS) ];
FOOT = max([ for (p = PTS) norm([p[0], p[1]]) ]);

// Every vertex is ON the cylinder: this is the whole point of the
// counterexample, so it is measured rather than assumed.
ONCYL = max([ for (j = [0:M]) for (k = [0:N-1])
                abs(norm([PTS[VI(j,k)][0], PTS[VI(j,k)][1]]) - R) ]);
assert(ONCYL < 1e-9, "a lantern vertex has left the cylinder");
assert(min(ZS) >= AIR, "the lantern dips below the plinth top");
assert(FOOT < PL_R,  "the lantern overhangs the plinth");

// Volume by the divergence theorem over the same faces; the sign is
// negated because the winding is inward.
function tdet(f) = let (A = PTS[f[0]], B = PTS[f[1]], C = PTS[f[2]])
    ( A[0]*(B[1]*C[2] - B[2]*C[1])
    + A[1]*(B[2]*C[0] - B[0]*C[2])
    + A[2]*(B[0]*C[1] - B[1]*C[0]) ) / 6;
VOL = -psum([ for (f = FACES) tdet(f) ], 0, len(FACES));
// A closed prismatoid: each band is an antiprism-like solid, and the
// whole thing has the volume of a right prism on the inscribed n-gon of
// the ODD rings and the EVEN rings alike -- (n/2) r^2 sin(2 pi/n) h --
// plus the m wedges the half-step leaves over.  Easier to state as the
// n-gon prism, which is what the deviation below is measured against.
VPRISM = (N/2) * R*R * sin(360/N) * H;

// Clearance from the lantern to the plaque slab.  The slab's nearest
// point to the axis is its far bottom arris, local (y,z) = (PQ_D, 0).
PQ_NEAR_Y = PQ_Y + PQ_D*cos(PQ_TILT);
CLEAR = -PQ_NEAR_Y - R;
assert(CLEAR > 0, "the plaque slab runs into the lantern");

A_TOT  = A_WALL + A_CAPS;
A_CAPX = N * R*R * sin(360/N);          // two regular n-gons, exactly
A_FORM = lantern_area(M, N);
TILT   = atan(M*SAG/H);                 // facet tilt off the vertical

echo(str("SCHWARZ LANTERN   r = ", R, "  h = ", H, "  n = ", N, "  m = ", M));
echo(str("BBOX      object ", OBB[0], " x ", OBB[1], " x ", OBB[2], " mm",
         "   (2r x 2r x h);  lowest z = ", min(ZS), " = AIR",
         ";  footprint radius ", FOOT, " <= plinth ", PL_R));
echo(str("MESH      triangles = ", len(FACES), " = 2nm wall ", len(WALL),
         " + 2n cap ", len(CAPS), ";  vertices = ", len(PTS), " = n(m+1)+2"));
echo(str("INSCRIBED max |sqrt(x^2+y^2) - r| over all ", (M+1)*N,
         " wall vertices = ", ONCYL, " mm   (every vertex is ON the cylinder)"));
echo(str("PLEAT     band h/m = ", BAND, " mm,  sagitta d = r(1-cos(pi/n)) = ",
         SAG, " mm,  facet tilt T = atan(md/h) = ", TILT, " deg"));
echo(str("SEC T     1/cos T = ", 1/cos(TILT), " = sqrt(1+(md/h)^2) = ",
         sqrt(1 + pow(M*SAG/H, 2)), ";  difference = ",
         1/cos(TILT) - sqrt(1 + pow(M*SAG/H, 2))));
assert(abs(1/cos(TILT) - sqrt(1 + pow(M*SAG/H, 2))) < 1e-12,
       "sec T is not the area inflation factor");
echo(str("SEC T     n-gon perimeter ratio n sin(pi/n)/pi = ", N*sin(180/N)/PI,
         ";  times sec T = ", (N*sin(180/N)/PI)/cos(TILT), " = the RATIO below"));
echo(str("RADIUS    swings between r = ", R, " and r cos(pi/n) = ", R*cos(HALF),
         " mm once per band"));

echo(str("AREA      wall, summed over the ", len(WALL),
         " exported triangles = ", A_WALL, " mm^2"));
echo(str("AREA      wall, closed form 2mnr sin(pi/n) sqrt((h/m)^2+d^2) = ",
         A_FORM, " mm^2   difference = ", A_WALL - A_FORM));
assert(abs(A_WALL - A_FORM) < 1e-9, "measured wall area disagrees with the formula");
echo(str("AREA      caps, summed = ", A_CAPS, " mm^2;  exactly n r^2 sin(2pi/n) = ",
         A_CAPX, ";  difference = ", A_CAPS - A_CAPX));
echo(str("AREA      cylinder 2 pi r h = ", A_CYL, " mm^2"));
echo(str("AREA      RATIO wall / 2 pi r h = ", RATIO,
         "   <- the plaque's claim, printed as ", f4(RATIO)));
echo(str("AREA      closed inscribed total (wall + caps) = ", A_TOT,
         " mm^2;  cylinder with its lids = ", A_CYL + 2*PI*R*R));
echo(str("VOLUME    ", VOL, " mm^3;  n-gon prism (n/2)r^2 sin(2pi/n) h = ",
         VPRISM, ";  the half-step wedges add ", VOL - VPRISM));

// ---- the divergence, at fixed n, as m climbs -------------------------
// A/(2 pi r h) = (n sin(pi/n)/pi) sqrt(1 + (m d / h)^2).  At fixed n the
// first factor is frozen and the second is linear in m for large m, so
// every factor of ten in m is a factor of ten in area.  Same cylinder,
// same n, same vertices lying on it, area without bound.
MS = [ M, 10*M, 100*M, 1000*M, 10000*M ];
echo(str("DIVERGE   fixed n = ", N, ", m climbing.  ratio = A / 2 pi r h:"));
for (mm = MS)
    echo(str("DIVERGE     m = ", mm, "   m/n^2 = ", mm/(N*N),
             "   ratio = ", lantern_ratio(mm, N)));

// ---- the three regimes, which is the actual theorem ------------------
// Let m = n^k and send n to infinity.  k = 1 converges to the cylinder,
// k = 3 diverges, and k = 2 converges to the WRONG finite number
// sqrt(1 + (pi^2 r / 2h)^2) -- the detail that kills any attempt to
// repair the definition by merely requiring the limit to exist.
LIM2 = sqrt(1 + pow(PI*PI*R/(2*H), 2));
echo(str("REGIMES   m = n^k, n -> infinity.  ratio = A / 2 pi r h:"));
echo(str("REGIMES     n        m=n            m=n^2          m=n^3"));
for (nn = [4, 8, 16, 32, 64, 128])
    echo(str("REGIMES     ", nn, "        ",
             lantern_ratio(nn, nn),      "   ",
             lantern_ratio(nn*nn, nn),   "   ",
             lantern_ratio(nn*nn*nn, nn)));
echo(str("REGIMES   k=1 -> 1 exactly;  k=2 -> sqrt(1+(pi^2 r/2h)^2) = ", LIM2,
         " (finite and WRONG);  k=3 -> infinity"));

echo(str("PLAQUE    line widths mm ", LRUN, "  slab ", [PQ_W, PQ_D, PQ_T],
         " tilted ", PQ_TILT, " off horizontal = ", 90 - PQ_TILT, " off upright"));
echo(str("CLEAR     slab's nearest approach to the axis y = ", PQ_NEAR_Y,
         ";  lantern radius ", R, ";  clearance = ", CLEAR, " mm"));
