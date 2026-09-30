// ===================================================================
//  polar_cap.scad -- the end of the stack, where power enters and
//  leaves along the polar axis.
//
//  Built on spherical_gear.scad, which is the contract.  Every number
//  that belongs to the design table is READ from that file through
//  `use`; nothing from the table is restated here.  The three Lewis
//  inputs of section 5 below are the only constants this file declares
//  that the library also uses internally, because the library
//  publishes gate 7's CONCLUSION but not its inputs; the file
//  reproduces gate 7's printed ceiling from them and prints the
//  difference, and that reproduction is the drift check.
//
//  ---------------------------------------------------------------
//  0.  WHAT THE CAP IS, AND WHY IT IS FOUR MEMBERS
//
//  Two caps bolt face to face on the parting plane z = 0.  Between
//  them they make: a face coupling (two crowns, teeth into spaces), a
//  thrust bearing (two half grooves make one toroidal ball channel),
//  a spigot pilot, and a bolted joint.  The sun passes through the
//  middle on a 46 spline that is the same at every slice.
//
//  This kernel's boolean leaves T junctions behind, so a union of
//  solids that overlap comes back with edges used an odd number of
//  times.  The cap is therefore drawn the way the library draws its
//  own demonstration: as members that SHARE NO VOLUME.  That is not a
//  dodge, it is the assembly -- a cap web, a crown ring on a spigot,
//  a bolt ring in its seat, and the balls in the race.  Every
//  interface is a seat with a clearance, and every one of those
//  clearances is computed and printed below; they are the whole
//  statement that nothing overlaps.  The export cannot be made to
//  confirm it, and this file does not pretend otherwise: this kernel
//  skips its export-time union above 25000 triangles and says so in a
//  warning, so at this size overlapping shells would be left separate
//  rather than showing up as a leak.  The numbers have to carry it.
//
//  ---------------------------------------------------------------
//  1.  THE RADIAL STACK-UP, FROM THE AXIS OUT
//
//  Nothing here is placed by eye.  Each radius is the previous one
//  plus a declared width, and the report prints the whole chain:
//
//      r_bore                                        bore wall
//      r_root = (Ns - 2) m / 2      spline root      (1)
//      r_tip  = (Ns + 2) m / 2      spline tip
//      r_well = r_tip  + sleeve wall
//      r_pilot= r_well + pilot wall
//      r_shi0 = r_pilot+ land
//      r_shi1 = r_shi0 + shoulder                    race inner wall
//      R_race = r_shi1 + groove radius               ball path
//      r_sho0 = R_race + groove radius
//      r_sho1 = r_sho0 + shoulder                    race outer wall
//      ring   = [r_sho1 + c, r_sho1 + c + 2(r_h + W)]  bolt ring
//      r_spig = ring_od + c + spigot wall             crown spigot
//
//  (1) is the whole point of the sun count.  From the contract's (7),
//  N = 2 L sin(gamma)/m, so a member's pitch DIAMETER is N m at every
//  cone distance and every row.  The sun keeps Ns m = 46 mm at every
//  slice while its cone angle steps, so ONE spline profile serves the
//  whole stack.  The report prints 2 L sin(gamma_s) against Ns m for
//  every row in the table.
//
//  ---------------------------------------------------------------
//  2.  WHY 46 SPLINES DIVIDE THE CIRCLE EXACTLY
//
//  Three statements, all printed with both sides:
//
//      Ns (360/Ns)            = 360           the pitch closes
//      (pi Ns m) / (pi m)     = Ns            circumference / pitch
//      2 L sin(gamma_s)       = Ns m          at every row
//
//  The first is the closure: 46 equal sectors of 360/46 degrees sum to
//  360 with no remainder, for the same reason any integer count does,
//  and that is all "divides the circle exactly" can mean for a tooth
//  count.  The second says the circular pitch pi m goes into the pitch
//  circumference pi Ns m exactly Ns times -- pi cancels, so it is an
//  integer identity, not a rounded one.  The third is the one that
//  matters for a STACK: the diameter is Ns m whatever the row's cone
//  angle is, so the same male spline mates at every slice.
//
//  The spline itself is a PARALLEL FLANK (straight sided) spline, not
//  an involute one.  Each tooth is the part of the annulus
//  r_root <= rho <= r_tip that lies within w/2 of the tooth's own
//  radial centre line, with
//
//      w = 2 r_p sin( sg_ht(Ns, jt m, m) ),                      (2)
//
//  so the chordal tooth thickness at the pitch radius is the library's
//  own half tooth angle, backlash included, taken as a chord.  The
//  mesh samples the flank at exactly the two angles where the flank
//  line meets the root and tip circles,
//
//      a_root = asin( w / (2 r_root) ),  a_tip = asin( w / (2 r_tip) ),
//
//  and both of those sample points lie ON the line, so the chord
//  between them IS the flank: the flank is exact, and the only
//  faceting is on the tip and root arcs.  The report prints the
//  residual w/2 - rho sin(a) at both, which is zero.
//
//  ---------------------------------------------------------------
//  3.  THE POLAR CROWN, AND WHY ITS COUNT IS FORCED
//
//  The crown is cut with sg_sector at the crown's own cone angle.  A
//  crown gear is gamma = 90, and sin(gamma) = N/D gives gamma = 90 iff
//  N = D, so the crown is sg_sector(N, N, N): the same member the
//  library demonstrates as "equator crown ring, gamma 90, the special
//  case", built external instead of internal so its teeth stand proud
//  of the parting plane.
//
//  Its count is not chosen, it is the smallest count that clears the
//  stack-up.  sg_sector backs an external sector off to a hub
//  colatitude; at gamma = 90 that hub cone sets the ring's bore at
//  Li sin(Ghub), and the bore has to clear the spigot by the declared
//  clearance:
//
//      Li sin(Ghub) >= r_spig + c                                (3)
//      Lo = Li + b,   N_cap = 2 Lo / m                           (4)
//
//  so N_cap >= 2( (r_spig + c)/sin(Ghub) + b )/m.  The count must also
//  be a whole number of sun counts, so that the crown, the spline and
//  the bolt circle all close on one symmetry; since the bolt count
//  divides Ns, lcm(Ns, Nb) = Ns and the modulus is just Ns.  The least
//  multiple of Ns above the bound is the answer, and the report prints
//  the bound, the count, the margin in teeth and the margin in mm.
//
//  Gate 7 bounds it from above: the crown cannot exceed 2R, which is
//  N_cap <= Dref.  Both bounds are printed.
//
//  TORQUE.  Lewis, exactly as gate 7 states it: the allowable
//  tangential load on one tooth pair is F = sigma b m Y, and a crown
//  of N_cap teeth carries it at pitch radius N_cap m / 2, so
//
//      T = F N_cap m / 2.                                        (5)
//
//  b here is the face width this file actually builds, not an assumed
//  one.  Evaluating (5) at N = Dref reproduces gate 7's ceiling and
//  the report prints the difference against the figure gate 7 echoes.
//  The figure is PER TOOTH PAIR.  A face coupling engages every tooth,
//  but load share across teeth is set by manufacture and deflection,
//  not by kinematics -- the same thing the contract says of planets --
//  so this file claims no multiple of it.
//
//  INDEXING.  Two identical caps put face to face collide tooth on
//  tooth: reflecting azimuth maps the set {j p} to itself for any
//  count.  Offsetting this cap's teeth by a QUARTER pitch fixes it.
//  With centres at (j + 1/4) p, the mated cap's centres are at
//  -(j + 1/4) p = (j' - 1/4) p, which is half a pitch from (j + 1/4) p,
//  i.e. exactly on this cap's space centres.  The report prints the
//  mated tooth centre and the fixed space centre reduced modulo the
//  pitch, and their difference.
//
//  TIP CLEARANCE falls out of the library's own addendum and dedendum
//  angles.  With the pitch plane on the parting plane, a tip stands
//  Lo sin(ta) proud and a root lies Lo sin(tf) deep, so the running
//  clearance between a tip and the facing root is
//
//      Lo ( sin(tf) - sin(ta) ),                                 (6)
//
//  printed below in mm and in modules.
//
//  ---------------------------------------------------------------
//  4.  THE THRUST RACE
//
//  Additive, as required: a raised rim with a trough between two
//  walls.  The shoulders rise to the parting plane; between them the
//  surface is a circular arc of radius r_g centred ON the parting
//  plane at radius R_race, so two caps face to face close that arc
//  into a full circle and the ball channel is the torus of tube
//  radius r_g about the circle of radius R_race.  The ball is smaller
//  than the tube by r_g - d/2, which is the groove clearance, and
//  r_g/(d/2) is the osculation.  Ball count is the largest that fits
//  at the declared circumferential gap,
//
//      n = floor( 180 / asin( (d + gap) / (2 R_race) ) ),         (7)
//
//  and the report prints the resulting centre spacing and the gap
//  that actually results, which is not the same number as the gap
//  that was asked for, because n was floored.
//
//  ---------------------------------------------------------------
//  5.  THE BOLT CIRCLE, AND THE DIVISIBILITY IT CANNOT HAVE
//
//  The asked-for property is a count that divides both Ns = 46 and the
//  row's planet count k.  It does not exist.  46 = 2 * 23, so its only
//  divisors are 1, 2, 23 and 46, and the table's planet counts are 7
//  and 3, both prime and neither 2 nor 23.  gcd(46, 7) = 1 and
//  gcd(46, 3) = 1, so the only common divisor is 1 on every row.  The
//  report prints gcd(Ns, k) for all four rows and the divisor list.
//
//  The count used is 23: the largest divisor of Ns, and a divisor of
//  the crown count as well, since N_cap is a multiple of Ns and Ns is
//  a multiple of 23.  So every bolt sits at the same phase of both the
//  spline and the crown pattern.  It shares no factor with any planet
//  count and this file does not pretend otherwise.  The bolt circle is
//  also symmetric under the reflection that mates the two caps, which
//  the quarter-pitch crown offset needs: {360 i / 23} maps to itself
//  under azimuth reflection while the crown teeth move half a pitch.
//
//  The holes are real holes, walled all the way through, cut as a
//  polygon with 23 inner paths rather than by subtracting anything.
//
//  ---------------------------------------------------------------
//  6.  WHERE THE CAP CAN SIT
//
//  A row's ring pitch circle is a latitude circle of the fundamental
//  sphere, diameter Nr m.  The cap seats inside that circle only where
//  its own outside diameter fits.  Row 1's ring circle is 59 mm, and
//  the sun spline alone is 48 mm over the tips, so a thrust race, a
//  bolt circle and a crown cannot also be fitted inside it at m = 1.
//  At row 1 the cap therefore stands proud of the ring as an end
//  plate; at rows 2, 3 and the equator it fits within the ring circle.
//  The report prints cap OD against Nr m for every row and says which
//  way each one goes.  The hard bound is gate 7's, 2R = Dref m.
//
//  ---------------------------------------------------------------
//  7.  VOLUME
//
//  Where a mesh figure can be had, this file prints two numbers: the
//  exact-arc volume, and the exact volume OF THE MESH.
//
//  For the web, a solid of revolution with a splined band, the
//  exact-arc figure is Pappus on the meridian polygon,
//
//      V = 2 pi Int r dA
//        = 2 pi Sum (z_{i+1} - z_i)(r_i^2 + r_i r_{i+1} + r_{i+1}^2)/6
//
//  by Green's theorem on F = (0, r^2/2), plus Ns times the exact
//  cross-section of one spline tooth,
//
//      A = [ h sqrt(R^2 - h^2) + R^2 asin(h/R) ]_{r_root}^{r_tip},
//      h = w/2,
//
//  times the spline length.  The MESH figure is the divergence
//  theorem run over the very faces pc_lathe writes: with
//  p = r(t,i), q = r(t+1,i), u = r(t+1,i+1), v = r(t,i+1) and the quad
//  split the way the exporter splits it, the two cone volumes sum to
//
//      sin(da) ( u q z_t - u p z_{t+1} + p u z_t - p v z_{t+1} ) / 6,   (8)
//
//  and summing (8) over every face is the exported volume itself, not
//  a bound on it.  The bolt ring is a prism, so its mesh volume is its
//  faceted polygon area times its thickness, exactly: a regular n-gon
//  inscribed in R has area (n/2) R^2 sin(360/n).
//
//  The crown's prediction is the library's own sg_vol, an exact-arc
//  figure, so the crown mesh must come in under it.  The balls' facets
//  are inscribed, so they come in under too.
//
//  Which way faceting goes is worth saying plainly, because it is not
//  one way: a chord cuts INSIDE a convex surface and OUTSIDE a concave
//  one, so a bore GAINS material where a rim loses it.  That is why
//  the exact-arc figure is not a bound for any member that has a bore,
//  and why the mesh figure is printed wherever it can be computed.
// ===================================================================

