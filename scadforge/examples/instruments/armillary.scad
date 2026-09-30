// ===================================================================
//  An armillary sphere, every ring placed by rotation from first
//  principles
//
//  Nine flat rings stand inside one another here and not one of them is
//  put where it looks right.  Each is a rectangle swept round a circle
//  and closed into a single polyhedron, and each is carried onto the
//  celestial sphere by a rotation written out from the definition of the
//  coordinate system it belongs to.  The model then measures the
//  consequences.  Where the ecliptic crosses the equator, how close the
//  ecliptic passes to the tropic it is tangent to, how far the pole
//  stands above the horizon, how much metal there is: all of those are
//  read back off the geometry, and none of them is asserted anywhere in
//  the file.
//
//  THE FRAME, WRITTEN OUT
//
//  Take x east, y north, z up, which is right handed.  At latitude phi
//  the north celestial pole stands in the meridian plane at altitude
//  phi, so
//
//      p = (0, cos phi, sin phi).
//
//  The celestial equator is the great circle perpendicular to p, and its
//  point on the meridian, at hour angle zero, is due south at altitude
//  90 - phi:
//
//      e1 = (0, -sin phi, cos phi).
//
//  Complete the triad and
//
//      e2 = p x e1 = (1, 0, 0),
//
//  which is due east.  That is worth a pause.  The east point of the
//  horizon is a point of the celestial equator at every latitude, and
//  here it falls out of a cross product rather than being put there by
//  hand.  It is also why the celestial equator meets the horizon at the
//  east and west points at every latitude and at every setting of the
//  sphere: the two planes share that line whatever else moves.  The two
//  rings here are nested and never touch, but their planes still cross
//  there, and the model reports the altitude of that crossing as zero.
//
//  A ring drawn in the xy plane is carried onto the equator by the map
//  sending (x, y, z) to (e1, e2, p).  Rx(phi - 90) sends (x, y, z) to
//  (e2, -e1, p), which is the triad wanted turned a quarter turn about
//  p, so the map is Rx(phi - 90) . Rz(-90).  Hour angle runs westward
//  from the meridian while the drawn ring's own angle runs
//  anticlockwise, so a point at hour angle H sits at local angle -H, and
//  the whole placement is one line:
//
//      sky(H) = Rx(phi - 90) . Rz(-90 - H).
//
//  Every celestial ring in the file is sky() of a ring in the xy plane.
//  The equator is sky(0).  The tropics are the same ring shrunk to
//  radius R cos d and lifted to R sin d, at declination d = +-EPS; the
//  polar circles are the same at d = +-(90 - EPS), the complement, which
//  is the whole of what makes them polar circles.  A circle of constant
//  declination is a surface of revolution about p, so the hour angle it
//  is drawn at does not matter and none is chosen.
//
//  WHY THE ECLIPTIC CROSSES THE EQUATOR WHERE IT DOES
//
//  Hour angle and right ascension are related by H = LST - RA, so the
//  vernal equinox, RA zero, sits at hour angle LST.  Go to the frame
//  sky(LST).  Its local x is that equinox.  Its local z is the pole.
//  Its local y is z cross x, which has hour angle LST - 90 and therefore
//  RA 90, six hours: the June solstice point of the equator.  Now tip
//  the ring by the obliquity about local x.  The x axis is fixed, so the
//  equinoxes stay on the equator, and local y rises to declination
//  +EPS, which is the June solstice.  That is the ecliptic, and its own
//  ring parameter is ecliptic longitude measured from the equinox, with
//  no further conversion:
//
//      ecliptic = sky(LST) . Rx(EPS) . (ring in the xy plane).
//
//  Nothing in that is a placement to taste.  Change LST and the whole
//  sphere turns about its polar axis, which is what the instrument is
//  for; the ecliptic's relation to the equator does not move, because
//  both are built in the same frame.
//
//  The model then refuses to take its own word for it.  It walks the
//  ecliptic's centreline station by station, carries each station into
//  equatorial coordinates and reads its declination.  The declination
//  crosses zero at longitudes 0 and 180 and reaches its extremes at 90
//  and 270, and the extreme value comes back as the obliquity to the
//  last digit carried.  The line of nodes is then computed a second
//  time by a route that knows nothing about how either ring was placed,
//  as the cross product of the two rings' poles, and the two answers
//  agree to zero.  At the stated setting the equinoxes lie 18.1441
//  degrees above and below the horizon on azimuths 114.309 and 294.309,
//  and the ring reports that as a measurement.
//
//  The crossing is shown as well as stated.  The ecliptic carries twelve
//  marks, one for each sign, and the equator carries two, placed in the
//  frame sky(LST) at longitudes 0 and 180.  Neither mark knows the other
//  exists.  Dropping each back into its own band's plane, to take out
//  the stand-off along that band's normal, leaves two rays that agree to
//  zero degrees and agree with the node from the ring poles to nine
//  millionth of a degree.
//
//  THE RINGS ARE NESTED, AND THE NESTING IS DERIVED
//
//  A real armillary's rings are nested, each turning inside the last,
//  and that is what is built here.  It also buys something this model
//  needs: if no two solids share any volume, the exported mesh is
//  exactly the mesh that was written, face for face, so the volume audit
//  is a prediction and any disagreement at all is a finding.
//
//  The radii are not chosen.  Each ring's outer corner is set one
//  clearance inside the previous ring's inner surface, and the chain is
//  solved rather than measured.  For the equatorial family the step is a
//  quadratic.  A band at declination d on a sphere of radius R reaches
//
//      R^2 + R (W cos d + T sin d) + (W^2 + T^2)/4
//
//  from the centre, squared, and among the five declinations the
//  bracket is largest at d = EPS.  It is the tropics and not the equator
//  that come nearest the meridian ring, because tipping a band towards
//  the pole loses less in radius than it gains in height.  Setting that
//  reach equal to the meridian ring's inner clearance gives one positive
//  root for R, and the equatorial sphere's radius is whatever that root
//  is.
//
//  The ecliptic is then the largest band clearing all five of them.  At
//  the solstices it is TANGENT to the tropics, which is the defining
//  relation between the two and the single worst case a boolean can be
//  handed: two surfaces that touch without crossing.  Carrying the
//  ecliptic on a smaller sphere turns the touch into a clearance that
//  can be measured, and the model measures it exactly rather than
//  sampling a distance field, because the distance from a point to a
//  flat annular band about the polar axis is closed form.  It comes back
//  as 0.600186 mm where 0.600 was designed, and that residue is the
//  chording, which is the next paragraph.
//
//  A ring is a polygon, not a circle, and the clearances live on its
//  INNER surface.  The outer vertices sit exactly on the outer radius,
//  but the inner surface is a ring of flats whose middles lie at
//  ri cos(180/n), inside the inner radius by ri (1 - cos(180/n)).  The
//  first version of this chain took every clearance from the
//  circumradius, and the model's own clearance audit came back at 0.5845
//  where 0.600 had been designed.  Fifteen microns is nothing.  Not
//  being able to predict them is not nothing, and the fix is to take
//  every clearance from the chord, which is what the file does now and
//  why IN_H, IN_M and IN_C exist.  Station counts are therefore settled
//  first, from a chord tolerance of 0.02 mm, before any radius is
//  derived from any other.
//
//  NO TWO OF THE THIRTY-FOUR SOLIDS SHARE A CUBIC MILLIMETRE
//
//  That is proved in the file, for all 561 pairs, and it has to be,
//  because the kernel would not have said so.  Measured on this build:
//  two 40 mm boxes overlapping by 8 microns export as two separate
//  shells, self-intersecting, enclosing 6400 where their union is
//  6399.36, with no warning of any kind; at 8.2 microns the same pair
//  merges and gives the union exactly.  The threshold scales with the
//  size of the solids, about two parts in ten thousand of it, and not
//  with where they sit.  So on an instrument 300 mm across, an
//  accidental overlap of up to about 0.03 mm would pass in silence and
//  the volume audit would pass with it.  Proving disjointness inside the
//  model is the only way the audit means anything.
//
//  The proof is a separating-axis certificate.  A pair is apart if the
//  shells they occupy, measured from the centre of the instrument, do
//  not overlap, or if their extents along any of six directions do not:
//  the three world axes, the polar axis, the pole of the ecliptic, and
//  the line joining the two solids' own centroids.  The polar axis is
//  the one that cannot be left out.  The five equatorial bands lie in
//  the same spherical shell and in overlapping boxes, and what separates
//  them is declination, which is exactly a slab test along p.  The
//  smallest margin over all 561 pairs is 0.600 mm, the clearance the
//  chain was built with, and thirty-three different pairs achieve it,
//  which is what a chain solved rather than measured looks like.
//
//  Getting a solid's inner reach right is half of that.  The outermost
//  point of a polyhedron is always a vertex, so rmax is a maximum over
//  vertices and is exact.  The innermost point is not: for a band it is
//  the middle of a flat, for a lofted prism it can be anywhere on a
//  face.  So the bands use the closed form and the lofts use the clamped
//  barycentric closest-point-on-triangle, which is exact and costs
//  nothing on the few hundred triangles they have between them.
//
//  WINDING, AND THE ONLY DIAGNOSTIC THERE IS FOR IT
//
//  polyhedron() wants each face wound clockwise seen from outside, so
//  the right-hand normal points INTO the solid.  For a sweep advancing
//  along d whose faces are [ring u vertex v, ring u+1 vertex v, ring u+1
//  vertex v+1, ring u vertex v+1], that forces the section perimeter to
//  be traversed in the direction d x n_out.  For a band the advance is
//  the azimuth and the outer face's outward normal is radial, so
//  traversal is theta x rho = -z: the section runs (ro, +t/2),
//  (ro, -t/2), (ri, -t/2), (ri, +t/2), which looks clockwise when the
//  radial and axial axes are drawn the ordinary way.  The cap at the
//  START of a loft takes the section order as given and the cap at the
//  END takes it reversed.
//
//  A solid wound the other way round renders identically and then loses
//  geometry in every boolean it touches.  The kernel does warn about it,
//  but only for a mesh it can see is closed and consistent, and a model
//  that hands it thirty-four polyhedra wants to know WHICH one.  So the
//  file sums the divergence theorem over each part's own triangles,
//  turns the sign once for the clockwise convention, and prints a
//  per-part volume; a part that comes back negative is marked.  The
//  nine bands also have a closed form, because their end faces are plane
//  annuli of polygons and their walls are plane rectangles, so no
//  triangulation choice can move them: t (n/2) sin(360/n) (ro^2 - ri^2).
//  The two routes agree to 1.3e-9 mm^3 on 965864.
//
//  Every quadrilateral in this model is planar.  That is not luck.  A
//  band's walls are vertical rectangles between two stations at one
//  radius, and a lofted prism's side face lies in the plane x = a z + b
//  through both of its rectangles, whatever the taper.  So unlike a
//  twisting sweep, where the two triangulations of a saddle straddle the
//  patch they stand for, nothing here depends on which diagonal is
//  taken, and the volume is exact rather than second order.
//
//  THE ALIDADE IS ONE POLYHEDRON
//
//  A sighting rule wants a slit at each end, and a slit is a subtraction
//  this model does not have.  So the alidade is a single closed
//  polyhedron swept along the sight line whose SECTION carries the slit:
//  an eight-sided polygon, a rectangle with a notch cut in from one
//  side, swept through fourteen stations.  Along the rule the notch is a
//  shallow groove; at each pinnule the section jumps to a tall plate and
//  the groove becomes a slit seven millimetres deep and four and a half
//  wide.  Sighting is through both slits, 183 mm apart, which is about a
//  degree and a half of resolution.  The notch is never allowed to close
//  to zero, because a section that degenerates puts zero-area triangles
//  with repeated vertices into the mesh and those read as leaks.
//
//  The caps are triangulated here rather than handed over.  A non-convex
//  polygon fanned from one corner throws triangles across the notch.
//  This kernel in fact ear-clips a non-convex coplanar face correctly, a
//  five-sided dart test comes back at exactly its true volume, but the
//  reference only promises "fan/ear" triangulation, and on a face whose
//  shape is the whole point it is better not to find out.  Six triangles
//  by hand, checked by the rule that every ear must see only interior.
//
//  The rule is laid on Arcturus.  Its hour angle is LST - RA and its
//  direction is sky(H) . Ry(90 - dec) applied to the local z axis; the
//  model reports the altitude that comes out of that frame and, beside
//  it, the altitude from the spherical triangle,
//  sin a = sin phi sin dec + cos phi cos dec cos H.  They agree to zero.
//  The rule's half length is not chosen either: it is the longest that
//  keeps its furthest corner one clearance inside the ecliptic's inner
//  flats, and the model prints that corner's distance beside the
//  allowance.
//
//  THE STAND, AND WHERE THE LATITUDE ENTERS IT
//
//  The plinth is the ninth flat ring, and it stands one horizon radius
//  below the horizon plane.  Four legs splay from it to the underside of
//  the horizon ring at the cardinal points.  The polar axis is the
//  column, which rises along -p from the plinth to the south celestial
//  pole of the meridian ring, and the stub, which continues along +p on
//  the far side; the model checks that the two are 180 degrees apart and
//  that the stub points at altitude phi, which is the latitude.  Set the
//  model to another latitude and the column swings, the plinth stays
//  level, and every ring follows, because nothing downstream of LAT is
//  written down anywhere.
//
//  The column's sections are square and normal to the polar axis, so its
//  foot is a plane cut at the latitude angle standing over a level
//  plinth.  The two faces meet at 51.5 degrees and the air between them
//  opens from 0.6 mm at the low corner to 16.8 mm at the high one, which
//  is where the column's foot length comes from: it is set so the lowest
//  corner clears the plinth by exactly GAP, and the rest follows from
//  the angle.  Which is the honest place to say what all the clearances
//  are.  Nothing in this model is fastened to anything.
//  Where a real instrument has a rivet, a tenon or a pivot in a drilled
//  hole, this one has GAP = 0.6 mm of air, because a hole is a
//  subtraction and a rivet is an overlap, and either would cost the
//  exact volume that the whole audit rests on.  The clearances are
//  stated rather than hidden, and they are the same 0.6 mm everywhere.
//
//  WHAT WENT WRONG
//
//  Twice, and both times the kernel's diagnostics pointed somewhere
//  else.  The first was a placement function applied to a list of
//  SECTIONS as though it were a flat list of points.  Rotating a list of
//  four points as if it were one point produces something shaped enough
//  like a point to survive, so polyhedron() received a points array a
//  quarter of the length it expected and reported "point index 6 out of
//  bounds; face dropped" six times over, followed by "closed and
//  consistently wound, but inside out" on the wreckage.  Neither message
//  is wrong and neither is near the mistake.  The file now keeps the two
//  kinds of placement visibly apart, place_ for points and placeS_ for
//  sections, and says why.
//
//  The second was the chording of the clearances above, which nothing
//  diagnosed at all and which only the model's own audit found, by
//  measuring a clearance it had designed and getting a different number
//  back.  That is the argument for building the audit before believing
//  the picture.
//
//  Total: 34 solids, 12484 triangles, 1128444.52 mm^3 predicted from the
//  closed forms and 1128444.49 measured in the exported mesh, a
//  disagreement of three parts in a hundred million, which is the f32
//  the STL is written in and nothing else.
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
T_B  = 10;                          // plinth thickness

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
// Station counts come first, because the clearances below are measured
// against the POLYGON each ring really is and not against the circle it
// approximates.  A ring's outer vertices sit exactly on its outer
// radius, but its inner surface is a ring of flats whose middles lie at
// ri cos(180/n), inside the inner radius by ri (1 - cos(180/n)).  Every
// clearance in the chain is a gap left inside an inner surface, so every
// one of them has to be taken from the chord.
function nst(ro) = 4*ceil(180/acos(1 - TOL/ro)/4);

