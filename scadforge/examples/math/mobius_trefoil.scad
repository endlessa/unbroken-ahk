// ===================================================================
//  A Mobius band and a trefoil knot, swept from their equations
//
//  Two closed tubes stand side by side here, and both of them fail to
//  close for the same reason: a tube is a cross-section carried along a
//  curve by a moving frame, and when the frame comes back to where it
//  started it has usually turned.  Everything else in this file is
//  bookkeeping.  The whole difficulty is what the frame does on the last
//  step of the lap, and the two objects answer it in the two different
//  ways that exist.  The band's frame comes back turned by exactly half
//  a turn, and the section is chosen so that half a turn maps it onto
//  itself.  The knot's frame comes back turned by an angle that is not
//  special at all, and the turn has to be measured and unwound.
//
//  Both are single closed polyhedra.  There is no boolean anywhere.
//
//  ---- the band ----------------------------------------------------
//
//  The mid-surface is the usual one,
//
//      c(t, w) = ((R + w cos(t/2)) cos t,
//                 (R + w cos(t/2)) sin t,
//                  w sin(t/2)),        w in [-W/2, W/2]
//
//  which is easier to read as a segment being carried round a circle.
//  Write e_r(t) = (cos t, sin t, 0) and e_z = (0, 0, 1).  Then
//
//      c(t, w) = R e_r(t) + w u(t),   u(t) = cos(t/2) e_r + sin(t/2) e_z
//
//  so the generating segment lies in the plane through the axis at
//  azimuth t, tilted by t/2 out of the horizontal.  Half the rate of
//  the azimuth: one lap of t turns the segment through 180 degrees.
//  Complete the frame with
//
//      v(t) = -sin(t/2) e_r + cos(t/2) e_z
//
//  so (u, v) is the (e_r, e_z) pair rotated by t/2 in its own plane, and
//  both are perpendicular to the direction of travel, which on the core
//  curve is exactly e_theta:  dc/dt at w = 0 is R e_theta.
//
//  The mid-surface's normal is v, because the tangent plane is spanned
//  by u and e_theta.  At t = 2*pi, u -> -u and v -> -v.  The normal has
//  reversed, so the surface has one side, and a face list laid out over
//  a full lap of it cannot be consistently wound.  That is not a bug to
//  be worked around.  It is the theorem.
//
//  Thickness is what rescues it.  Give the band a thickness T and the
//  section stops being a segment and becomes a rectangle,
//
//      p(t, a, b) = R e_r(t) + a u(t) + b v(t),
//                   a in [-W/2, W/2],  b in [-T/2, T/2]
//
//  and at t = 2*pi the frame change (u, v) -> (-u, -v) acts on the
//  section as (a, b) -> (-a, -b).  That is a half turn of the rectangle
//  about its own centre, and a rectangle is invariant under it.  The
//  ring of vertices at t = 2*pi is therefore the ring at t = 0 with its
//  vertex list rotated by half its length, and the tube closes.  The
//  solid is a solid torus, its boundary is a torus, and a torus is
//  orientable: the swept solid has an inside, even though the surface it
//  was thickened from has no sides.
//
//  The C2 symmetry of the rectangle is load-bearing and not decoration.
//  A triangular section would not close, because a half turn is not in
//  C3; a regular pentagon would not close either.  A rectangle, an
//  ellipse, a hexagon all do.  The section here is sampled with NA cells
//  across the width and NB across the thickness, in an order that is
//  symmetric under (a, b) -> (-a, -b), so the half turn is exactly the
//  integer index shift NS/2 and nothing else in the mesh knows about it.
//
//  One consequence is countable, and the model counts it.  The shift has
//  order two, so every longitudinal line of the mesh needs TWO laps to
//  return to its start: the NS section vertices pair off into NS/2
//  closed curves rather than NS.  Nothing on this solid closes in one
//  lap.  The two edges of the band's wide face are one curve of about
//  twice the circumference, and the four corner arcs of the rectangle
//  are two curves, not four.  Those lengths are echoed below, integrated
//  rather than asserted, because the corner of the section does not sit
//  at radius R and its curve is not a circle.  That single edge curve is
//  the one-sidedness of the mid-surface, surviving into the solid as an
//  arithmetic fact about the mesh.
//
//  The volume is exact and the twist does not enter it.  In cylindrical
//  coordinates the sweep is rho = R + a cos(t/2) - b sin(t/2),
//  theta = t, zeta = a sin(t/2) + b cos(t/2), so (rho, zeta) is (a, b)
//  rotated by t/2 and the Jacobian of (rho, theta, zeta) against
//  (t, a, b) has modulus 1.  With dV = rho drho dtheta dzeta,
//
//      V = integral rho dt da db = 2*pi*R*W*T
//
//  because the first moments of a centred rectangle vanish, which is
//  also why the twist cancels.  The sweep is embedded as long as
//  rho > 0, that is R > (W/2)|cos(t/2)| + (T/2)|sin(t/2)| for every t,
//  so R greater than the section's half-diagonal is enough.
//
//  ---- the knot ----------------------------------------------------
//
//  The centreline is
//
//      g(t) = (sin t + 2 sin 2t,  cos t - 2 cos 2t,  -sin 3t)
//
//  and the tube is round, radius r, so the section has no symmetry to
//  fall back on: the frame has to close by itself.
//
//  The usual reason to avoid a Frenet frame does not apply to this
//  curve, and saying otherwise would be repeating a slogan.  The
//  curvature is bounded away from zero here: its minimum over the lap is
//  echoed below and comes out near 0.206 per unit, so there is no
//  inflection, the principal normal never has to flip, and the Frenet
//  frame is continuous and periodic.  The reason to avoid it is the
//  other one.  The Frenet normal spins about the tangent at the torsion
//  rate, and over one lap that adds up to
//
//      integral tau ds = +127.485 degrees
//
//  of rotation that the geometry never asked for, shearing every quad in
//  the mesh.  A parallel-transported frame carries none of it.
//
//  Parallel transport is normally computed station by station, each
//  frame rotated off the last.  That is a fold, and in a language with
//  no mutable state a fold is a recursion 288 frames deep, carrying a
//  vector.  This kernel would survive that; its non-tail guard sits at
//  10000 frames, probed.  The reason not to do it is numerical, not
//  structural: in a chain every station inherits every rounding error
//  ahead of it, and the holonomy comes out as the residue of 288
//  compounded rotations rather than as a number in its own right.
//  Parallel transport has a closed form, and with it each station's
//  angle is one quadrature, independent of the others, and the holonomy
//  is a quantity the model computes rather than discovers.  Take any
//  smooth reference frame; here the one built from a fixed up-vector k,
//
//      T = g' / |g'|,   N_ref = (k x T)/|k x T|,   B_ref = T x N_ref
//
//  and let N = cos(th) N_ref + sin(th) B_ref.  Differentiating and
//  requiring N' . B = 0, which is what parallel means, gives
//
//      th' = -w,     w = N_ref' . B_ref
//
//  and w has an elementary form.  With m = k x T and |m|^2 = 1 - (k.T)^2,
//  the terms in m' . (T x m) that survive collapse to
//
//      w(t) = (T.k) (k . (g' x g'')) / ( |g'|^2 (1 - (k.T)^2) )
//
//  so th(t) = -integral w dt is a single quadrature along an analytic
//  integrand.  Integrating it as a running total is a prefix sum, and
//  the halving sum below computes those at logarithmic depth.
//
//  The up-vector has to be chosen, not assumed.  k must never be
//  parallel to T or N_ref is undefined and w blows up.  Taking k = e_z,
//  |k.T| = 3|cos 3t| / |g'|, and its maximum over the lap is exactly
//  1/sqrt(2): at t = 60 degrees, g' = (-3/2, 3*sqrt(3)/2, 3) whose
//  length is 3*sqrt(2), so T.k = 1/sqrt(2) there.  The tangent of this
//  trefoil never leaves 45 degrees of the horizontal plane, which is why
//  e_z is a good up-vector and e_x is not: with e_x the worst |k.T| is
//  0.982 and the reference frame nearly degenerates.  The model echoes
//  the worst case it actually met.
//
//  Transport does not close.  th(2*pi) is the holonomy of the loop,
//
//      Phi = -integral w dt = -127.485 degrees
//
//  so the frame comes back short of itself by that much and the tube has
//  a seam.  The cure is to spread the deficit evenly,
//
//      th_closed(t) = th(t) - Phi * t / (2*pi)
//
//  which closes exactly at t = 2*pi and leaves the least residual twist
//  any closing frame can have, short of adding a whole extra turn.
//
//  Two independent checks say the correction is right, and they are the
//  reason to trust the sign rather than the render.
//
//  First, Phi = -integral tau ds, modulo a whole turn.  The transported
//  frame differs from the Frenet frame by -integral tau ds and from the
//  fixed-up frame by th; both of those reference frames are pointwise
//  functions of the curve and so periodic; therefore the two holonomies
//  must agree up to 360 degrees.  The model computes them along
//  completely different routes, an algebraic twist rate and the Frenet
//  torsion, and echoes both.
//
//  Second, Calugareanu.  For a closed framed curve Lk = Wr + Tw, and Lk
//  is an integer.  Parallel transport has Tw = 0 by construction, so all
//  the twist in the closed framing is the correction itself,
//  Tw = +127.485/360 = +0.354126 turns.  The writhe of this trefoil, by
//  the Gauss double integral, is -3.354126.  They sum to -3.000000.  Get
//  the sign of the correction backwards and the sum is -3.708, which is
//  not an integer and cannot be a linking number.  This is the check
//  that a picture cannot give you.
//
//  The tube's volume is also exact, and for the same reason the band's
//  was.  For a round section of radius r carried by any frame along any
//  curve, the volume element is rho (1 - kappa rho cos phi) drho dphi ds,
//  and the cos phi term integrates away over a full circle, so
//
//      V = pi r^2 L,     L = integral |g'| dt
//
//  independent of curvature, torsion and framing.  It needs the tube to
//  be embedded: r below the minimum radius of curvature, and 2r below
//  the minimum distance between parts of the centreline that are not
//  neighbours.  Both margins are echoed.
//
//  ---- degrees -----------------------------------------------------
//
//  The kernel's trig is in degrees, so t runs 0 to 360 and g', g'', g'''
//  are written with the same coefficients the radian forms have.  Each
//  differentiation is then short by a factor pi/180.  Those factors
//  cancel in w, which is a ratio of equal degree in the derivatives, and
//  cancel again between a rate per degree and a step in degrees, so the
//  holonomy and the total torsion both come out directly in degrees with
//  no conversion anywhere.  The writhe is the one place a factor is
//  needed, because the Gauss integral is over the parameter twice and
//  the result is a dimensionless count of turns.
//
//  ---- what the export taught me -----------------------------------
//
//  Every wall quad on a twisting sweep is a saddle, not a plane, and a
//  quad's two triangles are not the quad.  Split a saddle on one
//  diagonal and the surface bulges one way; split it on the other and it
//  bulges the other way by the same amount, so the two triangulations
//  straddle the bilinear patch they are supposed to represent.
//  polyhedron() fan triangulates a four-vertex face, always from the
//  same corner, so on a band built from quads every saddle was biased
//  the same direction and the export came out 15091.18 against an exact
//  15079.64, high by 0.077 percent.
//
//  The size of that was not the problem.  The behaviour was.  The bias
//  falls off like 1/NT, first order, while the chording error of the core
//  circle falls off like 1/NT^2, so doubling the station count barely
//  moved it and the residue looked structural.  Measured on this kernel
//  at NA = 16, NB = 4, the quad mesh is high by 0.108, 0.077, 0.044 and
//  0.023 percent at NT = 96, 192, 384, 768: a clean first order.  Choose
//  the diagonal by the parity of (u + v) instead and neighbouring
//  saddles cancel: the same four meshes come out low by 0.0892, 0.0223,
//  0.00558 and 0.00139 percent, which is -822/NT^2 percent with the
//  coefficient sitting between 822.2 and 822.5 across a factor of eight
//  in NT.  Second order, and what is left is chording and nothing else.
//  So this model emits triangles, not quads.
//  That also settles the exported geometry here instead of leaving it to
//  whatever triangulation the kernel happens to use inside.
//
//  The winding rule, worked out rather than guessed.  polyhedron() wants
//  each face wound clockwise seen from outside, equivalently with the
//  right-hand normal pointing into the solid.  For a sweep with advance
//  direction d and faces [ring u vertex v, ring u+1 vertex v, ring u+1
//  vertex v+1, ring u vertex v+1] that forces the section perimeter to
//  be traversed in the direction
//
//      tau = d x n_out
//
//  On the knot the frame (N, B, T) is right-handed, B = T x N, so
//  increasing the section angle is already correct.  On the band it is
//  the other way: u x v = e_r x e_z = -e_theta, so (u, v, e_theta) is
//  LEFT-handed, and the section must run (-A, +B) -> (+A, +B) ->
//  (+A, -B) -> (-A, -B), which looks clockwise when the (a, b) plane is
//  drawn the ordinary way.
//
//  Reversing it was tried, on purpose, to see what the kernel would say.
//  It said nothing: the band exported closed, hole-free, consistently
//  wound, and at exactly minus the volume it has here.  The kernel does
//  warn when a polyhedron is not closed, and it warns when two faces
//  traverse a shared edge the same way round, so the third case, a mesh
//  that is impeccable except for being oriented the opposite way to the
//  documented convention, is the one with no diagnostic.  That is worth
//  knowing before trusting a render: the sign of the volume is the only
//  thing that tells you, and it costs one pass over the faces.
// ===================================================================