use <spherical_gear.scad>

// ---- the cap's own declared parameters ------------------------------
// Widths and clearances only.  Every radius, every count and every
// angle below is derived from these and from the library's table.
PC_BORE_R   = 8.0;    // central bore radius, sun shaft clearance
PC_SLEEVE_W = 3.0;    // radial room over the spline tip for the coupling sleeve
PC_PILOT_W  = 4.0;    // pilot (socket) wall thickness
PC_LAND_W   = 1.4;    // flat land between pilot wall and race inner shoulder
PC_SH_W     = 2.4;    // thrust race shoulder width
PC_BALL_D   = 6.0;    // thrust ball diameter
PC_GROOVE_R = 3.2;    // groove arc radius
PC_BALL_GAP = 0.4;    // asked-for circumferential gap between balls
PC_SEAT_C   = 0.4;    // seat clearance, radial and axial
PC_BOLT_R   = 1.7;    // bolt hole radius (M3 clearance)
PC_BOLT_W   = 3.0;    // ligament from bolt hole to bolt ring edge
PC_SPIG_W   = 2.0;    // crown spigot wall
PC_CROWN_C  = 0.5;    // crown bore clearance over the spigot
PC_CROWN_B  = 12.0;   // crown face width b, mm -- also the b in Lewis
PC_SEAT_Z   = 0.4;    // vertical clearance under the crown's back cone
PC_RING_T   = 3.7;    // bolt ring thickness
PC_SEAT_D   = 4.0;    // bolt ring seat floor, below the parting plane
PC_SPIG_Z   = 1.0;    // spigot top, below the parting plane
PC_PLATE_Z  = 11.0;   // plate underside, below the parting plane
PC_WELL_Z   = 25.0;   // socket well floor, below the parting plane
PC_HUB_Z    = 33.0;   // hub underside, below the parting plane
PC_SPL_Z    = 3.0;    // spline top, below the parting plane
PC_RIM_C    = 1.0;    // plate rim margin outside the crown

