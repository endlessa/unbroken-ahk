// ===================================================================
//  stack.scad -- the machine.  Four planetary rows on one polar axis,
//  the equatorial band as the bottom row's ring, one swappable collar
//  on its register, and a polar cap closing each end.
//
//  Every number here is READ: the design table from spherical_gear.scad,
//  each row's geometry from slice.scad, the register from
//  equator_band.scad, the collars' and the cap's envelopes from their
//  own files.  This file restates nothing.  What it adds is the one
//  thing no part file could have: the four rows in one frame, and the
//  arithmetic that says they do not touch.
//
//  ---------------------------------------------------------------
//  0.  WHAT ASSEMBLING IT FOUND
//
//  Four defects, none of which could be seen from inside a part file,
//  and all four now fixed where they belonged:
//
//    * `use` did not work two levels deep.  A part file opening with
//      `M = sg_m();` was evaluated before the contract it uses, so
//      every number it read came back undef.  One level deep there is
//      nothing to order, so nothing had ever noticed.
//    * the EM collar's clamp seat stood 21.2 mm outboard of the band's
//      register, over the spur teeth, clamping nothing -- and its clamp
//      was 18 mm wide against a 20 mm register, and its anti-rotation
//      feature was a keyway where the band offers a groove.  Both
//      collars had sized their clamps by hand against the band's
//      ECHOED report, because the band published no function to read.
//    * the pin boss band was beta +/- 2 GPIN, a bare factor of two.  On
//      row 1, where it was written, that is 12 degrees.  On the equator
//      row, whose planets are 154 teeth, it is 36, and the retainer
//      came out as a near-hemispherical shell that cut through the
//      band's register lands.
//    * the band's register relief was 2 mm, so its spur flange stood
//      12 mm above the groove centre.  Both collars publish an envelope
//      taller than that.  Either one's clamp lugs fouled the flange.
//
//  The pattern in all four is the same and it is worth naming: a
//  number that two files have to agree on was PUBLISHED by one and
//  COPIED by the other, and the copy was never compared with the
//  original.  An assembly is where the comparison happens.
//
//  ---------------------------------------------------------------
//  1.  WHERE THE ROWS GO
//
//  The contract's section 4 proves that one apex, one module and one
//  sun count cannot carry three latitudinal rows plus the equator, and
//  resolves it by going to a stack: per-slice apexes, one shaft, one
//  sun count Ns, one sun pitch diameter Ns m, stepped cone angles
//  arcsin(Ns/D_i).  This file places those apexes.
//
//  It does NOT place them by (15) -- the rule slice.scad publishes,
//  which puts each row's ring pitch circle on the fundamental sphere at
//  its own latitude.  That rule is what the contract's section 4 says
//  in words, and section 6 below evaluates it and shows it cannot be
//  built: a row is a spherical shell of cone distances 0.75L to L about
//  its apex, the equator row's shell is the whole northern hemisphere
//  of the fundamental sphere, and the other three rows' rings sit ON
//  the surface of that shell.  The suns collide outright, and section 6
//  finds the colliding points rather than arguing about them.
//
//  So the sphere does the job it can do, which is the one the
//  divisibility gate needs: Nr_i = Dref sin(colat_i) with Nr_i an
//  integer selects each row's ring pitch RADIUS, Nr_i m/2 = R
//  sin(colat_i).  Where the row then sits on the axis is a clearance
//  question, and this file answers it by stacking:
//
//      station 0 is the equator row, apex at the sphere centre;
//      station n+1 sits a declared gap above station n.             (16)
//
//  Each row is bounded by two z planes -- every member of it is a
//  spherical shell sector, so its z extreme is at a corner of its
//  (cone distance, colatitude) rectangle -- and (16) makes consecutive
//  rows' intervals disjoint.  Two solids separated by a plane cannot
//  touch, whatever their radii, so the whole between-station clearance
//  argument is one comparison per station and it is exact.  The rows
//  descend in size upwards, biggest at the bottom, which is what makes
//  the intervals nest neatly rather than merely miss.
//
//  ---------------------------------------------------------------
//  2.  THE FOUR SPEEDS
//
//  The sun is common to every row.  Each row's ring is a band with a
//  register, and a collar clamps that register: a chain sprocket, an
//  EM rotor, or a brake.  The carriers are tied to one output -- that
//  tie member is NOT modelled here and is named in section 7.
//
//  With the carriers tied and one ring held, Willis gives
//
//      w_sun / w_carrier = (Ns + Nr_i) / Ns                       (17)
//
//  so holding one row at a time is a gear SELECTION, and the four
//  ratios are exact rationals in the tooth counts.  Section 5 prints
//  them reduced, with the step between consecutive speeds.  Holding
//  the sun instead gives w_ring/w_carrier = (Ns + Nr_i)/Nr_i on the
//  same row, which is the other useful column.
//
//  Two rings driven at once is the differential mode, and it is the
//  reason the collars are the control surface: the torque split between
//  two live rings is set by drag current, not by kinematics.  The
//  contract says that plainly under its section 5 and this file does
//  not improve on it.
//
//  ---------------------------------------------------------------
//  3.  WHAT THIS FILE DRAWS
//
//  Members that SHARE NO VOLUME, like every part file here.  The
//  component count is therefore a necessary check -- 69 bodies must
//  come back as 69 components -- and it is NOT a proof of
//  non-overlap: two shells that interpenetrate without sharing a
//  vertex still count as two.  The printed clearances are the proof,
//  and every one of them is computed.
// ===================================================================

