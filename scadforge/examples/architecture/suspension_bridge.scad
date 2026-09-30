// ===================================================================
//  A suspension bridge, taken from its own statics
//
//  The main cable of a suspension bridge is a PARABOLA, not a catenary.
//  The two shapes answer two different questions.  A cable hanging under
//  its own weight carries a load that is uniform per unit of its own ARC
//  length, and the resulting shape is the catenary.  A cable carrying a
//  deck through closely spaced hangers carries a load that is uniform per
//  unit of HORIZONTAL length, and that is the parabola.  The derivation
//  is two lines.  The horizontal component H of the cable tension is the
//  same everywhere, because nothing pushes the cable sideways; vertical
//  equilibrium of an element then reads
//
//      d/dx ( H dz/dx ) = w        ->        z'' = w/H,
//
//  and with w and H both constant that integrates twice to z = w x^2/(2H).
//  So the sag is quadratic in x, the curvature is constant, and the whole
//  cable is one parabola whose only parameter is w/H.
//
//  This model uses the parabola, and then checks the decision instead of
//  asserting it.  The cable's own steel is not negligible here: it is
//  about a quarter of the load, and IT is distributed per unit arc.  So
//  the true load per unit horizontal length is
//
//      w(x) = w_deck + w_cable * sec(theta(x)),
//
//  which is largest at the towers, where the cable is steepest.  The
//  echoes below integrate that load once from the parabola's own slope
//  (one Picard step, which is accurate to second order in the correction)
//  and report the largest departure from the parabola.  It comes out
//  around 0.15 m on a sag of 139.5 m: one part in nine hundred.  That is
//  the licence to draw a parabola, quantified rather than assumed.
//
//  WHAT IS DERIVED FROM WHAT
//
//  The inputs are the span, the side span, the tower height above the
//  deck, the cable's clearance over the deck at midspan, and the deck's
//  camber.  Everything else follows.
//
//      deck top        z_d(x)  = CROWN (1 - (x/XEND)^2)
//      camber          CAMBER  = z_d(0) - z_d(XT)
//      sag             SAG     = TOWER - CLEAR - CAMBER
//      cable           z_c(x)  = z_d(0) + CLEAR + SAG (x/XT)^2
//
//  The sag is what is left of the tower height once the cable has been
//  given its clearance over a cambered deck.  Raise the camber and the
//  sag falls by the same amount; the tower does not grow to accommodate
//  it.  The hanger lengths then fall out as the gap between two
//  parabolas,
//
//      l(x) = CLEAR + SAG (x/XT)^2 + CROWN (x/XEND)^2,
//
//  and at the tower that collapses to exactly TOWER, because the camber
//  term there is precisely CAMBER.  That identity is echoed as a check on
//  the algebra: no hanger length is measured off anything, and the
//  longest one is the tower height itself.
//
//  THE SIDE SPANS ARE NOT A FREE CHOICE
//
//  A saddle that is free to slide on the tower top transmits no
//  horizontal force, so H is the SAME on both sides of it.  Since the
//  deck load per horizontal metre is also the same, z'' = w/H is the
//  same, and the side-span cable is a parabola with the IDENTICAL
//  coefficient as the main span.  Its shape is then fixed by two points:
//  the saddle and the anchorage.  Nothing about its curvature was chosen.
//  The consequence is echoed: the horizontal pulls of the two arms cancel
//  exactly at the saddle, so the tower carries a purely vertical load and
//  no bending.  That is why the arrangement is built this way.
//
//  THE CABLE IS DRAWN AS A POLYGON BECAUSE IT IS ONE
//
//  A cable loaded at discrete points is straight between them: it is a
//  funicular polygon, and the parabola is only its limit as the panels
//  shrink.  So the drawn tube runs from hanger point to hanger point,
//  with the panel points ON the parabola, and the echo reports how much
//  shorter the polygon is than the true arc (about 8 mm in 2058 m).
//  Every joint is mitred: the ring at each panel point lies in the plane
//  bisecting the two tangents.  Because the cable's path is planar and
//  the ring's reference direction is the transverse horizontal, the mitre
//  ellipse is the SAME curve whether it is projected from the incoming
//  cylinder or the outgoing one -- reflection in the bisecting plane
//  exchanges the two cylinders and fixes that plane pointwise.  So the
//  tube is watertight at a 53-degree knuckle without a fillet, and each
//  segment is a genuine prism between two planes through its end points,
//  which is what makes its volume exactly A times the axis length.
//
//  HOLLOW MEANS ONE POLYHEDRON
//
//  The steel members here are shells, not solids: the box girder, the
//  tower legs and the tower struts are each the region between two
//  surfaces, closed by an annulus at both ends.  That is a single closed
//  genus-one polyhedron, built as one polyhedron() call, with no boolean
//  anywhere.  The inner surface is a true constant-thickness inward
//  offset of the outer one, taken at each vertex along the angle bisector
//  scaled by 1/(1 + n1.n2), which is exact for any convex corner.  The
//  concrete members -- the piers and the anchorages -- are solid, because
//  concrete is.
//
//  The girder is swept by pure TRANSLATION, not by a frame that follows
//  the grade.  A cambered deck's cross sections are plumb, not normal to
//  the road surface, so translation is the physically right sweep, and it
//  has a useful side effect: every section is the same area in the same
//  plane, so the girder's volume is exactly its steel area times its
//  length, independent of the camber.  That is the closed form the volume
//  audit is checked against.
//
//  NO TWO OF THE 501 SOLIDS SHARE A CUBIC MILLIMETRE
//
//  Which took arranging, and is the reason for several dimensions that
//  would otherwise look arbitrary.  The tower legs stand clear of the
//  girder in y, and their struts stop just short of the legs' inner
//  faces rather than being buried in them.  The hanger planes are
//  outboard of the girder, so each hanger lands on a cantilever bracket
//  that reaches out past the fascia to meet it, and the hanger's foot
//  stops on the bracket's top rather than socketing into it.  The leg
//  tops are not placed at all: each is computed as the lowest the
//  cable's underside gets anywhere over that leg's own footprint, which
//  is the saddle knuckle dropped by the steeper of the two cable slopes
//  times the leg's half width.  Everywhere two members would meet,
//  GAP = 20 mm of air is left between them; over a 1952 m deck that is
//  one part in a hundred thousand, and it buys two things.  The exported
//  mesh is then exactly the mesh written here, face for face, so the
//  winding can be checked directly instead of through whatever a union
//  leaves behind.  And the volume audit at the foot of the file becomes
//  a prediction rather than an estimate: the sum of the closed forms is
//  what the exporter must measure, and any difference at all is a
//  finding.
//
//  WINDING.  polyhedron() wants each face listed so that the right-hand
//  normal points INTO the solid.  For a swept ring the rule used here is:
//  with sweep direction t and section axes (e1, e2) forming a
//  right-handed triple (e1, e2, t), list the ring counter-clockwise in
//  (e1, e2).  The girder sweeps along x with section axes (y, z); the
//  legs sweep along z with (x, y); the struts sweep along y with (z, x).
//  All three are right-handed, so all three take the same face template.
//  The end caps must then traverse the ring the OTHER way round from the
//  wall quads that meet them, which is the orientation-blind test that
//  actually catches mistakes: two faces sharing an edge must traverse it
//  in opposite directions.
//
//  The dimensions are a coherent design, not a copy of a particular
//  bridge.  They are in the region a 1280 m span puts you in, and the
//  echoed cable stress and cable mass are there so a reader can see that
//  they land where a real bridge lands rather than merely looking like it.
// ===================================================================

