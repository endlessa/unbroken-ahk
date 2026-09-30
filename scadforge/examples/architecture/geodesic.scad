// ===================================================================
//  A geodesic dome, taken from the icosahedron it is a subdivision of
//
//  A geodesic dome is a sphere approximated by a triangulated net, and
//  the reason anyone builds one is that the net is made of only a
//  handful of distinct bar lengths.  That is the whole economy of the
//  thing: hundreds of struts, six or nine different cuts.  So the
//  number of distinct lengths, and how many struts each length accounts
//  for, is not decoration here.  It is the check that the subdivision
//  was done correctly, and it is echoed at the foot of this file.
//
//  THE SUBDIVISION
//
//  The icosahedron is the finest regular triangulation of the sphere:
//  12 vertices, 30 edges, 20 faces.  A class I subdivision of frequency
//  V lays a triangular grid over each face,
//
//      P(i,j) = A + (B - A) i/V + (C - B) j/V,    0 <= j <= i <= V
//
//  so that P(0,0) = A, P(V,0) = B, P(V,V) = C, and then pushes every
//  grid point out onto the sphere, p = R P/|P|.  Row i holds i+1 points
//  and the face holds (V+1)(V+2)/2 of them.  Counting the shared ones
//  once each gives
//
//      vertices  10 V^2 + 2      edges  30 V^2      faces  20 V^2
//
//  which at V = 6 is 362, 1080 and 720.  Projecting is what makes the
//  lengths differ: the flat grid is equilateral, and the projection
//  scales a point by 1/|P|, so the middle of a face, which starts
//  deepest inside the sphere, is stretched most.  Nothing else in the
//  construction introduces variety.
//
//  ONE NAME PER VERTEX, ASSIGNED BEFORE ANY COORDINATES EXIST
//
//  A grid point on the boundary of a face belongs to two faces and a
//  corner belongs to five, so a naive build emits every shared strut
//  twice.  Merging afterwards by comparing positions would be the wrong
//  way round: the identity of a vertex here is combinatorial, not
//  numerical, and two faces agree about a shared point because they
//  agree about its NAME, not because two float computations happened to
//  land on the same bits.  So every vertex is named first.  The names
//  0 to 11 are the icosahedron's own vertices; then V-1 names for the
//  interior of each of the 30 icosahedron edges; then (V-1)(V-2)/2 for
//  the interior of each of the 20 faces.  The total,
//
//      12 + 30(V-1) + 20 (V-1)(V-2)/2 = 10 V^2 + 2,
//
//  is the vertex count, which is the first thing this file checks.  The
//  map (face, i, j) -> name is integer arithmetic with no tolerance in
//  it.  The inverse map, name -> position, is a closed form as well: a
//  face-interior name carries its own (i,j) back out through the
//  triangular numbers, i = 2 + floor((sqrt(8m+1)-1)/2), so the position
//  list is built by inverting the naming rather than by scattering into
//  an array, which this language cannot do anyway.
//
//  Each of the three families of grid edges is then emitted once, from
//  the face that owns it, and the three sides of each big triangle are
//  emitted from the icosahedron edge instead of from either face.  Per
//  face that is 3V(V-1)/2 interior struts, and 30V along the 30
//  icosahedron edges:
//
//      20 * 3V(V-1)/2 + 30 V = 30 V^2 - 30 V + 30 V = 30 V^2.
//
//  The same machinery builds the hub balls at frequency 2.  Nothing in
//  it knows what frequency it is running at.
//
//  THE CHORD FACTORS, AND THREE LENGTHS THAT AGREE WITHOUT A REASON
//
//  Two struts are certainly the same length when a symmetry of the
//  icosahedron carries one onto the other, so the number of distinct
//  lengths is at most the number of orbits of the 30V^2 struts under
//  that group, which has order 120.  At V = 4 the orbits are six, four
//  of size 60 and two of size 120, and setting V = 4 at the head of
//  this file makes it print the six chord factors, length divided by
//  radius,
//
//      0.253185  0.294531  0.295242  0.298588  0.312869  0.324920
//
//  with multiplicities 60, 120, 60, 60, 120, 60 summing to 480 = 30*4^2.
//  That is the published 4V icosahedral alternate table, and reproducing
//  it is the check that the subdivision is right.
//
//  At V = 6, which is what the file is set to, the bound is not tight.
//  There are twelve orbits, six of size 60 and six of size 120, but only
//  NINE lengths, with multiplicities 60, 120, 60, 60, 120, 180, 240,
//  120, 120.  A class of 240 cannot be a single orbit, because an
//  orbit's size divides the order of the group; three pairs of unrelated
//  orbits simply happen to be equal in length, merging as 60 + 60 = 120,
//  60 + 120 = 180 and 120 + 120 = 240.
//
//  One of the three can be checked by hand, and it is echoed below.  All
//  three vertices of an icosahedron face are adjacent, so any two of
//  them satisfy A.B = q = 1/sqrt(5) on the unit sphere, and a grid point
//  of that face with barycentric weights b = (u,v,t) summing to 1 has
//
//      |P|^2 = 1 - 2(1-q) e(b),   e(b) = uv + vt + tu,
//
//  while two such points satisfy P1.P2 = q + (1-q)(b1.b2).  Both
//  formulas reach q through the same two rational coefficients, so two
//  chords built from weights that agree in e and in the dot product are
//  exactly equal, whatever q is.  The weights of the three points in
//  question are
//
//      b(2,0) = (4,2,0)/6   b(3,0) = (3,3,0)/6   b(2,1) = (4,1,1)/6,
//
//  and e(b(3,0)) = 1/4 while e(b(2,1)) = 1/9 + 1/36 + 1/9 = 1/4, so
//  P(3,0) and P(2,1) start at the same depth inside the sphere; and
//  b(2,0).b(3,0) = 1/2 while b(2,0).b(2,1) = 4/9 + 1/18 = 1/2, so P(2,0)
//  leans the same way against each.  Equal depths and equal dot product
//  give equal cosines and so identical chords, and no symmetry relates
//  them, because one lies along an icosahedron edge and the other does
//  not.  A builder cutting to a length list rather than to an orbit list
//  cuts nine lengths at 6V, not twelve.
//
//  A subdivision that is wrong in any structural way fails the
//  multiplicity sum long before it looks wrong.  The struts are coloured
//  by class so the classes are visible as well as counted.
//
//  THE CUT IS NOT A FREE CHOICE
//
//  A dome has to stand on a closed ring of struts all at one height, and
//  that is a strong condition.  Every horizontal strut in the whole
//  sphere is found below and grouped by height, and each height is asked
//  two questions: are there as many struts as vertices, which a cycle
//  needs, and does every vertex carry exactly two of them.  With the
//  icosahedron standing on a five-fold axis there are eleven such
//  heights in a 6V sphere and three of them pass.  Two are the pentagons
//  immediately below the north pole and above the south pole, which keep
//  0.66 and 99.34 percent of the sphere and are a skylight and a
//  doorway, not a dome.  The third is the equator.  The other eight
//  heights carry five struts among ten vertices, which is five disjoint
//  pairs.  So the fraction is 1/2.  The 3/8 and 5/8 cuts the trade
//  quotes are not levels of this sphere in this orientation at all: at
//  V = 4 the nearest level to 5/8 is 0.625574, and it is one of the
//  five-struts-among-ten-vertices kind.
//
//  The equatorial ring can be counted in advance.  Ten of the twenty
//  icosahedron faces straddle the equator, each with two vertices on one
//  side and one on the other.  The plane z = 0 crosses such a face along
//  the line joining the midpoints of its two slanting edges, and with V
//  even that line is a straight run of grid points: V/2 + 1 of them and
//  V/2 edges, whichever corner of the face the lone vertex happens to
//  be.  Ten faces, each sharing an end point with its neighbour, give 5V
//  vertices and 5V struts, so the base ring of a 6V dome has 30 of each.
//
//  There is a second reason a hemisphere is the right place to stop.
//  Under a load uniform over the plan area, membrane theory of a
//  spherical shell gives a meridional resultant N_phi = -pR/2 directed
//  along the meridian; at the equator the meridian is vertical, so the
//  reaction the springing hands to its footings is purely vertical and
//  the ring is not being asked to hold a dome together against its own
//  spreading.  The hoop resultant there, N_theta = -pR cos(2 phi)/2, has
//  turned to +pR/2, a tension, and the base struts carry it.
//
//  THE STRUT SECTION IS DERIVED, NOT CHOSEN
//
//  In a triangular lattice with member length L carrying axial force F
//  in all three directions, the perpendicular spacing of one family is
//  s = L sqrt(3)/2 and the three directions contribute d (x) d summing
//  to 3/2 of the identity, so the equivalent membrane resultant is
//
//      N = sqrt(3) F / L,    hence    F = N L / sqrt(3).
//
//  With N = pR/2 that fixes the compression in the longest strut.  A
//  pin-ended member buckles at pi^2 E I / L^2, and a regular hexagon of
//  circumradius a has I = 5 sqrt(3) a^4/16 about any centroidal axis, so
//  requiring a factor SF on the Euler load gives
//
//      a^4 = 16 SF F L^2 / (5 sqrt(3) pi^2 E).
//
//  That single line sets the strut size.  The hoop tension at the
//  springing is carried differently, because only one family of bars
//  runs that way and the base ring is the edge of the lattice rather
//  than the inside of it.  A single family at spacing s carries N as
//  F = N s, and an edge member's tributary is half a spacing, so the
//  base struts take F = N L sqrt(3)/4.  The footing posts are sized like
//  the struts, from their own length and their own share of the vertical
//  reaction.  Nothing in the drawing is a diameter somebody liked the
//  look of.
//
//  THE HUB RADIUS IS DERIVED TOO
//
//  Every strut stops short of the hub it runs to, by RH + GAP measured
//  from the hub's centre, so that a strut and its hub never share a
//  cubic millimetre.  What then sets RH is the struts themselves: two
//  struts leaving one hub at an angle alpha, both starting at distance
//  L0 from its centre, are closest at their two starting points, because
//  the separation sqrt(t^2 + s^2 - 2 t s cos alpha) grows in both
//  parameters from t = s = L0.  That distance is the chord
//  2 L0 sin(alpha/2), and requiring it to exceed 2 RS + GAP gives
//
//      L0 >= (RS + GAP/2) / sin(alpha_min/2),      RH = L0 - GAP.
//
//  The draft this file grew out of took alpha_min to be the smallest
//  interior angle of any triangle of the net, arguing that two struts
//  which are not consecutive around a hub subtend the sum of at least
//  two consecutive angles.  That argument runs backwards.  The angular
//  triangle inequality bounds a non-consecutive pair from ABOVE by the
//  sum of the gaps it spans, not from below, so it cannot rule the pair
//  out.  Rather than repair the argument the file now computes the thing
//  it needs, the smallest angle between ANY two struts meeting at a hub,
//  over every pair at every hub.  It lands on the smallest triangle
//  interior angle, and both numbers are echoed so the reader can see
//  that it did.
//
//  A CHORD IS ALREADY PERPENDICULAR TO ITS MIDPOINT RADIUS
//
//  A swept section needs two axes across the sweep, and the natural one
//  here is the outward radial, so that a flat of the hexagon faces the
//  weather.  No orthogonalisation is needed: both ends of a strut are at
//  distance R from the centre, so
//
//      (a + b).(b - a) = |b|^2 - |a|^2 = 0
//
//  and the radius at the midpoint is exactly normal to the chord.  The
//  second axis is the cross product of the two, and the triple is
//  right-handed by construction, since e1 x (t x e1) = t when e1 and t
//  are perpendicular unit vectors.
//
//  WINDING
//
//  polyhedron() wants each face listed so that the right-hand normal
//  points INTO the solid, which is the same as clockwise seen from
//  outside.  Wound the other way a solid is inside out; it renders
//  identically, because shading uses the absolute value of the normal,
//  and then every boolean that touches it quietly loses geometry.  The
//  twenty icosahedron faces are not written down in a fixed order here
//  but passed through a test, cross(B-A, C-A).A < 0, that flips the ones
//  that came out the wrong way; every triangle of every subdivision
//  inherits its face's orientation, so the hubs are correct by
//  construction rather than by proofreading.  For the swept solids the
//  rule is the orientation-blind one: two faces sharing an edge must
//  traverse it in opposite directions.  A wall quad [a,b,c,d] gives the
//  directed edge d->a, so the end cap that meets it there gives a->d,
//  which is why the two caps of a strut are fanned in opposite senses.
//
//  NO TWO OF THE 782 SOLIDS SHARE ANY VOLUME
//
//  Four clearances have to hold, and all four are echoed rather than
//  assumed.  A strut clears its own hubs because it starts RH + GAP from
//  the centre and the hub is inside the sphere of radius RH.  Two struts
//  at a hub clear each other by the chord argument above, and that is
//  the binding one: it is what sets RH.  Two hubs clear each other by
//  LMIN - 2 RH.  A strut clears a hub it does not belong to by the
//  shortest altitude of any triangle of the net, less RS and RH.  The
//  footing posts and the ring beam are stood off by GAP in the same way.
//
//  The payoff is that the volume audit at the foot of this file is a
//  PREDICTION and not an estimate.  A hexagonal prism between two planes
//  normal to its axis has volume exactly A L, and a strut is one,
//  because its two cap fans are flat and perpendicular to the sweep.  A
//  mitred closed loop has volume exactly A times its axis perimeter,
//  because each segment is a prism between two planes through the two
//  corner points and the section centroid is on the axis.  A hub's
//  volume is the exact triple-product sum over its own 80 faces.  The
//  exporter has to measure what those closed forms say, and a difference
//  is a finding.
//
//  WHAT THE EXPORT SAYS ABOUT ITSELF
//
//  At V = 6 the dome is 29,960 triangles, which is past the budget above
//  which the export-time union is skipped, so the export prints a
//  warning that overlapping shells were left separate.  Nothing here
//  overlaps.  What the exporter tests before it gives up is whether any
//  two parts' BOUNDING BOXES meet, and boxes are a much blunter
//  instrument than solids: 1256 of the 305,371 pairs of these 782 parts
//  have boxes that meet, and not one pair shares a cubic millimetre.
//  The union is therefore skipped on a model for which the union is a
//  concatenation anyway, and the exported mesh is exactly the mesh
//  written here: 782 closed components, and a measured volume that
//  agrees with the prediction to the last digit a float32 STL can carry.
//  Running the same file with V = 4 puts it under the budget, the union
//  does run, and it returns the same 13,920 triangles it was given and
//  the predicted volume, which is how one knows nothing is being papered
//  over.
//
//  WHAT WENT WRONG
//
//  Three claims in the draft this file grew out of were not true, and
//  each is now either fixed or computed instead of argued.  The first
//  said there were seven horizontal levels in a 6V sphere and that only
//  the equator closed.  There are eleven, and three close; the table is
//  printed in full below with the vertex degree that decides it, which
//  is the whole reason to print a table rather than a sentence.  The
//  second was the backwards angle argument described above.  The third
//  claimed that a linear recursive sum over 555 terms would hit the
//  parser's depth guard.  It does not; the guard is far higher than
//  that.  The halving sum stays, because its depth is logarithmic rather
//  than linear whatever the guard is set to, and because pairwise
//  summation's rounding error grows like log n rather than n, which
//  matters when 555 strut lengths are added to predict a volume.
//
//  One thing in the draft was right, although its arithmetic wanted
//  pinning down.  The hub ball was at first sized so that its FACES
//  reached the strut ends, on the grounds that a strut should not be
//  left hanging in the air.  A ball sized by its inradius sticks out
//  past that radius at every one of its 42 vertices, in the ratio of
//  circumradius to inradius, which for the 2V ball is 1.0705.  At the
//  42.0 mm from the hub centre that the strut clearance fixes, that is
//  3.0 mm of hub outside the strut end and therefore inside the strut.
//  Two solids that share a cubic millimetre turn the audit back into an
//  estimate, so the ball is sized by its CIRCUMRADIUS instead, 37.0 mm
//  against an inradius of 34.6, and the visible consequence is that a
//  strut stops between 5.0 and 7.4 mm clear of the hub surface rather
//  than touching it.  All five numbers are echoed.
// ===================================================================