use <spherical_gear.scad>
use <slice.scad>
use <equator_band.scad>
use <collar_chain.scad>
use <collar_em.scad>
use <polar_cap.scad>

// ---- read from the contract -----------------------------------------
M     = sg_m();
R     = sg_r();
NS    = sg_ns();
DREF  = sg_dref();
ROWS  = sg_rows();
NROW  = len(ROWS);

// ---- design choices made HERE ---------------------------------------
STK_GAP  = 2.0;    // axial clearance between consecutive stations, mm
STK_CAPZ = 2.0;    // axial clearance from an end station to its cap, mm
STK_COLLAR = 1;    // 0 bare register, 1 the chain collar, 2 the EM collar
STK_CAPS   = 1;    // 1 draws both polar caps
STK_HITN   = 12;   // grid per side for section 6's collision search

// ---- which row a station carries ------------------------------------
// Biggest at the bottom, so station n counts down the table.  The
// equator row's ring is the equatorial band, so its slice is drawn
// without one.
function stk_row(n)  = NROW - 1 - n;
function stk_ring(i) = i != NROW - 1;

// ---- a member as a (cone distance, colatitude) rectangle ------------
// z = L cos G and r = L sin G are each monotone in L, and z is monotone
// in G, so z's extremes over the rectangle are at corners.  r is not
// monotone in G -- it peaks at G = 90 -- so its maximum is taken at the
// colatitude in range nearest 90.
function stk_zlo(m) = cos(m[3]) >= 0 ? m[0]*cos(m[3]) : m[1]*cos(m[3]);
function stk_zhi(m) = cos(m[2]) >= 0 ? m[1]*cos(m[2]) : m[0]*cos(m[2]);
function stk_rhi(m) = m[1]*sin(max(m[2], min(m[3], 90)));

// the six kinds of body a slice is made of, each as [L0, L1, G0, G1]
function stk_bodies(i) = concat(
  [ [sl_li(i), sl_lo(i), sl_ghs(i), sl_gs(i) + sl_ta(i)] ],          // sun
  stk_ring(i) ? [ [sl_li(i), sl_lo(i),
                   sl_gr(i) - sl_ta(i), sl_ghr(i)] ] : [],           // ring
  [ [sl_li(i), sl_lo(i), sl_beta(i) - sl_gp(i) - sl_ta(i),
                         sl_beta(i) + sl_gp(i) + sl_ta(i)] ],        // planets
  [ [sl_r0(i), sl_rhub1(i), sl_ghi(i), sl_grim1(i)] ],               // carrier
  [ [sl_rpin0(i), sl_rpin1(i),
     sl_beta(i) - sl_gpin(i), sl_beta(i) + sl_gpin(i)] ],            // pins
  [ [sl_rret0(i), sl_rret1(i), sl_grim0(i), sl_grim1(i)] ] );        // retainer

function stk_slo(i)  = min([ for (m = stk_bodies(i)) stk_zlo(m) ]);
function stk_shi(i)  = max([ for (m = stk_bodies(i)) stk_zhi(m) ]);
function stk_srad(i) = max([ for (m = stk_bodies(i)) stk_rhi(m) ]);

// ---- the stations, (16), bottom up ----------------------------------
function stk_apex(n) =
  n <= 0 ? 0
         : stk_apex(n-1) + stk_shi(stk_row(n-1)) + STK_GAP
           - stk_slo(stk_row(n));