RI_H   = RO_H - W_H;
N_H    = nst(RO_H);
IN_H   = RI_H*cos(180/N_H);
SH_H   = norm([RO_H, T_H/2]);

RO_M   = sqrt(pow(IN_H - GAP, 2) - pow(T_M/2, 2));
RI_M   = RO_M - W_M;
N_M    = nst(RO_M);
IN_M   = RI_M*cos(180/N_M);
SH_M   = norm([RO_M, T_M/2]);

// The equatorial family is five flat bands on one sphere: the equator,
// the two tropics at declination +-EPS, and the two polar circles at
// +-(90 - EPS).  The band that reaches furthest from the centre is a
// tropic, not the equator, because tipping a band towards the pole buys
// less in radius than it costs in height.  Write the family's outer
// reach at declination d and it is
//     R^2 + R (W cos d + T sin d) + (W^2 + T^2)/4,
// and among the five declinations the bracket is largest at d = EPS.
// Setting that equal to the meridian ring's inner clearance leaves a
// quadratic in R with one positive root.  The sphere's radius is
// therefore not a choice: it is the largest that puts the tropics'
// outer corners one clearance inside the meridian ring.
BQ     = W_E*cos(EPS) + T_E*sin(EPS);
CQ     = (W_E*W_E + T_E*T_E)/4;
KQ     = pow(IN_M - GAP, 2);
R_S    = (-BQ + sqrt(BQ*BQ - 4*CQ + 4*KQ))/2;

