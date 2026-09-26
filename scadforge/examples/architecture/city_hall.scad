// ===================================================================
//  London City Hall  --  Foster + Partners, 2002
//
//  The building is usually called a deformed sphere, which is true but
//  unbuildable: a sphere's surface is doubly curved, so every glazing
//  panel on it would be a different piece of bent glass.  What was
//  actually built is a SHEARED sphere sliced into horizontal bands, and
//  that one move changes everything -- because a band cut from a sheared
//  sphere between two horizontal planes is an OBLIQUE CIRCULAR CONE.
//
//  A cone is a ruled surface.  Every point on it lies on a straight line
//  from the apex, so the surface between two neighbouring rulings is
//  flat, and the cladding is flat glass in flat frames.  Curvature in
//  the silhouette, straightness in every panel.
//
//  So the whole skin here is one thing said twice:
//
//      ring(t)  =  circle of radius r(t) centred at (0, lean*t, z(t))
//
//  Take a sphere in polar angle phi, keep the band PHI0..PHI1, map it to
//  height, and slide each ring sideways in proportion to its height.
//  The sliding is the lean -- the south face overhangs and shades the
//  floor below it, which is why the building leans at all.  Consecutive
//  rings are parallel circles of different radius and different centre,
//  and the ruled surface joining them is, exactly, a tilted cone frustum.
//
//  The apex of each frustum and its axis tilt are echoed below: they are
//  not a figure of speech, they are where the cone actually is.
//
//  The ramp is the other half of the building's reputation -- a helix
//  climbing the full height around the chamber, about 500 m of it.  A
//  helix is the one curve that is its own rule: constant radius,
//  constant rise per turn, so every tread is the same tread.
//
//  Dimensions are a reconstruction from the published overall figures
//  (45 m tall, ten storeys, a ramp of roughly 500 m).  The METHOD is the
//  building's; the numbers are fitted to it, not taken from drawings.
// ===================================================================

H     = 45;        // overall height, m
RMAX  = 18;        // radius at the widest girth, m
PHI0  = 52;        // polar angle where the sphere meets the ground
PHI1  = 152;       // polar angle at the top rim
LEAN  = 13.0;      // how far the top ring sits south of the base, m
FLOORS = 10;       // storeys, and so floor bands
MU    = 11;        // rings per storey
NV    = 52;        // panels around
LIP   = 0.26;      // how far a floor band stands proud, m
BANDW = 0.055;     // band half-height, as a fraction of a storey

TURNS = 7.00;      // turns of the ramp over the full height
RCL   = 2.0;       // ramp clearance inside the skin, m
RW    = 2.2;       // ramp width, m
RT    = 0.45;      // ramp slab thickness, m
NR    = 460;       // stations along the ramp

PLINTH = 1.6;      // podium height, m
PR     = 1.14;     // podium radius, as a fraction of the base radius

GLASS = [0.55, 0.66, 0.74];
FRAME = [0.32, 0.36, 0.41];
RAMP  = [0.86, 0.85, 0.80];
STONE = [0.55, 0.54, 0.51];

// ---- the profile ---------------------------------------------------
// A sphere is parameterised by polar angle, not by height: equal steps
// in phi give equal steps of ARC, which is what keeps the panels even.
// Height comes from the sphere's own z = -R cos(phi), renormalised so
// the band PHI0..PHI1 spans exactly H.
function zeta(phi) = -cos(phi);
Z0 = zeta(PHI0);
Z1 = zeta(PHI1);
function phi_at(t)  = PHI0 + t*(PHI1 - PHI0);
function tz(phi)    = (zeta(phi) - Z0) / (Z1 - Z0);   // 0..1 up the height

// A floor band, placed rather than sampled.  Writing the band as a bump
// in the radius and hoping the uniform rings landed on it made its
// thickness depend on where the sampling fell -- the bands came out
// uneven up the building.  So the ring heights are LISTED, with four of
// them at each storey line: the profile radius just below, the same
// height stepped out by the lip, the lip again just above, and back to
// the profile.  Two rings at one height with different radii give a flat
// annulus; two radii at different heights give the band's face.  The
// band is then exactly BANDW tall wherever it is, and it is itself a
// cone slice -- the same construction, one storey tall instead of ten.
BW = BANDW/FLOORS;
function storey_rings(k) =
  concat(
    [ for (i = [0 : MU-1]) [ k/FLOORS + (i/MU)/FLOORS, 0 ] ],
    k >= FLOORS-1 ? []
      : let( hb = (k+1)/FLOORS )
        [ [hb - BW, 0], [hb - BW, 1], [hb + BW, 1], [hb + BW, 0] ] );

LEVELS = concat([ for (k = [0 : FLOORS-1]) each storey_rings(k) ], [[1, 0]]);

function r_at(h, lip = 0) = RMAX * sin(phi_at(h)) + LIP*lip;
function cy_at(h)         = LEAN * h;                  // the shear = the lean