function stk_top(n) = stk_apex(n) + stk_shi(stk_row(n));
function stk_bot(n) = stk_apex(n) + stk_slo(stk_row(n));

// ---- the equator station: band, register, collar --------------------
EB_RCH  = eb_if_reach();          // [r_in, r_out, z_lo, z_hi]
COLZ    = eb_if_groove()[1];      // the register plane, in world z
CC_ENV  = cc_if_envelope();       // [half height, half length across the bolt]
EM_ENV  = em_if_span();           // [z_lo, z_hi] about the clamp centre
COL_LO  = STK_COLLAR == 1 ? COLZ - CC_ENV[0]
        : STK_COLLAR == 2 ? COLZ + EM_ENV[0] : EB_RCH[2];
COL_HI  = STK_COLLAR == 1 ? COLZ + CC_ENV[0]
        : STK_COLLAR == 2 ? COLZ + EM_ENV[1] : EB_RCH[2];
COL_RAD = STK_COLLAR == 1 ? cc_if_tip()
        : STK_COLLAR == 2 ? em_if_rad() : 0;
COL_NAME = STK_COLLAR == 1 ? "chain" : STK_COLLAR == 2 ? "EM" : "none";

// ---- the two caps ---------------------------------------------------
PCS     = pc_if_span();           // [z_lo, z_hi] without the balls
STK_TOP = stk_top(NROW-1);
STK_BOT = min(min(EB_RCH[2], COL_LO), stk_bot(0));
CAP_T   = STK_TOP + STK_CAPZ - PCS[0];    // top cap's parting plane
CAP_B   = STK_BOT - STK_CAPZ + PCS[0];    // bottom cap's, flipped
HEIGHT  = (CAP_T + PCS[1]) - (CAP_B - PCS[1]);

// ---- (15), the placement that cannot be built -----------------------
// The sun's SOLID hub band: bore to root cone, material at every
// azimuth.  Two of these overlapping in the (r, z) half plane is a
// collision outright -- no question of whether teeth interleave.
function stk_hub(i) = [sl_li(i), sl_lo(i), sl_ghs(i), sl_gs(i) - sl_tf(i)];
function stk_in(m, a, p) =
  let( L = sqrt(p[0]*p[0] + (p[1]-a)*(p[1]-a)), G = atan2(p[0], p[1]-a) )
    L >= m[0] && L <= m[1] && G >= m[2] && G <= m[3];
function stk_grid(m, a, n) =
  [ for (u = [0:n]) for (v = [0:n])
      let( L = m[0] + (m[1]-m[0])*u/n, G = m[2] + (m[3]-m[2])*v/n )
        [L*sin(G), a + L*cos(G)] ];
function stk_hits(i, j, ai, aj) =
  [ for (p = stk_grid(stk_hub(i), ai, STK_HITN))
      if (stk_in(stk_hub(j), aj, p)) p ];

// ===================================================================
//  THE MACHINE
// ===================================================================
module stack() {
    for (n = [0:NROW-1])
      translate([0, 0, stk_apex(n)])
        sl_slice(stk_row(n), stk_ring(stk_row(n)));
    equator_band();
    if (STK_COLLAR == 1) translate([0, 0, COLZ]) collar_chain();
    if (STK_COLLAR == 2) translate([0, 0, COLZ]) collar_em();
    if (STK_CAPS) {
        translate([0, 0, CAP_T]) polar_cap(false);
        // Turned over, not mirrored: rotate([180,0,0]) has determinant
        // +1, so the shell's winding is carried through unchanged, and
        // it is also what you do to the real part.
        translate([0, 0, CAP_B]) rotate([180, 0, 0]) polar_cap(false);
    }
}
stack();

// ===================================================================
//  REPORT
// ===================================================================
echo("=== stack: the four rows, the band, a collar, two caps ===");
echo("--- read from the contract, not restated ---");
echo(str("   sg_m() = ", M, " mm   sg_r() = ", R, " mm   sg_ns() = ", NS,
         "   sg_dref() = ", DREF, "   rows = ", NROW));
echo(str("   every row's sun has the same pitch diameter Ns m = ", NS*M,
         " mm and the same count ", NS, ".  The cone angle steps:"));