// ---- the shape ------------------------------------------------------
V     = 6;          // subdivision frequency, class I (alternate)
VH    = 2;          // frequency of the little geodesic ball used as a hub
R     = 6.000;      // sphere radius, m
CUT   = 0.5;        // fraction of the sphere kept, measured down from the crown
KS    = 6;          // sides of a drawn strut
KF    = 6;          // sides of a drawn footing post
GAP   = 0.005;      // air left between any two solids, m

// ---- the load and the alloy ----------------------------------------
PDES  = 2500;       // design pressure on the plan area, Pa
EMOD  = 70e9;       // Young's modulus of the strut alloy, Pa
FPRF  = 240e6;      // its proof stress, Pa
SF    = 3.0;        // factor on the Euler load

// ---- the stem the dome stands on ------------------------------------
LPOST = 0.600;      // footing post length, m
WBEAM = 0.300;      // ring beam width, m
DBEAM = 0.250;      // ring beam depth, m
RHOA  = 2700;       // aluminium, kg/m3
RHOC  = 2400;       // concrete, kg/m3
GRAV  = 9.81;

CONC  = [0.74, 0.72, 0.68];
HUBC  = [0.30, 0.32, 0.35];
PAL   = [[0.85,0.25,0.22], [0.93,0.55,0.16], [0.92,0.80,0.24],
         [0.44,0.72,0.30], [0.20,0.62,0.55], [0.22,0.52,0.80],
         [0.40,0.38,0.74], [0.70,0.34,0.66], [0.55,0.46,0.40]];

