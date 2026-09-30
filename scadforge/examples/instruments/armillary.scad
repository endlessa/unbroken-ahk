// ===================================================================
//  PLACEHOLDER HEADER, rewritten once the numbers are in
// ===================================================================

// ---- the sky -------------------------------------------------------
EPS   = 23.4393;      // mean obliquity of the ecliptic, J2000, degrees
LAT   = 51.4779;      // Royal Observatory, Greenwich
LST   = 300;          // local sidereal time 20h 00m, as an angle
RA_A  = 213.9154;     // Arcturus, right ascension, degrees
DEC_A =  19.1824;     // Arcturus, declination

// ---- the metal -----------------------------------------------------
GAP  = 0.6;           // air left between any two solids
TOL  = 0.02;          // chord tolerance that sets each ring's station count

RO_H = 150;  W_H = 16;  T_H = 5;    // horizon ring
W_M  = 14;   T_M = 5;               // meridian ring
W_E  = 11;   T_E = 4;               // the five equatorial bands
W_C  = 10;   T_C = 4;               // the ecliptic band
T_B  = 14;                          // plinth thickness

// ---- vector rotations, applied in the function layer so every ------
// ---- vertex is known in world coordinates and can be audited -------
function rxv(a, v) = [ v[0], v[1]*cos(a) - v[2]*sin(a), v[1]*sin(a) + v[2]*cos(a) ];
function ryv(a, v) = [ v[0]*cos(a) + v[2]*sin(a), v[1], -v[0]*sin(a) + v[2]*cos(a) ];
function rzv(a, v) = [ v[0]*cos(a) - v[1]*sin(a), v[0]*sin(a) + v[1]*cos(a), v[2] ];

// Halving sum: pairwise, so the rounding error grows like log n rather
// than n, and so the mesh-volume sum over every triangle in the model
// does not recurse once per triangle.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

// ===================================================================
//  DERIVED DIMENSIONS
// ===================================================================
RI_H = RO_H - W_H;
SH_H = norm([RO_H, T_H/2]);                 // horizon ring, outermost point

RO_M = sqrt(pow(RI_H - GAP, 2) - pow(T_M/2, 2));
RI_M = RO_M - W_M;
SH_M = norm([RO_M, T_M/2]);

// The equatorial family is five flat bands on one sphere: the equator,
// the two tropics at declination +-EPS, and the two polar circles at
// +-(90 - EPS).  The band that reaches furthest from the centre is a
// tropic, not the equator, because tipping a band towards the pole buys
// less in radius than it costs in height.  Write the family's outer
// reach for declination d and it is
//     R^2 + R (W cos d + T sin d) + (W^2 + T^2)/4,
// largest at d = EPS among the five.  Setting that equal to the meridian
// ring's inner clearance is a quadratic in R with one positive root.
BQ   = W_E*cos(EPS) + T_E*sin(EPS);
CQ   = (W_E*W_E + T_E*T_E)/4;
KQ   = pow(RI_M - GAP, 2);
R_S  = (-BQ + sqrt(BQ*BQ - 4*CQ + 4*KQ))/2;

DECS = [0, EPS, -EPS, 90 - EPS, -(90 - EPS)];
function fam_r(d)   = R_S*cos(d);
function fam_z(d)   = R_S*sin(d);
function fam_in(d)  = norm([fam_r(d) - W_E/2, max(abs(fam_z(d)) - T_E/2, 0)]);
function fam_out(d) = norm([fam_r(d) + W_E/2, abs(fam_z(d)) + T_E/2]);
FIN  = min([ for (d = DECS) fam_in(d) ]);
FOUT = max([ for (d = DECS) fam_out(d) ]);

// The ecliptic is the largest band whose outer corner clears the whole
// equatorial family by GAP.
R_C  = sqrt(pow(FIN - GAP, 2) - pow(T_C/2, 2)) - W_C/2;
SH_C = norm([R_C + W_C/2, T_C/2]);

// Station counts: enough that the outer edge's sagitta is under TOL.
function nst(ro) = 4*ceil(180/acos(1 - TOL/ro)/4);

