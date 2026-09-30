// ===================================================================
//  A CONCERT GRAND PIANO, drawn out of its string scale
//
//  Everything in a grand piano that looks like a styling decision is
//  really a consequence of one equation.  A stretched string of length
//  L, tension T and mass per unit length mu sounds
//
//      f = (1/2L) sqrt(T/mu),
//
//  and the group sqrt(T/mu) is the speed c at which a transverse wave
//  runs along the string.  So the scale of an instrument is
//
//      L = c / (2 f),
//
//  and the whole design reduces to choosing the wave speed note by
//  note.  It is worth writing c out in terms of what the maker can
//  actually pick.  The tension is carried by a steel core of area A at
//  a working stress sigma, so T = sigma A.  The mass is the core's mass
//  multiplied by whatever is wound onto it, so mu = beta rho A with rho
//  the density of steel and beta >= 1 the factor by which the winding
//  multiplies the mass.  The area cancels:
//
//      c = sqrt( sigma / (beta rho) ).
//
//  A plain wire has beta = 1, so at constant stress c is constant, and
//  L is exactly inversely proportional to f.  That is the ideal scale,
//  and it halves the string every octave.  Anchored on a 51 mm top C it
//  asks for 7.76 m at A0.  Nobody builds that.
//
//  THE LAW USED HERE, AND WHAT IT COSTS
//
//  Two departures from L proportional to 1/f, each paid for in a
//  different currency.
//
//  The first runs through the whole plain-wire compass.  Instead of
//  halving every octave the length is taken to fall by a fixed ratio
//  RT = 1.88 per octave, which stretches the treble and shortens the
//  tenor relative to the ideal.  Since 2 L f = c, holding L f below the
//  ideal means c rises toward the treble, and c rising at constant
//  density means stress rising: the model reports sigma at every
//  landmark note and it climbs from 763 MPa at the bottom plain string
//  to 1431 MPa at the top.  That is the price of foreshortening, and it
//  is paid in steel.  It is also the reason the top notes of a piano
//  are the ones that break.  Even so, 1.88 per octave only takes A0
//  from 7.76 m to 4.96 m.
//
//  The second departure is the winding, and it is confined to the
//  bottom 26 notes.  Copper wound onto the core raises mu without
//  raising T, so c falls by sqrt(beta) and the string may be shortened
//  by the same factor at the same pitch and the same stress.  Here the
//  longest string is set to 1980 mm, the bass ratio per octave that
//  follows is 1.191, and the required beta comes out at 1.05 just below
//  the break and 9.13 at A0.  A plain wire at the same 850 MPa sounding
//  A0 would be 5.98 m long; 1.98 m times sqrt(9.13) is 5.98 m, which is
//  the identity the echo checks.  The price of the winding is stiffness
//  relative to tension.  The inharmonicity coefficient
//
//      B = pi^3 E d^4 / (64 T L^2)
//
//  depends on the CORE diameter, which the winding does not change, so
//  at fixed stress and tension B goes as 1/L^2, which is to say B goes
//  as beta.  Dividing A0's length by 3.02 multiplies its inharmonicity
//  by 9.13.  The bass of a piano sounds the way it does because of
//  exactly that trade.
//
//  Nothing else in the scale is assumed.  The per-string tension is
//  fixed at 800 N and the core diameters fall out of sigma, landing on
//  0.844 mm at top C, 1.032 mm at A4 and 1.155 mm at the lowest plain
//  string, which is where real piano wire gauges are.
//
//  WHY THE BENTSIDE IS A LOGARITHMIC SPIRAL
//
//  The rim exists to enclose the longest string, and the shape it takes
//  follows from the scale being geometric.  Put the bridge pins at a
//  uniform spacing across the instrument and the note index becomes
//  linear in x, so a length that is geometric in note index is
//  exponential in x, and the long bridge, which is the locus of those
//  pins, is an exponential curve in plan.  Push that curve outward by
//  the back length and the rim thickness and the bentside is an
//  exponential too.
//
//  An exponential is what a logarithmic spiral looks like near its own
//  tail, and the spiral is the better description once the case has to
//  turn a full half revolution from the cheek to the spine.  A
//  logarithmic spiral is the curve whose radius of curvature grows
//  geometrically with the turning angle, R = A exp(-a (phi - phi0)),
//  which is the same statement about equal ratios per equal steps that
//  produced the scale.  So the rim here is one spiral arc from the
//  cheek, closed by a single constant-radius arc at the tail and a
//  straight spine.  Its parameters are not chosen.  Writing the path as
//  the integral of R(phi) (cos phi, sin phi) dphi gives both end
//  displacements in closed form, and demanding that the spiral end at
//  the tail radius and that the tail arc land on the spine fixes the
//  spiral completely once the decay rate a is known.  The decay rate is
//  then solved, by bisection in the model, so that the rim clears the
//  hitch pin of the longest string by exactly 45 mm.  Every other note
//  clears by more.  The echo names which note binds; it is A0, which is
//  the sentence the rim was built to satisfy.
//
//  What comes out is R = 5.1 m at the cheek falling to the 330 mm tail
//  radius after 83 degrees of turn, a spine straight for 2.35 m, and an
//  instrument 2.78 m long.  None of those three numbers was typed in.
//
//  THE KEYBOARD IS NOT EVENLY DIVIDED
//
//  An octave spans 165 mm and holds seven white keys and five black
//  ones.  The white keys are simply 165/7 = 23.571 mm each at the
//  front.  Behind the front the black keys interrupt them, and the
//  black keys are NOT on a 165/12 grid.  The rule used here is the one
//  that makes a keyboard playable: within each group of black keys the
//  white tails left over must come out equal.  For the two-key group
//  the three tails of C, D and E and the two black keys must fill 3w,
//  and equal tails force the sharps to sit b/6 outside the white-key
//  boundaries they straddle.  For the three-key group the four tails of
//  F, G, A and B and three black keys fill 4w, and equal tails force
//  b/4.  So with b the black key width,
//
//      C# = w - b/6   D# = 2w + b/6   F# = 4w - b/4   G# = 5w   A# = 6w + b/4
//
//  measured from the left edge of C, and the tails come out at w - 2b/3
//  in the two-key group and w - 3b/4 in the three-key group.  Only G#
//  sits over a white-key boundary.  The others are displaced by 2.28 mm
//  and 3.43 mm, away from the centre of their own group in both cases,
//  and the largest departure from an even twelfth of the octave is
//  1.49 mm at F#.  Both identities, 3(w - 2b/3) + 2b = 3w and
//  4(w - 3b/4) + 3b = 4w, are exact, and the model echoes them.
//
//  THE RIM IS ONE PIECE WITH A SECTION THAT CHANGES
//
//  The case rim is a bent lamination, and it is drawn as a single swept
//  polyhedron with a ten-sided section that is different at every one
//  of its 155 stations.  Two things vary.  The rim's own width runs
//  from 82 mm where it dies into the cheek, which the keybed and the
//  cheek blocks tie together anyway, to 118 mm along the spine, which
//  carries the instrument's length as a beam.  And the inner rim, the
//  ledge the soundboard is glued to, swells from a token 8 mm at the
//  two open ends to 62 mm behind the belly rail, because in front of
//  the belly rail there is no soundboard for it to carry.  The section
//  is not convex: the ledge is a step in the inner face, and the two
//  end caps are ten-sided non-convex faces.
//
//  THE PLATE IS DRAWN WHERE THE LOAD IS
//
//  Only two pieces of the iron frame are here, and they are the two the
//  strings actually bear on.  The front bearing is swept along the
//  agraffe line with its crown set, note by note, just under that
//  note's own string, so its top edge is the scale drawn in section.
//  The hitch rail is swept along the locus of hitch pins, which is the
//  bridge curve pushed out by the back lengths.  That locus is not the
//  rim: a grand's tenor strings end over the middle of the soundboard,
//  and the echo reports the deepest hitch pin at 654 mm inboard of the
//  rim's inner face.  Only at the tail do the two come together, and
//  that is the one place where the rim's own clearance binds.  The rail
//  also lies wholly outboard of the pins rather than straddling them,
//  because at the bass end of the long bridge the curve has climbed so
//  steeply that the pins of one note are 19 mm from where the curve has
//  reached two notes lower, and a rail centred on the pins runs into
//  it.  The struts that join the front bearing to the hitch rail are
//  left out; every one of them has strings on both sides and placing
//  them is a different problem from the one this model is about.
//
//  NO TWO SOLIDS SHARE A CUBIC MILLIMETRE
//
//  Which is worth the trouble it costs.  Every place two parts meet,
//  1.5 mm of air is left between them, so the exported mesh is exactly
//  the mesh written here, face for face.  The volume audit at the foot
//  of the file is then a prediction and not an estimate: it sums the
//  divergence-theorem volume of every polyhedron from the same vertex
//  arrays that are handed to polyhedron(), with the same fan
//  triangulation of every quad, and any disagreement with the exporter
//  is a finding rather than a rounding.  It is also much faster,
//  because a union of disjoint solids never has to build a BSP.
//
//  The arrangement that makes this possible is worth naming.  The
//  strings do not touch the bridges: each string is a four-point
//  polyline from the tuning pin over the front bearing to the bridge
//  crown and down to the hitch pin, mitred at every knuckle, and it
//  rides 1 mm clear of the crown it would really bear on.  The bass
//  strings cross over the tenor strings, which is what overstringing
//  means, and the model measures the vertical gap at the crossing
//  rather than asserting it.  The tuning pins stand on the pinblock
//  instead of being driven into it.  The ribs stop short of the inner
//  rim instead of being let into it.
//
//  WINDING.  polyhedron() wants each face wound so the right-hand
//  normal points INTO the solid.  Every swept part here uses one
//  template: with sweep direction t and section axes (e1, e2) forming a
//  right-handed triple (e1, e2, t), the section ring is listed
//  counter-clockwise in (e1, e2), the wall quads follow, and the two
//  end caps traverse their rings the OTHER way round from the wall
//  quads that meet them.  The rim sweeps along its own path with
//  (e1, e2) = (z, outward), the bridges the same, the strings with the
//  section spanned by the string's own horizontal normal and its
//  binormal.  The orientation-blind test is the one that catches
//  mistakes: two faces sharing an edge must traverse it in opposite
//  directions.
//
//  A CAP FACE IS NOT A TRIANGULATION
//
//  polyhedron() accepts faces with more than three vertices and splits
//  them itself, by fanning from the face's first vertex.  A fan covers
//  the face only when the face is star-shaped from that vertex, and it
//  produces a triangle of zero area whenever the fan vertex is
//  collinear with two consecutive others.  Both cases turn up here.
//  The rim's ten-sided section is a tee, and no fan from any of its own
//  vertices avoids both faults at once: the ones that see the whole
//  section lie on the straight inner face and collapse a triangle
//  against it.  So its caps are given as an explicit list of eight
//  triangles, the ledge rectangle split by a diagonal and the convex
//  remainder fanned from the outer arris.  A white key with a black key
//  on each side puts four of its vertices on one line at the step, and
//  its caps are split by hand too, into the front strip and the tail.
//  The symptom of getting this wrong is worth knowing, because it is
//  quiet: the export drops each degenerate triangle and hands back
//  three boundary edges in its place, while the signed volume stays
//  exactly right, since a fan's signed areas telescope whether or not
//  its triangles lie inside the face at all.
//
//  The dimensions are a coherent design rather than a copy of any one
//  instrument.  They land where a 2.78 m concert grand lands, and the
//  echoed stresses, wire gauges, inharmonicity and total string tension
//  are there so a reader can check that they do.
// ===================================================================