// ---- the five inputs everything else comes from ---------------------
SPAN   = 1280;      // main span, tower centre to tower centre, m
SIDE   = 336;       // side span, tower centre to anchorage face, m
PANEL  = 16;        // hanger spacing, m; divides both spans exactly
TOWER  = 152;       // saddle above the deck at the tower, m
CLEAR  = 6;         // cable above the deck crown at midspan, m
CROWN  = 15;        // deck rise from the anchorages to midspan, m
ZANCH  = 9;         // cable height where it enters the anchorage, m

// ---- girder section -------------------------------------------------
WT2    = 11.00;     // top half width, m
WB2    = 7.50;      // soffit half width, m
DEEP   = 4.50;      // structural depth, m
FASCIA = 1.20;      // depth of the vertical edge face, m
TPL    = 0.020;     // equivalent plate thickness of the shell, m

// ---- cable and hangers ----------------------------------------------
RCAB   = 0.44;      // wrapped cable radius, m
NCAB   = 10;        // sides of the drawn cable
PACK   = 0.80;      // steel fraction of the wrapped circle
YCAB   = 13.60;     // cable plane, m either side of the centreline
RHANG  = 0.13;      // hanger rope radius, m
NHANG  = 8;         // sides of a drawn hanger
ROPE   = 0.75;      // metallic fraction of a rope
GAP    = 0.02;      // erection gap between any two members, m

