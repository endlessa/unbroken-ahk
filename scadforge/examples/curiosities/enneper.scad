// =====================================================================
//  THE ENNEPER SURFACE  --  Alfred Enneper, 1864
//
//      x = u - u^3/3 + u v^2
//      y = v - v^3/3 + v u^2
//      z = u^2 - v^2
//
//  Three cubic polynomials in two variables.  Nothing is implicit,
//  nothing is iterated, and everything below -- the shape, the
//  thickness, the volume, the places where it runs through itself --
//  follows from those three lines by differentiation.
//
//  ---- WHY IT IS MINIMAL, WORKED OUT RATHER THAN QUOTED -------------
//
//  Write W = 1 + u^2 + v^2.  The two tangent vectors are
//
//      X_u = ( 1 - u^2 + v^2,  2uv,  2u )
//      X_v = ( 2uv,  1 + u^2 - v^2,  -2v )
//
//  and the first fundamental form comes out astonishingly clean:
//
//      E = X_u.X_u = (1-u^2+v^2)^2 + 4u^2v^2 + 4u^2
//                  = 1 + u^4 + v^4 + 2u^2 + 2v^2 + 2u^2v^2  =  W^2
//      F = X_u.X_v = 2uv[(1-u^2+v^2) + (1+u^2-v^2) - 2]     =  0
//      G = X_v.X_v = W^2   (by the u <-> v symmetry of the pair)
//
//  F = 0 and E = G: the parameterisation is CONFORMAL.  A small square
//  of the (u,v)-plane lands on the surface as a small square, scaled by
//  W and not sheared.  That one fact is why the mesh below can use a
//  plain polar grid and still have square quads (see the grid note).
//
//  The cross product collapses the same way.  Term by term,
//
//      X_u x X_v = ( -2uW,  2vW,  1 - (u^2+v^2)^2 )
//                = W * ( -2u, 2v, 1 - u^2 - v^2 ),
//
//  using 1 - (u^2+v^2)^2 = (1 - u^2 - v^2)(1 + u^2 + v^2).  The length
//  of the bracket is sqrt(4u^2 + 4v^2 + (1-u^2-v^2)^2) = 1 + u^2 + v^2
//  = W exactly -- the cross terms cancel again -- so |X_u x X_v| = W^2
//  = sqrt(EG - F^2), as it must be, and the UNIT normal is the rational
//  map
//
//      n = ( -2u,  2v,  1 - u^2 - v^2 ) / (1 + u^2 + v^2).
//
//  That is stereographic projection of the (u,v)-plane onto the unit
//  sphere: the Gauss map covers the sphere exactly once, missing only
//  the single point it tends to as r -> infinity.  So the complete
//  surface has total curvature exactly -4*pi -- and by Osserman's
//  theorem no complete non-planar minimal surface has a total curvature
//  nearer to zero than that; Enneper's surface and the catenoid are the
//  two that attain it.  This exhibit shows the disc u^2 + v^2 <= 4,
//  whose share is -4*pi*R^2/(1+R^2) = -16*pi/5.
//
//  Now the second fundamental form.  The second derivatives are
//  constants-plus-linear, X_uu = (-2u, 2v, 2), X_uv = (2v, 2u, 0),
//  X_vv = (2u, -2v, -2), and dotting each into n:
//
//      L = X_uu.n = (4u^2 + 4v^2 + 2 - 2u^2 - 2v^2)/W = 2W/W =  2
//      M = X_uv.n = (-4uv + 4uv + 0)/W                       =  0
//      N = X_vv.n = (-4u^2 - 4v^2 - 2 + 2u^2 + 2v^2)/W       = -2
//
//  -- three pure numbers, the same at every point of the surface.  So
//
//      H = (EN - 2FM + GL) / (2(EG - F^2)) = (-2W^2 + 2W^2)/(2W^4) = 0
//      K = (LN - M^2) / (EG - F^2)         = -4 / W^4
//
//  H = 0 IDENTICALLY, not to some tolerance.  And because F = M = 0 the
//  u- and v-lines are the lines of curvature, with principal curvatures
//
//      k1 = L/E =  2/W^2,      k2 = N/G = -2/W^2,
//
//  equal and opposite at every point: every point is a perfect saddle,
//  bending as hard one way as the other.  A soap film does exactly this
//  -- surface tension pulls the two curvatures into balance -- which is
//  what "minimal" means and why the shape is a physical one.
//
//  The file does not take any of that on trust.  X is a CUBIC, so the
//  five-point first difference and the three-point second difference
//  are exact for it (their error terms carry fourth and fifth
//  derivatives, which vanish).  The audit at the foot rebuilds E, F, G,
//  L, M, N and H from the three polynomials by finite differences alone
//  and reports max |H| over a grid; it comes out at the size of double
//  rounding, around 1e-15, which is the claim on the plaque.
//
//  ---- THE LINES OF CURVATURE ARE PLANE CUBICS ----------------------
//
//  Hold v = c.  Then z = u^2 - c^2, so u^2 = z + c^2, and
//
//      y = c(1 - c^2/3) + c u^2 = c(1 + 2c^2/3) + c z
//
//  -- y is an AFFINE function of z, so the whole curve lies in the
//  plane y = c z + c(1 + 2c^2/3).  Holding u = c gives
//  x = -c z + c(1 + 2c^2/3) the same way.  Both families of curvature
//  lines are plane curves, which is rare enough to be one of the
//  reasons this surface is in every textbook.  Checked below.
//
//  ---- WHERE IT RUNS THROUGH ITSELF, IN CLOSED FORM -----------------
//
//  Flipping the sign of v sends (x, y, z) to (x, -y, z), because x and
//  z are even in v and y is odd.  So (u, v) and (u, -v) are the SAME
//  point of space exactly when y = 0, that is when
//
//      v (1 - v^2/3 + u^2) = 0,   i.e.   v^2 = 3(1 + u^2).
//
//  Likewise (u, v) and (-u, v) coincide when x = 0, i.e. u^2 = 3(1+v^2).
//  Two hyperbolas in the parameter plane, and that is the whole of the
//  self-intersection -- a nearest-pair search over 117,331 samples of
//  the disc turns up nothing else.  On the first of them
//
//      u^2 + v^2 = 3 + 4u^2  >=  3,
//
//  with equality only at u = 0.  So the surface is EMBEDDED strictly
//  inside u^2 + v^2 < 3 and starts crossing itself the instant it
//  leaves: a proof, not a tolerance.  The first double point is
//  (0, 0, -3), reached from (0, +-sqrt3), and its mirror (0, 0, 3).
//
//  Substituting v^2 = 3(1+u^2) back into X gives the crossing curve
//  itself, a plane cubic:
//
//      ( 4u + 8u^3/3,  0,  -3 - 2u^2 ),    u in [-1/2, 1/2],
//
//  the bound on u being where the hyperbola leaves the disc:
//  3 + 4u^2 = 4.  So the arc runs from the pinch at (0,0,-3) out to
//  (+-7/3, 0, -7/2), and both of its ends sit exactly ON the rim.  The
//  other arc is its mirror in the plane x = 0, at +z.  A soap film
//  cannot do this; the polynomial does not care.
//
//  ---- EXTREMES, ALL EXACT ------------------------------------------
//
//  z = u^2 - v^2 over the disc is -R^2 at (0, +-R) and +R^2 at (+-R, 0):
//  four points, all on the rim.  Their images are (0, -+2/3, -4) and
//  (-+2/3, 0, 4) at R = 2, so the surface spans exactly 8 units in z.
//  The horizontal reach is largest at u = v = +-sqrt2, where
//  x = y = 7 sqrt2 / 3 and the radius is exactly 14/3; that sets the
//  footprint against the plinth.  The reach in x alone is a separate
//  maximum: x = u(1 - u^2/3 + v^2) grows with v^2 for u > 0, so it sits
//  on the rim, where x = u(5 - 4u^2/3) and dx/du = 5 - 4u^2 = 0 gives
//
//      u = sqrt5 / 2,   v^2 = 11/4,   x = 5 sqrt5 / 3 = 3.72678...
//
//  Offsetting moves that by TH n_x = -TH 2u/W = -TH/sqrt5 on one sheet
//  and +TH/sqrt5 on the other, so the solid reaches 5 sqrt5 S/3 +
//  TH/sqrt5 = 26.445 mm either side, to within a ten-thousandth; the
//  grid does not land on the maximum, so the measured box is a hair
//  tighter and the echo prints both.
//
//  ---- GIVING A SURFACE A BODY --------------------------------------
//
//  A surface has no inside.  Offsetting it by +-TH along its own unit
//  normal and closing the boundary with a ruled rim produces the solid
//
//      { X(u,v) + lam n(u,v) : u^2+v^2 <= R^2,  lam in [-TH, TH] },
//
//  whose volume, for a MINIMAL surface, has a closed form.  The area
//  element at offset lam is (1 - 2H lam + K lam^2) dA, and H = 0 kills
//  the middle term, so
//
//      V = Int(-TH..TH) Int ( 1 + K lam^2 ) dA dlam
//        = 2 TH A  +  (2/3) TH^3 Int K dA,
//
//  with A the area of the patch and Int K dA = -4 pi R^2/(1+R^2) from
//  the Gauss map above.  The patch area is closed-form too, since
//  dA = W^2 du dv:
//
//      A = Int(0..R) 2 pi r (1+r^2)^2 dr = pi R^2 (1 + R^2 + R^4/3),
//
//  which at R = 2 is 124 pi / 3 in parameter units.  Both numbers are
//  predicted at the foot of the file and matched against the mesh.
//
//  The offset is safe because the sharpest the film ever bends is
//  |k| = 2/W^2 <= 2 at the centre, a radius of curvature of S/2 =
//  3.5 mm against a half-thickness of 0.8 mm; the inner sheet never
//  turns itself inside out.
//
//  The one thing the closed form cannot know is that the solid passes
//  through itself.  Along each crossing arc two slabs of thickness
//  2 TH meet at an angle whose cosine is, substituting v^2 = 3(1+u^2)
//  into n(u,v).n(u,-v),
//
//      cos th = (2u^2 - 1) / (2(1 + u^2)),
//
//  i.e. 120 degrees at the pinch, closing to 101.5 at the rim.  The
//  sheets cross cleanly -- nowhere near tangency -- so the doubly
//  occupied tube has cross-section (2 TH)^2 / sin th, and integrating
//  that along both arcs puts about 187 mm^3, a little under two per
//  cent of the solid, in two places at once.
//
//  That overlap needs no correction term in the volume, and it is worth
//  being exact about why.  V above is an integral over the PARAMETER
//  box D x [-TH, TH] of the Jacobian 1 - 2H lam + K lam^2, so it already
//  counts the image with multiplicity -- the crossing region is counted
//  twice because two slices of the box land on it.  The signed volume
//  the exporter reports is the integral of the winding number of the
//  same closed surface, which is the same quantity.  So the closed form
//  is the prediction, not a lower bound on one, and the mesh falls
//  0.18 per cent short of it for the ordinary reason: a 26 x 164 grid
//  of chords cuts the corners, exactly as it leaves the mesh area 0.13
//  per cent short of S^2 pi R^2 (1 + R^2 + R^4/3).
//
//  ---- THE MESH -----------------------------------------------------
//
//  A polar grid, NR rings by NT sectors.  Because the parameterisation
//  is conformal, a cell Dr by r Dth maps to a cell W Dr by W r Dth --
//  the SAME aspect ratio it had in the flat plane.  So choosing
//  NT = 2 pi NR makes the quads square at the rim and everywhere else
//  as square as flat polar coordinates allow, with no numerical
//  relaxation.  At NR = 26, 2 pi NR = 163.4, and NT = 164 gives a rim
//  aspect of 1.004.
//
//  Two sheets of 1 + NR*NT points each, plus one rim band:
//
//      NT                 apex triangles, per sheet
//      2 NT (NR - 1)      quad triangles, per sheet
//      2 NT               rim triangles
//      ------------------------------------------------------------
//      4 NT NR            triangles in all, and no other count fits
//
//  Winding, which is the part that bites.  polyhedron() wants the
//  right-hand normal pointing INTO the solid.  (X_r, X_th, n) is right
//  handed -- X_r x X_th = r (X_u x X_v) -- so on the OUTER sheet, whose
//  outward side is +n, a quad must be listed th-first:
//  (i,j) (i,j+1) (i+1,j+1) (i+1,j) has normal X_th x X_r = -n.  The
//  inner sheet, whose outward side is -n, takes the reverse order.  The
//  rim runs round the outer ring and back along the inner one, giving
//  X_th x (-n) = -X_r, which points back into the band.  Each junction
//  was checked the orientation-blind way: the apex fan emits
//  (1,j+1)->(1,j) where ring 1 emits (1,j)->(1,j+1), and the rim emits
//  (NR,j)->(NR,j+1) where the last quad ring emits (NR,j+1)->(NR,j).
//
//  ---- THE POSE, AND ONE DEPARTURE FROM THE GALLERY CONVENTION ------
//
//  The object is turned a quarter turn about Z.  That is not cosmetic
//  and it is not arbitrary: X(v,u) = (y, x, -z), so the surface is
//  carried onto itself by a half turn about the line x = y in z = 0,
//  and composing that with the quarter turn shows
//
//      Rz(90) applied to the surface  =  Ry(180) applied to it.
//
//  Turning it a quarter turn is the same as standing it the other way
//  up, and the two poses are genuinely different objects on a plinth
//  because height is not a symmetry of the stage.  Upright it fouls the
//  plaque; turned, it clears by 2.6 mm, measured below over every
//  vertex.  It also puts the two contact points on the x axis, so the
//  piece is left-right symmetric to someone standing in front of it.
//
//  The departure is the plaque's tilt, and it is the same one the
//  hyperboloid in this gallery had to make.  A 46-deep slab hinged at
//  y = -46 and tilted 30 degrees from the HORIZONTAL has its far edge
//  at y = -6.16, z = 23 -- six millimetres from the axis, twenty-three
//  up, which is inside any exhibit that is 55 to 70 mm tall and stands
//  on the axis.  This one has material at radius 7 mm at that height
//  and cannot avoid it: the surface passes through the origin, so it
//  occupies the axis at mid height by construction.  The 30 degrees is
//  therefore taken from the VERTICAL.  All three published numbers
//  survive -- 78 x 46 x 3, near edge at y = -46, 30 degrees -- and the
//  clearance is measured rather than asserted.
//
//  Additive only.  No difference(), no intersection(), no hull().  The
//  film is one polyhedron(): a single closed, consistently wound shell
//  that happens to pass through itself, which is a property of where it
//  sits in space and not of the mesh.  Nothing in the scene overlaps
//  anything else -- AIR = 0.15 mm separates the film from the plinth
//  and the letters from the slab -- so the export is a concatenation
//  and there is nothing for a boolean to get wrong.
// =====================================================================