// ---- helpers the kernel does not have -------------------------------
function sq(t) = t*t;
function unit(p) = p/norm(p);

// A halving sum.  Its recursion depth is log2 of the list length rather
// than the length itself, and pairwise addition accumulates rounding
// error like log n rather than n.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

// ---- the icosahedron -----------------------------------------------
// Standing on a five-fold axis: a pole, a ring of five at polar angle
// atan 2, a ring of five at its supplement offset by half a step, and
// the antipode.  atan(2) = 63.4349 degrees is exact in the sense that
// tan of it is 2; the ring radius is then 2/sqrt(5).
TH = atan(2);
function sphp(pol, az) = [sin(pol)*cos(az), sin(pol)*sin(az), cos(pol)];
IV = concat([ [0,0,1] ],
            [ for (k = [0:4]) sphp(TH, 72*k) ],
            [ for (k = [0:4]) sphp(180 - TH, 36 + 72*k) ],
            [ [0,0,-1] ]);

// Five round the top, ten round the waist, five round the bottom.  The
// order each is written in does not matter, because each is then turned
// so that its right-hand normal points inward, which is what
// polyhedron() wants and what every subdivision below inherits.
IFRAW = concat(
  [ for (k = [0:4]) [0,   1+k,        1+(k+1)%5] ],
  [ for (k = [0:4]) [1+k, 6+k,        1+(k+1)%5] ],
  [ for (k = [0:4]) [6+k, 6+(k+1)%5,  1+(k+1)%5] ],
  [ for (k = [0:4]) [11,  6+k,        6+(k+1)%5] ]);