DECS   = [0, EPS, -EPS, 90 - EPS, -(90 - EPS)];
function fam_r(d)   = R_S*cos(d);
function fam_z(d)   = R_S*sin(d);
function n_fam(d)   = nst(fam_r(d) + W_E/2);
function fam_in(d)  = norm([ (fam_r(d) - W_E/2)*cos(180/n_fam(d)),
                             max(abs(fam_z(d)) - T_E/2, 0) ]);
function fam_out(d) = norm([ fam_r(d) + W_E/2, abs(fam_z(d)) + T_E/2 ]);
FIN    = min([ for (d = DECS) fam_in(d) ]);
FOUT   = max([ for (d = DECS) fam_out(d) ]);

// The ecliptic is the largest band whose outer corner clears the whole
// equatorial family by GAP.
R_C    = sqrt(pow(FIN - GAP, 2) - pow(T_C/2, 2)) - W_C/2;
N_C    = nst(R_C + W_C/2);
SH_C   = norm([R_C + W_C/2, T_C/2]);
IN_C   = (R_C - W_C/2)*cos(180/N_C);

// ---- the stand -----------------------------------------------------
Z_PL = -RO_H;                       // plinth top, one horizon radius down
R_LT = (RO_H + RI_H)/2;  Z_LT = -T_H/2 - GAP;     // leg top
R_LF = 162;              Z_LF = Z_PL + GAP;       // leg foot
A_LT = 6;   B_LT = 5.5;
A_LF = 8.5; B_LF = 9.5;
A_CT = 8.5; A_CF = 13;
Z_CT = -(RO_M + GAP);                             // column top, along -p
Z_CF = (Z_PL + GAP + A_CF*cos(LAT))/sin(LAT);     // column foot, see header
RO_B = R_LF + A_LF + 7;
RI_B = 96;