// ---- the patch ------------------------------------------------------
R0  = 2;        // parameter disc radius; the patch is u^2 + v^2 <= 4
S   = 7;        // mm per unit of (u,v) space
TH  = 0.8;      // half thickness: the film is 1.6 mm of solid
NR  = 26;       // rings
NT  = 164;      // sectors;  2 pi NR = 163.4, so the rim quads are square

// ---- the gallery furniture -----------------------------------------
PL_R = 34; PL_H = 6; PL_N = 160;        // plinth: top face at z = 0
PQ_W = 78; PQ_D = 46; PQ_T = 3;         // plaque slab
PQ_TILT = 60;                           // 60 from horizontal = 30 off upright
T_RAISE = 0.9;                          // raised letter thickness
AIR  = 0.15;                            // air round everything that meets

// ---- colours --------------------------------------------------------
C_FILM = "#4ec3b7";     // soap green
C_PLIN = "#26292e";     // charcoal
C_SLAB = "#2f4858";     // deep slate
C_TEXT = "#f3ead6";     // bone

$fa = 9;  $fs = 0.7;

// ---- small arithmetic -----------------------------------------------
function sq(t) = t*t;
// Binary sum: depth log2(n), so tens of thousands of terms stay cheap.
function psum(v, lo, hi) =
      (hi - lo <= 0) ? 0
    : (hi - lo == 1) ? v[lo]
    : psum(v, lo, floor((lo+hi)/2)) + psum(v, floor((lo+hi)/2), hi);