for (i = [0:NROW-1])
  echo(str("      ", ROWS[i][0], "  Nr = ", sl_nr(i), "  D = ", sl_dd(i),
           "  Np = ", sl_np(i), "  k = ", sl_k(i),
           "   gs = asin(", NS, "/", sl_dd(i), ") = ", sl_gs(i),
           "   L = ", sl_lo(i), "   L sin(gs) = ", sl_lo(i)*sin(sl_gs(i)),
           " = Ns m/2 = ", NS*M/2));

echo("--- 1. the stations, (16), bottom up ---");
for (n = [0:NROW-1])
  echo(str("   station ", n, "  ", ROWS[stk_row(n)][0],
           "   apex z = ", stk_apex(n),
           "   own extent ", stk_slo(stk_row(n)), " to ",
           stk_shi(stk_row(n)), "   world ", stk_bot(n), " to ", stk_top(n),
           "   radius ", stk_srad(stk_row(n)),
           n == 0 ? "   (the equator row: its ring is the band)"
                  : str("   gap above station ", n-1, " = ",
                        stk_bot(n) - stk_top(n-1))));
echo(str("   consecutive stations are separated by the planes z = ",
         [ for (n = [1:NROW-1]) (stk_top(n-1) + stk_bot(n))/2 ],
         ", each gap ", STK_GAP,
         " mm.  Two solids on opposite sides of a plane cannot touch, so",
         " no member of one station can reach any member of another",
         " whatever their radii; that is the whole between-station",
         " clearance argument and it is exact."));
echo(str("   the four suns end up at z = ",
         [ for (n = [0:NROW-1]) let(i = stk_row(n))
             stk_apex(n) + sl_lo(i)*cos(sl_gs(i)) ],
         " on one axis, all at pitch radius ", NS*M/2,
         ", so one shaft carries all four"));

echo("--- 2. the equator station: band, register, collar ---");
echo(str("   the band occupies r ", EB_RCH[0], " to ", EB_RCH[1], ", z ",
         EB_RCH[2], " to ", EB_RCH[3],
         "; the equator row's slice, drawn without its ring because the",
         " band IS its ring, occupies z ", stk_slo(NROW-1), " to ",
         stk_shi(NROW-1), " out to r ", stk_srad(NROW-1)));
echo(str("   those two z ranges OVERLAP, by ",
         EB_RCH[3] - stk_slo(NROW-1),
         " mm, and they have to: the band's highest material is its crown",
         " ring's tooth tips and the slice's lowest is the planet tips",
         " that mesh with them.  That is a mesh, not a clash, and the",
         " clearance in it is the one the slice's own report prints as its",
         " ring/planet figure"));
echo(str("   which is only legitimate if the band's crown ring IS the ring",
         " the slice would have drawn.  Both are the same library sector",
         " on the same arguments -- N = Nref = ", sl_nr(NROW-1), ", D = ",
         sl_dd(NROW-1), ", internal, face fraction 0.25, m = ", M,
         ", phi = ", sg_phi(), ", jt = ", sg_jt(),
         " -- and the only thing that could differ is the clocking, which",
         " for this row is sl_clock = ", sl_clock(NROW-1),
         ": zero, because frac((Np-2)/2) = frac(", (sl_np(NROW-1)-2)/2,
         ") = 0.  So the band's unclocked crown is this row's ring",
         " exactly: ", sl_clock(NROW-1) == 0));
echo(str("   register: floor ", eb_if_floor(), ", lands ", eb_if_land()[0],
         " each ", eb_if_land()[1], " wide, groove ", eb_if_groove()[0],
         " centred z = ", COLZ, ", a collar bores ", eb_if_collar()[0],
         " with a rib ", eb_if_collar()[2], " wide to ", eb_if_collar()[1]));
echo(str("   the register allows a hub ", eb_if_hub(),
         " mm tall symmetric on the groove"));
echo(str("   fitted: the ", COL_NAME, " collar.  It occupies z ", COL_LO,
         " to ", COL_HI, " out to r ", COL_RAD));
echo(str("   its top stands ", eb_if_span()[1] - COL_HI,
         " mm below the band's spur flange underside at ", eb_if_span()[1],
         ": clear ", COL_HI <= eb_if_span()[1]));
echo(str("   the chain collar's own envelope is +/-", CC_ENV[0],
         " mm about the register plane and the EM collar's is ", EM_ENV[0],
         " to ", EM_ENV[1], ".  Against the allowance of ",
         eb_if_hub()/2, " mm above the groove centre: chain ",
         eb_if_hub()/2 - CC_ENV[0], " mm to spare, EM ",
         eb_if_hub()/2 - EM_ENV[1],
         ".  Both positive is what the band's relief was lengthened to",
         " 10.5 mm for; at the 2 mm it started with both were negative"));