// ---- the alidade ---------------------------------------------------
AW = 3.2;  AV = 2.6;  AG = 1.0;  AD = 0.8;        // the rule's section
PW = 9.5;  PV = 11.5; PG = 2.2;  PD = 7.0;        // a pinnule
BW = 4.6;  BV = 3.8;  BG = 1.2;  BD = 1.2;        // the pivot swell
ALLOW = IN_C - GAP;
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
function placeS_sky(S, H)      = [ for (s = S) [ for (q = s) rxv(LAT-90, rzv(-90-H, q)) ] ];
function placeS_star(S, H, dc) = [ for (s = S) [ for (q = s) rxv(LAT-90, rzv(-90-H, ryv(90-dc, q))) ] ];
function shift(P, d)     = [ for (q = P) q + d ];
function skyv(H, v)      = rxv(LAT-90, rzv(-90-H, v));

// ===================================================================
//  THE PARTS
// ===================================================================
N_B = nst(RO_B);

function fam_band(d) =
  let( n = n_fam(d) )
  shift(band_pts(fam_r(d) + W_E/2, fam_r(d) - W_E/2, T_E, n), [0, 0, fam_z(d)]);

// The ecliptic carries the twelve signs, one mark every thirty degrees
// of longitude.  The two equinox marks are tall and thin, the two
// solstice marks square and lower, the other eight small, so which is
// which survives a render with no colour.  Each stands GAP clear of the
// band's face on the ecliptic's own north side.
function ecl_mark(lam, a, b, h) =
  placeS_ecl(placeS_z(
    [ rect_sec(R_C, 0, T_C/2 + GAP,     a, b),
      rect_sec(R_C, 0, T_C/2 + GAP + h, a, b) ], lam));