// ---- the stand -----------------------------------------------------
Z_PL = -RO_H;                       // plinth top, one horizon radius down
R_LT = (RO_H + RI_H)/2;  Z_LT = -T_H/2 - GAP;     // leg top
R_LF = 165;              Z_LF = Z_PL + GAP;       // leg foot
A_LT = 8;   B_LT = 7;
A_LF = 11;  B_LF = 12;
A_CT = 8.5; A_CF = 13;
Z_CT = -(RO_M + GAP);                             // column top, along -p
Z_CF = (Z_PL + GAP + A_CF*cos(LAT))/sin(LAT);     // column foot, see header
RO_B = R_LF + A_LF + 6;
RI_B = 95;

// ---- the alidade ---------------------------------------------------
AW = 3.2;  AV = 2.6;  AG = 1.0;  AD = 0.8;        // the rule's section
PW = 9.5;  PV = 11.5; PG = 2.2;  PD = 7.0;        // a pinnule
BW = 4.6;  BV = 3.8;  BG = 1.2;  BD = 1.2;        // the pivot swell
ALLOW = R_C - W_C/2 - GAP;
L_A   = sqrt(ALLOW*ALLOW - AW*AW - AV*AV);
H_A   = LST - RA_A;                               // Arcturus, hour angle

// ===================================================================
//  PRIMITIVES
// ===================================================================
// A flat annular band.  Section order is (ro, +t/2), (ro, -t/2),
// (ri, -t/2), (ri, +t/2), which with the stations running anticlockwise
// puts every right-hand face normal into the metal.  See the header.
function band_pts(ro, ri, t, n) =
  [ for (i = [0 : n-1]) let( a = 360*i/n, c = cos(a), s = sin(a) ) each
      [ [ro*c, ro*s,  t/2], [ro*c, ro*s, -t/2],
        [ri*c, ri*s, -t/2], [ri*c, ri*s,  t/2] ] ];

function band_faces(n) =
  [ for (i = [0 : n-1]) for (k = [0 : 3])
      let( j = (i+1)%n, l = (k+1)%4,
           a = 4*i + k, b = 4*j + k, c = 4*j + l, d = 4*i + l )
        each [ [a,b,c], [a,c,d] ] ];

// Exact volume of the n-gon band: the two end faces are plane annuli of
// n-gons, the walls are plane rectangles, so no triangulation choice can
// move it.
function band_vol(ro, ri, t, n) = t*(n/2)*sin(360/n)*(ro*ro - ri*ri);

// A loft: a list of sections, each a closed polygon of the same vertex
// count, already placed in space and advancing along the list.  cap is a
// triangulation of the section, wound the same way round as the section
// itself; it is used as given at the first section and reversed at the
// last.
function loft_pts(S) = [ for (s = S) each s ];
function loft_faces(S, cap) =
  let( m = len(S), q = len(S[0]) )
  concat(
    [ for (i = [0 : m-2]) for (k = [0 : q-1])
        let( l = (k+1)%q,
             a = i*q + k, b = (i+1)*q + k, c = (i+1)*q + l, d = i*q + l )
          each [ [a,b,c], [a,c,d] ] ],
    cap,
    [ for (f = cap) [ (m-1)*q + f[2], (m-1)*q + f[1], (m-1)*q + f[0] ] ] );

// An axis-aligned rectangle in a z = const plane, anticlockwise.
function rect_sec(cx, cy, z, a, b) =
  [ [cx-a, cy-b, z], [cx+a, cy-b, z], [cx+a, cy+b, z], [cx-a, cy+b, z] ];
RECT_CAP = [ [0,1,2], [0,2,3] ];

// The alidade's section: a rectangle with a sighting notch cut in from
// the +y side.  Eight vertices, anticlockwise, and the notch is never
// allowed to close, so no station is degenerate.
function alid_sec(z, w, v, g, d) =
  [ [-w,-v,z], [w,-v,z], [w,v,z], [g,v,z], [g,v-d,z], [-g,v-d,z], [-g,v,z], [-w,v,z] ];
// Ear-clipped by hand.  A fan from one corner would put triangles across
// the notch, which is what polyhedron() would do with an eight-sided
// face, so the caps are triangulated here instead of being handed over.
ALID_CAP = [ [1,2,3], [1,3,4], [1,4,5], [5,6,7], [0,1,5], [0,5,7] ];