// ---- the scale ------------------------------------------------------
NB     = 26;        // wound bass notes, A0 up to A#2
RT     = 1.88;      // plain-wire length ratio per octave
LTOP   = 51.0;      // speaking length at C8, mm
STEP   = 1.09;      // length jump across the tenor break
LA0    = 1980.0;    // longest string, mm
RHO    = 7.850e-6;  // steel, kg/mm3
EMOD   = 2.00e5;    // steel, MPa = N/mm2
SIGB   = 850.0;     // bass core working stress, MPa
TEN    = 800.0;     // tension per string, N
NSING  = 8;         // single-strung notes at the bottom

// ---- keyboard -------------------------------------------------------
OCT    = 165.0;     // an octave, mm
BKW    = 13.7;      // black key width, mm
KGAP   = 0.8;       // air between adjacent keys, mm
WKL    = 150.0;     // visible white key length, mm
BKL    = 95.0;      // black key length, mm
BKTOP  = 10.2;      // black key top width after the bevel, mm
YKF    = 20.0;      // front of the white keys, mm

// ---- string layout --------------------------------------------------
YPIN   = [320, 340, 360, 380];  // four staggered rows of tuning pins
XTB    = [-572, -243];   // bass string band at the front bearing
XTT    = [-242,  568];   // plain string band at the front bearing
YTERM  = 450.0;     // the front bearing line
XB88   = 560.0;     // long bridge at C8
SPB_T  = 12.0;      // unison spacing at the bridge, treble
SPB_B  = 18.0;      // and at the tenor break
XBB    = [-520, -90];    // bass bridge, A0 to the break
SUNI3  = 4.6;       // string spacing inside a triple
SUNI2  = 5.6;       // and inside a double
BK0    = 45.0;      // back length, constant part
BK1    = 0.05;      // back length, part proportional to the speaking length
ANGF   = 0.6;       // front bearing angle at the bridge, degrees
ANGB   = 2.0;       // back bearing angle at the bridge, degrees

// ---- case -----------------------------------------------------------
CHEEKW = 55.0;      // cheek block, mm
YCH    = 520.0;     // keybed depth: the case is parallel this far back
RTAIL  = 330.0;     // tail bending radius, mm
CLR    = 45.0;      // rim inner face to the nearest hitch pin, mm
GAP    = 1.5;       // air between any two solids, mm
NPHI   = 150;       // stations around the bent part of the rim
ZRB    = -155.0;    // rim underside
ZRT    =  115.0;    // rim top, where the lid lands
RCH    = 9.0;       // chamfer on the outer arrises
WCHEEK = 82.0;      // rim width at the cheek
WSPINE = 118.0;     // rim width at the spine
SH0    = 8.0;       // inner rim projection at the open ends
SH1    = 62.0;      // inner rim projection behind the belly rail
SHRMP  = 35.0;      // degrees of turn over which it swells
ZLT    = -10.5;     // top of the inner rim: the soundboard glue line
ZLB    = -65.5;     // its underside

// ---- soundboard, ribs, bridges --------------------------------------
TSB    = 9.0;       // soundboard thickness
YBRT   = 420.0;     // belly rail meets the rim, treble side
YBRS   = 940.0;     // and spine side
NRIB   = 12;
RIBA   = 120.0;     // rib direction, degrees from +x
RIBW   = 25.0;
RIBH   = 20.0;
RIBTAP = 70.0;      // length of the ramp at each rib end
HLB    = [27.0, 32.0];   // long bridge height, C8 end and break end
HBB    = [58.0, 62.0];   // bass bridge height, break end and A0 end
WBRL   = [34.0, 28.0];   // long bridge width at the board and at the crown
WBRB   = [38.0, 32.0];
BROVER = 45.0;      // bridge overrun past the end note

// ---- plate, pinblock, front of the case -----------------------------
HRW    =  30.0;     // hitch rail width, outboard of the pins
ZHR    = [2.5, 16.0];
YPB    = [290.0, 415.0];  ZPB = [-8.0, 20.0];
PINR   = 3.4; PINZ = [22.0, 76.0]; ZCOIL = 34.0;
ZKEYT  = -18.0;     // white key top
ZKEYB  = -40.0;
ZKBED  = [-80.0, -46.0];
ZFLOOR = -748.0;    // white key top 730 mm above the floor

// ---- lid, legs, lyre ------------------------------------------------
TLID   = 20.0;
YFOLD  = 500.0;
LSTICK = 980.0;
STKPHI = 106.0;     // where the stick foot stands, degrees of turn
STKFR  = 0.50;      // socket position as a fraction of the hinge-to-foot span
LEGTOP = 110.0; LEGBOT = 82.0; LEGY = 65.0; LEGPHI = 168.0;
CAST   = 62.0;      // caster height

WOOD  = [0.30, 0.16, 0.10];
SB    = [0.84, 0.66, 0.38];
IRON  = [0.72, 0.62, 0.36];
WIRE  = [0.78, 0.78, 0.80];
IVORY = [0.96, 0.95, 0.90];
EBONY = [0.10, 0.09, 0.09];