// The equator carries two marks, on the rays where the ecliptic crosses
// it.  Placing them in the sky(LST) frame at longitudes 0 and 180 puts
// them on the SAME two rays from the centre as the ecliptic's equinox
// fins, without either mark knowing about the other, so the two rings
// pointing at the same place is a thing the model shows and not a thing
// it claims.
function equ_mark(lam, a, b, h) =
  placeS_sky(placeS_z(
    [ rect_sec(R_S, 0, T_E/2 + GAP,     a, b),
      rect_sec(R_S, 0, T_E/2 + GAP + h, a, b) ], lam), LST);

ZNAME = ["Aries","Taurus","Gemini","Cancer","Leo","Virgo","Libra","Scorpio",
         "Sagittarius","Capricorn","Aquarius","Pisces"];
// radial half, tangential half, height
function zsize(k) = k == 0 || k == 6 ? [1.6, 3.0, 9]
                  : k == 3 || k == 9 ? [2.6, 2.6, 5]
                                     : [1.4, 2.0, 3];

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
  [ for (k = [0 : 11]) let( z = zsize(k) )
      loftpart(str("sign ", ZNAME[k], " lon ", 30*k),
               ecl_mark(30*k, z[0], z[1], z[2]), RECT_CAP, GOLD) ],
  [ loftpart("node mark on the equator, vernal",
             equ_mark(  0, 2.0, 2.4, 4.5), RECT_CAP, STEEL),
    loftpart("node mark on the equator, autumnal",
             equ_mark(180, 2.0, 2.4, 4.5), RECT_CAP, STEEL) ],
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
function dms(a)   = let( x = abs(a), m = (x - floor(x))*60 )
  str(a < 0 ? "-" : "", floor(x), "d ", floor(m), "' ",
      round((m - floor(m))*600)/10, "\"");