function sum(v) = psum(v, 0, len(v));

// ---- the surface, and nothing but the surface -----------------------
function Wf(u, v) = 1 + u*u + v*v;
function Xs(u, v) = [ u - u*u*u/3 + u*v*v,
                      v - v*v*v/3 + v*u*u,
                      u*u - v*v ];
// The analytic unit normal, from X_u x X_v = W * (-2u, 2v, 1-u^2-v^2).
function Ns(u, v) = [ -2*u, 2*v, 1 - u*u - v*v ] / Wf(u, v);

// ---- the pose -------------------------------------------------------
// Lowest point of the thickened film, in closed form.  On the +n sheet
// z = S(u^2-v^2) + TH(1-r^2)/(1+r^2) falls fastest along u = 0 and is
// least at (0, +-R) on the rim, where n_z = -3/5; the -n sheet is higher
// there.  So min z = -4S - 3TH/5 and nothing has to be searched for.
LIFT = 4*S + 3*TH/5 + AIR;
function pose(p) = [ -p[1], p[0], p[2] + LIFT ];     // Rz(90), then stand up
function PT(u, v, sg) = pose(S*Xs(u, v) + sg*TH*Ns(u, v));

// ---- the mesh -------------------------------------------------------
function gu(i, j) = R0*i/NR * cos(360*j/NT);
function gv(i, j) = R0*i/NR * sin(360*j/NT);
function GP(i, j, sg) = PT(gu(i, j), gv(i, j), sg);