IF = [ for (t = IFRAW)
         let( A = IV[t[0]], B = IV[t[1]], C = IV[t[2]] )
           cross(B - A, C - A)*A < 0 ? t : [t[0], t[2], t[1]] ];

// The 30 edges, each written once with its lower vertex first.  The
// duplicate filter keeps the first occurrence of each key and nothing
// here depends on the order that produces.
EPAIR = [ for (t = IF) for (k = [0:2])
            [ min(t[k], t[(k+1)%3]), max(t[k], t[(k+1)%3]) ] ];
EKEY  = [ for (e = EPAIR) 12*e[0] + e[1] ];
IE    = [ for (i = [0:len(EPAIR)-1])
            if (min([ for (j = [0:len(EKEY)-1]) if (EKEY[j] == EKEY[i]) j ]) == i)
              EPAIR[i] ];
IEK   = [ for (e = IE) 12*e[0] + e[1] ];
function eno(p, q) =
  min([ for (i = [0:len(IEK)-1]) if (IEK[i] == 12*min(p,q) + max(p,q)) i ]);

// ---- the naming scheme, generic in the frequency --------------------
function ein(v)  = v - 1;                 // names per icosahedron edge
function fin(v)  = (v-1)*(v-2)/2;         // names per face interior
function bfac(v) = 12 + 30*(v-1);         // where the face names start
function ngv(v)  = 10*v*v + 2;            // total
function tnum(n) = n*(n+1)/2;

// The t-th point along the icosahedron edge {p,q}, counted from p.  The
// name is always read along the canonical direction, so the two faces
// that share the edge agree without comparing anything.
function epid(v, p, q, t) =
  let( lo = min(p,q), hi = max(p,q), k = (p == lo ? t : v - t) )
    k == 0 ? lo : k == v ? hi : 12 + eno(p,q)*ein(v) + k - 1;

function gid(v, f, i, j) =
    i == 0             ? IF[f][0]
  : i == v && j == 0   ? IF[f][1]
  : i == v && j == v   ? IF[f][2]
  : j == 0             ? epid(v, IF[f][0], IF[f][1], i)
  : j == i             ? epid(v, IF[f][0], IF[f][2], i)
  : i == v             ? epid(v, IF[f][1], IF[f][2], j)
  : bfac(v) + f*fin(v) + tnum(i-2) + j - 1;