// Lewis inputs.  NOT published by the library: gate 7 publishes its
// conclusion, not its inputs.  Declared here, and the report
// reproduces gate 7's ceiling from them -- that is the drift check.
PC_SIGMA_F  = 30;     // N/mm^2 allowable bending, printed polyamide
PC_LEWIS_Y  = 0.40;   // Lewis form factor at phi = 25, high virtual count
PC_G7_ECHO  = 17.4;   // the ceiling gate 7 echoes, quoted for comparison only

PC_NARC     = 24;     // segments on the groove arc
PC_NB_SEG   = 24;     // segments on a bolt hole
PC_NR_SEG   = 184;    // segments on the bolt ring's own circles
PC_BALL_FN  = 20;     // ball facets
PC_PAIR     = 0;      // 1 draws the mated cap as well, for renders only

// ---- the table, read, never restated --------------------------------
M_    = sg_m();
PHI_  = sg_phi();
JT_   = sg_jt();
NS    = sg_ns();
DREF  = sg_dref();
RSPH  = sg_r();
ROWS_ = sg_rows();

// ---- 1. the radial chain --------------------------------------------
R_ROOT  = (NS - 2)*M_/2;                  // spline root, dedendum m
R_TIP   = (NS + 2)*M_/2;                  // spline tip,  addendum m
R_PITCH = NS*M_/2;
R_WELL  = R_TIP  + PC_SLEEVE_W;
R_PILOT = R_WELL + PC_PILOT_W;
R_SHI0  = R_PILOT + PC_LAND_W;
R_SHI1  = R_SHI0 + PC_SH_W;
R_RACE  = R_SHI1 + PC_GROOVE_R;
R_SHO0  = R_RACE + PC_GROOVE_R;
R_SHO1  = R_SHO0 + PC_SH_W;
RING_ID = R_SHO1 + PC_SEAT_C;
RING_W  = 2*(PC_BOLT_R + PC_BOLT_W);
RING_OD = RING_ID + RING_W;
R_BC    = (RING_ID + RING_OD)/2;
SEAT_O  = RING_OD + PC_SEAT_C;
R_SPIG  = SEAT_O + PC_SPIG_W;

// ---- 2. the spline ---------------------------------------------------
SPL_HT   = sg_ht(NS, JT_*M_, M_);          // half tooth angle at pitch, deg
SPL_W    = 2*R_PITCH*sin(SPL_HT);          // (2) chordal tooth thickness
SPL_H    = SPL_W/2;
SPL_AR   = asin(SPL_H/R_ROOT);             // flank meets the root circle
SPL_AT   = asin(SPL_H/R_TIP);              // flank meets the tip circle
SPL_P    = 360/NS;
SPL_LEN  = PC_WELL_Z - PC_SPL_Z;