// ---- helpers the kernel does not have -------------------------------
RAD = PI/180;
function sq(t) = t*t;
function unit(v) = v/norm(v);

// A halving sum.  A linear recursion over 230 terms walks into the
// parser's depth guard; halving makes the depth logarithmic.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

function shoelace(P) = let( n = len(P) )
  0.5*sum([ for (i = [0:n-1]) P[i][0]*P[(i+1)%n][1] - P[(i+1)%n][0]*P[i][1] ]);

// The divergence theorem's summand for one triangle, with the sign for
// a face wound the way polyhedron() wants it.  a.(b x c)/6 is the
// tetrahedron on the origin and an OUTWARD-facing triangle; these faces
// point inward, so it is negated, and the total comes out positive.
function tdet(a, b, c) =
  -(a[0]*(b[1]*c[2]-b[2]*c[1]) + a[1]*(b[2]*c[0]-b[0]*c[2])
    + a[2]*(b[0]*c[1]-b[1]*c[0]))/6;

// A swept solid: a ring of K points at each of M+1 stations, closed by
// the ring itself at both ends.  Walls counter-clockwise as seen from
// outside means listed as below; the start cap runs the ring forwards
// and the end cap backwards, which is the opposite traversal of every
// shared edge.
module sweep(G, conv = 4) {
    K = len(G[0]); M = len(G) - 1;
    polyhedron(
      points = [ for (u = [0:M]) each G[u] ],
      faces = concat(
        [ for (u = [0:M-1]) for (v = [0:K-1])
            [ u*K+v, (u+1)*K+v, (u+1)*K+(v+1)%K, u*K+(v+1)%K ] ],
        [ [ for (v = [0:K-1]) v ] ],
        [ [ for (v = [0:K-1]) M*K + (K-1-v) ] ]),
      convexity = conv);
}
// The same mesh's exact volume, with every quad fanned from its first
// vertex, which is how a quad face is triangulated on the way out.
function swvol(G) = let( K = len(G[0]), M = len(G) - 1 )
    sum([ for (u = [0:M-1]) sum([ for (v = [0:K-1])
            tdet(G[u][v], G[u+1][v], G[u+1][(v+1)%K])
          + tdet(G[u][v], G[u+1][(v+1)%K], G[u][(v+1)%K]) ]) ])
  + sum([ for (v = [1:K-2]) tdet(G[0][0], G[0][v], G[0][v+1]) ])
  + sum([ for (v = [1:K-2]) tdet(G[M][K-1], G[M][K-1-v], G[M][K-2-v]) ]);

// The same, but with the cap given as an explicit triangle list, for a
// section that no fan from one of its own vertices can cover.
module sweept(G, CT, conv = 4) {
    K = len(G[0]); M = len(G) - 1;
    polyhedron(
      points = [ for (u = [0:M]) each G[u] ],
      faces = concat(
        [ for (u = [0:M-1]) for (v = [0:K-1])
            [ u*K+v, (u+1)*K+v, (u+1)*K+(v+1)%K, u*K+(v+1)%K ] ],
        [ for (f = CT) f ],
        [ for (f = CT) [ M*K+f[2], M*K+f[1], M*K+f[0] ] ]),
      convexity = conv);
}
function swvolt(G, CT) = let( K = len(G[0]), M = len(G) - 1 )
    sum([ for (u = [0:M-1]) sum([ for (v = [0:K-1])
            tdet(G[u][v], G[u+1][v], G[u+1][(v+1)%K])
          + tdet(G[u][v], G[u+1][(v+1)%K], G[u][(v+1)%K]) ]) ])
  + sum([ for (f = CT) tdet(G[0][f[0]], G[0][f[1]], G[0][f[2]]) ])
  + sum([ for (f = CT) tdet(G[M][f[2]], G[M][f[1]], G[M][f[0]]) ]);