BRKX   = 0.36;      // bracket thickness along the bridge, m
BRKY   = 14.30;     // bracket reach, m from the centreline

// ---- towers and foundations -----------------------------------------
LEGY0  = 11.70;     // leg inner face, m   (girder edge is at 11.00)
LEGY1  = 15.50;     // leg outer face, m
HXBOT  = 6.00;      // leg half width along the bridge, at the pier, m
HXTOP  = 3.20;      // and at the top, m
TLEG   = 0.14;      // leg shell thickness, m
HXSAD  = 1.00;      // saddle block half width along the bridge, m
CLRG   = 0.15;      // clearance under the cable, m
HXSTR  = 2.40;      // strut half width along the bridge, m
DSTR   = 2.40;      // strut depth, m
TSTR   = 0.10;      // strut shell thickness, m
STRZ   = [0, 72, 146];   // strut soffits, m
ZPIERT = -14;       // pier top, m
ZPIERB = -18;       // pier underside, m
PIERX  = 8.50;      // pier half length, m
PIERY0 = 10.50;     // pier inner face, m
PIERY1 = 16.70;     // pier outer face, m

// ---- anchorages -----------------------------------------------------
BLKX   = 60;        // anchorage block length, m
BLKY0  = 11.50;     // inner face, m
BLKY1  = 22.50;     // outer face, m
BLKTOP = 12;        // top, m
MU     = 0.60;      // sliding friction under the block
RHOC   = 24.0;      // reinforced concrete, kN/m3
GAM    = 78.5;      // steel, kN/m3
WSUP   = 100;       // surfacing, furniture and live load, kN/m
UTS    = 1570000;   // bridge wire tensile strength, kPa

STEEL = [0.62, 0.66, 0.70];
CABLE = [0.86, 0.80, 0.62];
ROAD  = [0.42, 0.44, 0.47];
CONC  = [0.72, 0.70, 0.66];

// ---- helpers the kernel does not have -------------------------------
function sq(t) = t*t;
function asinh_(u) = ln(u + sqrt(u*u + 1));

// A halving sum.  A linear recursion over 400 terms would walk straight
// into the parser's depth guard; halving makes the depth logarithmic.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

// ---- the geometry, solved ------------------------------------------
XT   = SPAN/2;
XEND = XT + SIDE;
NS   = round(2*XEND/PANEL);      // panels end to end
JTL  = round(SIDE/PANEL);        // station index of the left tower
JTR  = NS - JTL;                 // and the right
function xs(j) = -XEND + PANEL*j;

function zdeck(x) = CROWN*(1 - sq(x/XEND));
CAMBER = zdeck(0) - zdeck(XT);
SAG    = TOWER - CLEAR - CAMBER;
A      = SAG/sq(XT);             // z'' / 2 = w / 2H
ZC0    = zdeck(0) + CLEAR;       // cable at midspan
ZCT    = zdeck(XT) + TOWER;      // cable at the saddle

// Same H both sides of a free saddle, so the same A.  Two points then
// fix the side-span parabola completely.
MS     = (ZANCH - ZCT - A*sq(SIDE))/SIDE;
function zcab(x) =
  abs(x) <= XT ? ZC0 + A*sq(x)
               : ZCT + MS*(abs(x) - XT) + A*sq(abs(x) - XT);

MMAIN = 2*A*XT;                  // cable slope at the tower, main arm
MSIDE = -MS;                     // and side arm, as a positive number
MEND  = -(MS + 2*A*SIDE);        // slope where it enters the anchorage
MSTEEP = max(MMAIN, MSIDE);

// Hanger stations: every panel point except the towers and the ends.
HJ = [ for (j = [1 : NS-1]) if (j != JTL && j != JTR) j ];
function hlen(x) = zcab(x) - zdeck(x);

// ---- 2D section helpers --------------------------------------------
function shoelace(P) = let( n = len(P) )
  0.5*sum([ for (i = [0:n-1]) P[i][0]*P[(i+1)%n][1] - P[(i+1)%n][0]*P[i][1] ]);