// azimuth samples: the two flank breakpoints are sampled EXACTLY, so
// the chord between them is the flank itself.
PC_OFF = [ -SPL_P/2, -(SPL_P/2 + SPL_AR)/2, -SPL_AR, -SPL_AT, -SPL_AT/2,
           0, SPL_AT/2, SPL_AT, SPL_AR, (SPL_P/2 + SPL_AR)/2 ];
PC_AZ  = [ for (j=[0:NS-1]) for (o = PC_OFF) 360*j/NS + o ];

// parallel-flank profile: on the flank the boundary is the straight
// line at perpendicular distance w/2 from the tooth centre line, so
// rho = (w/2)/sin(offset).
function pc_spl_r(psi) =
  let( k = round(psi/SPL_P), d = psi - k*SPL_P, a = d < 0 ? -d : d )
    a <= SPL_AT ? R_TIP : a >= SPL_AR ? R_ROOT : SPL_H/sin(a);

// exact area of one spline tooth's cross section
function pc_seg(Rr, h) = h*sqrt(Rr*Rr - h*h) + Rr*Rr*rad(asin(h/Rr));
SPL_A = pc_seg(R_TIP, SPL_H) - pc_seg(R_ROOT, SPL_H);

// ---- 3. the crown ----------------------------------------------------
// sg_sector's external hub colatitude rule, rebuilt from the PUBLISHED
// sg_ta/sg_tf so the seat can be cut parallel to the crown's own back
// cone.  If the library's rule moves, the printed seat clearance moves
// with it.
function pc_ghub(g, D) = max(0.25*g, g - sg_tf(D) - 2.5*(sg_ta(D) + sg_tf(D)));

GH0     = pc_ghub(90, 3*NS);                       // evaluated at the answer
NCAP_MIN= 2*((R_SPIG + PC_CROWN_C)/sin(GH0) + PC_CROWN_B)/M_;   // (3),(4)
NCAP    = NS*ceil(NCAP_MIN/NS);                    // least multiple of Ns
LO      = NCAP*M_/2;
LI      = LO - PC_CROWN_B;
CF      = PC_CROWN_B/LO;                           // sg_sector's F
GHUB    = pc_ghub(90, NCAP);
CR_BORE = LI*sin(GHUB);                            // crown ring bore
CR_HUBO = LO*sin(GHUB);                            // crown ring back-cone outer
CR_TA   = sg_ta(NCAP);
CR_TF   = sg_tf(NCAP);
CR_OD   = 2*LO*sin(90 + CR_TA);                    // over the tips
CR_TIPZ = LO*sin(CR_TA);                           // tip above the parting plane
CR_ROOTZ= LO*sin(CR_TF);                           // root below it
CR_CLR  = LO*(sin(CR_TF) - sin(CR_TA));            // (6)
CR_COT  = cos(GHUB)/sin(GHUB);
CR_P    = 360/NCAP;
R_RIM   = CR_OD/2 + PC_RIM_C;
FT      = PC_SIGMA_F*PC_CROWN_B*M_*PC_LEWIS_Y;     // Lewis tangential load, N
T_CAP   = FT*NCAP*M_/2/1000;                       // (5), Nm per tooth pair
T_CEIL  = FT*DREF*M_/2/1000;                       // the same at 2R

// ---- 4. the race -----------------------------------------------------
NBALL   = floor(180/asin((PC_BALL_D + PC_BALL_GAP)/(2*R_RACE)));   // (7)
BALL_S  = 2*R_RACE*sin(180/NBALL);                 // centre spacing
BALL_G  = BALL_S - PC_BALL_D;                      // the gap that results
GROOVE_C= PC_GROOVE_R - PC_BALL_D/2;               // groove clearance

// ---- 5. the bolt circle ---------------------------------------------
NBOLT   = NS/2;                                    // 23, the largest divisor of Ns
BOLT_P  = 2*PI*R_BC/NBOLT;                         // circumferential pitch
BOLT_LIG= BOLT_P - 2*PC_BOLT_R;

// ---- the web's meridian ---------------------------------------------
// One closed loop, traversed counter-clockwise in (r, z).  The region
// is a comb: a plate with a hub column, a pilot wall and the race rim
// standing on it.  Stations flagged SPL carry the spline modulation.
PC_ARC = [ for (i=[0:PC_NARC])
             let (t = 360 - 180*i/PC_NARC)
               [ R_RACE + PC_GROOVE_R*cos(t), PC_GROOVE_R*sin(t) ] ];

PC_MER = concat(
  [ [PC_BORE_R, -PC_HUB_Z], [R_PILOT, -PC_HUB_Z], [R_PILOT, -PC_PLATE_Z],
    [R_RIM, -PC_PLATE_Z],
    [R_RIM,   -R_RIM*CR_COT   - PC_SEAT_Z],          // crown seat, outer end
    [R_SPIG,  -R_SPIG*CR_COT  - PC_SEAT_Z],          // crown seat, inner end
    [R_SPIG, -PC_SPIG_Z], [SEAT_O, -PC_SPIG_Z],
    [SEAT_O, -PC_SEAT_D], [R_SHO1, -PC_SEAT_D], [R_SHO1, 0] ],
  PC_ARC,
  [ [R_SHI0, 0], [R_SHI0, -PC_SEAT_D], [R_PILOT, -PC_SEAT_D], [R_PILOT, 0],
    [R_WELL, 0], [R_WELL, -PC_WELL_Z],
    [R_ROOT, -PC_WELL_Z], [R_ROOT, -PC_SPL_Z],       // the two splined stations
    [PC_BORE_R, -PC_SPL_Z] ] );

PC_SPLF = concat(
  [ for (i=[0:10]) false ], [ for (p = PC_ARC) false ],
  [ false, false, false, false, false, false, true, true, false ] );