// ---- placement -----------------------------------------------------
// sky(H) carries a shape built in the local xy plane onto the celestial
// sphere so that local +x sits at hour angle H on the equator and local
// +z is the north celestial pole.
function place_sky(P, H) = [ for (q = P) rxv(LAT-90, rzv(-90-H, q)) ];
function place_ecl(P)    = [ for (q = P) rxv(LAT-90, rzv(-90-LST, rxv(EPS, q))) ];
function place_pol(P)    = [ for (q = P) rxv(LAT-90, q) ];
function place_star(P, H, dec) =
  [ for (q = P) rxv(LAT-90, rzv(-90-H, ryv(90-dec, q))) ];
function place_z(P, a)   = [ for (q = P) rzv(a, q) ];
function place_mer(P)    = [ for (q = P) ryv(90, q) ];

// The same placements applied to a LIST OF SECTIONS rather than a flat
// list of points.  Keeping the two apart is not decoration: a placement
// that walks a section list as if it were a point list silently hands
// polyhedron() a points array of the wrong length, and what comes back
// is a handful of dropped faces and an "inside out" warning that points
// nowhere near the mistake.
function placeS_z(S, a)        = [ for (s = S) [ for (q = s) rzv(a, q) ] ];
function placeS_ecl(S)         = [ for (s = S) [ for (q = s) rxv(LAT-90, rzv(-90-LST, rxv(EPS, q))) ] ];
function placeS_pol(S)         = [ for (s = S) [ for (q = s) rxv(LAT-90, q) ] ];
function placeS_star(S, H, dc) = [ for (s = S) [ for (q = s) rxv(LAT-90, rzv(-90-H, ryv(90-dc, q))) ] ];
function shift(P, d)     = [ for (q = P) q + d ];
function skyv(H, v)      = rxv(LAT-90, rzv(-90-H, v));

// ===================================================================
//  THE PARTS
// ===================================================================
N_H = nst(RO_H);
N_M = nst(RO_M);
N_C = nst(R_C + W_C/2);
N_B = nst(RO_B);
function n_fam(d) = nst(fam_r(d) + W_E/2);

function fam_band(d) =
  let( n = n_fam(d) )
  shift(band_pts(fam_r(d) + W_E/2, fam_r(d) - W_E/2, T_E, n), [0, 0, fam_z(d)]);

// Ecliptic markers.  The two equinox marks are tall and thin, the two
// solstice marks low and square, so which is which survives a render.
function ecl_mark(lam, a, b, h) =
  placeS_ecl(placeS_z(
    [ rect_sec(R_C, 0, T_C/2 + GAP,     a, b),
      rect_sec(R_C, 0, T_C/2 + GAP + h, a, b) ], lam));

// Horizon marks, at the four cardinal azimuths measured from north.
function hor_mark(az, a, b, h) =
  placeS_z([ rect_sec(R_LT, 0, T_H/2 + GAP,     a, b),
            rect_sec(R_LT, 0, T_H/2 + GAP + h, a, b) ], 90 - az);

// A leg: a splayed tapered prism with a short foot block, +x radial.
function leg(az) =
  placeS_z(
    [ rect_sec(R_LF, 0, Z_LF,     A_LF, B_LF),
      rect_sec(R_LF, 0, Z_LF + 8, A_LF, B_LF),
      rect_sec(R_LT, 0, Z_LT,     A_LT, B_LT) ], 90 - az);

// The polar column.  Its sections are square and normal to the polar
// axis, so its foot meets the level plinth at the latitude angle.
COLUMN = placeS_pol(
  [ rect_sec(0, 0, Z_CF,      A_CF, A_CF),
    rect_sec(0, 0, Z_CF + 5,  A_CF, A_CF),
    rect_sec(0, 0, Z_CF + 9,  9.5,  9.5),
    rect_sec(0, 0, Z_CT - 4,  A_CT, A_CT),
    rect_sec(0, 0, Z_CT,      A_CT, A_CT) ]);