// Inward unit normal of the edge leaving vertex i of a CCW polygon.
function enorm(P, i) = let( n = len(P), e = P[(i+1)%n] - P[i] )
  [-e[1], e[0]]/norm(e);

// The offset vertex is where the two offset edges meet.  For inward
// normals n1, n2 that point is p + t (n1 + n2)/(1 + n1.n2): exact at a
// right angle, exact along a straight edge, exact everywhere convex.
function offset_in(P, t) =
  [ for (i = [0:len(P)-1])
      let( n = len(P), n1 = enorm(P, (i-1+n)%n), n2 = enorm(P, i) )
        P[i] + t*(n1 + n2)/(1 + n1*n2) ];

// ---- the three solid builders --------------------------------------
// A shell between two swept rings, closed by an annulus at each end.
// One closed polyhedron of genus one.  Wall quads on the outside are
// wound as the sweep template; the inner ones are reversed, because the
// solid lies outside the inner surface.
module shell(OG, IG, conv = 8) {
    K = len(OG[0]); M = len(OG) - 1; B = (M+1)*K;
    polyhedron(
      points = concat([ for (u = [0:M]) each OG[u] ],
                      [ for (u = [0:M]) each IG[u] ]),
      faces = concat(
        [ for (u = [0:M-1]) for (v = [0:K-1])
            [ u*K+v, (u+1)*K+v, (u+1)*K+(v+1)%K, u*K+(v+1)%K ] ],
        [ for (u = [0:M-1]) for (v = [0:K-1])
            [ B+u*K+v, B+u*K+(v+1)%K, B+(u+1)*K+(v+1)%K, B+(u+1)*K+v ] ],
        [ for (v = [0:K-1]) [ v, (v+1)%K, B+(v+1)%K, B+v ] ],
        [ for (v = [0:K-1]) [ M*K+(v+1)%K, M*K+v, B+M*K+v, B+M*K+(v+1)%K ] ]),
      convexity = conv);
}

// A solid swept tube, capped by a fan to a centre point at each end.
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

// An axis-aligned block, corners in any order.
module box(p, q) {
    a = [min(p[0],q[0]), min(p[1],q[1]), min(p[2],q[2])];
    b = [max(p[0],q[0]), max(p[1],q[1]), max(p[2],q[2])];
    polyhedron(
      points = [ [a[0],a[1],a[2]], [b[0],a[1],a[2]], [b[0],b[1],a[2]], [a[0],b[1],a[2]],
                 [a[0],a[1],b[2]], [b[0],a[1],b[2]], [b[0],b[1],b[2]], [a[0],b[1],b[2]] ],
      faces = [ [0,1,2,3], [4,7,6,5], [0,4,5,1], [3,2,6,7], [0,3,7,4], [1,5,6,2] ],
      convexity = 2);
}

// ---- the girder -----------------------------------------------------
// CCW in (y, z): along the soffit, up the edge, back along the deck.
SEC  = [ [-WT2,-FASCIA], [-WB2,-DEEP], [WB2,-DEEP],
         [ WT2,-FASCIA], [ WT2,0],     [-WT2,0] ];
SECI = offset_in(SEC, TPL);
AOUT = shoelace(SEC);
AIN  = shoelace(SECI);
ADECK = AOUT - AIN;

function gring(sec, x) = [ for (p = sec) [x, p[0], zdeck(x) + p[1]] ];

color(ROAD)
  shell([ for (j = [0:NS]) gring(SEC,  xs(j)) ],
        [ for (j = [0:NS]) gring(SECI, xs(j)) ], 4);

// ---- the cable ------------------------------------------------------
// The path, its tangents, and the bisecting plane at every panel point.
// At the two ends the plane is x = const, so the tube is cut off square
// against the anchorage face it dies into.
function cp(j) = [xs(j), 0, zcab(xs(j))];
function tg(j) = let( d = cp(j+1) - cp(j) ) d/norm(d);
function mplane(j) = j == 0 || j == NS ? [1,0,0]
                   : let( s = tg(j-1) + tg(j) ) s/norm(s);
function tref(j) = j == NS ? tg(NS-1) : tg(j);