// A swept tube capped by a fan to a point at each end.
module tube(G, C0, C1, conv = 3) {
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
function tubevol(G, C0, C1) = let( K = len(G[0]), M = len(G) - 1 )
    sum([ for (u = [0:M-1]) sum([ for (v = [0:K-1])
            tdet(G[u][v], G[u+1][v], G[u+1][(v+1)%K])
          + tdet(G[u][v], G[u+1][(v+1)%K], G[u][(v+1)%K]) ]) ])
  + sum([ for (v = [0:K-1]) tdet(C0, G[0][v], G[0][(v+1)%K]) ])
  + sum([ for (v = [0:K-1]) tdet(C1, G[M][(v+1)%K], G[M][v]) ]);

// A vertical prism on a counter-clockwise plan polygon.
function przrings(P, z0, z1) =
  [ [ for (p = P) [p[0], p[1], z0] ], [ for (p = P) [p[0], p[1], z1] ] ];
module prz(P, z0, z1, conv = 3) { sweep(przrings(P, z0, z1), conv); }
function przvol(P, z0, z1) = shoelace(P)*(z1 - z0);

module box(p, q) {
    a = [min(p[0],q[0]), min(p[1],q[1]), min(p[2],q[2])];
    b = [max(p[0],q[0]), max(p[1],q[1]), max(p[2],q[2])];
    prz([[a[0],a[1]],[b[0],a[1]],[b[0],b[1]],[a[0],b[1]]], a[2], b[2], 2);
}
function boxvol(p, q) = abs((q[0]-p[0])*(q[1]-p[1])*(q[2]-p[2]));

// Tail-recursive bisection.  Sixty halvings of any bracket is exact to
// the last bit a double carries.
function bisect(f, lo, hi, n) =
  n <= 0 ? (lo + hi)/2
  : f((lo + hi)/2) ? bisect(f, (lo + hi)/2, hi, n - 1)
                   : bisect(f, lo, (lo + hi)/2, n - 1);

// ===================================================================
//  THE SCALE
// ===================================================================
function freq(k) = 27.5*pow(2, (k - 1)/12);
LBRK = LTOP*pow(RT, (88 - (NB + 1))/12)*STEP;   // the topmost wound string
RB   = pow(LA0/LBRK, 12/(NB - 1));              // bass ratio per octave
function slen(k) = k > NB ? LTOP*pow(RT, (88 - k)/12)
                          : LBRK*pow(RB, (NB - k)/12);
function wspeed(k) = 2*slen(k)*freq(k)/1000;        // m/s
function sigma(k)  = k > NB ? RHO*sq(wspeed(k))*1e3 : SIGB;   // MPa
function beta(k)   = sigma(k)/(RHO*sq(wspeed(k))*1e3);
function dcore(k)  = sqrt(4*TEN/(PI*sigma(k)));     // mm
function inharm(k) = pow(PI,3)*EMOD*pow(dcore(k),4)/(64*TEN*sq(slen(k)));
function nuni(k)   = k <= NSING ? 1 : k <= NB ? 2 : 3;
function backlen(k) = BK0 + BK1*slen(k);

// ===================================================================
//  THE KEYBOARD
//  Seven white keys to 165 mm, five black ones placed so that the white
//  tails come out equal within each group.
// ===================================================================
WK   = OCT/7;
function pc(k) = (k + 20)%12;
function isblk(k) = pc(k)==1 || pc(k)==3 || pc(k)==6 || pc(k)==8 || pc(k)==10;
WCNT = [ for (k = [1:88]) isblk(k) ? 0 : 1 ];
function nwhite(k) = k <= 0 ? 0 : sum(WCNT, 0, k);   // whites among keys 1..k
NWK  = nwhite(88);
XK0  = -NWK*WK/2;
// Displacement of a sharp from the white boundary it straddles.  Equal
// tails in the two-key group give b/6; in the three-key group, b/4.
function bdisp(k) = pc(k)==1 ? -BKW/6 : pc(k)==3 ?  BKW/6
                  : pc(k)==6 ? -BKW/4 : pc(k)==8 ?  0 : BKW/4;
function blackx(k) = XK0 + nwhite(k - 1)*WK + bdisp(k);
function wleft(k)  = XK0 + (nwhite(k) - 1)*WK;
function tailL(k)  = k > 1  && isblk(k-1) ? blackx(k-1) + BKW/2 : wleft(k);
function tailR(k)  = k < 88 && isblk(k+1) ? blackx(k+1) - BKW/2 : wleft(k) + WK;
YKB  = YKF + WKL;            // back of the keys
YBK0 = YKB - BKL;            // front of the black keys

// ===================================================================
//  THE STRINGS
//  Unison centres at the bridge are laid out first, because the bridge
//  is where the scale is drawn; the pins then follow from the plan
//  angle each string needs to reach its own bridge pin.
// ===================================================================
SPB  = [ for (k = [27:87]) SPB_T + (SPB_B - SPB_T)*(88 - (k + 1))/61 ];
function xlb(k) = XB88 - sum(SPB, k - 27, 61);
function xbb(k) = XBB[0] + (k - 1)*(XBB[1] - XBB[0])/(NB - 1);
function xbridge(k) = k <= NB ? xbb(k) : xlb(k);
function suni(n) = n == 3 ? SUNI3 : n == 2 ? SUNI2 : 0;

// One entry per string: [note, index in its unison, unison size, x at the bridge]
STR = [ for (k = [1:88]) let (n = nuni(k)) for (j = [0:n-1])
          [ k, j, n, xbridge(k) + (j - (n - 1)/2)*suni(n) ] ];
NS    = len(STR);
NBASS = sum([ for (k = [1:NB]) nuni(k) ]);
// The front bearing, not the pinblock, is where string spacing has to
// be right, so the terminations are the even layout and the pins follow
// from the plan angle each string needs.  Doing it the other way round
// crosses strings inside a unison: four staggered pin rows give four
// different non-speaking lengths, so four different sideways drifts,
// and two strings 4.35 mm apart at the pin arrive at the bearing in the
// wrong order.  This way the plan order at the bearing is the plan
// order everywhere, because every string is one straight line and its
// two ends are both in order.
function xterm(i) = i < NBASS ? XTB[0] + i*(XTB[1] - XTB[0])/(NBASS - 1)
                              : XTT[0] + (i - NBASS)*(XTT[1] - XTT[0])/(NS - NBASS - 1);
function sk(i)   = STR[i][0];
function sxb(i)  = STR[i][3];
function ssin(i) = (sxb(i) - xterm(i))/slen(sk(i));
function scos(i) = sqrt(1 - sq(ssin(i)));
function sdir(i) = [ssin(i), scos(i)];
function pfront(i) = (YTERM - YPIN[i%4])/scos(i);
function sterm(i)= [xterm(i), YTERM];
function spin(i) = sterm(i) - pfront(i)*sdir(i);
function xpin(i) = spin(i)[0];
function sbrg(i) = sterm(i) + slen(sk(i))*sdir(i);
function shit(i) = sbrg(i) + backlen(sk(i))*sdir(i);

function brgtop(k) = k <= NB ? HBB[0] + (HBB[1] - HBB[0])*(NB - k)/(NB - 1)
                             : HLB[0] + (HLB[1] - HLB[0])*(88 - k)/61;
function zbrg(k)  = brgtop(k) + 1.0 + dcore(k)/2;
function zterm(k) = zbrg(k) - slen(k)*tan(ANGF);
function zhit(k)  = zbrg(k) - backlen(k)*tan(ANGB);

// ===================================================================
//  THE RIM
//  A straight cheek run, a logarithmic spiral, a constant-radius tail
//  and a straight spine.  The spiral's decay rate is the one unknown,
//  and it is solved so that the longest string's hitch pin clears the
//  inner face by exactly CLR.
// ===================================================================
XTR =  NWK*WK/2 + CHEEKW;
XSP = -XTR;
HIT = [ for (i = [0:NS-1]) shit(i) ];
IA0 = 0;                                   // A0's first string is index 0

// Displacement of a spiral arc of radius A exp(-a(phi-90deg)) from 90
// degrees to phiB, in closed form.  d/dphi [e^-a phi (-a cos + sin)]
// is (1+a^2) e^-a phi cos phi, and likewise for the sine.
function spA(a, phiB) = RTAIL*exp(a*(phiB - 90)*RAD);
function spend(a, phiB) =
  let( A = spA(a, phiB), q = 1 + a*a )
  [ XTR + (RTAIL*(-a*cos(phiB) + sin(phiB)) - A)/q + RTAIL*(-1 - sin(phiB)),
    YCH + (RTAIL*(-a*sin(phiB) - cos(phiB)) + A*a)/q + RTAIL*cos(phiB) ];
// End x falls as phiB rises, so one bisection places the tail arc.
function fphi(a, lo, hi, n) =
  n <= 0 ? (lo + hi)/2
  : spend(a, (lo + hi)/2)[0] > XSP ? fphi(a, (lo + hi)/2, hi, n - 1)
                                   : fphi(a, lo, (lo + hi)/2, n - 1);
function rimpt(a, phiB, phi) =
  let( A = spA(a, phiB), q = 1 + a*a )
    phi <= phiB
  ? [ XTR + A*(exp(-a*(phi - 90)*RAD)*(-a*cos(phi) + sin(phi)) - 1)/q,
      YCH + A*(exp(-a*(phi - 90)*RAD)*(-a*sin(phi) - cos(phi)) + a)/q ]
  : let( b = rimpt(a, phiB, phiB) )
    [ b[0] + RTAIL*(sin(phi) - sin(phiB)), b[1] + RTAIL*(cos(phiB) - cos(phi)) ];
function nout(phi) = [sin(phi), -cos(phi)];
// For a convex body the distance from an interior point to the boundary
// is the smallest of the distances to its supporting lines.
function rclear(a, phiB, P, n) =
  min([ for (i = [0:n]) let (ph = 90 + 180*i/n)
          (rimpt(a, phiB, ph) - P)*nout(ph) ]);
function fa(lo, hi, n) =
  n <= 0 ? (lo + hi)/2
  : let( m = (lo + hi)/2 )
    rclear(m, fphi(m, 90.001, 269.999, 60), HIT[IA0], 90) < CLR
      ? fa(m, hi, n - 1) : fa(lo, m, n - 1);
SPIA  = fa(0.30, 3.00, 44);
PHIB  = fphi(SPIA, 90.001, 269.999, 60);
RCHEEK = spA(SPIA, PHIB);

function rimW(phi) = WCHEEK + (WSPINE - WCHEEK)*(phi - 90)/180;
function rimSH(phi) = SH0 + (SH1 - SH0)*min(1, (phi - 90)/SHRMP, (270 - phi)/SHRMP);

// Stations: the two front corners, the belly-rail stations on the two
// straight runs, and the bent part.
PATH = concat(
  [ [XTR, 0, 90], [XTR, YBRT, 90] ],
  [ for (i = [0:NPHI]) let (ph = 90 + 180*i/NPHI, p = rimpt(SPIA, PHIB, ph))
      [p[0], p[1], ph] ],
  [ [ spend(SPIA, PHIB)[0], YBRS, 270 ], [ spend(SPIA, PHIB)[0], 0, 270 ] ]);
NP   = len(PATH) - 1;
IBR0 = 1; IBR1 = NP - 1;          // the two belly-rail stations
function pp(i)  = [PATH[i][0], PATH[i][1]];
function pph(i) = PATH[i][2];
function pn(i)  = nout(pph(i));
function poff(i, q) = pp(i) + q*pn(i);

// Overall string diameter.  The winding is copper, so a mass ratio beta
// over a steel core of diameter d needs an outside diameter
// d sqrt(1 + (beta-1) rho_steel/rho_copper).
RHOCU = 8.960e-6;
function dout(k) = dcore(k)*sqrt(1 + (beta(k) - 1)*RHO/RHOCU);

UN = [ for (k = [1:88]) nuni(k) ];
function noff(k) = sum(UN, 0, k - 1);
function ictr(k) = noff(k) + floor((nuni(k) - 1)/2);
function nbrg(k) = sbrg(ictr(k));
function nterm(k)= sterm(ictr(k));

// ===================================================================
//  THE RIM, ONE SWEPT POLYHEDRON
//  Section listed clockwise in (outward, up), which is counter-
//  clockwise in (e1, e2) = (up, outward), and (up, outward, heading)
//  is a right-handed triple.
// ===================================================================
function rimsec(phi) = let( W = rimW(phi), S = rimSH(phi) )
  [ [0, ZRB], [0, ZLB], [-S, ZLB], [-S, ZLT], [0, ZLT], [0, ZRT],
    [W - RCH, ZRT], [W, ZRT - RCH], [W, ZRB + RCH], [W - RCH, ZRB] ];
function rimring(i) = let( p = pp(i), n = pn(i) )
  [ for (s = rimsec(pph(i))) [p[0] + s[0]*n[0], p[1] + s[0]*n[1], s[1]] ];
// The section is a tee: the ledge sticks out at q < 0 while the rim
// itself fills q in [0, W].  A fan from a vertex on the outer face
// leaves the face; a fan from one on the inner face stays inside but
// collapses a triangle against it.  So split it by hand into the ledge
// rectangle and the convex remainder, the second fanned from the outer
// arris.  Eight triangles for ten vertices, as a triangulation has.
RIMCAP = [[1,2,3],[1,3,4],[7,8,9],[7,9,0],[7,0,1],[7,1,4],[7,4,5],[7,5,6]];
RIMG = [ for (i = [0:NP]) rimring(i) ];
color(WOOD) sweept(RIMG, RIMCAP, 6);

// ===================================================================
//  SOUNDBOARD, BELLY RAIL AND RIBS
// ===================================================================
BPOLY = [ for (i = [IBR0:IBR1]) poff(i, -GAP) ];
NBP   = len(BPOLY);
BINS  = [ for (j = [0:NBP-1]) j < NBP - 1 ? rimSH(pph(IBR0 + j)) + 8 : 40 ];
ABOARD = shoelace(BPOLY);
color(SB) prz(BPOLY, ZLT + GAP, ZLT + GAP + TSB, 4);

// Outward edge normals of a counter-clockwise polygon.
function bedge(j) = BPOLY[(j+1)%NBP] - BPOLY[j];
function bnorm(j) = let( e = bedge(j) ) [e[1], -e[0]]/norm(e);

// Clip a line q + t u to the polygon, each edge moved in by its own inset.
function ribhi(q, u) =
  min([ for (j = [0:NBP-1]) let( n = bnorm(j), d = u*n )
          if (d > 1e-6) (-BINS[j] - (q - BPOLY[j])*n)/d ]);
function riblo(q, u) =
  max([ for (j = [0:NBP-1]) let( n = bnorm(j), d = u*n )
          if (d < -1e-6) (-BINS[j] - (q - BPOLY[j])*n)/d ]);

// The ribs are spread evenly across the board's own extent in the
// direction they are spaced along, so the spacing is a consequence of
// the board rather than a number typed in.  Where a station's chord is
// shorter than the two end ramps the rib is left out.
RIBU = [cos(RIBA), sin(RIBA)];
RIBV = [sin(RIBA), -cos(RIBA)];
WRIB = [ min([ for (p = BPOLY) p*RIBV ]), max([ for (p = BPOLY) p*RIBV ]) ];
RIBSP = (WRIB[1] - WRIB[0])/(NRIB + 1);
function ribq(r) = (WRIB[0] + RIBSP*(r + 1))*RIBV;
function ribsec(t0, t1) = let( z0 = ZLT - GAP - RIBH, z1 = ZLT - GAP )
  [ [t0, z0], [t1, z0], [t1 - RIBTAP, z1], [t0 + RIBTAP, z1] ];
function ribrings(r) =
  let( q = ribq(r), t0 = riblo(q, RIBU), t1 = ribhi(q, RIBU) )
  [ for (w = [-RIBW/2, RIBW/2])
      [ for (s = ribsec(t0, t1))
          [ q[0] + s[0]*RIBU[0] + w*RIBV[0], q[1] + s[0]*RIBU[1] + w*RIBV[1], s[1] ] ] ];
RIBG = [ for (r = [0:NRIB-1]) ribrings(r) ];
RIBOK = [ for (r = [0:NRIB-1])
            ribhi(ribq(r), RIBU) - riblo(ribq(r), RIBU) > 2.2*RIBTAP ];
for (r = [0:NRIB-1]) if (RIBOK[r]) color(SB) sweep(RIBG[r], 2);

BRAIL0 = poff(IBR0, -GAP - 30);
BRAIL1 = poff(IBR1, -GAP - 30);
BRU    = unit(BRAIL1 - BRAIL0);
BRV    = [BRU[1], -BRU[0]];
function brailring(e) =
  let( q = BRAIL0 + e*(BRAIL1 - BRAIL0) )
  [ for (s = [[-27.5, -44], [-27.5, -12], [27.5, -12], [27.5, -44]])
      [ q[0] + s[0]*BRV[0], q[1] + s[0]*BRV[1], s[1] ] ];
BRAILG = [ brailring(0), brailring(1) ];
color(WOOD) sweep(BRAILG, 2);

// ===================================================================
//  BRIDGES
//  A swept trapezoid whose crown height is the note's own, so the
//  section changes at every station.
// ===================================================================
function bsec(wb, wt, z1) = [ [-wb, GAP], [-wt, z1], [wt, z1], [wb, GAP] ];
function tang(P, m) = let( n = len(P) - 1 )
    m == 0 ? unit(P[1] - P[0])
  : m == n ? unit(P[n] - P[n-1])
  : unit(P[m+1] - P[m-1]);
function brring(P, m, wb, wt, z1) =
  let( q = P[m], t = tang(P, m), v = [t[1], -t[0]] )
  [ for (s = bsec(wb, wt, z1)) [ q[0] + s[0]*v[0], q[1] + s[0]*v[1], s[1] ] ];

LBP0 = [ for (k = [27:88]) nbrg(k) ];
LBP  = concat([ LBP0[0]  + BROVER*unit(LBP0[0] - LBP0[1]) ], LBP0,
               [ LBP0[61] + BROVER*unit(LBP0[61] - LBP0[60]) ]);
LBZ  = concat([brgtop(27)], [ for (k = [27:88]) brgtop(k) ], [brgtop(88)]);
LBG  = [ for (m = [0:len(LBP)-1]) brring(LBP, m, WBRL[0]/2, WBRL[1]/2, LBZ[m]) ];
color(SB) sweep(LBG, 4);

BBP0 = [ for (k = [1:NB]) nbrg(k) ];
BBP  = concat([ BBP0[0] + BROVER*unit(BBP0[0] - BBP0[1]) ], BBP0,
               [ BBP0[NB-1] + BROVER*unit(BBP0[NB-1] - BBP0[NB-2]) ]);
BBZ  = concat([brgtop(1)], [ for (k = [1:NB]) brgtop(k) ], [brgtop(NB)]);
BBG  = [ for (m = [0:len(BBP)-1]) brring(BBP, m, WBRB[0]/2, WBRB[1]/2, BBZ[m]) ];
color(SB) sweep(BBG, 4);

// ===================================================================
//  PLATE: the hitch rail and the front bearing
// ===================================================================
// The hitch pins are not all near the rim.  A grand's tenor strings end
// over the middle of the soundboard, 600 mm inboard of anything, and
// the plate carries them there.  So the hitch rail is swept along the
// locus of hitch pins itself, which is the bridge curve pushed out by
// the back lengths, in two runs: one for the wound bass and one for the
// plain strings.  Only at the tail do the two coincide, and that is the
// one place the rim's own clearance binds.
// The rail lies wholly on the far side of the pins from the bridge.
// It has to: at the bass end of the long bridge the curve is so steep
// that the hitch pins of one note sit only 19 mm from where the curve
// has got to two notes further down, and a rail straddling the pins
// would run straight into it.
function hrring(P, m, z1) =
  let( q = P[m], t = tang(P, m), v = [t[1], -t[0]] )
  [ for (s = [[-HRW-2, ZHR[0]], [-HRW-2, z1], [-2, z1], [-2, ZHR[0]]])
      [ q[0] + s[0]*v[0], q[1] + s[0]*v[1], s[1] ] ];
function hrpath(k0, k1) =
  let( H = [ for (k = [k0:k1]) shit(ictr(k)) ], n = len(H) - 1 )
    concat([ H[0] + 30*unit(H[0] - H[1]) ], H, [ H[n] + 30*unit(H[n] - H[n-1]) ]);
HRP = hrpath(NB + 1, 88);  HRB = hrpath(1, NB);
HRG = [ for (m = [0:len(HRP)-1]) hrring(HRP, m, ZHR[1]) ];
HBG = [ for (m = [0:len(HRB)-1]) hrring(HRB, m, ZHR[1] + 14) ];
color(IRON) sweep(HRG, 5);
color(IRON) sweep(HBG, 5);

FBP = [ for (k = [1:88]) [nterm(k)[0], 450] ];
function fbring(m) =
  let( q = FBP[m], t = tang(FBP, m), v = [t[1], -t[0]],
       z1 = zterm(m + 1) - dout(m + 1)/2 - GAP )
  [ for (s = [[-27, 2], [-27, z1], [-1, z1], [-1, 2]])
      [ q[0] + s[0]*v[0], q[1] + s[0]*v[1], s[1] ] ];
FBG = [ for (m = [0:87]) fbring(m) ];
color(IRON) sweep(FBG, 5);

// ===================================================================
//  STRINGS, mitred at every knuckle
// ===================================================================
NSIDE = 5;
function spath(i) = let( k = sk(i) )
  [ [spin(i)[0], spin(i)[1], ZCOIL], [sterm(i)[0], sterm(i)[1], zterm(k)],
    [sbrg(i)[0],  sbrg(i)[1],  zbrg(k)], [shit(i)[0], shit(i)[1], zhit(k)] ];
function sseg(P, j) = unit(P[j+1] - P[j]);
function smit(P, j) = j == 0 ? sseg(P, 0) : j == 3 ? sseg(P, 2)
                    : unit(sseg(P, j-1) + sseg(P, j));
function sref(P, j) = j == 3 ? sseg(P, 2) : sseg(P, j);
function sring(i, P, j, r) =
  let( h = [scos(i), -ssin(i), 0], t = sref(P, j), b = cross(t, h), m = smit(P, j) )
  [ for (v = [0:NSIDE-1])
      let( g = 180/NSIDE + 360*v/NSIDE, u = cos(g)*h + sin(g)*b,
           s = -r*(u*m)/(t*m) )
        P[j] + r*u + s*t ];
STRG = [ for (i = [0:NS-1])
           let( P = spath(i), r = dout(sk(i))/2 )
             [ for (j = [0:3]) sring(i, P, j, r) ] ];
for (i = [0:NS-1]) color(WIRE) tube(STRG[i], spath(i)[0], spath(i)[3], 2);

// Tuning pins, standing on the block rather than driven into it.
NPSIDE = 5;
function pinrings(i) =
  [ for (z = PINZ)
      [ for (v = [0:NPSIDE-1]) let( g = 180/NPSIDE + 360*v/NPSIDE )
          [ xpin(i) + PINR*cos(g), YPIN[i%4] + PINR*sin(g), z ] ] ];
PING = [ for (i = [0:NS-1]) pinrings(i) ];
for (i = [0:NS-1]) color(IRON)
  tube(PING[i], [xpin(i), YPIN[i%4], PINZ[0]], [xpin(i), YPIN[i%4], PINZ[1]], 2);

// ===================================================================
//  THE KEYBOARD, 88 THIN REPEATS
//  A white key is a tee in plan: full width at the front, cut back
//  between whichever black keys stand beside it.  No vertex of a tee
//  sees the whole of it, so the polygon is rotated to start at the
//  inner corner of the step, which does.
// ===================================================================
ZBK = [-34.0, -6.5];
function wkhasL(k) = tailL(k) + KGAP/2 > wleft(k) + KGAP/2 + 0.01;
function wkhasR(k) = tailR(k) - KGAP/2 < wleft(k) + WK - KGAP/2 - 0.01;
function wkpoly(k) =
  let( xl = wleft(k) + KGAP/2, xr = wleft(k) + WK - KGAP/2,
       xtl = tailL(k) + KGAP/2, xtr = tailR(k) - KGAP/2 )
    concat([ [xl, YKF], [xr, YKF] ],
           wkhasR(k) ? [ [xr, YBK0], [xtr, YBK0] ] : [],
           [ [xtr, YKB], [xtl, YKB] ],
           wkhasL(k) ? [ [xtl, YBK0], [xl, YBK0] ] : []);
// D, G and A have a black key on both sides, and then four of the tee's
// vertices lie on one line at the step.  No fan from any vertex avoids
// putting three of them in one triangle, so the cap is split by hand
// into the front strip and the tail: six triangles for eight vertices,
// which is what a triangulation of an octagon has.
function wkcap(k) =
    wkhasL(k) && wkhasR(k) ? [[0,1,2],[0,2,3],[0,3,6],[0,6,7],[3,4,5],[3,5,6]]
  : wkhasR(k) ? [[0,1,2],[0,2,3],[0,3,4],[0,4,5]]
  : wkhasL(k) ? [[4,5,0],[4,0,1],[4,1,2],[4,2,3]]
  : [[0,1,2],[0,2,3]];
function bkrings(k) =
  let( c = blackx(k), hb = (BKW - KGAP)/2, ht = BKTOP/2 )
  [ [ [c-hb, YBK0, ZBK[0]], [c+hb, YBK0, ZBK[0]],
      [c+hb, YKB,  ZBK[0]], [c-hb, YKB,  ZBK[0]] ],
    [ [c-ht, YBK0+3, ZBK[1]], [c+ht, YBK0+3, ZBK[1]],
      [c+ht, YKB,    ZBK[1]], [c-ht, YKB,    ZBK[1]] ] ];
WKP = [ for (k = [1:88]) if (!isblk(k)) wkpoly(k) ];
WKC = [ for (k = [1:88]) if (!isblk(k)) wkcap(k) ];
BKG = [ for (k = [1:88]) if ( isblk(k)) bkrings(k) ];
for (i = [0:NWK-1]) color(IVORY)
  sweept(przrings(WKP[i], ZKEYB, ZKEYT), WKC[i], 3);
for (G = BKG) color(EBONY) sweep(G, 2);

// ===================================================================
//  KEYBED, KEYSLIP, CHEEKS, FALLBOARD, NAMEBOARD, MUSIC DESK
// ===================================================================
XKW = NWK*WK/2;
// The keybed and the cheek blocks stop at the inner rim's ledge, not at
// the rim's inner face: the ledge is 8 mm proud of it all along the
// straight runs, and a keybed cut to the face buries its corners in it.
XCLR = XTR - SH0 - GAP;
DF = [268, 32]; DT = [292, 152]; TDESK = 18;
DU = unit(DT - DF); DN = [DU[1], -DU[0]];
DESKG = [ for (x = [-450, 450])
            [ for (s = [DF, DF + TDESK*DN, DT + TDESK*DN, DT]) [x, s[0], s[1]] ] ];
color(WOOD) sweep(DESKG, 2);

BOXES = [
  [ [-XCLR, 0, ZKBED[0]],    [XCLR, 400, ZKBED[1]],      WOOD ],  // keybed
  [ [-XKW, 2, -45],          [XKW, 16, -12],             WOOD ],  // keyslip
  [ [ XKW + GAP, 0, ZKBED[1] + GAP], [ XCLR, 250, 2],    WOOD ],  // cheeks
  [ [-XKW - GAP, 0, ZKBED[1] + GAP], [-XCLR, 250, 2],    WOOD ],
  [ [-XKW, 178, -14],        [XKW, 250.5, 0],            WOOD ],  // fallboard
  [ [-XKW, 252, -14],        [XKW, 266, 30],             WOOD ],  // nameboard
  [ [-XTR+GAP, YPB[0], ZPB[0]], [XTR-GAP, YPB[1], ZPB[1]], WOOD ],// pinblock
  [ [-XTR+GAP, 0, ZRB],      [XTR-GAP, 130, -84],        WOOD ],  // front rail
  [ [-135, 175, -640],       [135, 330, -603],           WOOD ],  // lyre box
  [ [-100, 58, -630],        [-64, 173, -618],           IRON ],  // pedals
  [ [ -18, 58, -630],        [  18, 173, -618],          IRON ],
  [ [  64, 58, -630],        [ 100, 173, -618],          IRON ] ];
for (b = BOXES) color(b[2]) box(b[0], b[1]);
V_BOX = sum([ for (b = BOXES) boxvol(b[0], b[1]) ]);

// ===================================================================
//  LEGS, CASTERS, PEDAL LYRE
// ===================================================================
function octa(cx, cy, z, across) =
  let( r = across/2/cos(22.5) )
  [ for (v = [0:7]) let( g = 22.5 + 45*v ) [cx + r*cos(g), cy + r*sin(g), z] ];
ZLGT = ZRB - GAP; ZLGB = ZFLOOR + CAST + GAP;
ISTK = 2 + round((STKPHI - 90)*NPHI/180);
ILEG = 2 + round((LEGPHI - 90)*NPHI/180);
LEGXY = concat([ for (sx = [-1, 1]) [sx*(XTR - 100), LEGY] ],
               [ poff(ILEG, rimW(pph(ILEG))/2) ]);
LEGG = [ for (c = LEGXY) [ octa(c[0], c[1], ZLGB, LEGBOT), octa(c[0], c[1], ZLGT, LEGTOP) ] ];
CASG = [ for (c = LEGXY)
           [ for (z = [ZFLOOR, ZFLOOR + CAST])
               [ for (v = [0:7]) let( g = 22.5 + 45*v )
                   [c[0] + 30*cos(g), c[1] + 30*sin(g), z] ] ] ];
for (G = LEGG) color(WOOD) sweep(G, 3);
for (G = CASG) color(IRON) sweep(G, 3);

LYRG = [ octa(0, 250, -601.5, 52), octa(0, 250, -81.5, 70) ];
color(WOOD) sweep(LYRG, 3);

function barrings(p0, p1, a0, a1) =
  [ [ for (s = [[-a0,-a0],[a0,-a0],[a0,a0],[-a0,a0]]) [p0[0]+s[0], p0[1]+s[1], p0[2]] ],
    [ for (s = [[-a1,-a1],[a1,-a1],[a1,a1],[-a1,a1]]) [p1[0]+s[0], p1[1]+s[1], p1[2]] ] ];
BRACEG = [ for (sx = [-1, 1])
             barrings([sx*112, 322, -599], [sx*112, 392, -84], 11, 8) ];
for (G = BRACEG) color(WOOD) sweep(G, 2);

// ===================================================================
//  THE LID, IN TWO LEAVES, AND THE STICK THAT SETS ITS ANGLE
//  The prop is not drawn at a chosen angle.  A stick of a given length
//  standing in that cup and reaching that socket can only hold the lid
//  at one angle, and it is found by bisection.
// ===================================================================
XLIDR = XTR + rimW(90); XHINGE = XSP - rimW(270);
STKF  = poff(ISTK, rimW(pph(ISTK))/2);
XSOCK = XHINGE + STKFR*(STKF[0] - XHINGE);
STKA = [17, 13];
function zunder(a, x) = ZRT + tan(a)*(x - XHINGE) + GAP/cos(a);
function stktop(a) = [XSOCK, STKF[1], zunder(a, XSOCK - STKA[1]) - GAP];
function stklen(a) = norm(stktop(a) - [STKF[0], STKF[1], ZRT + GAP]);
function fal(lo, hi, n) = n <= 0 ? (lo + hi)/2
  : stklen((lo + hi)/2) < LSTICK ? fal((lo + hi)/2, hi, n - 1)
                                 : fal(lo, (lo + hi)/2, n - 1);
ALPHA = fal(3, 82, 50);
function lidxf(p) = [ XHINGE + (p[0]-XHINGE)*cos(ALPHA) - (p[2]-ZRT)*sin(ALPHA),
                      p[1],
                      ZRT + (p[0]-XHINGE)*sin(ALPHA) + (p[2]-ZRT)*cos(ALPHA) ];
function xfr(G) = [ for (r = G) [ for (p = r) lidxf(p) ] ];
function outer(i) = poff(i, rimW(pph(i)));
LIDM = concat([ [XLIDR, YFOLD] ], [ for (i = [2:NP-2]) outer(i) ], [ [XHINGE, YFOLD] ]);
LIDF = [ [XHINGE, YFOLD], [XLIDR, YFOLD], [XLIDR, 2*YFOLD], [XHINGE, 2*YFOLD] ];
LIDMG = xfr(przrings(LIDM, ZRT + GAP, ZRT + GAP + TLID));
LIDFG = xfr(przrings(LIDF, ZRT + 2*GAP + TLID, ZRT + 2*GAP + 2*TLID));
color(WOOD) sweep(LIDMG, 4);
color(WOOD) sweep(LIDFG, 3);
STICKG = barrings([STKF[0], STKF[1], ZRT + GAP], stktop(ALPHA), STKA[0], STKA[1]);
color(WOOD) sweep(STICKG, 2);

// ===================================================================
//  WHAT THE GEOMETRY SAYS
// ===================================================================
// Nothing below draws anything.  It measures what was drawn.

// Where a bass string passes over the long bridge and over the tenor
// strings it crosses, and by how much it clears them.
function bxbr(i, k) =
  let( T = sterm(i), d = sdir(i), B = nbrg(k), s = (B - T)*d )
    s > 0 && s < slen(sk(i)) && norm(B - T - s*d) < WBRL[0]/2 + 5
      ? zterm(sk(i)) + s*tan(ANGF) - dout(sk(i))/2 - brgtop(k) : 1e6;
XBRIDGE = min([ for (i = [0:NBASS-1]) min([ for (k = [27:88]) bxbr(i, k) ]) ]);
function bxst(i, j) =
  let( a = sterm(i), u = sdir(i), b = sterm(j), v = sdir(j),
       den = u[0]*v[1] - u[1]*v[0] )
    abs(den) < 1e-9 ? 1e6
  : let( w = b - a, si = (w[0]*v[1] - w[1]*v[0])/den,
                    sj = (w[0]*u[1] - w[1]*u[0])/den )
      si > 0 && si < slen(sk(i)) && sj > 0 && sj < slen(sk(j))
        ? (zterm(sk(i)) + si*tan(ANGF) - dout(sk(i))/2)
        - (zterm(sk(j)) + sj*tan(ANGF) + dout(sk(j))/2) : 1e6;
XSTRING = min([ for (i = [0:NBASS-1])
                  min([ for (j = [NBASS:NS-1]) bxst(i, j) ]) ]);
NXING   = sum([ for (i = [0:NBASS-1])
                  sum([ for (j = [NBASS:NS-1]) bxst(i, j) < 1e5 ? 1 : 0 ]) ]);

// Every hitch pin's distance inboard of the rim's inner face, and
// whether it lands on the plate's hitch rail there.
function hdist(P) = [ for (i = [2:NP-2]) (pp(i) - P)*pn(i) ];
HCLR  = [ for (i = [0:NS-1]) min(hdist(shit(i))) ];

// Volumes, taken from the same vertex arrays that were handed to
// polyhedron() and split into triangles the same way.
V_RIM  = swvolt(RIMG, RIMCAP);
V_SB   = przvol(BPOLY, ZLT + GAP, ZLT + GAP + TSB);
V_RIB  = sum([ for (r = [0:NRIB-1]) RIBOK[r] ? swvol(RIBG[r]) : 0 ]);
V_BRL  = swvol(BRAILG);
V_LB   = swvol(LBG);  V_BB = swvol(BBG);
V_HR   = swvol(HRG) + swvol(HBG);  V_FB = swvol(FBG);
V_STR  = sum([ for (i = [0:NS-1]) tubevol(STRG[i], spath(i)[0], spath(i)[3]) ]);
V_PIN  = sum([ for (i = [0:NS-1])
                 tubevol(PING[i], [xpin(i), YPIN[i%4], PINZ[0]],
                                  [xpin(i), YPIN[i%4], PINZ[1]]) ]);
V_WK   = sum([ for (i = [0:NWK-1]) przvol(WKP[i], ZKEYB, ZKEYT) ]);
V_BK   = sum([ for (G = BKG) swvol(G) ]);
V_DSK  = swvol(DESKG);
V_LEG  = sum([ for (G = LEGG) swvol(G) ]) + sum([ for (G = CASG) swvol(G) ]);
V_LYR  = swvol(LYRG) + sum([ for (G = BRACEG) swvol(G) ]);
V_LID  = swvol(LIDMG) + swvol(LIDFG) + swvol(STICKG);
V_ALL  = V_RIM + V_SB + V_RIB + V_BRL + V_LB + V_BB + V_HR + V_FB
       + V_STR + V_PIN + V_WK + V_BK + V_BOX + V_DSK + V_LEG + V_LYR + V_LID;

// Two closed forms that do not go through the mesh at all.  A mitred
// tube is a chain of prisms between planes through its own joints, so
// its volume is exactly the section area times the axis length.
ASTR = [ for (i = [0:NS-1]) 0.5*NSIDE*sq(dout(sk(i))/2)*sin(360/NSIDE) ];
LSTR = [ for (i = [0:NS-1]) let( P = spath(i) )
           norm(P[1]-P[0]) + norm(P[2]-P[1]) + norm(P[3]-P[2]) ];
V_STR_C = sum([ for (i = [0:NS-1]) ASTR[i]*LSTR[i] ]);
V_PIN_C = NS*0.5*NPSIDE*sq(PINR)*sin(360/NPSIDE)*(PINZ[1] - PINZ[0]);

NRIBOK = sum([ for (r = [0:NRIB-1]) RIBOK[r] ? 1 : 0 ]);
//        rim board ribs   bellyrail bridges rails bearing strings+pins
NSOLID = 1 +  1 + NRIBOK +    1     +   2   +  2  +   1   +   2*NS
//        keys  desk boxes       legs+casters post braces lid stick
       +   88 +  1 + len(BOXES) +  6        +  1  +  2   + 2 +  1;
CASELEN = max([ for (i = [0:NP]) pp(i)[1] + rimW(pph(i))*pn(i)[1] ]);
CASEW   = XLIDR - XHINGE;

echo("=== SCALE ===");
echo("L = c/2f with c = sqrt(sigma/(beta rho)); plain wire has beta = 1.");
echo("speaking length  A0", slen(1), "mm   A4", slen(49), "mm   C8", slen(88), "mm");
echo("plain scale falls", RT, "per octave, not 2; bass falls", RB,
     "per octave below the break at key", NB);
echo("the ideal halving law from C8 asks for", LTOP*pow(2, 87/12),
     "mm at A0; at", RT, "per octave it still asks", LTOP*pow(RT, 87/12), "mm");
echo("a plain wire at", SIGB, "MPa sounding A0 would be",
     sqrt(SIGB/RHO/1e3)/(2*freq(1))*1000, "mm;",
     "the winding gives beta", beta(1), "and sqrt(beta) is", sqrt(beta(1)),
     "which is exactly", slen(1)*sqrt(beta(1)), "mm back");
echo("working stress MPa   A0", sigma(1), "  lowest plain string", sigma(NB+1),
     "  A4", sigma(49), "  C8", sigma(88),
     "-- the plain scale is bought with stress, and it is the top notes",
     "that work nearest their breaking strength");
echo("winding factor beta: 1 everywhere above the break,", beta(NB),
     "just below it and", beta(1), "at A0, so the topmost wound string is",
     "barely wound and the bottom one carries nine times its core's mass");
echo("core diameter mm     A0", dcore(1), "  A4", dcore(49), "  C8", dcore(88));
echo("overall diameter at A0", dout(1), "mm, the core plus enough copper",
     "to multiply its mass per length by", beta(1));
echo("inharmonicity B      A0", inharm(1), "  A4", inharm(49), "  C8", inharm(88));
echo("B goes as 1/L^2 at fixed core and tension, so shortening A0 by",
     sqrt(beta(1)), "multiplies its inharmonicity by", beta(1));
echo("strings", NS, "=", NSING, "single +", NB - NSING, "double +", 88 - NB,
     "triple; tension", TEN, "N each, total", NS*TEN/1000, "kN =",
     NS*TEN/9806.65, "tonnes force");
echo("the pin field is an output, not an input: it lands between x",
     min([ for (i = [0:NS-1]) xpin(i) ]), "and", max([ for (i = [0:NS-1]) xpin(i) ]),
     "with the closest two pins in one row", 
     min([ for (i = [0:NS-5]) abs(xpin(i+4) - xpin(i)) ]), "mm apart,",
     "against a pin", 2*PINR, "mm across");

echo("=== KEYBOARD ===");
echo("octave", OCT, "mm; white key", WK, "mm; 52 white keys span", NWK*WK, "mm");
echo("black key width b", BKW, "; sharps sit off the white boundary by",
     "b/6 =", BKW/6, "in the two-key group and b/4 =", BKW/4, "in the three");
echo("white tails: C D E get w - 2b/3 =", WK - 2*BKW/3,
     " F G A B get w - 3b/4 =", WK - 3*BKW/4, "mm");
echo("identities 3(w-2b/3)+2b - 3w =", 3*(WK - 2*BKW/3) + 2*BKW - 3*WK,
     " and 4(w-3b/4)+3b - 4w =", 4*(WK - 3*BKW/4) + 3*BKW - 4*WK);
echo("measured tails: D", tailR(42) - tailL(42), " G", tailR(47) - tailL(47),
     " (D is a two-key group, G a three-key group)");
BNOM = [ WK - BKW/6, 2*WK + BKW/6, 4*WK - BKW/4, 5*WK, 6*WK + BKW/4 ];
BPCS = [1, 3, 6, 8, 10];
BDEV = [ for (j = [0:4]) BNOM[j] - (BPCS[j] + 0.5)*OCT/12 ];
echo("sharp centres from the left edge of C", BNOM);
echo("an even twelfth of the octave would put them at",
     [ for (j = [0:4]) (BPCS[j] + 0.5)*OCT/12 ], "so they are out by", BDEV,
     "mm, the largest", max([ for (d = BDEV) abs(d) ]), "mm at F#");

echo("=== RIM ===");
echo("spiral decay", SPIA, "per radian: the bending radius halves every",
     ln(2)/SPIA/RAD, "degrees of turn");
echo("radius at the cheek", RCHEEK, "mm, falling to the tail radius", RTAIL,
     "after", PHIB - 90, "degrees; the tail arc then turns", 270 - PHIB);
echo("spine straight for", spend(SPIA, PHIB)[1], "mm of a case", CASELEN,
     "mm long and", CASEW, "mm wide");
echo("tightest clearance from the inner face to a hitch pin", min(HCLR),
     "mm, at key", sum([ for (i = [0:NS-1]) HCLR[i] == min(HCLR) ? sk(i) : 0 ]),
     "-- the rim is fitted to the longest string and to nothing else");
echo("deepest hitch pin inboard of the face", max(HCLR),
     "mm, which is why the hitch rail follows the pins and not the rim");
echo("rim width runs", WCHEEK, "mm at the cheek to", WSPINE, "at the spine;",
     "the soundboard ledge", SH0, "mm at the open ends to", SH1, "behind the belly rail");

echo("=== SOUNDBOARD ===");
echo("plan area", ABOARD/1e6, "m2 on a case plan of",
     shoelace(concat([ for (i = [0:NP]) pp(i) ]))/1e6, "m2 inside the rim");
echo("thickness", TSB, "mm;", NRIBOK, "of", NRIB,
     "rib stations carry a rib, spaced", RIBSP, "mm, running at", RIBA,
     "degrees; a station whose chord is shorter than its own two end",
     "ramps carries nothing");
echo("rib chords mm", [ for (r = [0:NRIB-1])
        ribhi(ribq(r), RIBU) - riblo(ribq(r), RIBU) ]);
echo("long bridge spans x", xlb(27), "to", xlb(88),
     "; bass bridge", xbb(1), "to", xbb(NB));

echo("=== OVERSTRINGING ===");
echo(NXING, "crossings of a bass string over a plain one; the bass clears",
     "the tenor strings by", XSTRING, "mm and the long bridge by", XBRIDGE, "mm");

echo("=== SOLIDS AND VOLUME ===");
echo("solids", NSOLID, "; no two share any volume, so the exported mesh is",
     "the mesh written here and the total below is a prediction");
echo("mm3 -- rim", V_RIM, "soundboard", V_SB, "ribs", V_RIB, "belly rail", V_BRL);
echo("mm3 -- long bridge", V_LB, "bass bridge", V_BB, "hitch rail", V_HR,
     "front bearing", V_FB);
echo("mm3 -- strings", V_STR, "tuning pins", V_PIN, "white keys", V_WK,
     "black keys", V_BK);
echo("mm3 -- case boxes", V_BOX, "music desk", V_DSK, "legs", V_LEG,
     "lyre", V_LYR, "lid and stick", V_LID);
echo("predicted total", V_ALL, "mm3 =", V_ALL/1e9, "m3;",
     "its last six digits, since echo prints only six:",
     V_ALL - floor(V_ALL/1e6)*1e6);
echo("strings by closed form (area x mitred axis length)", V_STR_C,
     "against the mesh", V_STR, "difference", V_STR - V_STR_C);
echo("pins by closed form", V_PIN_C, "against the mesh", V_PIN,
     "difference", V_PIN - V_PIN_C);
echo("lid propped at", ALPHA, "degrees by a stick", stklen(ALPHA),
     "mm long standing", STKF[0], "mm out from the centreline");