// The north polar stub, the other end of the same axis.
STUB = placeS_pol(
  [ rect_sec(0, 0, RO_M + GAP,      6,   6),
    rect_sec(0, 0, RO_M + GAP + 3,  6,   6),
    rect_sec(0, 0, RO_M + GAP + 11, 4.5, 4.5),
    rect_sec(0, 0, RO_M + GAP + 16, 4.5, 4.5) ]);

// The alidade, one closed polyhedron: a rectangular rule that swells at
// the centre pivot and rises into a notched pinnule near each end.
APROF = [ [ -L_A,       AW, AV, AG, AD ],
          [ -L_A + 2.0, AW, AV, AG, AD ],
          [ -L_A + 2.8, PW, PV, PG, PD ],
          [ -L_A + 5.6, PW, PV, PG, PD ],
          [ -L_A + 6.4, AW, AV, AG, AD ],
          [ -9.0,       AW, AV, AG, AD ],
          [ -7.5,       BW, BV, BG, BD ],
          [  7.5,       BW, BV, BG, BD ],
          [  9.0,       AW, AV, AG, AD ],
          [  L_A - 6.4, AW, AV, AG, AD ],
          [  L_A - 5.6, PW, PV, PG, PD ],
          [  L_A - 2.8, PW, PV, PG, PD ],
          [  L_A - 2.0, AW, AV, AG, AD ],
          [  L_A,       AW, AV, AG, AD ] ];
ALID = placeS_star([ for (p = APROF) alid_sec(p[0], p[1], p[2], p[3], p[4]) ],
                  H_A, DEC_A);

BRASS = [0.78, 0.62, 0.30];
STEEL = [0.55, 0.58, 0.62];
GOLD  = [0.88, 0.74, 0.34];
SLATE = [0.38, 0.40, 0.44];
COPP  = [0.72, 0.45, 0.28];

// ---- how far each solid reaches from the centre --------------------
// rmax is the largest vertex distance, which is exact: the furthest
// point of a polyhedron from any point is a vertex.  rmin is not, and
// the difference is what the clearances are made of.  For a band the
// closest point to the centre is the middle of a flat on the inner
// polygon, at ri cos(180/n), and never a vertex.  For a loft the
// closest point can be anywhere on a face, so it is found exactly, by
// the clamped barycentric closest-point-on-triangle.
function band_rmin(ri, t, n, z0) = norm([ ri*cos(180/n), max(abs(z0) - t/2, 0) ]);
function vrmax(P) = max([ for (q = P) norm(q) ]);

function tri_near(a, b, c) =
  let( ab = b - a, ac = c - a, d1 = ab*(-a), d2 = ac*(-a) )
  (d1 <= 0 && d2 <= 0) ? a :
  let( d3 = ab*(-b), d4 = ac*(-b) )
  (d3 >= 0 && d4 <= d3) ? b :
  let( vc = d1*d4 - d3*d2 )
  (vc <= 0 && d1 >= 0 && d3 <= 0) ? a + ab*(d1/(d1 - d3)) :
  let( d5 = ab*(-c), d6 = ac*(-c) )
  (d6 >= 0 && d5 <= d6) ? c :
  let( vb = d5*d2 - d1*d6 )
  (vb <= 0 && d2 >= 0 && d6 <= 0) ? a + ac*(d2/(d2 - d6)) :
  let( va = d3*d6 - d5*d4 )
  (va <= 0 && (d4 - d3) >= 0 && (d5 - d6) >= 0)
      ? b + (c - b)*((d4 - d3)/((d4 - d3) + (d5 - d6))) :
  let( den = 1/(va + vb + vc) ) a + ab*(vb*den) + ac*(vc*den);

function mesh_rmin(P, F) = min([ for (f = F) norm(tri_near(P[f[0]], P[f[1]], P[f[2]])) ]);

function bandpart(name, pts, n, col, rmin) =
  [ name, pts, band_faces(n), col, rmin, vrmax(pts) ];
function loftpart(name, S, cap, col, rmin = -1) =
  let( pts = loft_pts(S), fac = loft_faces(S, cap) )
  [ name, pts, fac, col, rmin < 0 ? mesh_rmin(pts, fac) : rmin, vrmax(pts) ];