// ---- the band ------------------------------------------------------
R_M  = 30;      // core circle radius
W_M  = 20;      // band width, along the generating segment
T_M  = 4;       // band thickness, normal to the mid-surface
NT_M = 192;     // stations round the core
NA   = 16;      // section cells across the width
NB   = 4;       // section cells across the thickness

// ---- the knot ------------------------------------------------------
S_K  = 9;       // scale applied to g(t)
R_K  = 3.6;     // tube radius
NT_K = 288;     // stations along the centreline
NC   = 28;      // sides of the tube's section
MW   = 120;     // samples per axis for the writhe double integral
XK   = 82;      // where the knot sits on x
SEPD = 50;      // "not neighbours" for the clearance check, degrees of t

BAND = [0.82, 0.56, 0.28];
KNOT = [0.30, 0.52, 0.70];

// ---- the two utilities ---------------------------------------------
// Halving sum.  A running total recurses once per element; this kernel's
// non-tail guard is 10000 frames, probed, so nothing in this file is long
// enough to trip it and the honest reason for the halving form is the
// other one.  Pairwise summation's rounding error grows like log n where
// a running total's grows like n, and the prefix sums below are read as
// angles that then set vertex positions, so a drift of a few ulps in the
// last station is a visible seam.  It is also what keeps the file safe if
// the station counts are raised.  Called once per station, so it is the
// hot path here.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0
  : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