function hms(a)   = let( h = a/15, m = (h - floor(h))*60 )
  str(floor(h), "h ", floor(m), "m ", round((m - floor(m))*600)/10, "s");

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
// Where the equator meets the horizon, from the two planes' poles and
// nothing else.  It has to be the east and west points, at altitude
// zero, at every latitude, because e2 = p x e1 came out as due east.
EW = unitv(cross(NCP, [0,0,1]));
echo("FRAME      equator meets horizon at alt", altof(EW), " azimuths",
     azof(EW), "and", azof(-EW),
     " ; e1, the equator on the meridian, is", skyv(0, [1,0,0]),
     "and p x e1 is", cross(NCP, skyv(0, [1,0,0])), "which is due east");

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
CIDX  = [ for (i = [0 : N_C-1]) if (abs(ECL_D[i]) <= DMIN) i ];
CW    = [ for (i = CIDX) rxv(LAT-90, rzv(-90-LST, ECL_C[i])) ];
echo("EQUINOX    read off the ring: altitudes", [ for (v = CW) altof(v) ],
     " azimuths", [ for (v = CW) azof(v) ],
     " ; the two are", acos(max(-1, min(1, unitv(CW[0])*unitv(CW[1])))), "degrees apart");

// ---- the nesting ---------------------------------------------------
echo("NESTING    horizon  Ro", RO_H, "Ri", RI_H, "t", T_H,
     "-> reaches [", IN_H, ",", SH_H, "] from the centre");