PARTS = concat(
  [ bandpart("horizon",   band_pts(RO_H, RI_H, T_H, N_H), N_H, BRASS,
              band_rmin(RI_H, T_H, N_H, 0)),
    bandpart("meridian",  place_mer(band_pts(RO_M, RI_M, T_M, N_M)), N_M, BRASS,
              band_rmin(RI_M, T_M, N_M, 0)) ],
  [ for (i = [0 : 4]) bandpart(
      ["equator","tropic of Cancer","tropic of Capricorn",
       "arctic circle","antarctic circle"][i],
      place_sky(fam_band(DECS[i]), 0), n_fam(DECS[i]),
      i == 0 ? GOLD : COPP,
      band_rmin(fam_r(DECS[i]) - W_E/2, T_E, n_fam(DECS[i]), fam_z(DECS[i]))) ],
  [ bandpart("ecliptic", place_ecl(band_pts(R_C + W_C/2, R_C - W_C/2, T_C, N_C)),
             N_C, GOLD, band_rmin(R_C - W_C/2, T_C, N_C, 0)),
    bandpart("plinth", shift(band_pts(RO_B, RI_B, T_B, N_B), [0,0,Z_PL - T_B/2]),
             N_B, SLATE, band_rmin(RI_B, T_B, N_B, Z_PL - T_B/2)) ],
  [ for (i = [0 : 3]) loftpart(str("leg ", ["N","E","S","W"][i]),
       leg(90*i), RECT_CAP, STEEL) ],
  [ loftpart("polar column", COLUMN, RECT_CAP, STEEL),
    loftpart("polar stub",   STUB,   RECT_CAP, STEEL) ],
  [ loftpart("mark vernal equinox",    ecl_mark(  0, 1.6, 3.0, 9), RECT_CAP, STEEL),
    loftpart("mark June solstice",     ecl_mark( 90, 2.6, 2.6, 4), RECT_CAP, STEEL),
    loftpart("mark autumnal equinox",  ecl_mark(180, 1.6, 3.0, 9), RECT_CAP, STEEL),
    loftpart("mark December solstice", ecl_mark(270, 2.6, 2.6, 4), RECT_CAP, STEEL) ],
  [ for (i = [0 : 3]) loftpart(str("mark ", ["N","E","S","W"][i]),
       hor_mark(90*i, 2.4, 2.4, i == 0 ? 10 : 5), RECT_CAP, STEEL) ],
  // The alidade is the one solid that encloses the centre of the
  // instrument, so its shell runs from zero, not from its wall.
  [ loftpart("alidade", ALID, ALID_CAP, STEEL, 0) ] );

for (P = PARTS) color(P[3]) polyhedron(points = P[1], faces = P[2], convexity = 8);

// ===================================================================
//  WHAT THE GEOMETRY SAYS
// ===================================================================
function det3(a, b, c) = a[0]*(b[1]*c[2] - b[2]*c[1])
                       + a[1]*(b[2]*c[0] - b[0]*c[2])
                       + a[2]*(b[0]*c[1] - b[1]*c[0]);

// polyhedron() winds each face clockwise seen from outside, so the
// right-hand normal points into the metal and the divergence sum comes
// out negative for a solid.  The sign is turned round once, here, and a
// part that still reports a negative volume is one that was wound the
// wrong way round.  That is the only diagnostic there is for it: an
// inside-out solid renders exactly like a right-side-out one.
function mesh_vol(P, F) =
  -sum([ for (f = F) det3(P[f[0]], P[f[1]], P[f[2]]) ])/6;

function unitv(v) = v/norm(v);
function altof(v) = asin(v[2]/norm(v));
function azof(v)  = (atan2(v[0], v[1]) + 360) % 360;
function dms(a)   = str(floor(a), "d ", floor((a - floor(a))*60), "' ",
                        round(((a - floor(a))*60 - floor((a - floor(a))*60))*600)/10, "\"");

VOLS  = [ for (P = PARTS) mesh_vol(P[1], P[2]) ];
NTRI  = sum([ for (P = PARTS) len(P[2]) ]);
NVERT = sum([ for (P = PARTS) len(P[1]) ]);
VTOT  = sum(VOLS);