echo(str("   the two collars are interchangeable on this register",
         " because both READ it: the chain collar's derived bore ",
         cc_if_bore(), " and the EM collar's seat ", em_if_seat(),
         " are the same number, difference ",
         cc_if_bore() - em_if_seat()));

echo("--- 3. the caps ---");
echo(str("   a cap spans z ", PCS[0], " to ", PCS[1], " out to r ",
         pc_if_rad(), ", drawn without balls: a cap at the END of a stack",
         " has no mate to close its half groove into a ball channel"));
echo(str("   top cap parting plane z = ", CAP_T, ", so its lowest point is ",
         CAP_T + PCS[0], " against the top station's ", STK_TOP, ": gap ",
         CAP_T + PCS[0] - STK_TOP));
echo(str("   bottom cap turned over, parting plane z = ", CAP_B,
         ", so its highest point is ", CAP_B - PCS[0], " against the",
         " stack's lowest ", STK_BOT, ": gap ",
         STK_BOT - (CAP_B - PCS[0])));
echo(str("   the cap's crown is ", pc_if_crown()[0],
         " teeth on a gamma = 90 cone of distance ", pc_if_crown()[1],
         ", and the sun spline is ", NS, " teeth -- the same count at",
         " every slice, which is why one cap fits either end"));
echo(str("   overall: z ", CAP_B - PCS[1], " to ", CAP_T + PCS[1], " = ",
         HEIGHT, " mm tall, over a largest radius of ",
         max(EB_RCH[1], COL_RAD, pc_if_rad(),
             max([ for (n = [0:NROW-1]) stk_srad(stk_row(n)) ])),
         " mm"));

echo("--- 4. the stack is not a sphere, and here is the price ---");
echo(str("   the fundamental sphere is R = ", R, ", so a machine that fit",
         " inside it would be ", 2*R, " mm tall.  This one is ", HEIGHT,
         " mm, a factor of ", HEIGHT/(2*R),
         ".  The contract's section 4 is explicit that a shared apex has",
         " no solution and that a stack is the resolution; a stack of",
         " four bevel sets is long, and that length is the cost of the",
         " exactness, not a modelling artefact"));

echo("--- 5. the drive table, exact rationals in the tooth counts ---");
echo(str("   Willis per row, (w_s - w_c)/(w_r - w_c) = -Nr/Ns:"));
for (i = [0:NROW-1])
  echo(str("      ", ROWS[i][0], "   -", sl_nr(i), "/", NS, " = -",
           sl_nr(i)/gcd_(sl_nr(i), NS), "/", NS/gcd_(sl_nr(i), NS),
           " = ", -sl_nr(i)/NS, "   gcd = ", gcd_(sl_nr(i), NS)));
echo("   (17) one ring held, carriers tied: w_sun/w_carrier = (Ns+Nr)/Ns");
for (i = [0:NROW-1])
  echo(str("      ", ROWS[i][0], "   (", NS, "+", sl_nr(i), ")/", NS, " = ",
           NS+sl_nr(i), "/", NS, " = ",
           (NS+sl_nr(i))/gcd_(NS+sl_nr(i), NS), "/",
           NS/gcd_(NS+sl_nr(i), NS), " = ", (NS+sl_nr(i))/NS,
           "   assembly gate (Ns+Nr)/k = ", (NS+sl_nr(i))/sl_k(i),
           "   integer: ", (NS+sl_nr(i)) % sl_k(i) == 0));
echo(str("   so the stack is a ", NROW, " speed with ratios ",
         [ for (i = [0:NROW-1]) (NS+sl_nr(i))/NS ],
         ", a spread of ", (NS+sl_nr(NROW-1))/(NS+sl_nr(0))));
echo(str("   steps between consecutive speeds: ",
         [ for (i = [1:NROW-1]) (NS+sl_nr(i))/(NS+sl_nr(i-1)) ],
         " -- uneven, and stated as such: the counts were chosen to make",
         " each row close exactly, not to space the ratios"));
echo(str("   sun held instead: w_ring/w_carrier = (Ns + Nr)/Nr = ",
         [ for (i = [0:NROW-1]) (NS+sl_nr(i))/sl_nr(i) ]));
echo(str("   carrier held: w_ring/w_sun = -Ns/Nr = ",
         [ for (i = [0:NROW-1]) -NS/sl_nr(i) ]));