function unit(v) = v / norm(v);

// ---- one closed tube, no caps --------------------------------------
// rings is a list of equal-length vertex rings.  Ring index nu means
// ring 0 again, with its vertices rotated by sh.  That single integer is
// the entire half twist of the band; the knot passes sh = 0.
function wid(u, v, nu, nv, sh) =
    u < nu ? u*nv + (v % nv) : (v + sh) % nv;

module closed_tube(rings, sh = 0, conv = 12) {
    nu = len(rings);
    nv = len(rings[0]);
    polyhedron(
      points = [ for (r = rings) each r ],
      faces  = [ for (u = [0 : nu-1]) for (v = [0 : nv-1])
                   let( a = wid(u,   v,   nu, nv, sh),
                        b = wid(u+1, v,   nu, nv, sh),
                        c = wid(u+1, v+1, nu, nv, sh),
                        d = wid(u,   v+1, nu, nv, sh) )
                   // saddle quads: alternate the diagonal so the two
                   // triangulation biases cancel instead of accumulating
                   each ((u + v) % 2 == 0 ? [ [a,b,c], [a,c,d] ]
                                          : [ [a,b,d], [b,c,d] ]) ],
      convexity = conv );
}

// ===================================================================
//  MOBIUS BAND
// ===================================================================
A_M = W_M/2;
B_M = T_M/2;