echo(str("ARMILLARY  ", len(PARTS), " solids, ", NVERT, " vertices, ", NTRI,
         " triangles, total volume ", VTOT, " mm3"));

// ---- the sky the instrument is set to ------------------------------
NCP  = rxv(LAT-90, [0,0,1]);                       // north celestial pole
ECP  = rxv(LAT-90, rzv(-90-LST, rxv(EPS, [0,0,1])));   // ecliptic pole
VEQ  = skyv(LST, [1,0,0]);                         // local +x of the ecliptic frame
NODE = unitv(cross(NCP, ECP));

echo(str("SET TO     latitude ", dms(LAT), " N, local sidereal time ",
         floor(LST/15), "h ", round((LST/15 - floor(LST/15))*60), "m, obliquity ", dms(EPS)));
echo("SET TO     polar axis altitude", altof(NCP), "= the latitude, azimuth", azof(NCP),
     "; ecliptic pole is", acos(NCP*ECP), "from it, which is the obliquity");

// The line of nodes, two ways.  The first is the x axis of the frame the
// ecliptic ring was built in, so it is where the equinox was PUT.  The
// second is the intersection of the two ring planes, computed from their
// poles alone and knowing nothing about how either ring was placed.  If
// the placement is right they are the same line.
echo("EQUINOX    frame axis  alt", altof(VEQ), " az", azof(VEQ));
echo("EQUINOX    plane cross alt", altof(NODE), " az", azof(NODE),
     " ; angle between the two routes", acos(max(-1, min(1, unitv(VEQ)*NODE))), "degrees");

// Where the ecliptic ring actually crosses the equator, measured off the
// ring itself rather than asserted.  Each centreline station is carried
// into equatorial coordinates and its declination read off.
ECL_C = [ for (i = [0 : N_C-1]) let( a = 360*i/N_C )
            rxv(EPS, [R_C*cos(a), R_C*sin(a), 0]) ];
ECL_D = [ for (q = ECL_C) asin(q[2]/norm(q)) ];
DMIN  = min([ for (d = ECL_D) abs(d) ]);
DMAX  = max(ECL_D);
CROSS = [ for (i = [0 : N_C-1]) if (abs(ECL_D[i]) <= DMIN) 360*i/N_C ];
SOLST = [ for (i = [0 : N_C-1]) if (abs(abs(ECL_D[i]) - DMAX) < 1e-9) 360*i/N_C ];
echo("ECLIPTIC   ", N_C, "stations; declination crosses zero at longitudes", CROSS,
     " (worst |dec| there", DMIN, "degrees) and reaches its extremes at", SOLST);
echo("ECLIPTIC   extreme declination", DMAX, "against the obliquity", EPS,
     " difference", DMAX - EPS, "degrees");
echo("ECLIPTIC   the crossing at longitude 0 sits at alt", altof(ECL_C[0]),
     " az", azof(rxv(LAT-90, rzv(-90-LST, ECL_C[0]))),
     " and its opposite at alt", altof(ECL_C[N_C/2]));

// ---- the nesting ---------------------------------------------------
echo("NESTING    horizon  Ro", RO_H, "Ri", RI_H, "t", T_H, "-> shell [",
     RI_H*cos(180/N_H), ",", SH_H, "]");
echo("NESTING    meridian Ro", RO_M, "Ri", RI_M, "t", T_M, "-> shell [",
     RI_M*cos(180/N_M), ",", SH_M, "]  clear of the horizon by", RI_H - GAP - SH_M + GAP);
echo("NESTING    equatorial sphere R", R_S, "-> family shell [", FIN, ",", FOUT,
     "]  binding band is the tropic, at declination", EPS);
echo("NESTING    ecliptic sphere R", R_C, "-> shell [", R_C - W_C/2, ",", SH_C, "]");
echo("NESTING    the four clearances", RI_H - SH_M, RI_M - FOUT, FIN - SH_C,
     ALLOW - L_A > 0 ? "and the alidade fits" : "AND THE ALIDADE DOES NOT FIT");