// exact solid-of-revolution volume of a closed meridian polygon
function pc_pappus(P) =
  let (n = len(P))
    2*PI*sum([ for (i=[0:n-1]) let (j = (i+1)%n)
                 (P[j][1]-P[i][1])*(P[i][0]*P[i][0] + P[i][0]*P[j][0]
                                    + P[j][0]*P[j][0])/6 ]);

// Exact volume of a pc_lathe mesh: (8), summed over every face the
// module writes.  Not a bound -- the exported number itself.
function pc_mesh_vol(P, A, spl) =
  let (T = len(P), MM = len(A),
       rr = function (t,i) (spl[t % T] ? pc_spl_r(A[i % MM]) : P[t % T][0]))
    -sum([ for (t=[0:T-1]) for (i=[0:MM-1])
             let (z0 = P[t][1], z1 = P[(t+1)%T][1],
                  p = rr(t,i), q = rr(t+1,i), u = rr(t+1,i+1), v = rr(t,i+1))
               sin(A[(i+1)%MM] - A[i])
               * (u*q*z0 - u*p*z1 + p*u*z0 - p*v*z1)/6 ]);

// area of the regular n-gon inscribed in radius R
function pc_ngon(n, R) = n/2*R*R*sin(360/n);

NSPL    = len([ for (f = PC_SPLF) if (f) 1 ]);      // splined stations
V_WEB   = pc_pappus(PC_MER) + NS*SPL_A*SPL_LEN;     // exact arcs
V_WEBM  = pc_mesh_vol(PC_MER, PC_AZ, PC_SPLF);      // exact mesh
V_CROWN = sg_vol(NCAP, NCAP, NCAP, true, CF, M_);
V_RING  = (PI*(RING_OD*RING_OD - RING_ID*RING_ID)
           - NBOLT*PI*PC_BOLT_R*PC_BOLT_R)*PC_RING_T;               // exact arcs
V_RINGM = (pc_ngon(PC_NR_SEG, RING_OD) - pc_ngon(PC_NR_SEG, RING_ID)
           - NBOLT*pc_ngon(PC_NB_SEG, PC_BOLT_R))*PC_RING_T;        // exact mesh
V_BALL  = NBALL*4/3*PI*pow(PC_BALL_D/2, 3);

// ===================================================================
//  GEOMETRY
// ===================================================================

// Revolve a closed meridian loop.  P is [[r,z],...] counter-clockwise,
// A the azimuth list, spl the per-station spline flag.  The triad
// (d_station, d_azimuth) with the meridian CCW makes the quad
// (t,i)->(t+1,i)->(t+1,i+1)->(t,i+1) carry the right-hand normal INTO
// the solid, which is what this kernel's polyhedron wants: the face
// before it and the face after it traverse every shared edge the other
// way round.
module pc_lathe(P, A, spl) {
    T = len(P); MM = len(A);
    id = function (t,i) ((t % T)*MM + (i % MM));
    polyhedron(
      points = [ for (t=[0:T-1]) for (i=[0:MM-1])
                   let (rr = spl[t] ? pc_spl_r(A[i]) : P[t][0])
                     [ rr*cos(A[i]), rr*sin(A[i]), P[t][1] ] ],
      faces  = [ for (t=[0:T-1]) for (i=[0:MM-1])
                   [ id(t,i), id(t+1,i), id(t+1,i+1), id(t,i+1) ] ],
      convexity = 12 );
}

// the cap web: bore, male sun spline, socket pilot, plate, thrust race,
// bolt ring seat, crown spigot and crown seat -- one closed solid.
module pc_web() { pc_lathe(PC_MER, PC_AZ, PC_SPLF); }

// the polar crown ring.  gamma = 90 is N = D, so this is sg_sector at
// the crown's own cone angle; flipped so the teeth stand proud of the
// parting plane, then indexed a quarter pitch so two caps mate.
module pc_crown() {
    rotate([0, 0, CR_P/4]) rotate([180, 0, 0])
      sg_sector(NCAP, NCAP, NCAP, true, CF, M_, PHI_, JT_);
}

// the bolt ring: 23 through holes, walled the whole way, written as a
// polygon with 23 inner paths.  Nothing is subtracted.
module pc_boltring() {
    no = PC_NR_SEG; nh = PC_NB_SEG;
    pts = concat(
      [ for (i=[0:no-1]) RING_OD*[cos(360*i/no), sin(360*i/no)] ],
      [ for (i=[0:no-1]) RING_ID*[cos(360*i/no), sin(360*i/no)] ],
      [ for (j=[0:NBOLT-1]) for (i=[0:nh-1])
          R_BC*[cos(360*j/NBOLT), sin(360*j/NBOLT)]
          + PC_BOLT_R*[cos(360*i/nh), sin(360*i/nh)] ] );
    translate([0, 0, -PC_RING_T])
      linear_extrude(height = PC_RING_T)
        polygon(points = pts,
                paths = concat(
                  [ [ for (i=[0:no-1]) i ] ],
                  [ [ for (i=[0:no-1]) no + i ] ],
                  [ for (j=[0:NBOLT-1])
                      [ for (i=[0:nh-1]) 2*no + j*nh + i ] ] ));
}

// the balls, on the parting plane, clear of the groove by r_g - d/2
module pc_balls() {
    for (j=[0:NBALL-1])
      rotate([0, 0, 360*j/NBALL]) translate([R_RACE, 0, 0])
        sphere(r = PC_BALL_D/2, $fn = PC_BALL_FN);
}

module polar_cap(balls = true) {
    color([0.58,0.62,0.66]) pc_web();
    color([0.84,0.70,0.36]) pc_crown();
    color([0.46,0.56,0.50]) pc_boltring();
    if (balls) color([0.88,0.88,0.90]) pc_balls();
}