// The section perimeter, traversed so the right-hand normal of each wall
// face points into the solid.  Symmetric under (a, b) -> (-a, -b) by
// construction: entry k and entry k + NA + NB are negatives, which is
// what makes the half turn a pure index shift.
SECT = concat(
  [ for (k = [0 : NA-1]) [ -A_M + 2*A_M*k/NA,  B_M ] ],
  [ for (k = [0 : NB-1]) [  A_M,  B_M - 2*B_M*k/NB ] ],
  [ for (k = [0 : NA-1]) [  A_M - 2*A_M*k/NA, -B_M ] ],
  [ for (k = [0 : NB-1]) [ -A_M, -B_M + 2*B_M*k/NB ] ]);

NS = len(SECT);       // 2*(NA + NB)
SH = NS/2;            // the half turn, as an index shift

function mob_ring(u) =
  let( t  = 360*u/NT_M,
       er = [cos(t), sin(t), 0],
       ez = [0, 0, 1],
       uu =  cos(t/2)*er + sin(t/2)*ez,
       vv = -sin(t/2)*er + cos(t/2)*ez )
  [ for (p = SECT) R_M*er + p[0]*uu + p[1]*vv ];

color(BAND) closed_tube([ for (u = [0 : NT_M-1]) mob_ring(u) ], SH);