// The ring at height fraction h: a plain circle, displaced sideways.
function ring(hl, n) =
  let( h = hl[0], r = r_at(h, hl[1]), cy = cy_at(h), z = H*h )
  [ for (j = [0 : n-1])
      let( a = 360*j/n ) [ r*cos(a), r*sin(a) + cy, z ] ];

// Halving recursion, not a running total: a list this long added one
// element at a time recurses once per element, and the ramp has 460 of
// them.  Splitting the range keeps the depth logarithmic -- and took the
// whole model from twenty seconds to half of one.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0
  : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

function centroid(p) =
  let( n = len(p) ) [ for (k = [0:2]) sum([ for (q = p) q[k] ])/n ];

// ---- a ring grid, closed into a solid ------------------------------
module shell(grid, conv = 6) {
    MU = len(grid) - 1;
    MV = len(grid[0]);
    body = [ for (u = [0:MU]) each grid[u] ];
    pts  = concat(body, [centroid(grid[0])], [centroid(grid[MU])]);
    B0 = (MU+1)*MV; B1 = B0 + 1;
    polyhedron(
      points = pts,
      faces = concat(
        [ for (u = [0:MU-1]) for (v = [0:MV-1])
            [ u*MV + v, u*MV + (v+1)%MV, (u+1)*MV + (v+1)%MV, (u+1)*MV + v ] ],
        [ for (v = [0:MV-1]) [ B0, v, (v+1)%MV ] ],
        [ for (v = [0:MV-1]) [ B1, MU*MV + (v+1)%MV, MU*MV + v ] ]),
      convexity = conv );
}

// ---- the skin ------------------------------------------------------
color(GLASS)
  shell([ for (L = LEVELS) ring(L, NV) ]);

// ---- the helical ramp ----------------------------------------------
// Radius follows the skin inward by a fixed clearance, so the ramp
// leans with the building instead of cutting through the south wall.
function ramp_r(h) = max(4.5, RMAX*sin(phi_at(h)) - RCL);
function ramp_c(h) = 360 * TURNS * h;

function ramp_station(h) =
  let( a  = ramp_c(h), rr = ramp_r(h), cy = cy_at(h), z = H*h,
       ca = cos(a), sa = sin(a),
       ri = rr - RW/2, ro = rr + RW/2 )
  [ [ ri*ca, ri*sa + cy, z      ],
    [ ro*ca, ro*sa + cy, z      ],
    [ ro*ca, ro*sa + cy, z - RT ],
    [ ri*ca, ri*sa + cy, z - RT ] ];

color(RAMP)
  shell([ for (i = [0:NR]) ramp_station(0.055 + (0.955 - 0.055)*i/NR) ]);

// ---- the crown -----------------------------------------------------
// The top ring is where the sphere's band was cut, so on its own it
// reads as a slice rather than a roof.  A shallow cap closes it: the
// same sphere the skin came from, squashed, sitting on the ring it left.
TOPR = RMAX*sin(PHI1);
color(GLASS)
  translate([0, cy_at(1), H]) scale([1, 1, 0.19]) sphere(r = TOPR, $fn = 48);

// ---- podium --------------------------------------------------------
color(STONE)
  translate([0, 0, -PLINTH])
    cylinder(h = PLINTH, r = PR*RMAX*sin(PHI0), $fn = 96);

// ---- what the geometry says ----------------------------------------
echo("rings", len(LEVELS), "panels round", NV);
echo("height", H, "widest diameter", 2*RMAX,
     "base diameter", 2*RMAX*sin(PHI0), "top diameter", 2*RMAX*sin(PHI1));
echo("widest girth at", tz(90), "of the height");
echo("lean", LEAN, "m over", H, "m =", atan(LEAN/H), "degrees off vertical");

// Every slice is a cone.  Here is where three of them have their apex,
// and how far each axis leans from the vertical.
module cone_report(h0, h1) {
    r0 = RMAX*sin(phi_at(h0)); r1 = RMAX*sin(phi_at(h1));
    c0 = [0, cy_at(h0), H*h0]; c1 = [0, cy_at(h1), H*h1];
    k  = r0 / (r0 - r1);
    apex = [ for (i=[0:2]) c0[i] + (c1[i] - c0[i])*k ];
    echo("slice", h0, "->", h1, " apex", apex,
         " axis tilt", atan((c1[1]-c0[1]) / (c1[2]-c0[2])), "deg");
}
cone_report(0.10, 0.20);
cone_report(0.45, 0.55);
cone_report(0.80, 0.90);

// The ramp, measured rather than asserted.
RLEN = sum([ for (i = [1:NR])
    let( a = ramp_station(0.055 + 0.9*(i-1)/NR)[0],
         b = ramp_station(0.055 + 0.9*i/NR)[0] )
      norm(b - a) ]);
echo("ramp turns", TURNS, " length", RLEN, "m",
     " mean gradient 1 in", RLEN/(H*0.9));
// Worth noticing: fit the ramp to the published 500 m and the gradient
// that falls out is about 1 in 12 -- which is the gradient a ramp has to
// have to be walkable and wheelable.  The length was not chosen to make
// that happen; it is what 500 m of ramp climbing 45 m has to mean.