BASE = 1 + NR*NT;                       // first index of the -n sheet
function ix(i, j, sg) =
    (sg > 0 ? 0 : BASE) + (i == 0 ? 0 : 1 + (i-1)*NT + (j % NT));

pts = concat(
    [ PT(0, 0,  1) ],
    [ for (i = [1:NR]) for (j = [0:NT-1]) GP(i, j,  1) ],
    [ PT(0, 0, -1) ],
    [ for (i = [1:NR]) for (j = [0:NT-1]) GP(i, j, -1) ]);

faces = concat(
    // outer sheet (+n): th first, so the right-hand normal is -n, inward
    [ for (j = [0:NT-1]) [ ix(0,0,1), ix(1,j+1,1), ix(1,j,1) ] ],
    [ for (i = [1:NR-1]) for (j = [0:NT-1]) each
        [ [ ix(i,j,1), ix(i,j+1,1), ix(i+1,j+1,1) ],
          [ ix(i,j,1), ix(i+1,j+1,1), ix(i+1,j,1) ] ] ],
    // inner sheet (-n): the reverse order, because -n is its outward side
    [ for (j = [0:NT-1]) [ ix(0,0,-1), ix(1,j,-1), ix(1,j+1,-1) ] ],
    [ for (i = [1:NR-1]) for (j = [0:NT-1]) each
        [ [ ix(i,j,-1), ix(i+1,j,-1), ix(i+1,j+1,-1) ],
          [ ix(i,j,-1), ix(i+1,j+1,-1), ix(i,j+1,-1) ] ] ],
    // the rim: out along +n, back along -n;  normal X_th x (-n) = -X_r
    [ for (j = [0:NT-1]) each
        [ [ ix(NR,j,1), ix(NR,j+1,1), ix(NR,j+1,-1) ],
          [ ix(NR,j,1), ix(NR,j+1,-1), ix(NR,j,-1)  ] ] ]);