// ===================================================================
//  TREFOIL KNOT
// ===================================================================
KUP = [0, 0, 1];      // the fixed up-vector for the reference frame

function gam(t)  = [   sin(t) +  2*sin(2*t),   cos(t) -  2*cos(2*t),  -sin(3*t) ];
function gam1(t) = [   cos(t) +  4*cos(2*t),  -sin(t) +  4*sin(2*t), -3*cos(3*t) ];
function gam2(t) = [  -sin(t) -  8*sin(2*t),  -cos(t) +  8*cos(2*t),  9*sin(3*t) ];
function gam3(t) = [  -cos(t) - 16*cos(2*t),   sin(t) - 16*sin(2*t), 27*cos(3*t) ];

// Rotation rate of the fixed-up reference frame about the tangent.
// Degrees of frame rotation per degree of t, the pi/180 factors having
// cancelled between numerator and denominator.
function twist_rate(t) =
  let( d1 = gam1(t), tt = unit(d1), kt = KUP*tt )
    kt * (KUP * cross(d1, gam2(t))) / ((d1*d1) * (1 - kt*kt));

function curv(t)    = let( d1 = gam1(t) ) norm(cross(d1, gam2(t))) / pow(norm(d1), 3);
function torsion(t) = let( c = cross(gam1(t), gam2(t)) ) (c * gam3(t)) / (c*c);