echo("--- 6. (15) evaluated: the sphere placement cannot be built ---");
echo(str("   slice.scad publishes (15), z_apex = R cos(colat) - L",
         " cos(gamma_r), which puts each row's ring pitch circle on the",
         " fundamental sphere at its own latitude.  Those apexes are:"));
for (i = [0:NROW-1])
  echo(str("      ", ROWS[i][0], "   colat = ", sl_colat(i), "   apex z = ",
           sl_apex(i), "   ring circle at z = ",
           sl_apex(i) + sl_lo(i)*cos(sl_gr(i)), ", r = ",
           sl_lo(i)*sin(sl_gr(i)), ", so |p| = ",
           sqrt(pow(sl_apex(i) + sl_lo(i)*cos(sl_gr(i)), 2)
                + pow(sl_lo(i)*sin(sl_gr(i)), 2)), " = R = ", R));
echo(str("   the test: each row's sun has a SOLID hub band from its bore",
         " cone to its root cone, material at every azimuth, so two hub",
         " bands sharing a point of the (r,z) half plane is a collision",
         " and not a question of whether teeth interleave.  Sampling one",
         " on a ", STK_HITN+1, "x", STK_HITN+1,
         " grid and testing membership in the other:"));
for (i = [0:NROW-2])
  for (j = [i+1:NROW-1])
    let( h = stk_hits(i, j, sl_apex(i), sl_apex(j)) )
      echo(str("      ", ROWS[i][0], " sun into ", ROWS[j][0], " sun: ",
               len(h), " of ", pow(STK_HITN+1, 2), " points inside",
               len(h) > 0 ? str(", e.g. r = ", h[0][0], " z = ", h[0][1])
                          : "  -- clear"));
echo(str("   and the same test at the stations this file actually builds:"));
for (i = [0:NROW-2])
  for (j = [i+1:NROW-1])
    let( ai = stk_apex(NROW-1-i), aj = stk_apex(NROW-1-j),
         h = stk_hits(i, j, ai, aj) )
      echo(str("      ", ROWS[i][0], " sun into ", ROWS[j][0], " sun: ",
               len(h), " points inside", len(h) == 0 ? "  -- clear" : ""));
echo(str("   the sphere placement also fails for a reason the sun test",
         " does not need to reach: the equator row's own shell runs from",
         " cone distance ", sl_li(NROW-1), " to ", sl_lo(NROW-1),
         " about the sphere centre, and every other row's ring circle",
         " lies at |p| = ", R,
         " -- ON that shell's outer surface.  The equator row fills the",
         " space the other three rows' rings need."));

echo(str("   measured outside this file, for the record: the two written",
         " on their own come to 120655.422248 and 120655.422446 mm^3 over",
         " the same bounding box, 106480 triangles against 106484 -- the",
         " same solid, written by two paths that close the boundary",
         " differently by four triangles"));

echo("--- 7. not modelled ---");
echo("   the polar shaft that ties the four suns and the two cap splines");
echo("   together; the carrier tie that makes (17) a single output; the");
echo("   bolts; the housing and the stator's ground path; the bearings");
echo("   other than the ball channel the cap pair forms; the chain and");
echo("   its pinion; the cable, terminals and bus of the EM collar.");
echo("   Rows 1, 2 and 3 have no register on their rings, so only the");
echo("   equator row can take a collar at all.  That is the next piece");
echo("   of work and it is not started: three more bands.");

echo("--- 8. what the export must report ---");
NBODY = sum([ for (n = [0:NROW-1]) let(i = stk_row(n))
                (stk_ring(i) ? 4 : 3) + 2*sl_k(i) ])
      + 2 + (STK_COLLAR == 1 ? 14 : STK_COLLAR == 2 ? 108 : 0)
      + (STK_CAPS ? 6 : 0);
echo(str("   bodies: ",
         [ for (n = [0:NROW-1]) let(i = stk_row(n))
             str(ROWS[i][0], " ", (stk_ring(i) ? 4 : 3) + 2*sl_k(i)) ],
         " + band 2 + ", COL_NAME, " collar ",
         STK_COLLAR == 1 ? 14 : STK_COLLAR == 2 ? 108 : 0,
         " + 2 caps ", STK_CAPS ? 6 : 0, " = ", NBODY));
echo(str("   so validate.py must report ", NBODY, " components, 0 holes,",
         " 0 inconsistently wound edges and 0 T-junctions -- there is no",
         " boolean anywhere in this file"));