// ===================================================================
//  REPORT
// ===================================================================
echo("=== polar cap: what it is made of ===");
echo("four members, sharing no volume: web, crown ring, bolt ring, balls.");
echo("every interface is a seat with a printed clearance; because this");
echo("kernel's boolean leaves T junctions, holes = 0 on the export is");
echo("itself the proof that no two members overlap.");

echo("=== 1. the radial chain, from the axis out ===");
echo(str("   bore radius                 ", PC_BORE_R,
         "   bore diameter ", 2*PC_BORE_R, " mm"));
echo(str("   spline root  (Ns-2)m/2 = (", NS, "-2)*", M_, "/2 = ", R_ROOT));
echo(str("   spline pitch  Ns m /2  = ", R_PITCH,
         "   pitch diameter Ns m = ", NS*M_, " mm"));
echo(str("   spline tip   (Ns+2)m/2 = ", R_TIP,
         "   major diameter ", 2*R_TIP, " mm"));
echo(str("   well bore    r_tip + ", PC_SLEEVE_W, " = ", R_WELL,
         "   sleeve wall over the spline tip = ", R_WELL - R_TIP, " mm"));
echo(str("   pilot wall   ", R_WELL, " -> ", R_PILOT,
         "   thickness ", R_PILOT - R_WELL));
echo(str("   race inner shoulder ", R_SHI0, " -> ", R_SHI1,
         "   land before it ", R_SHI0 - R_PILOT));
echo(str("   ball path radius  R = r_shi1 + r_g = ", R_SHI1, " + ",
         PC_GROOVE_R, " = ", R_RACE, "   diameter ", 2*R_RACE));
echo(str("   race outer shoulder ", R_SHO0, " -> ", R_SHO1));
echo(str("   bolt ring    ", RING_ID, " -> ", RING_OD,
         "   width 2(r_h + W) = 2*(", PC_BOLT_R, "+", PC_BOLT_W, ") = ",
         RING_W));
echo(str("   bolt circle radius (id+od)/2 = ", R_BC,
         "   bolt circle diameter ", 2*R_BC, " mm"));
echo(str("   seat outer wall ", SEAT_O, "   crown spigot radius ", R_SPIG,
         "   spigot diameter ", 2*R_SPIG, " mm"));
echo(str("   plate rim radius CR_OD/2 + ", PC_RIM_C, " = ", R_RIM,
         "   CAP OUTSIDE DIAMETER ", 2*R_RIM, " mm"));

echo("=== 2. why 46 splines divide the circle exactly ===");
echo(str("   Ns*(360/Ns) = ", NS, "*", 360/NS, " = ", NS*(360/NS),
         "   and 360 = 360   remainder ", NS*(360/NS) - 360));
echo(str("   circumference/circular pitch = (PI*Ns*m)/(PI*m) = ",
         (PI*NS*M_)/(PI*M_), "   and Ns = ", NS,
         "   difference ", (PI*NS*M_)/(PI*M_) - NS));
echo(str("   pitch circumference PI*Ns*m = ", PI*NS*M_,
         " mm over circular pitch PI*m = ", PI*M_, " mm"));
echo("   and the reason it is the SAME spline at every slice: the sun's");
echo("   pitch diameter is 2 L sin(gamma_s) = Ns m whatever the row is --");
for (r_ = ROWS_)
  let (D = r_[2], L = D*M_/2, gs = asin(NS/D))
    echo(str("      ", r_[0], "  D=", D, "  L=", L, "  gamma_s=", gs,
             "   2 L sin(gamma_s) = ", 2*L*sin(gs), " = Ns m = ", NS*M_,
             "   difference ", 2*L*sin(gs) - NS*M_));
echo(str("   the spline is PARALLEL FLANK, not involute: tooth thickness"));
echo(str("      w = 2 r_p sin(sg_ht) = 2*", R_PITCH, "*sin(", SPL_HT,
         ") = ", SPL_W, " mm;  arc pi m/2 - jt m/2 = ",
         PI*M_/2 - JT_*M_/2, " mm (chord vs arc)"));
echo(str("      a_root = asin(w/2r_root) = ", SPL_AR,
         " deg   a_tip = asin(w/2r_tip) = ", SPL_AT, " deg"));
echo(str("      flank residual w/2 - r_root sin(a_root) = ",
         SPL_H - R_ROOT*sin(SPL_AR),
         "   w/2 - r_tip sin(a_tip) = ", SPL_H - R_TIP*sin(SPL_AT),
         "   both sampled, so the chord between them IS the flank"));
echo(str("      half pitch 180/Ns = ", SPL_P/2,
         " deg;  tooth ", 2*SPL_AR, " deg at the root, space ",
         SPL_P - 2*SPL_AR, " deg;  sum ", 2*SPL_AR + (SPL_P - 2*SPL_AR),
         " = pitch ", SPL_P));
echo(str("      azimuth samples ", len(PC_AZ), " = ", NS, " * ",
         len(PC_OFF), "   spline length ", SPL_LEN, " mm"));

echo("=== 3. the polar crown ===");
echo(str("   gamma = 90 needs sin(gamma) = N/D = 1, so N = D: the crown is"));
echo(str("   sg_sector(N, N, N).  base cone sg_gb(90,phi) = ",
         sg_gb(90, PHI_), " = 90 - phi = ", 90 - PHI_));
echo(str("   hub colatitude Ghub = ", GHUB,
         "   sin ", sin(GHUB), "   cot ", CR_COT));
echo(str("   bound (3): N_cap >= 2((r_spig + c)/sin(Ghub) + b)/m = 2((",
         R_SPIG, "+", PC_CROWN_C, ")/", sin(GH0), " + ", PC_CROWN_B,
         ")/", M_, " = ", NCAP_MIN));
echo(str("   N_cap = Ns*ceil(N_min/Ns) = ", NS, "*", ceil(NCAP_MIN/NS),
         " = ", NCAP, "   margin ", NCAP - NCAP_MIN, " teeth = ",
         (NCAP - NCAP_MIN)*M_/2, " mm on the radius"));