HK    = 360/NT_K;
WRATE = [ for (i = [0 : NT_K]) twist_rate(HK*i) ];
// One trapezoid increment per step.  Parallel transport is the running
// total of these, negated.
DTH   = [ for (i = [0 : NT_K-1]) HK*(WRATE[i] + WRATE[i+1])/2 ];
PHI   = -sum(DTH);                                  // holonomy, degrees

function theta(i) = -sum(DTH, 0, i) - PHI*i/NT_K;   // closed framing

function knot_ring(i) =
  let( t  = HK*i,
       tt = unit(gam1(t)),
       nr = unit(cross(KUP, tt)),
       br = cross(tt, nr),
       c  = S_K*gam(t) + [XK, 0, 0],
       th = theta(i) )
  [ for (j = [0 : NC-1])
      let( a = 360*j/NC + th ) c + R_K*cos(a)*nr + R_K*sin(a)*br ];

color(KNOT) closed_tube([ for (i = [0 : NT_K-1]) knot_ring(i) ]);

// ===================================================================
//  WHAT THE GEOMETRY SAYS
// ===================================================================
VM   = NT_M*NS;                 // band vertices
FM   = 2*NT_M*NS;               // band triangles
VK   = NT_K*NC;
FK   = 2*NT_K*NC;

echo("BAND  R", R_M, " width", W_M, " thickness", T_M,
     " stations", NT_M, " section vertices", NS);
echo("BAND  half turn closes the section as index shift", SH,
     " of", NS, " so cycles of length 2:", SH, "mesh lines, each two laps long");
echo("BAND  embedded needs R >", sqrt(A_M*A_M + B_M*B_M), " and R =", R_M);
echo("BAND  volume 2*pi*R*W*T =", 2*PI*R_M*W_M*T_M);
echo("BAND  core circumference", 2*PI*R_M,
     " inscribed polygon", NT_M*2*R_M*sin(180/NT_M),
     " so chording costs", 100*(1 - sin(180/NT_M)/(PI/NT_M)), "percent");
echo("BAND  mesh V", VM, " E", 3*FM/2, " F", FM,
     " Euler V-E+F =", VM - 3*FM/2 + FM, " (a torus, as it must be)");
// The band's own edge, measured.  A corner of the section traced from
// t = 0 comes back at t = 360 as the OPPOSITE corner, so a corner curve
// needs t to run to 720 before it closes.  With rho = R + a cos(t/2)
// - b sin(t/2) and zeta = a sin(t/2) + b cos(t/2), the speed is
// sqrt(rho^2 + (a^2 + b^2)/4), because the two in-plane derivatives
// always sum in quadrature to (a^2+b^2)/4.
function edge_len(a, b, n = 720) =
  sum([ for (i = [0 : n-1])
          let( t = 720*(i + 0.5)/n,
               rho = R_M + a*cos(t/2) - b*sin(t/2) )
            sqrt(rho*rho + (a*a + b*b)/4) * (720/n)*PI/180 ]);
echo("BAND  mid-surface edge (a = +-W/2, b = 0) is ONE closed curve of length",
     edge_len(A_M, 0), " which is the one-sidedness, measured:",
     "it needs t to reach 720 to close, against", 2*PI*R_M, "for the core");
echo("BAND  the solid's four corner arcs pair into TWO closed curves, length",
     edge_len(A_M, B_M), "each; the pairs differ only by a phase in t/2",
     "so the other measures", edge_len(A_M, -B_M));

// The worst conditioning the reference frame actually met.
KTMAX = max([ for (i = [0 : NT_K-1]) abs(KUP*unit(gam1(HK*i))) ]);
KMAX  = max([ for (i = [0 : NT_K-1]) curv(HK*i) ]);
KMIN  = min([ for (i = [0 : NT_K-1]) curv(HK*i) ]);

// Arc length two ways: the smooth integral, and the polygon the mesh
// actually has.
GSPD  = [ for (i = [0 : NT_K]) norm(gam1(HK*i)) ];
LSM   = S_K * sum([ for (i = [0 : NT_K-1]) (GSPD[i] + GSPD[i+1])/2 ]) * HK*PI/180;
LPOLY = sum([ for (i = [0 : NT_K-1]) norm(S_K*(gam(HK*(i+1)) - gam(HK*i))) ]);