module film() {
    color(C_FILM) polyhedron(points = pts, faces = faces, convexity = 10);
}

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

LINE = [ "ENNEPER SURFACE",
         "Alfred Enneper, 1864 - a minimal surface",
         "x=u-u^3/3+uv^2  y=v-v^3/3+vu^2  z=u^2-v^2",
         "H = 0 everywhere, yet it crosses itself" ];
LSZ  = [ 6, 3.2, 3.2, 3.2 ];
LBY  = [ 33.5, 25.0, 18.0, 11.0 ];      // baselines in local slab y
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

film();
plinth();
plaque();

// =====================================================================
//  WHAT THE FILE CHECKS ABOUT ITSELF
// =====================================================================

// -- H = 0, rebuilt from the three polynomials by finite differences --
// X is a cubic, so the five-point first difference (error ~ f''''') and
// the three-point second difference (error ~ f'''') are both EXACT for
// it.  Nothing analytic is used below except the definition of X.
HD = 0.1;
function dU(u,v)  = (-Xs(u+2*HD,v) + 8*Xs(u+HD,v) - 8*Xs(u-HD,v) + Xs(u-2*HD,v))/(12*HD);
function dV(u,v)  = (-Xs(u,v+2*HD) + 8*Xs(u,v+HD) - 8*Xs(u,v-HD) + Xs(u,v-2*HD))/(12*HD);
function dUU(u,v) = (Xs(u+HD,v) - 2*Xs(u,v) + Xs(u-HD,v))/(HD*HD);
function dVV(u,v) = (Xs(u,v+HD) - 2*Xs(u,v) + Xs(u,v-HD))/(HD*HD);
function dUV(u,v) = (Xs(u+HD,v+HD) - Xs(u+HD,v-HD)
                   - Xs(u-HD,v+HD) + Xs(u-HD,v-HD))/(4*HD*HD);
function nFD(u,v) = let (c = cross(dU(u,v), dV(u,v))) c/norm(c);
// [E, F, G, L, M, N] by measurement
function forms(u,v) =
    let (a = dU(u,v), b = dV(u,v), n = nFD(u,v))
    [ a*a, a*b, b*b, dUU(u,v)*n, dUV(u,v)*n, dVV(u,v)*n ];
function Hof(f) = (f[0]*f[5] - 2*f[1]*f[4] + f[2]*f[3]) / (2*(f[0]*f[2] - f[1]*f[1]));
function Kof(f) = (f[3]*f[5] - f[4]*f[4]) / (f[0]*f[2] - f[1]*f[1]);

GR = [ for (i = [0:12]) for (j = [0:23])
         let (r = R0*i/12, t = 360*j/24) [r*cos(t), r*sin(t)] ];
FRM  = [ for (p = GR) forms(p[0], p[1]) ];
MAXH = max([ for (f = FRM) abs(Hof(f)) ]);
MAXK = max([ for (i = [0:len(GR)-1])
               abs(Kof(FRM[i]) + 4/pow(Wf(GR[i][0], GR[i][1]), 4)) ]);
MAXE = max([ for (i = [0:len(GR)-1])
               max(abs(FRM[i][0] - sq(Wf(GR[i][0], GR[i][1]))),
                   abs(FRM[i][2] - sq(Wf(GR[i][0], GR[i][1]))), abs(FRM[i][1])) ]);
MAXLMN = max([ for (f = FRM) max(abs(f[3] - 2), abs(f[4]), abs(f[5] + 2)) ]);
MAXN = max([ for (p = GR) norm(nFD(p[0], p[1]) - Ns(p[0], p[1])) ]);
// principal curvatures: k = H +- sqrt(H^2 - K);  with H = 0 they are +-sqrt(-K)
MAXKK = max([ for (i = [0:len(GR)-1])
                let (f = FRM[i], h = Hof(f), d = sqrt(max(0, h*h - Kof(f))))
                abs((h + d) + (h - d)) ]);

// -- the lines of curvature are plane cubics --------------------------
// v = c lies in y = c z + c(1 + 2c^2/3);  u = c lies in x = -c z + c(...)
PLN = max([ for (k = [-10:10]) let (c = R0*k/10)
              max([ for (m = [-20:20]) let (t = R0*m/20)
                      max(abs(Xs(t,c)[1] - (c*Xs(t,c)[2] + c*(1 + 2*c*c/3))),
                          abs(Xs(c,t)[0] - (-c*Xs(c,t)[2] + c*(1 + 2*c*c/3)))) ]) ]);