// ---- the near tangency the subject is really about -----------------
// At the solstices the ecliptic touches the tropics.  That is the one
// place in the celestial sphere where two great and small circles are
// tangent rather than crossing, and tangency is the case a boolean
// handles worst.  Carrying the ecliptic on a smaller sphere turns the
// touch into a measured clearance.  Distance from a point to a flat
// annular band about the polar axis is exact and closed form, so this is
// a measurement and not a sample of a distance field.
function ann_d(rho, zz, ri, ro, z0, t) =
  norm([ max(max(ri - rho, rho - ro), 0), max(abs(zz - z0) - t/2, 0) ]);

ECL_V = concat(
  [ for (q = band_pts(R_C + W_C/2, R_C - W_C/2, T_C, N_C)) rxv(EPS, q) ],
  [ for (i = [0 : N_C-1]) for (k = [0 : 3])
      let( a = band_pts(R_C + W_C/2, R_C - W_C/2, T_C, N_C)[4*i + k],
           b = band_pts(R_C + W_C/2, R_C - W_C/2, T_C, N_C)[4*((i+1)%N_C) + k] )
        rxv(EPS, (a + b)/2) ]);

function ecl_gap(d) =
  let( n = n_fam(d) )
  min([ for (q = ECL_V)
          ann_d(norm([q[0], q[1]]), q[2],
                (fam_r(d) - W_E/2)*cos(180/n), fam_r(d) + W_E/2, fam_z(d), T_E) ]);

echo("TANGENCY   ecliptic to equator", ecl_gap(0),
     " to tropic of Cancer", ecl_gap(EPS), " to tropic of Capricorn", ecl_gap(-EPS));
echo("TANGENCY   ecliptic to arctic circle", ecl_gap(90-EPS),
     " to antarctic circle", ecl_gap(-(90-EPS)), " ; the guarantee was", FIN - SH_C);

// ---- no two of the solids share a cubic millimetre -----------------
// Each solid is certified against each other one by two sufficient
// tests.  Either the shells they lie in, measured from the centre of the
// instrument, do not overlap, or their axis-aligned boxes do not.  The
// bands' inner reach is the CHORD of the inner polygon, not its
// circumradius, because the closest point of an n-gon band to the centre
// is the middle of a flat, not a vertex.  Getting that wrong overstates
// every clearance by about fifteen microns here, which is four percent
// of the clearance being claimed.
function pmin(P, k) = min([ for (q = P) q[k] ]);
function pmax(P, k) = max([ for (q = P) q[k] ]);
BOXES = [ for (P = PARTS) [ pmin(P[1],0), pmax(P[1],0), pmin(P[1],1),
                            pmax(P[1],1), pmin(P[1],2), pmax(P[1],2) ] ];
SHELL = [ for (P = PARTS) [ P[4], P[5] ] ];
NP = len(PARTS);
function sep(i, j) =
  let( A = SHELL[i], B = SHELL[j], X = BOXES[i], Y = BOXES[j] )
    max([ A[0]-B[1], B[0]-A[1],
          X[0]-Y[1], Y[0]-X[1], X[2]-Y[3], Y[2]-X[3], X[4]-Y[5], Y[4]-X[5] ]);
SEPS = [ for (i = [0 : NP-2]) for (j = [i+1 : NP-1]) sep(i, j) ];
SMIN = min(SEPS);
echo(str("DISJOINT   ", len(SEPS), " pairs, ",
         len([ for (s = SEPS) if (s > 0) s ]), " certified apart, tightest ", SMIN, " mm"));
echo("DISJOINT   the tightest pairs are",
     [ for (i = [0 : NP-2]) for (j = [i+1 : NP-1])
         if (sep(i,j) < SMIN + 1e-9) str(PARTS[i][0], " | ", PARTS[j][0]) ]);