// Total torsion, in degrees, by the same cancellation as the twist rate.
TORS = sum([ for (i = [0 : NT_K-1])
    let( p = torsion(HK*i)*GSPD[i], q = torsion(HK*(i+1))*GSPD[i+1] )
      HK*(p + q)/2 ]);

// Writhe, Gauss double integral over the parameter square.  The diagonal
// is omitted: the integrand is bounded there, the numerator vanishing to
// the same order as the cube of the separation, so dropping a strip of
// width 1/MW costs O(1/MW).
HW = 360/MW;
WP = [ for (i = [0 : MW-1]) gam(HW*i) ];
WT = [ for (i = [0 : MW-1]) gam1(HW*i) ];
WRITHE = sum([ for (i = [0 : MW-1])
           sum([ for (j = [0 : MW-1])
                   i == j ? 0
                 : let( r = WP[i] - WP[j] )
                     (cross(WT[i], WT[j]) * r) / pow(norm(r), 3) ]) ])
         * pow(2*PI/MW, 2) / (4*PI);

// Clearance: the closest approach of two pieces of centreline that are
// not neighbours along it.  Sampled every fourth station, which is
// plenty for a bound.
SEP = min([ for (i = [0 : 4 : NT_K-1]) for (j = [0 : 4 : NT_K-1])
              let( d = abs(i - j), dt = min(d, NT_K - d)*HK )
                dt < SEPD ? 1e9 : norm(S_K*(gam(HK*i) - gam(HK*j))) ]);

echo("KNOT  scale", S_K, " tube radius", R_K,
     " stations", NT_K, " section sides", NC);
echo("KNOT  up-vector e_z: worst |k.T| =", KTMAX,
     " (1/sqrt2 =", 1/sqrt(2), ") so the tangent stays within",
     asin(KTMAX), "degrees of horizontal");
echo("KNOT  curvature per unit: min", KMIN, " max", KMAX,
     " -> no inflection, Frenet is defined; min radius of curvature",
     S_K/KMAX, "vs tube radius", R_K);
echo("KNOT  clearance: closest non-neighbour centreline approach", SEP,
     " tube diameter", 2*R_K, " margin", SEP - 2*R_K);
echo("KNOT  arc length: smooth integral", LSM, " mesh polygon", LPOLY,
     " short by", 100*(1 - LPOLY/LSM), "percent");
echo("KNOT  frame holonomy Phi =", PHI, "degrees =", PHI/360, "turns");
echo("KNOT  total torsion integral tau ds =", TORS, "degrees");
echo("KNOT  check Phi + integral tau ds =", PHI + TORS, "degrees (0 mod 360)");
echo("KNOT  writhe Wr =", WRITHE, " correction twist Tw =", -PHI/360,
     " Lk = Wr + Tw =", WRITHE - PHI/360, " (must be an integer)");
echo("KNOT  volume pi r^2 L =", PI*R_K*R_K*LSM);
echo("KNOT  mesh is a", NC, "-gon tube: section area",
     0.5*NC*R_K*R_K*sin(360/NC), "vs pi r^2", PI*R_K*R_K,
     " deficit", 100*(1 - 0.5*NC*sin(360/NC)/PI), "percent");
echo("KNOT  so the mesh should export", 0.5*NC*R_K*R_K*sin(360/NC)*LPOLY);
echo("KNOT  mesh V", VK, " E", 3*FK/2, " F", FK,
     " Euler V-E+F =", VK - 3*FK/2 + FK);

echo("TOTAL triangles", FM + FK,
     " expected export volume", 2*PI*R_M*W_M*T_M + 0.5*NC*R_K*R_K*sin(360/NC)*LPOLY);