echo("NESTING    meridian Ro", RO_M, "Ri", RI_M, "t", T_M,
     "-> reaches [", IN_M, ",", SH_M, "]");
echo("NESTING    equatorial sphere R", R_S, "-> family reaches [", FIN, ",", FOUT,
     "], the binding band being a tropic at declination", EPS);
echo("NESTING    ecliptic sphere R", R_C, "-> reaches [", IN_C, ",", SH_C, "]");
echo("NESTING    the chain of clearances", IN_H - SH_M, IN_M - FOUT, FIN - SH_C,
     IN_C - PARTS[len(PARTS)-1][5], " all of which should be", GAP);

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

// Every vertex of the ecliptic band, and every midpoint of the edges
// that run along it, carried into equatorial coordinates.  The band's
// four section vertices ARE its four corner curves, so this samples the
// whole of the surface that can come nearest a ring of the family.
ECL_P = band_pts(R_C + W_C/2, R_C - W_C/2, T_C, N_C);
ECL_V = concat(
  [ for (q = ECL_P) rxv(EPS, q) ],
  [ for (i = [0 : N_C-1]) for (k = [0 : 3])
      rxv(EPS, (ECL_P[4*i + k] + ECL_P[4*((i+1)%N_C) + k])/2) ]);

function ecl_gap(d) =
  let( n = n_fam(d) )
  min([ for (q = ECL_V)
          ann_d(norm([q[0], q[1]]), q[2],
                (fam_r(d) - W_E/2)*cos(180/n), fam_r(d) + W_E/2, fam_z(d), T_E) ]);

echo("TANGENCY   ecliptic to equator", ecl_gap(0),
     " to tropic of Cancer", ecl_gap(EPS), " to tropic of Capricorn", ecl_gap(-EPS));
echo("TANGENCY   ecliptic to arctic circle", ecl_gap(90-EPS),
     " to antarctic circle", ecl_gap(-(90-EPS)), " ; the guarantee was", FIN - SH_C);
// The tropics are named after the signs the sun stands in when it
// reaches them, and the model can show that rather than repeat it: the
// ecliptic's extreme declinations fall at longitudes 90 and 270, which
// are the first points of Cancer and Capricorn, and those declinations
// are the declinations of the two tropic bands.
echo("TROPICS    the ecliptic is extreme at longitudes", SOLST,
     "which are the first points of", ZNAME[3], "and", ZNAME[9],
     "; its declinations there are", DMAX, "and", -DMAX,
     "and the two tropic bands sit at", EPS, "and", -EPS);

// ---- no two of the solids share a cubic millimetre -----------------
// A separating-axis certificate over every pair.  A pair is apart if the
// shells they occupy, measured from the centre of the instrument, do not
// overlap, or if their extents along any one of six directions do not:
// the three world axes, the polar axis, the pole of the ecliptic, and
// the line joining the two solids' own centroids.  The world axes and
// the centroid line do most of the work.  The polar axis is the one that
// cannot be left out, because the five equatorial bands lie in the same
// spherical shell and in overlapping boxes, and what separates them is
// declination, which is exactly a slab test along the polar axis.
//
// The bands' inner reach is the CHORD of the inner polygon and not its
// circumradius, because the closest point of an n-gon band to the centre
// is the middle of a flat.  Getting that wrong overstates every
// clearance here by about fifteen microns, four percent of the clearance
// being claimed, and that is how it was found.
AXES  = [ [1,0,0], [0,1,0], [0,0,1], NCP, ECP ];
SPANS = [ for (P = PARTS) [ for (u = AXES)
            let( d = [ for (q = P[1]) q*u ] ) [ min(d), max(d) ] ] ];
SHELL = [ for (P = PARTS) [ P[4], P[5] ] ];
CENT  = [ for (P = PARTS) sum(P[1])/len(P[1]) ];
NP    = len(PARTS);
function axsep(i, j, u) =
  let( a = [ for (q = PARTS[i][1]) q*u ], b = [ for (q = PARTS[j][1]) q*u ] )
    max(min(a) - max(b), min(b) - max(a));