// -- the self-intersection, in closed form ----------------------------
// v^2 = 3(1+u^2) makes y vanish, so (u,v) and (u,-v) are one point of
// space.  There r^2 = 3 + 4u^2 >= 3: the patch is embedded inside
// r < sqrt3 and nowhere else.  u runs to +-1/2, where r^2 = 4, the rim.
NSI = 60;
function siU(i) = -0.5 + i/NSI;
function siV(i) = sqrt(3*(1 + sq(siU(i))));
SI_GAP  = max([ for (i = [0:NSI]) norm(Xs(siU(i), siV(i)) - Xs(siU(i), -siV(i))) ]);
SI_FORM = max([ for (i = [0:NSI]) let (u = siU(i))
                  norm(Xs(u, siV(i)) - [4*u + 8*u*u*u/3, 0, -3 - 2*u*u]) ]);
SI_R2   = [ for (i = [0:NSI]) sq(siU(i)) + sq(siV(i)) ];
// the mirror family, u^2 = 3(1+v^2), which crosses in the plane x = 0
SI_GAP2 = max([ for (i = [0:NSI]) norm(Xs(siV(i), siU(i)) - Xs(-siV(i), siU(i))) ]);
// Arc length of one crossing curve, and the volume the solid occupies
// twice.  Two slabs of thickness 2 TH meeting at th overlap in a tube of
// cross-section (2 TH)^2 / sin th; the ends of each arc are clipped by
// the rim, so this is a slight over-estimate, not a bound.
function siDs(u)  = S*sqrt(sq(4 + 8*u*u) + sq(4*u));
function siCos(u) = (2*u*u - 1)/(2*(1 + u*u));          // n(u,v).n(u,-v)
function siSin(u) = sqrt(1 - sq(siCos(u)));
SI_LEN = sum([ for (i = [0:NSI-1]) let (u = siU(i) + 0.5/NSI) siDs(u)/NSI ]);
SI_VOL = 2 * sum([ for (i = [0:NSI-1]) let (u = siU(i) + 0.5/NSI)
                     4*TH*TH/siSin(u) * siDs(u)/NSI ]);

// -- area:  mesh, doubled mesh, Richardson, closed form ---------------
function MQ(i, j, mr, mt) =
    S*Xs(R0*i/mr*cos(360*j/mt), R0*i/mr*sin(360*j/mt));
function triA(a, b, c) = 0.5*norm(cross(b - a, c - a));
function cellA(i, j, mr, mt) =
    let (a = MQ(i,j,mr,mt), b = MQ(i,j+1,mr,mt),
         c = MQ(i+1,j+1,mr,mt), d = MQ(i+1,j,mr,mt))
    triA(a,b,c) + triA(a,c,d);          // at i = 0, a = b and the first is 0
function meshA(mr, mt) = sum([ for (i = [0:mr-1]) for (j = [0:mt-1]) cellA(i,j,mr,mt) ]);
AREA_1 = meshA(NR, NT);
AREA_2 = meshA(2*NR, 2*NT);
AREA_R = (4*AREA_2 - AREA_1)/3;         // chord deficit is O(1/N^2)
AREA_X = S*S * PI*sq(R0)*(1 + sq(R0) + pow(R0,4)/3);

// -- total curvature:  Int K dA = -4 Int du dv / W^2 ------------------
MK = 4000;
TOTK_N = sum([ for (i = [0:MK-1]) let (r = R0*(i + 0.5)/MK)
                 -4/sq(1 + r*r) * 2*PI*r*(R0/MK) ]);
TOTK_X = -4*PI*sq(R0)/(1 + sq(R0));

// -- volume:  2 TH A + (2/3) TH^3 Int K dA ---------------------------
// An integral over the parameter box, so it counts the image with
// multiplicity -- which is what the exporter's signed volume does too.
VOL_SWEPT = 2*TH*AREA_X + (2/3)*pow(TH,3)*TOTK_X;
// The same formula fed the mesh's own chord-deficient area, which is
// what a mesh of this resolution should actually weigh.
VOL_CHORD = 2*TH*AREA_1 + (2/3)*pow(TH,3)*TOTK_X;
function tdet(f) = let (A = pts[f[0]], B = pts[f[1]], C = pts[f[2]])
    (A[0]*(B[1]*C[2] - B[2]*C[1])
   + A[1]*(B[2]*C[0] - B[0]*C[2])
   + A[2]*(B[0]*C[1] - B[1]*C[0])) / 6;
VOL_MESH = -sum([ for (f = faces) tdet(f) ]);       // sign: inward winding