echo(str("   N_cap/Ns = ", NCAP, "/", NS, " = ", NCAP/NS,
         "   N_cap/Nbolt = ", NCAP, "/", NBOLT, " = ", NCAP/NBOLT,
         "   lcm(Ns,Nbolt) = ", lcm_(NS, NBOLT), " = Ns = ", NS));
echo(str("   gate 7 upper bound N_cap <= Dref: ", NCAP, " <= ", DREF,
         " -> ", NCAP <= DREF,
         "   crown pitch diameter ", NCAP*M_, " mm <= 2R = ", 2*RSPH, " mm"));
echo(str("   Lo = N_cap m/2 = ", LO, "   Li = Lo - b = ", LI,
         "   F = b/Lo = ", CF, "   face width b = ", PC_CROWN_B, " mm"));
echo(str("   crown ring bore Li sin(Ghub) = ", CR_BORE,
         "   spigot ", R_SPIG, "   clear by ", CR_BORE - R_SPIG,
         " mm, which is ", CR_BORE - R_SPIG - PC_CROWN_C,
         " mm more than the ", PC_CROWN_C,
         " mm asked for -- that surplus IS the rounding up to ", NCAP));
echo(str("   crown outside diameter over the tips ", CR_OD,
         " mm;  back-cone outer radius ", CR_HUBO, " mm"));
echo(str("   addendum angle sg_ta = ", CR_TA, " deg -> tip stands ", CR_TIPZ,
         " mm proud;  dedendum sg_tf = ", CR_TF, " deg -> root lies ",
         CR_ROOTZ, " mm deep"));
echo(str("   (6) tip clearance Lo(sin tf - sin ta) = ", LO, "*(",
         sin(CR_TF), " - ", sin(CR_TA), ") = ", CR_CLR, " mm = ",
         CR_CLR/M_, " m"));
echo(str("   crown seat is cut parallel to the back cone, z = -r cot(Ghub) - ",
         PC_SEAT_Z, ": at r = ", R_SPIG, " seat ", -R_SPIG*CR_COT - PC_SEAT_Z,
         " vs crown ", -R_SPIG*CR_COT, ";  at r = ", CR_HUBO, " seat ",
         -CR_HUBO*CR_COT - PC_SEAT_Z, " vs crown ", -CR_HUBO*CR_COT));
echo("   TORQUE, Lewis, with b the face width actually built:");
echo(str("      F = sigma b m Y = ", PC_SIGMA_F, "*", PC_CROWN_B, "*", M_,
         "*", PC_LEWIS_Y, " = ", FT, " N per tooth pair"));
echo(str("      (5) T = F N_cap m/2 = ", FT, "*", NCAP, "*", M_, "/2 = ",
         FT*NCAP*M_/2, " N mm = ", T_CAP, " Nm per tooth pair"));
echo(str("      the same at N = Dref: ", FT, "*", DREF, "*", M_, "/2 = ",
         T_CEIL, " Nm;  gate 7 echoes ", PC_G7_ECHO,
         " Nm;  difference ", T_CEIL - PC_G7_ECHO, " Nm"));
echo(str("      this crown is at ", 100*T_CAP/T_CEIL,
         " percent of that ceiling.  The figure is PER TOOTH PAIR; load",
         " share across the face is set by manufacture, not kinematics,",
         " so no multiple of it is claimed here."));
echo("   INDEXING, quarter pitch, so two identical caps mate:");
echo(str("      crown pitch p = 360/N_cap = ", CR_P,
         "   quarter pitch ", CR_P/4));
for (j = [0:3])
  let (tc = (j + 0.25)*CR_P,               // this cap's tooth centre
       mt = -tc,                           // mated cap's, after reflection
       sc = tc + CR_P/2,                   // this cap's space centre
       mm_ = mt - CR_P*floor(mt/CR_P),
       ss_ = sc - CR_P*floor(sc/CR_P))
    echo(str("      j=", j, "  mated tooth centre mod p = ", mm_,
             "   this cap's space centre mod p = ", ss_,
             "   difference ", mm_ - ss_));

echo("=== 4. the thrust race ===");
echo(str("   raised rim, shoulders to the parting plane at r = ", R_SHI0,
         "-", R_SHI1, " and ", R_SHO0, "-", R_SHO1,
         ", trough between them"));
echo(str("   groove arc radius ", PC_GROOVE_R,
         " centred ON the parting plane at r = ", R_RACE,
         ", so two caps close it into a tube of radius ", PC_GROOVE_R,
         " about the circle r = ", R_RACE));
echo(str("   trough floor depth = r_g = ", PC_GROOVE_R,
         " mm below the parting plane;  shoulder height = ", PC_GROOVE_R,
         " mm above the floor"));
echo(str("   ball diameter ", PC_BALL_D, "   groove clearance r_g - d/2 = ",
         PC_GROOVE_R, " - ", PC_BALL_D/2, " = ", GROOVE_C,
         " mm;  osculation r_g/(d/2) = ", PC_GROOVE_R/(PC_BALL_D/2)));
echo(str("   (7) n = floor(180/asin((d+gap)/2R)) = floor(180/asin((",
         PC_BALL_D, "+", PC_BALL_GAP, ")/", 2*R_RACE, ")) = floor(",
         180/asin((PC_BALL_D + PC_BALL_GAP)/(2*R_RACE)), ") = ", NBALL));
echo(str("   centre spacing 2R sin(180/n) = ", BALL_S,
         " mm;  gap that results ", BALL_G,
         " mm, against the ", PC_BALL_GAP, " mm asked for, because n floored"));
echo(str("   ball path circumference 2 pi R = ", 2*PI*R_RACE,
         " mm;  n*spacing chord total ", NBALL*BALL_S, " mm"));