// The ring: a circle about the transverse horizontal, slid along the
// tangent until it lies in the bisecting plane.  The phase puts a vertex
// exactly at the bottom, which is where the hangers meet it.
function cring(j, y) =
  let( t = tref(j), b = cross(t, [0,1,0]), m = mplane(j) )
  [ for (k = [0:NCAB-1])
      let( g = 180/NCAB + 360*k/NCAB,
           u = cos(g)*[0,1,0] + sin(g)*b,
           s = -RCAB*(u*m)/(t*m) )
        [xs(j), y, zcab(xs(j))] + RCAB*u + s*t ];

module main_cable(y) {
    tube([ for (j = [0:NS]) cring(j, y) ],
         [xs(0), y, zcab(xs(0))], [xs(NS), y, zcab(xs(NS))], 6);
}
for (sg = [-1, 1]) color(CABLE) main_cable(sg*YCAB);

// ---- hangers and their brackets -------------------------------------
// The top of a hanger is the lowest the cable's drawn underside reaches
// anywhere over that hanger's own footprint, less a relief, so the two
// solids never share volume.
//
// "Over its own footprint" is the whole of it, and taking the ring's
// lowest VERTEX instead is wrong by an amount that grows with the cable's
// slope.  The ring is perpendicular to the axis, so its bottom vertex sits
// RCAB*sin(slope) along the cable from the panel point -- 0.21 m where the
// cable is steepest -- and the tube's underside directly over the hanger
// is lower than that vertex by RCAB*sin^2/cos.  Measured against the
// exported mesh, hanger tops were inside the cable by up to 97 mm near the
// towers, and only at midspan, where the cable is flat, was the 20 mm the
// header promises actually there.  The union kernel found it: 96 hangers
// and one cable came back as ONE connected component instead of 97
// disjoint solids.
//
// The underside is read off the same vertices the mesh is drawn from, so
// this is exact for the drawn tube rather than exact for the ideal one.
// CBK is the index of the ring vertex at the bottom, which exists because
// NCAB is even and the ring's phase puts g = 270 in its set.
CBK = (270*NCAB - 180)/360;
function cbot(j) = cring(j, YCAB)[CBK];

// z of the cable's bottom edge at x, on whichever segment spans it.
function bz(x) =
  min([ for (i = [0 : NS-1])
          let( p = cbot(i), q = cbot(i+1) )
            if (p[0] <= x && x <= q[0])
              p[2] + (q[2] - p[2])*(x - p[0])/(q[0] - p[0]) ]);

// The bottom edge sags, so it is convex, and the lowest it gets over an
// interval is at one end of the interval unless a vertex of the polyline
// falls inside -- which happens at midspan, where the low point is.  Both
// cases are taken.  The footprint uses RHANG rather than the prism's true
// half width RHANG*cos(180/NHANG), which is conservative by 8%.
function hangtop(j) =
  let( a = xs(j) - RHANG, b = xs(j) + RHANG )
    min(concat([ bz(a), bz(b) ],
               [ for (i = [0:NS]) if (a < cbot(i)[0] && cbot(i)[0] < b) cbot(i)[2] ]))
    - GAP;
function hangbot(j) = zdeck(xs(j)) + GAP;

module prism(cx, cy, z0, z1, n, r, ph) {
    G = [ for (u = [0:1])
            [ for (k = [0:n-1]) let( g = ph + 360*k/n )
                [cx + r*cos(g), cy + r*sin(g), u == 0 ? z0 : z1] ] ];
    tube(G, [cx,cy,z0], [cx,cy,z1], 2);
}

for (j = HJ) for (sg = [-1, 1]) {
    color(CABLE) prism(xs(j), sg*YCAB, hangbot(j), hangtop(j),
                       NHANG, RHANG, 180/NHANG);
    color(STEEL) box([xs(j) - BRKX/2, sg*(WT2 + GAP), zdeck(xs(j)) - FASCIA],
                     [xs(j) + BRKX/2, sg*BRKY,          zdeck(xs(j))]);
}

// ---- towers ---------------------------------------------------------
// The saddle knuckle's underside, and how far the cable has fallen by
// the time it leaves the leg's footprint and the saddle block's.
ZKNUCK = min([ for (v = cring(JTR, YCAB)) v[2] ]);
LEGTOP = ZKNUCK - HXTOP*MSTEEP - CLRG;
SADTOP = ZKNUCK - HXSAD*MSTEEP - CLRG;