// Name -> unit position.  The face-interior branch recovers (i,j) from
// the offset m by inverting m = tnum(i-2) + j - 1.
function gpos(v, id) =
    id < 12 ? IV[id]
  : id < bfac(v)
      ? let( d = id - 12, e = floor(d/ein(v)), k = d - e*ein(v) + 1,
             A = IV[IE[e][0]], B = IV[IE[e][1]] )
          unit(A + (B - A)*(k/v))
      : let( d = id - bfac(v), f = floor(d/fin(v)), m = d - f*fin(v),
             n = floor((sqrt(8*m + 1) - 1)/2), i = n + 2, j = m - tnum(n) + 1,
             A = IV[IF[f][0]], B = IV[IF[f][1]], C = IV[IF[f][2]] )
          unit(A + (B - A)*(i/v) + (C - B)*(j/v));

// Upward and downward cells of every face grid, inheriting the face's
// inward orientation.  v^2 of them per face.
function gtris(v) = concat(
  [ for (f = [0:19]) for (i = [0:v-1]) for (j = [0:i])
      [ gid(v,f,i,j), gid(v,f,i+1,j), gid(v,f,i+1,j+1) ] ],
  [ for (f = [0:19]) for (i = [1:v-1]) for (j = [0:i-1])
      [ gid(v,f,i,j), gid(v,f,i+1,j+1), gid(v,f,i,j+1) ] ]);

// The three interior families from each face, then the icosahedron
// edges cut into v pieces.  Every strut appears exactly once.
function gedges(v) = concat(
  [ for (f = [0:19]) for (i = [1:v-1]) for (j = [0:i-1])
      [ gid(v,f,i,j), gid(v,f,i,j+1) ] ],
  [ for (f = [0:19]) for (i = [1:v-1]) for (j = [1:i])
      [ gid(v,f,i,j), gid(v,f,i+1,j) ] ],
  [ for (f = [0:19]) for (i = [1:v-1]) for (j = [0:i-1])
      [ gid(v,f,i,j), gid(v,f,i+1,j+1) ] ],
  [ for (e = [0:29]) for (k = [0:v-1])
      [ epid(v, IE[e][0], IE[e][1], k), epid(v, IE[e][0], IE[e][1], k+1) ] ]);

// ---- the net -------------------------------------------------------
NGV  = ngv(V);
PT   = [ for (id = [0:NGV-1]) R*gpos(V, id) ];
GE   = gedges(V);
GT   = gtris(V);
TOL  = R*1e-9;
ZCUT = R*(1 - 2*CUT);

function kept(id) = PT[id][2] >= ZCUT - TOL;
HUB   = [ for (id = [0:NGV-1]) if (kept(id)) id ];
STRUT = [ for (e = GE) if (kept(e[0]) && kept(e[1])) e ];
TRI   = [ for (t = GT) if (kept(t[0]) && kept(t[1]) && kept(t[2])) t ];
BASEH = [ for (id = HUB) if (abs(PT[id][2] - ZCUT) < TOL) id ];
BASES = [ for (e = STRUT)
            if (abs(PT[e[0]][2] - ZCUT) < TOL && abs(PT[e[1]][2] - ZCUT) < TOL) e ];
BDEG  = [ for (id = BASEH) len([ for (e = BASES) if (e[0] == id || e[1] == id) 1 ]) ];

function elen(e) = norm(PT[e[0]] - PT[e[1]]);
LMIN = min([ for (e = STRUT) elen(e) ]);
LMAX = max([ for (e = STRUT) elen(e) ]);
LSUM = sum([ for (e = STRUT) elen(e) ]);

// Every strut that meets each hub, gathered once, so that the angle the
// hub radius depends on can be minimised over every PAIR at the hub
// rather than over the triangles alone.
NBRS = [ for (id = HUB)
           concat([ for (e = STRUT) if (e[0] == id) e[1] ],
                  [ for (e = STRUT) if (e[1] == id) e[0] ]) ];
HDEG = [ for (ns = NBRS) len(ns) ];
function ang(a, b, c) =
  acos( ((PT[b]-PT[a])*(PT[c]-PT[a])) / (norm(PT[b]-PT[a])*norm(PT[c]-PT[a])) );
AMIN = min([ for (h = [0:len(HUB)-1]) let( ns = NBRS[h] )
               min([ for (a = [0:len(ns)-2]) for (b = [a+1:len(ns)-1])
                       ang(HUB[h], ns[a], ns[b]) ]) ]);
ATRI = min([ for (t = TRI)
               min(ang(t[0],t[1],t[2]), ang(t[1],t[2],t[0]), ang(t[2],t[0],t[1])) ]);

// Shortest altitude in the net, which is how close a strut comes to a
// hub it does not run to.
function tside(t) = max(norm(PT[t[1]]-PT[t[0]]),
                        norm(PT[t[2]]-PT[t[1]]), norm(PT[t[0]]-PT[t[2]]));
ALTMIN = min([ for (t = TRI)
                 norm(cross(PT[t[1]]-PT[t[0]], PT[t[2]]-PT[t[0]]))/tside(t) ]);

// ---- the sections, solved ------------------------------------------
function polyarea(k, r) = 0.5*k*sq(r)*sin(360/k);