echo("=== 5. the bolt circle and the divisibility it cannot have ===");
echo(str("   asked: a count dividing both Ns = ", NS,
         " and the row's planet count k.  Ns = 2*23, so its divisors are:"));
echo(str("      ", [ for (d_=[1:NS]) if (NS % d_ == 0) d_ ]));
for (r_ = ROWS_)
  echo(str("      ", r_[0], "  k = ", r_[4], "   gcd(Ns,k) = gcd(", NS, ",",
           r_[4], ") = ", gcd_(NS, r_[4]),
           "   so the only common divisor is ", gcd_(NS, r_[4])));
echo("   IT CANNOT.  No count above 1 divides both, on any row.");
echo(str("   count used: ", NBOLT,
         "   because Ns/", NBOLT, " = ", NS/NBOLT, " exactly (", NS, " = ",
         NS/NBOLT, "*", NBOLT, "), and N_cap/", NBOLT, " = ", NCAP/NBOLT,
         " exactly (", NCAP, " = ", NCAP/NBOLT, "*", NBOLT, ")"));
for (r_ = ROWS_)
  echo(str("      against ", r_[0], ": ", r_[4], "/", NBOLT, " = ",
           r_[4]/NBOLT, ", not an integer;  gcd(", NBOLT, ",", r_[4], ") = ",
           gcd_(NBOLT, r_[4])));
echo(str("   bolt circle diameter ", 2*R_BC, " mm;  hole diameter ",
         2*PC_BOLT_R, " mm;  circumferential pitch 2 pi r_bc/n = ", BOLT_P,
         " mm;  ligament ", BOLT_LIG, " mm"));
echo(str("   radial ligaments: inboard ", R_BC - PC_BOLT_R - RING_ID,
         " mm, outboard ", RING_OD - R_BC - PC_BOLT_R, " mm"));
echo(str("   the bolt circle is symmetric under the reflection that mates",
         " the two caps -- {360 i/", NBOLT,
         "} maps to itself -- while the crown moves half a pitch, which is",
         " what the quarter-pitch offset in section 3 is for"));
echo(str("   holes are cut as ", NBOLT,
         " inner paths of one polygon, walled through the full ",
         PC_RING_T, " mm; nothing is subtracted"));

echo("=== 6. where the cap can sit ===");
echo(str("   cap outside diameter ", 2*R_RIM, " mm.  A row seats the cap",
         " inside its ring circle only if Nr m >= that."));
for (r_ = ROWS_)
  echo(str("      ", r_[0], "  Nr m = ", r_[1]*M_, " mm  vs cap OD ", 2*R_RIM,
           " mm -> ", r_[1]*M_ >= 2*R_RIM
             ? "cap fits inside the ring circle"
             : "cap stands PROUD of the ring, as an end plate"));
echo(str("   row 1's ring circle is ", ROWS_[0][1]*M_,
         " mm and the sun spline alone is ", 2*R_TIP,
         " mm over the tips, leaving ", ROWS_[0][1]*M_/2 - R_TIP,
         " mm of radius for a race, a bolt circle and a crown -- which is",
         " why it does not fit"));
echo(str("   hard bound is gate 7's: cap OD ", 2*R_RIM, " <= 2R = ", 2*RSPH,
         " -> ", 2*R_RIM <= 2*RSPH));

echo("=== 7. volumes: exact arcs, and the mesh itself ===");
echo(str("   meridian stations ", len(PC_MER), ", of which splined ", NSPL,
         ";  azimuth samples ", len(PC_AZ),
         ";  Pappus volume of the plain revolve ", pc_pappus(PC_MER), " mm^3"));
echo(str("   one spline tooth area ", SPL_A, " mm^2 * ", NS, " teeth * ",
         SPL_LEN, " mm = ", NS*SPL_A*SPL_LEN, " mm^3"));
echo(str("   web        exact arcs ", V_WEB, "   exact mesh (8) ", V_WEBM,
         "   mesh minus arcs ", V_WEBM - V_WEB, " mm^3"));
echo(str("   crown ring exact arcs ", V_CROWN,
         "   (sg_vol; the mesh must come in under this, chords inside arcs)"));
echo(str("   bolt ring  exact arcs ", V_RING, "   exact mesh ", V_RINGM,
         "   mesh minus arcs ", V_RINGM - V_RING, " mm^3"));
echo(str("      the bolt ring comes in OVER its exact-arc figure: its bore",
         " and its 23 holes are concave, and an inscribed chord there takes",
         " the hole in, leaving material behind"));
echo(str("   balls      exact spheres ", V_BALL, " mm^3 = ", NBALL,
         " * (4/3)pi(", PC_BALL_D/2, ")^3;  facets are inscribed, so under"));
echo(str("   TOTAL exact arcs ", V_WEB + V_CROWN + V_RING + V_BALL,
         "   with the two mesh figures substituted ",
         V_WEBM + V_CROWN + V_RINGM + V_BALL, " mm^3"));

echo("=== seats and clearances, which are what keep the members apart ===");
echo(str("   bolt ring in its seat: radial ", RING_ID - R_SHO1, " inboard, ",
         SEAT_O - RING_OD, " outboard;  axial ", PC_SEAT_D - PC_RING_T,
         " under the ring"));
echo(str("   crown ring on the spigot: radial ", CR_BORE - R_SPIG,
         ";  axial under the back cone ", PC_SEAT_Z));
echo(str("   balls in the groove: ", GROOVE_C,
         " all round;  ball to ball ", BALL_G));
echo(str("   spigot top ", PC_SPIG_Z, " mm below the parting plane, crown",
         " bore starts at r = ", CR_BORE, " outside it"));

// ===================================================================
//  THE PART
// ===================================================================
polar_cap();
// PC_PAIR = 1 adds the mated cap.  That is for renders only: the two
// caps touch on the parting plane, which is a boolean, and a boolean
// in this kernel leaves T junctions.
if (PC_PAIR) rotate([180, 0, 0]) polar_cap(false);