// CCW in (x, y), swept along z, so ya < yb always.  Mirroring a leg to
// negative y reverses orientation, which is exactly why the two faces of
// the pair are passed as (-LEGY1, -LEGY0) rather than negated in place.
function legsec(hx, ya, yb) = [ [-hx,ya], [hx,ya], [hx,yb], [-hx,yb] ];
function lring(sec, xc, z) = [ for (p = sec) [xc + p[0], p[1], z] ];

// CCW in (z, x), swept along y: (z, x, y) is right-handed.
function strutsec(z0, z1) = [ [z0,-HXSTR], [z1,-HXSTR], [z1,HXSTR], [z0,HXSTR] ];
function sring(sec, xc, y) = [ for (p = sec) [xc + p[1], y, p[0]] ];

module tower(xc) {
    for (sg = [-1, 1])
      let( ya = sg > 0 ? LEGY0 : -LEGY1, yb = sg > 0 ? LEGY1 : -LEGY0 ) {
        color(CONC) box([xc - PIERX, sg*PIERY0, ZPIERB],
                        [xc + PIERX, sg*PIERY1, ZPIERT]);
        color(STEEL)
          shell([ lring(legsec(HXBOT, ya, yb), xc, ZPIERT + GAP),
                  lring(legsec(HXTOP, ya, yb), xc, LEGTOP) ],
                [ lring(offset_in(legsec(HXBOT, ya, yb), TLEG), xc, ZPIERT + GAP),
                  lring(offset_in(legsec(HXTOP, ya, yb), TLEG), xc, LEGTOP) ], 6);
        color(STEEL) box([xc - HXSAD, ya, LEGTOP + GAP], [xc + HXSAD, yb, SADTOP]);
      }
    for (z0 = STRZ)
      color(STEEL)
        shell([ sring(strutsec(z0, z0 + DSTR), xc, -(LEGY0 - GAP)),
                sring(strutsec(z0, z0 + DSTR), xc,   LEGY0 - GAP) ],
              [ sring(offset_in(strutsec(z0, z0 + DSTR), TSTR), xc, -(LEGY0 - GAP)),
                sring(offset_in(strutsec(z0, z0 + DSTR), TSTR), xc,   LEGY0 - GAP) ], 6);
}
module tower_pair() { for (sx = [-1, 1]) tower(sx*XT); }

// ---- anchorages -----------------------------------------------------
// A gravity block, sized by the sliding check and nothing else.  The
// cable leaves it pulling inward with H and UPWARD with H tan(theta), so
// the uplift subtracts from the friction: W >= H/mu + H tan(theta).
// Passive pressure on the buried faces is ignored, which is why the
// block comes out as large as it does.
SUMELL  = sum([ for (j = HJ) hlen(xs(j)) ]);
AROPE   = PI*sq(RHANG)*ROPE;
AHDRAWN = 0.5*NHANG*sq(RHANG)*sin(360/NHANG);
ACDRAWN = 0.5*NCAB*sq(RCAB)*sin(360/NCAB);
ACSTEEL = PI*sq(RCAB)*PACK;
VBRK    = BRKX*(BRKY - WT2 - GAP)*FASCIA;

WGIRD = ADECK*GAM;
WBRK  = 2*VBRK*GAM/PANEL;
WHANG = 2*AROPE*SUMELL*GAM/(2*XEND);
WCAB  = 2*ACSTEEL*GAM;                    // per unit ARC of cable
WDK   = WGIRD + WBRK + WHANG + WSUP;      // per unit HORIZONTAL length

// One Picard step: integrate the true load, using the parabola's own
// slope for sec(theta).  Prefix sums, so H y' and H y are range sums over
// lists that already exist rather than a fresh recursion per station.
NQ  = 80;
HQ  = XT/NQ;
WM  = [ for (i = [0:NQ-1]) WDK + WCAB*sqrt(1 + sq(2*A*(i + 0.5)*HQ)) ];
PL  = [ for (i = [0:NQ]) sum(WM, 0, i)*HQ ];
TRP = [ for (i = [0:NQ-1]) (PL[i] + PL[i+1])/2*HQ ];
YL  = [ for (i = [0:NQ]) sum(TRP, 0, i) ];
HTOT = YL[NQ]/SAG;                        // both cables together, kN
HPAR = (WDK + WCAB)*sq(SPAN)/(8*SAG);     // the naive parabola value
DEVP = max([ for (i = [0:NQ]) abs(YL[i]/HTOT - SAG*sq(i/NQ)) ]);