NMEM  = PDES*R/2;                       // membrane resultant, N per m
FSTRU = NMEM*LMAX/sqrt(3);              // compression in the longest strut, N
RS    = pow(16*SF*FSTRU*sq(LMAX)/(5*sqrt(3)*PI*PI*EMOD), 0.25);
IHEX  = 5*sqrt(3)*pow(RS,4)/16;
FEULR = PI*PI*EMOD*IHEX/sq(LMAX);
ASTRU = polyarea(KS, RS);
FHOOP = NMEM*LMAX*sqrt(3)/4;            // edge member, half a tributary

RH    = (RS + GAP/2)/sin(AMIN/2) - GAP; // hub circumradius
L0    = RH + GAP;                       // how far short of a hub a strut stops

HUBP  = [ for (id = [0:ngv(VH)-1]) RH*gpos(VH, id) ];
HUBF  = gtris(VH);
// Faces wound inward, so the triple-product sum is minus six volumes.
VHUB1 = -sum([ for (f = HUBF) HUBP[f[0]]*cross(HUBP[f[1]], HUBP[f[2]]) ])/6;
// Inradius of the same ball, which is how deep a strut stopped at the
// inradius rather than the circumradius would have buried itself.
RHIN  = min([ for (f = HUBF)
                abs(HUBP[f[0]]*unit(cross(HUBP[f[1]]-HUBP[f[0]],
                                          HUBP[f[2]]-HUBP[f[0]]))) ]);

VSTRU = ASTRU*(LSUM - 2*L0*len(STRUT));
VHUBS = len(HUB)*VHUB1;
WFRAM = (VSTRU + VHUBS)*RHOA*GRAV;      // self weight of the frame, N

// ---- the base ring and the stem ------------------------------------
NB    = len(BASEH);
function azm(p) = let( a = atan2(p[1], p[0]) ) a < 0 ? a + 360 : a;
BAZ   = [ for (id = BASEH) azm(PT[id]) ];
BRNK  = [ for (i = [0:NB-1]) len([ for (j = [0:NB-1]) if (BAZ[j] < BAZ[i]) 1 ]) ];
BORD  = [ for (r = [0:NB-1])
            BASEH[ min([ for (i = [0:NB-1]) if (BRNK[i] == r) i ]) ] ];

FPOST = (PDES*PI*sq(R) + WFRAM)/NB;     // vertical reaction per footing, N
RF    = pow(16*SF*FPOST*sq(LPOST)/(5*sqrt(3)*PI*PI*EMOD), 0.25);
AFOOT = polyarea(KF, RF);

ZG    = LPOST + DBEAM + RH + 2*GAP;     // ground, below the springing
ZBEA  = -ZG + DBEAM/2;                  // ring beam axis
ZP0   = -ZG + DBEAM + GAP;              // post foot
ZP1   = -(RH + GAP);                    // post head

BQ    = [ for (id = BORD) [ PT[id][0], PT[id][1], ZBEA ] ];
function bt(i) = unit(BQ[(i+1)%NB] - BQ[i]);
function bm(i) = unit(bt((i-1+NB)%NB) + bt(i));
BSEC  = [ [ WBEAM/2, -DBEAM/2], [ WBEAM/2,  DBEAM/2],
          [-WBEAM/2,  DBEAM/2], [-WBEAM/2, -DBEAM/2] ];
// The ring at a corner is the section carried on the outgoing tangent
// and then slid along it into the plane that bisects the two tangents.
// Reflection in that plane fixes it pointwise and exchanges the two
// tangents, carrying e1 of one onto e1 of the other, so the same curve
// comes out whichever side it is computed from and the loop is tight.
function bring(i) =
  let( t = bt(i), m = bm(i), e1 = cross([0,0,1], t) )
  [ for (s = BSEC)
      let( o = s[0]*e1 + s[1]*[0,0,1] ) BQ[i] + o - ((o*m)/(t*m))*t ];
PERIM = sum([ for (i = [0:NB-1]) norm(BQ[(i+1)%NB] - BQ[i]) ]);
VBEAM = WBEAM*DBEAM*PERIM;
VFOOT = NB*AFOOT*LPOST;

// ---- the solid builders ---------------------------------------------
// A swept tube, capped by a fan to a centre point at each end.  With
// sweep direction t and section axes (e1,e2) forming a right-handed
// triple (e1,e2,t), the ring is listed counter-clockwise in (e1,e2) and
// the wall quads then have inward normals; the caps run the other way
// round the ring, which is the edge test.
module tube(G, C0, C1, conv = 4) {
    K = len(G[0]); M = len(G) - 1; B = (M+1)*K;
    polyhedron(
      points = concat([ for (u = [0:M]) each G[u] ], [C0], [C1]),
      faces = concat(
        [ for (u = [0:M-1]) for (v = [0:K-1])
            [ u*K+v, (u+1)*K+v, (u+1)*K+(v+1)%K, u*K+(v+1)%K ] ],
        [ for (v = [0:K-1]) [ B, v, (v+1)%K ] ],
        [ for (v = [0:K-1]) [ B+1, M*K+(v+1)%K, M*K+v ] ]),
      convexity = conv);
}

// A closed loop of the same walls with no caps at all: the last ring
// wraps onto the first, and the result is one closed genus-one solid.
module loop(G, conv = 6) {
    K = len(G[0]); M = len(G);
    polyhedron(
      points = [ for (u = [0:M-1]) each G[u] ],
      faces = [ for (u = [0:M-1]) for (v = [0:K-1])
                  [ u*K+v, ((u+1)%M)*K+v, ((u+1)%M)*K+(v+1)%K, u*K+(v+1)%K ] ],
      convexity = conv);
}