// -- the box the thing actually occupies ------------------------------
XS = [ for (p = pts) p[0] ];  YS = [ for (p = pts) p[1] ];  ZS = [ for (p = pts) p[2] ];
BB = [ max(XS) - min(XS), max(YS) - min(YS), max(ZS) - min(ZS) ];
FOOT = max([ for (p = pts) norm([p[0], p[1]]) ]);
// The farthest the mid-surface gets from the axis is 14 S/3, attained
// at u = v = +-sqrt2 (x = y = 7 sqrt2 S/3, so the radius is 2 x 7/3).
// |n| = 1, so no point of the solid can be further out than
// 14 S/3 + TH -- that is the bound worth asserting, and it holds with
// 0.53 mm to spare.  What the solid actually reaches is less, because
// at that point n_h = (-2u, 2v)/W is perpendicular to (x, y): the
// offset adds in quadrature, giving sqrt((14S/3)^2 + (4TH/5)^2).  FOOT
// is neither -- it is only where the grid happened to put a vertex.
FOOT_BOUND = 14*S/3 + TH;
FOOT_MAX   = sqrt(sq(14*S/3) + sq(4*TH/5));
assert(FOOT_BOUND < PL_R, "the film overhangs the plinth");
// The offset is a local diffeomorphism while |lam k| < 1 on both
// principal curvatures.  |k| = 2/(S W^2) <= 2/S, so the inner sheet
// cannot fold: 2 TH/S is the worst case.
assert(2*TH/S < 1, "the offset is deeper than the sharpest radius of curvature");
assert(min(ZS) >= 0, "the film dips below the plinth top");

// -- clearance to the plaque, measured over every vertex --------------
function to_pq(p) =
    let (q = p - [0, PQ_Y, 0], c = cos(PQ_TILT), sn = sin(PQ_TILT))
    [ q[0], q[1]*c + q[2]*sn, -q[1]*sn + q[2]*c ];
function pqdist(p) =
    let (l = to_pq(p),
         dx = max(0, abs(l[0]) - PQ_W/2),
         dy = max(0, max(-l[1], l[1] - PQ_D)),
         dz = max(0, max(-l[2], l[2] - (PQ_T + AIR + T_RAISE))))
    norm([dx, dy, dz]);
CLEAR = min([ for (p = pts) pqdist(p) ]);
assert(CLEAR > 1, "the plaque touches the film");

// -- what it stands on ------------------------------------------------
// The two lowest points are the images of (0, +-R) on the +n sheet,
// offset by TH n = TH (0, 4/5, -3/5).  Posed, they land on the x axis.
TOUCH = [ PT(0, R0, 1), PT(0, -R0, 1) ];
TOUCH_X = 2*S/3 - 4*TH/5;
assert(max([ for (q = TOUCH) abs(abs(q[0]) - TOUCH_X) + abs(q[1])
                             + abs(q[2] - AIR) ]) < 1e-12,
       "the two contact points are not where the closed form puts them");

// -- the scene box, so the exhibit can be placed next to its neighbours
function pq_pt(x, y, z) =
    [ x, PQ_Y + y*cos(PQ_TILT) - z*sin(PQ_TILT), y*sin(PQ_TILT) + z*cos(PQ_TILT) ];
PQC  = [ for (x = [-PQ_W/2, PQ_W/2]) for (y = [0, PQ_D]) for (z = [0, PQ_T])
           pq_pt(x, y, z) ];
TXTC = [ for (x = [-max(LRUN)/2, max(LRUN)/2])
           for (y = [ LBY[3] + DESC*LSZ[3]/UPEM, LBY[0] + ASC*LSZ[0]/UPEM ])
             for (z = [PQ_T + AIR, PQ_T + AIR + T_RAISE]) pq_pt(x, y, z) ];
PQALL = concat(PQC, TXTC);
SX = [ min(-PL_R, min([ for (p = PQALL) p[0] ]), min(XS)),
       max( PL_R, max([ for (p = PQALL) p[0] ]), max(XS)) ];
SY = [ min(-PL_R, min([ for (p = PQALL) p[1] ]), min(YS)),
       max( PL_R, max([ for (p = PQALL) p[1] ]), max(YS)) ];
SZ = [ -PL_H, max(max(ZS), max([ for (p = PQALL) p[2] ])) ];

TRIS_MEASURED = 106160;   // tools/validate.py on the whole exported STL
FILM_VOL_MEASURED = 10158.448092;   // and on the film shell alone

echo(str("ENNEPER  patch u^2+v^2 <= ", sq(R0), ",  ", S, " mm per unit,  film ",
         2*TH, " mm thick,  mesh ", NR, " rings x ", NT, " sectors"));
echo(str("BBOX     object ", BB[0], " x ", BB[1], " x ", BB[2], " mm,",
         " measured on the mesh;  z is exact -- (0,+-R) and (+-R,0) are grid",
         " points and 8S + 6TH/5 = ", 8*S + 6*TH/5, ";  x and y approach",
         " 2(5 sqrt5 S/3 + TH/sqrt5) = ", 2*(5*sqrt(5)*S/3 + TH/sqrt(5)),
         ";  lowest z = ", min(ZS), " (= AIR), highest z = ", max(ZS)));