TMAX  = HTOT/2*sqrt(1 + sq(MMAIN));
VBLK  = (HTOT/2/MU + HTOT/2*MEND)/RHOC;
HBLK  = VBLK/(BLKX*(BLKY1 - BLKY0));

module anchorage(sx) {
    for (sg = [-1, 1])
      color(CONC) box([sx*(XEND + GAP),        sg*BLKY0, BLKTOP - HBLK],
                      [sx*(XEND + GAP + BLKX), sg*BLKY1, BLKTOP]);
}

tower_pair();
for (sx = [-1, 1]) anchorage(sx);

// ---- arc length -----------------------------------------------------
// s = integral sqrt(1 + (2Ax + c)^2) dx, and with u = 2Ax + c that is
// (u sqrt(1+u^2) + asinh u)/(4A) evaluated at the ends.
function AF(u) = (u*sqrt(1 + u*u) + asinh_(u))/(4*A);
SMAIN = AF(MMAIN) - AF(-MMAIN);
SSIDE = AF(2*A*SIDE + MS) - AF(MS);
SCAB  = SMAIN + 2*SSIDE;
NI    = 400;
HI    = SPAN/NI;
SNUM  = sum([ for (i = [0:NI-1]) sqrt(1 + sq(2*A*(-XT + (i + 0.5)*HI)))*HI ]);
POLY  = sum([ for (j = [0:NS-1]) norm(cp(j+1) - cp(j)) ]);

// ---- volume audit ---------------------------------------------------
LDRAWN  = sum([ for (j = HJ) hangtop(j) - hangbot(j) ]);
V_DECK  = ADECK*2*XEND;
V_CAB   = 2*ACDRAWN*POLY;
V_HANG  = 2*AHDRAWN*LDRAWN;
V_BRK   = 2*len(HJ)*VBRK;
HLEG    = LEGTOP - ZPIERT - GAP;
V_LEG   = 4*(4*TLEG*(HXBOT + HXTOP)/2 + 2*TLEG*(LEGY1 - LEGY0) - 4*sq(TLEG))*HLEG;
V_STR   = 6*(2*TSTR*(2*HXSTR + DSTR) - 4*sq(TSTR))*2*(LEGY0 - GAP);
V_SAD   = 4*2*HXSAD*(LEGY1 - LEGY0)*(SADTOP - LEGTOP - GAP);
V_PIER  = 4*2*PIERX*(PIERY1 - PIERY0)*(ZPIERT - ZPIERB);
V_BLK   = 4*BLKX*(BLKY1 - BLKY0)*HBLK;
V_SUM   = V_DECK + V_CAB + V_HANG + V_BRK + V_LEG + V_STR + V_SAD + V_PIER + V_BLK;

// ---- what the geometry says ----------------------------------------
echo("span", SPAN, "side spans", SIDE, "deck length", 2*XEND,
     "panel", PANEL, "panels", NS);
echo("camber over the main span", CAMBER, " sag", SAG,
     " sag ratio 1 in", SPAN/SAG, " parabola A", A);
echo("cable at midspan", ZC0, " at the saddle", ZCT,
     " difference minus sag (should be 0)", ZCT - ZC0 - SAG);
echo("cable slope at the tower: main arm", MMAIN, "=", atan(MMAIN),
     "deg, side arm", MSIDE, "=", atan(MSIDE), "deg");
echo("knuckle at the saddle", atan(MMAIN) + atan(MSIDE), "deg");
echo("horizontal pull each side of the saddle is H, so the net thrust on",
     "the tower is 0 and the vertical reaction is H x", MMAIN + MSIDE);
echo("hangers", 2*len(HJ), "in", len(HJ), "planes x 2 cables; shortest",
     min([ for (j = HJ) hlen(xs(j)) ]),
     " longest", max([ for (j = HJ) hlen(xs(j)) ]));
echo("identity: hanger length at the tower", CLEAR + SAG + CROWN*sq(XT/XEND),
     "should equal the tower height", TOWER);
echo("total hanger length", SUMELL, "m; drawn length", LDRAWN, "m");