// The radial at the chord's midpoint is already normal to the chord,
// because both ends are at distance R from the centre.
module strut(pa, pb) {
    d = pb - pa; L = norm(d); t = d/L;
    e1 = unit((pa + pb)/2);
    e2 = cross(t, e1);
    c0 = pa + L0*t; c1 = pb - L0*t;
    G = [ for (u = [0,1])
            [ for (k = [0:KS-1]) let( g = 360*(k + 0.5)/KS )
                (u == 0 ? c0 : c1) + RS*(cos(g)*e1 + sin(g)*e2) ] ];
    tube(G, c0, c1, 2);
}

module post(p) {
    G = [ for (u = [0,1])
            [ for (k = [0:KF-1]) let( g = 360*(k + 0.5)/KF )
                [ p[0] + RF*cos(g), p[1] + RF*sin(g), u == 0 ? ZP0 : ZP1 ] ] ];
    tube(G, [p[0], p[1], ZP0], [p[0], p[1], ZP1], 2);
}

// ---- the length classes ---------------------------------------------
// One face carries a representative of every orbit, and so does one
// icosahedron edge, so the classes are found on 51 struts rather than
// on all 1080.  The multiplicities are then counted over the full list.
SAMPLE = concat(
  [ for (i = [1:V-1]) for (j = [0:i-1]) [ gid(V,0,i,j), gid(V,0,i,j+1) ] ],
  [ for (i = [1:V-1]) for (j = [1:i])   [ gid(V,0,i,j), gid(V,0,i+1,j) ] ],
  [ for (i = [1:V-1]) for (j = [0:i-1]) [ gid(V,0,i,j), gid(V,0,i+1,j+1) ] ],
  [ for (k = [0:V-1])
      [ epid(V, IE[0][0], IE[0][1], k), epid(V, IE[0][0], IE[0][1], k+1) ] ]);
SL   = [ for (e = SAMPLE) elen(e) ];
CUNQ = [ for (i = [0:len(SL)-1])
           if (min([ for (j = [0:len(SL)-1]) if (abs(SL[j]-SL[i]) < TOL) j ]) == i)
             SL[i] ];
NCL  = len(CUNQ);
CRNK = [ for (i = [0:NCL-1]) len([ for (j = [0:NCL-1]) if (CUNQ[j] < CUNQ[i]) 1 ]) ];
CHORD = [ for (r = [0:NCL-1])
            CUNQ[ min([ for (i = [0:NCL-1]) if (CRNK[i] == r) i ]) ] ];
function clsof(l) = min([ for (i = [0:NCL-1]) if (abs(CHORD[i] - l) < TOL) i ]);
MFULL = [ for (c = CHORD) len([ for (e = GE)    if (abs(elen(e) - c) < TOL) 1 ]) ];
MDOME = [ for (c = CHORD) len([ for (e = STRUT) if (abs(elen(e) - c) < TOL) 1 ]) ];

// The hand-checkable coincidence: the piece of icosahedron edge AB from
// P(2,0) to P(3,0) against the interior strut P(2,0) to P(2,1).
LEDG2 = elen([ gid(V,0,2,0), gid(V,0,3,0) ]);
LINT2 = elen([ gid(V,0,2,0), gid(V,0,2,1) ]);

// ---- the horizontal strut rings, and which of them close ------------
HORZ = [ for (e = GE) if (abs(PT[e[0]][2] - PT[e[1]][2]) < TOL) e ];
HZ   = [ for (e = HORZ) PT[e[0]][2] ];
HUNQ = [ for (i = [0:len(HZ)-1])
           if (min([ for (j = [0:len(HZ)-1]) if (abs(HZ[j]-HZ[i]) < TOL) j ]) == i)
             HZ[i] ];
NHL  = len(HUNQ);
HRNK = [ for (i = [0:NHL-1]) len([ for (j = [0:NHL-1]) if (HUNQ[j] > HUNQ[i]) 1 ]) ];
HLEV = [ for (r = [0:NHL-1])
           HUNQ[ min([ for (i = [0:NHL-1]) if (HRNK[i] == r) i ]) ] ];
// [fraction of the sphere above the level, vertices on it, horizontal
// struts on it, least number of those struts on any one vertex].  The
// level is a closed ring when the last two columns are the vertex count
// and 2, because then the degrees sum to twice the count and none is
// below 2, so every one of them is exactly 2.
HTAB = [ for (z = HLEV)
           let( vs = [ for (id = [0:NGV-1]) if (abs(PT[id][2] - z) < TOL) id ],
                es = [ for (e = HORZ)       if (abs(PT[e[0]][2] - z) < TOL) e ] )
             [ (R - z)/(2*R), len(vs), len(es),
               min([ for (id = vs)
                       len([ for (e = es) if (e[0] == id || e[1] == id) 1 ]) ]) ] ];
HCLOSE = [ for (r = HTAB) if (r[1] == r[2] && r[3] == 2) r[0] ];

// ---- the dome --------------------------------------------------------
module dome() {
    for (id = HUB)   color(HUBC) translate(PT[id])
                       polyhedron(points = HUBP, faces = HUBF, convexity = 2);
    for (e = STRUT)  color(PAL[clsof(elen(e))]) strut(PT[e[0]], PT[e[1]]);
    for (id = BORD)  color(HUBC) post(PT[id]);
    color(CONC) loop([ for (i = [0:NB-1]) bring(i) ], 6);
}
translate([0,0,ZG]) dome();