function sep(i, j) = max(concat(
  [ SHELL[i][0] - SHELL[j][1], SHELL[j][0] - SHELL[i][1] ],
  [ for (k = [0 : len(AXES)-1])
      max(SPANS[i][k][0] - SPANS[j][k][1], SPANS[j][k][0] - SPANS[i][k][1]) ],
  [ axsep(i, j, unitv(CENT[j] - CENT[i])) ] ));
SEPS  = [ for (i = [0 : NP-2]) for (j = [i+1 : NP-1]) [i, j, sep(i, j)] ];
SMIN  = min([ for (e = SEPS) e[2] ]);
NGOOD = len([ for (e = SEPS) if (e[2] > 0) 1 ]);
echo(str("DISJOINT   ", len(SEPS), " pairs, ", NGOOD, " certified apart, tightest ",
         SMIN, " mm", NGOOD == len(SEPS) ? "" : "   *** SOME PAIR MAY OVERLAP ***"));
echo("DISJOINT   the tightest pairs are",
     [ for (e = SEPS) if (e[2] < SMIN + 1e-9) str(PARTS[e[0]][0], " | ", PARTS[e[1]][0]) ]);

// The two rings really do point at the same place.  Each node mark's
// centroid is compared with the equinox fin's on the other ring; the
// small residue is the stand-off, which is along each band's own normal
// and so along two different directions.
NA = len(PARTS);
function findpart(t) = [ for (i = [0 : NA-1]) if (PARTS[i][0] == t) i ][0];
IV = findpart("sign Aries lon 0");
IL = findpart("sign Libra lon 180");
JV = findpart("node mark on the equator, vernal");
JL = findpart("node mark on the equator, autumnal");
// Each mark stands off its own band along that band's normal, and the
// two normals are the obliquity apart, so comparing mark centroids
// directly compares two points that are deliberately off the ray.  Drop
// each centroid back into its own band's plane and what is left is the
// ray the band is pointing along.
function inplane(v, n) = unitv(v - n*(v*n));
echo("CROSSING   vernal node: the ecliptic's Aries mark lies over a point of its band",
     "at alt", altof(inplane(CENT[IV], ECP)), "az", azof(inplane(CENT[IV], ECP)),
     "and the equator's mark over a point at alt", altof(inplane(CENT[JV], NCP)),
     "az", azof(inplane(CENT[JV], NCP)));
echo("CROSSING   vernal node: those two rays differ by",
     acos(max(-1, min(1, inplane(CENT[IV], ECP)*inplane(CENT[JV], NCP)))),
     "degrees, and each differs from the node computed from the two ring poles by",
     acos(max(-1, min(1, inplane(CENT[IV], ECP)*NODE))),
     acos(max(-1, min(1, inplane(CENT[JV], NCP)*NODE))));
echo("CROSSING   autumnal node: rays at alt", altof(inplane(CENT[IL], ECP)),
     "and", altof(inplane(CENT[JL], NCP)), ", differing by",
     acos(max(-1, min(1, inplane(CENT[IL], ECP)*inplane(CENT[JL], NCP)))), "degrees");
echo("CROSSING   the marks themselves sit",
     acos(unitv(CENT[IV])*inplane(CENT[IV], ECP)), "and",
     acos(unitv(CENT[JV])*inplane(CENT[JV], NCP)),
     "degrees off their rays, which is the stand-off and nothing else");

// ---- volume, two routes in here and a third in the export ----------
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

echo("VOLUME     mesh sum over every triangle", VTOT, "mm3, of which the nine",
     "bands are", sum([ for (i = [0:8]) VOLS[i] ]), ", the stand",
     sum([ for (i = [9:14]) VOLS[i] ]), ", the alidade", VOLS[len(VOLS)-1],
     "and the", len(VOLS) - 16, "marks", sum([ for (i = [15:len(VOLS)-2]) VOLS[i] ]));
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
echo("ALIDADE    laid on Arcturus, RA", hms(RA_A), " dec", dms(DEC_A),
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