// The one joint in the model whose clearance is not a constant, checked
// rather than asserted -- and checked against the IDEAL cable, not the
// drawn one, so it is not the tautology that testing hangtop against its
// own definition would be.
//
// A circular tube of radius RCAB about an axis at slope theta has its
// underside, measured straight down from the axis, at RCAB/cos(theta) --
// not RCAB, and not the RCAB*cos(theta) that the ring's bottom VERTEX sits
// at. The drawn tube is a NCAB-gon inscribed in that circle, so its
// underside is the higher of the two, and the difference between the two
// columns below is exactly the chording. Both must clear the hanger.
function ctheta(j) = let( t = tref(j) ) atan(t[2]/t[0]);
function cideal(j) = zcab(xs(j)) - RCAB/cos(ctheta(j));
RELDRAWN = min([ for (j = HJ) bz(xs(j)) - hangtop(j) ]);
RELIDEAL = min([ for (j = HJ) cideal(j) - hangtop(j) ]);
echo("hanger relief, worst of", len(HJ), "panels: under the drawn cable",
     RELDRAWN*1000, "mm, under the ideal circular cable", RELIDEAL*1000,
     "mm; GAP is", GAP*1000, "mm and both must be at least that");
echo("steepest cable at a hanger", max([ for (j = HJ) ctheta(j) ]),
     "deg, where the ring's bottom vertex is", 
     RCAB*(1/cos(max([ for (j = HJ) ctheta(j) ]))
           - cos(max([ for (j = HJ) ctheta(j) ])))*1000,
     "mm above the underside -- which is why the vertex is not the relief");

echo("arc length main span", SMAIN, " numeric midpoint integral", SNUM,
     " difference", SMAIN - SNUM);
echo("excess over the chord", SMAIN - SPAN,
     " small-sag prediction 8f^2/3L", 8*sq(SAG)/(3*SPAN),
     " overstated by", 100*(8*sq(SAG)/(3*SPAN)/(SMAIN - SPAN) - 1), "percent");
echo("arc length side span", SSIDE, " whole cable", SCAB,
     " funicular polygon", POLY, " chord deficit", SCAB - POLY);

echo("load kN/m: girder", WGIRD, "brackets", WBRK, "hangers", WHANG,
     "surfacing and live", WSUP, "-> per horizontal metre", WDK);
echo("cable steel", WCAB, "kN/m of arc, which is",
     100*WCAB/(WDK + WCAB), "percent of the load");
echo("sec theta at the tower", sqrt(1 + sq(MMAIN)),
     " so w runs from", WDK + WCAB, "to", WDK + WCAB*sqrt(1 + sq(MMAIN)),
     "kN/m, a spread of", 100*(sqrt(1 + sq(MMAIN)) - 1)*WCAB/(WDK + WCAB), "percent");
echo("H from the true load", HTOT/1000, "MN; from a flat w", HPAR/1000,
     "MN; largest departure of the true shape from the parabola", DEVP,
     "m =", 100*DEVP/SAG, "percent of the sag");
echo("cable tension at the tower", TMAX/1000, "MN per cable; steel area",
     ACSTEEL, "m2; working stress", TMAX/ACSTEEL/1000, "MPa; factor on UTS",
     UTS/(TMAX/ACSTEEL));
echo("cable steel mass", 2*ACSTEEL*SCAB*GAM/9.81, "t; girder steel",
     V_DECK*GAM/9.81, "t =", ADECK*GAM/9.81, "t per metre");

echo("anchorage: pull", HTOT/2/1000, "MN horizontal and", HTOT/2*MEND/1000,
     "MN uplift; block needed", VBLK, "m3, so", HBLK, "m deep");

echo("girder section: outer", AOUT, "inner", AIN, "steel", ADECK,
     "m2; volume is exactly area x length because the sweep is a translation");
echo("leg top", LEGTOP, "saddle block top", SADTOP,
     " cable underside at the knuckle", ZKNUCK,
     " clearance over the leg footprint", ZKNUCK - HXTOP*MSTEEP - LEGTOP);

echo("solids:", 1 + 2 + 4*len(HJ) + 4 + 6 + 4 + 4 + 4);
echo("volume m3 -- girder", V_DECK, "cables", V_CAB, "hangers", V_HANG,
     "brackets", V_BRK, "legs", V_LEG, "struts", V_STR, "saddles", V_SAD,
     "piers", V_PIER, "anchorages", V_BLK);
echo("no two solids share volume, so the exported total should be", V_SUM, "m3");