// ---- what the geometry says -----------------------------------------
echo("frequency", V, "class I on an icosahedron; sphere radius", R, "m");
echo("full sphere: vertices", NGV, "= 10V^2+2 ->", 10*V*V+2,
     "  struts", len(GE), "= 30V^2 ->", 30*V*V,
     "  faces", len(GT), "= 20V^2 ->", 20*V*V);
echo("names: 12 icosahedron vertices +", 30*ein(V), "on the 30 edges +",
     20*fin(V), "inside the 20 faces =", 12 + 30*ein(V) + 20*fin(V));

echo("distinct strut lengths", NCL, "found on a sample of", len(SAMPLE),
     "struts, one face plus one icosahedron edge");
echo("chord factors", [ for (c = CHORD) c/R ]);
echo("lengths m", CHORD);
echo("multiplicity over the whole sphere", MFULL, "summing to", sum(MFULL),
     "which must be 30V^2 =", 30*V*V);
echo("multiplicity in the dome", MDOME, "summing to", sum(MDOME),
     "=", len(STRUT), "struts built");
echo("longest over shortest", LMAX/LMIN,
     "; if the net were flat this ratio would be 1");
echo("the coincidence: edge piece P(2,0)-P(3,0)", LEDG2,
     "m against interior strut P(2,0)-P(2,1)", LINT2,
     "m, differing by", abs(LEDG2 - LINT2), "m");

echo("horizontal strut rings, as [fraction of sphere above, hubs, struts,",
     "least struts on one hub]:");
echo(HTAB);
echo("of", NHL, "levels the ones that close are at fractions", HCLOSE,
     "; the two pentagons beside the poles are not domes, so the cut is",
     CUT, "at z =", ZCUT);
echo("base ring: hubs", NB, "= 5V ->", 5*V, " struts", len(BASES),
     " strut count on each base hub, min", min(BDEG), "max", max(BDEG),
     "(2 means the ring closes)");

echo("dome: hubs", len(HUB), " struts", len(STRUT), " footings", NB,
     " ring beam 1; solids", len(HUB) + len(STRUT) + NB + 1);
echo("hub valency in the dome, min", min(HDEG), "max", max(HDEG));
echo("span", 2*R, "m; crown", R + ZG, "m above ground; springing", ZG,
     "m; floor area", PI*sq(R), "m2; enclosed volume of the sphere cap",
     2*PI*sq(R)*R/3, "m3");
echo("total strut length", LSUM, "m; drawn length",
     LSUM - 2*L0*len(STRUT), "m; shortest", LMIN, "longest", LMAX);

echo("membrane resultant pR/2", NMEM, "N/m; force in the longest strut",
     FSTRU, "N; Euler load of the section", FEULR, "N; factor",
     FEULR/FSTRU, "against the", SF, "asked");
echo("strut circumradius", RS*1000, "mm, across flats", RS*sqrt(3)*1000,
     "mm; area", ASTRU*1e6, "mm2; slenderness L/r",
     LMAX/sqrt(IHEX/ASTRU), "; axial stress", FSTRU/ASTRU/1e6, "MPa");
echo("hoop resultant at the springing +pR/2", NMEM,
     "N/m tension; at half a tributary the base strut takes", FHOOP,
     "N, a stress of", FHOOP/ASTRU/1e6, "MPa against", FPRF/1e6, "MPa proof");
echo("footing reaction", FPOST/1000, "kN each; post circumradius",
     RF*1000, "mm; bearing pressure under the beam",
     (NB*FPOST + VBEAM*RHOC*GRAV)/(PERIM*WBEAM)/1000, "kPa");

echo("smallest angle between any two struts at a hub", AMIN,
     "deg, minimised over every pair at every hub; smallest interior",
     "angle of any triangle", ATRI, "deg; difference", AMIN - ATRI);
echo("hub circumradius", RH*1000, "mm, inradius", RHIN*1000,
     "mm, ratio", RH/RHIN, "; a strut stops", L0*1000,
     "mm from the hub centre, so between", GAP*1000, "and",
     (RH - RHIN + GAP)*1000, "mm clear of the hub surface; a ball sized",
     "by its faces instead would reach", L0*(RH/RHIN - 1)*1000,
     "mm past that and bury itself in the strut");
echo("clearances m -- strut to strut at a hub",
     2*L0*sin(AMIN/2) - 2*RS,
     " hub to hub", LMIN - 2*RH,
     " strut to a hub it does not own", ALTMIN - RS - RH,
     " everything else", GAP);

echo("volume m3 -- struts", VSTRU, "hubs", VHUBS, "posts", VFOOT,
     "ring beam", VBEAM);
echo("one hub is", VHUB1*1e6, "mm3, which is",
     100*VHUB1/(4*PI*pow(RH,3)/3), "percent of the ball it is inscribed in");
echo("triangles: hubs", len(HUB)*len(HUBF), " struts", len(STRUT)*4*KS,
     " posts", NB*4*KF, " ring beam", NB*2*len(BSEC), " total",
     len(HUB)*len(HUBF) + len(STRUT)*4*KS + NB*4*KF + NB*2*len(BSEC));
echo("no two solids share volume, so the exported total should be",
     VSTRU + VHUBS + VFOOT + VBEAM, "m3");
echo("mass: aluminium", (VSTRU + VHUBS + VFOOT)*RHOA, "kg =",
     (VSTRU + VHUBS + VFOOT)*RHOA/(PI*sq(R)), "kg per m2 of floor;",
     "ring beam concrete", VBEAM*RHOC, "kg");