// ---- volume, three ways --------------------------------------------
// The bands have a closed form: the end faces are plane annuli of
// n-gons and the walls are plane rectangles, so the volume is the
// prism formula and no triangulation choice can move it.  The mesh sum
// above is an independent route over the triangles that were actually
// written.  Because nothing overlaps anything, the export cannot change
// either number, so the total is a prediction and not an estimate.
BANDV = [ ["horizon",  band_vol(RO_H, RI_H, T_H, N_H)],
          ["meridian", band_vol(RO_M, RI_M, T_M, N_M)],
          ["equator",  band_vol(fam_r(DECS[0]) + W_E/2, fam_r(DECS[0]) - W_E/2, T_E, n_fam(DECS[0]))],
          ["tropic C", band_vol(fam_r(DECS[1]) + W_E/2, fam_r(DECS[1]) - W_E/2, T_E, n_fam(DECS[1]))],
          ["tropic K", band_vol(fam_r(DECS[2]) + W_E/2, fam_r(DECS[2]) - W_E/2, T_E, n_fam(DECS[2]))],
          ["arctic",   band_vol(fam_r(DECS[3]) + W_E/2, fam_r(DECS[3]) - W_E/2, T_E, n_fam(DECS[3]))],
          ["antarctic",band_vol(fam_r(DECS[4]) + W_E/2, fam_r(DECS[4]) - W_E/2, T_E, n_fam(DECS[4]))],
          ["ecliptic", band_vol(R_C + W_C/2, R_C - W_C/2, T_C, N_C)],
          ["plinth",   band_vol(RO_B, RI_B, T_B, N_B)] ];

for (i = [0 : len(PARTS)-1])
  echo(str("  part ", i, "  ", PARTS[i][0], "  ", len(PARTS[i][2]), " tris  V ", VOLS[i],
           "  shell [", PARTS[i][4], ", ", PARTS[i][5], "]",
           VOLS[i] > 0 ? "" : "   *** INSIDE OUT ***"));

echo("VOLUME     mesh sum over every triangle", VTOT, "mm3");
echo("VOLUME     closed form for the nine bands",
     sum([ for (b = BANDV) b[1] ]), " mesh sum for the same nine",
     sum([ for (i = [0:8]) VOLS[i] ]));
echo("VOLUME     worst band disagreement",
     max([ for (i = [0:8]) abs(BANDV[i][1] - VOLS[i]) ]), "mm3");

// ---- chording -------------------------------------------------------
echo("CHORDING   station counts", [N_H, N_M, n_fam(0), n_fam(EPS), n_fam(90-EPS), N_C, N_B],
     " worst outer sagitta",
     max([ RO_H*(1 - cos(180/N_H)), RO_M*(1 - cos(180/N_M)),
           (R_C + W_C/2)*(1 - cos(180/N_C)), RO_B*(1 - cos(180/N_B)),
           (fam_r(0) + W_E/2)*(1 - cos(180/n_fam(0))) ]), "against a tolerance of", TOL);

// ---- the alidade ----------------------------------------------------
SDIR = rxv(LAT-90, rzv(-90-H_A, ryv(90-DEC_A, [0,0,1])));
ALTC = asin(sin(LAT)*sin(DEC_A) + cos(LAT)*cos(DEC_A)*cos(H_A));
echo("ALIDADE    laid on Arcturus, RA", dms(RA_A/15*15), " dec", dms(DEC_A),
     " hour angle", H_A);
echo("ALIDADE    direction from the frame: alt", altof(SDIR), " az", azof(SDIR));
echo("ALIDADE    altitude from the spherical triangle:", ALTC,
     " difference", altof(SDIR) - ALTC, "degrees");
echo("ALIDADE    half length", L_A, " furthest vertex from the centre",
     PARTS[len(PARTS)-1][5], " allowed", ALLOW);

// ---- the stand ------------------------------------------------------
CFZ = [ for (q = COLUMN[0]) q[2] ];
echo("STAND      plinth top at z", Z_PL, " column foot spans z", min(CFZ), "to", max(CFZ),
     " so the slanted foot stands between", min(CFZ) - Z_PL, "and", max(CFZ) - Z_PL,
     "mm above it");
echo("STAND      column axis and stub axis differ by",
     acos(unitv(rxv(LAT-90,[0,0,1])) * unitv(rxv(LAT-90,[0,0,-1]))),
     "degrees, and the stub points at altitude", altof(rxv(LAT-90,[0,0,1])));

// ---- Euler ----------------------------------------------------------
echo("TOPOLOGY   per part V - E + F, bands are tori and must give 0, lofts are",
     "spheres and must give 2:",
     [ for (P = PARTS) len(P[1]) - 3*len(P[2])/2 + len(P[2]) ]);