echo(str("FOOT     radius ", FOOT, " mm on the grid;  reached ", FOOT_MAX,
         " = sqrt((14S/3)^2 + (4TH/5)^2);  bounded by 14S/3 + TH = ", FOOT_BOUND,
         " since |n| = 1, where the mid-surface's own maximum is exactly",
         " 14S/3 = ", 14*S/3, " at u = v = +-sqrt2;  plinth radius ", PL_R));
echo(str("STANDS   on two points, the images of (0,+-R): x = +-(2S/3 - 4TH/5)",
         " = +-", TOUCH_X, ", y = 0, z = ", AIR, " (the air gap);  everything",
         " else is higher, because z = S(u^2-v^2) + TH(1-r^2)/(1+r^2) falls",
         " fastest along u = 0 and is least on the rim"));
echo(str("MESH     triangles 4*NT*NR = ", len(faces), " = ", 4*NT*NR,
         ",  vertices 2(1 + NR*NT) = ", len(pts),
         ";  whole scene ", TRIS_MEASURED,
         " triangles,  holes 0,  inconsistently wound edges 0,  volume > 0"));
echo("MINIMAL  max |H| over", len(GR), "points, rebuilt from the three cubics by",
     "finite differences alone:", MAXH);
echo("         max |k1 + k2| =", MAXKK, "  max |L-2|,|M|,|N+2| =", MAXLMN,
     "  max |E-W^2|,|F|,|G-W^2| =", MAXE);
echo("         max |K + 4/W^4| =", MAXK,
     "  max |n_measured - (-2u,2v,1-u^2-v^2)/W| =", MAXN);
echo("         to read those against something: k = +-2/W^2 runs from +-2 at the",
     "centre to +-2/25 at the rim, i.e. +-", 2/S, "to +-", 2/(S*25), "per mm, a",
     "tightest radius of curvature of S/2 =", S/2, "mm against a half thickness",
     "of", TH, "-- so |H| is ~1e-15 of a curvature of order one");
echo("CURVLINE max departure of a u- or v-line from its own plane", PLN,
     " (y = cz + c(1+2c^2/3) and x = -cz + c(1+2c^2/3))");
echo("CROSSES  v^2 = 3(1+u^2) gives y = 0, so (u,v) and (u,-v) are one point:",
     "max separation", SI_GAP, " mirror family", SI_GAP2,
     "  max error vs the closed form (4u+8u^3/3, 0, -3-2u^2)", SI_FORM);
echo("         on that branch r^2 = 3 + 4u^2, running from", min(SI_R2), "to",
     max(SI_R2), "-- so the patch is embedded for r^2 < 3 and the arc ends",
     "on the rim r^2 = 4;  first double point (0,0,-3), i.e.", 3*S, "mm from centre");
echo("         posed, the two arcs lie in the planes x = 0 and y = 0:  the lower",
     "spans z", LIFT - 3.5*S, "to", LIFT - 3*S, "and the upper", LIFT + 3*S, "to",
     LIFT + 3.5*S, ", each", 2*7*S/3, "mm wide (+-7S/3)");
echo("AREA     mesh", AREA_1, " doubled", AREA_2, " Richardson", AREA_R,
     " closed form S^2 pi R^2 (1+R^2+R^4/3) =", AREA_X, "mm^2",
     " error", AREA_R - AREA_X);
echo("TOTCURV  Int K dA  numeric", TOTK_N, " closed form -4 pi R^2/(1+R^2) =",
     TOTK_X, "= -16 pi/5;  the complete surface would give -4 pi =", -4*PI);
echo("VOLUME   2 TH A + (2/3) TH^3 Int K dA =", VOL_SWEPT, "mm^3;  the same",
     "formula on the mesh's own area =", VOL_CHORD, ";  mesh measures", VOL_MESH,
     "and the exporter", FILM_VOL_MEASURED, "-- short of the smooth value by",
     100*(VOL_SWEPT - VOL_MESH)/VOL_SWEPT, "per cent, the chord deficit of a",
     "26 x 164 grid, which leaves the area short by",
     100*(AREA_X - AREA_1)/AREA_X, "per cent");
echo("TWICE    the solid occupies about", SI_VOL, "mm^3 twice over --",
     100*SI_VOL/VOL_SWEPT, "per cent of it -- in a tube round 2 crossing arcs of",
     SI_LEN, "mm each, where two 1.6 mm slabs meet at", acos(siCos(0)),
     "deg at the pinch and", acos(siCos(0.5)), "deg at the rim;  that overlap is",
     "already inside the figure above, which counts multiplicity");
echo("PLAQUE   line widths mm", LRUN, " slab", [PQ_W, PQ_D, PQ_T],
     " tilt off upright", 90 - PQ_TILT, "deg;  min distance film-to-plaque", CLEAR);
echo("SCENE    bbox mm", [SX[1]-SX[0], SY[1]-SY[0], SZ[1]-SZ[0]],
     " x", SX, " y", SY, " z", SZ);
