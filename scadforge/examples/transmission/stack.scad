// ===================================================================
//  stack.scad -- the machine.  Planetary slices bolted end to end on
//  one polar axis, each slice-to-slice pole joint a facing pair of
//  polar caps, two of the slices carrying an equatorial band, and the
//  two bands carrying DIFFERENT collars: a roller-chain sprocket on
//  one, an axial-flux machine for regenerative braking and assist on
//  the next.
//
//  Every number here is READ: the design table from spherical_gear.scad,
//  each row's geometry from slice.scad, the register from
//  equator_band.scad, the two collars' and the cap's envelopes from
//  their own files, through the cc_if_*, em_if_*, eb_if_* and pc_if_*
//  accessors those files publish.  This file restates none of them.
//  What it adds is the one thing no part file could have: several
//  slices in one frame, the joint between them, and the arithmetic that
//  says they do not touch and that the ratio does not care what order
//  they were bolted in.
//
//  Nothing below is asserted.  Every clearance, every identity and
//  every ratio named in this header is printed by the report at the
//  bottom of this file with both sides shown, and the section numbers
//  here are the report's section numbers.
//
//  ---------------------------------------------------------------
//  0.  WHAT ASSEMBLING IT FOUND
//
//  An earlier assembly pass found four defects, none of which could be
//  seen from inside a part file, and all four were fixed where they
//  belonged.  They are recorded here because the pattern matters, not
//  because this file re-finds them:
//
//    * `use` did not work two levels deep.  A part file opening with
//      `M = sg_m();` was evaluated before the contract it uses, so
//      every number it read came back undef.  One level deep there is
//      nothing to order, so nothing had ever noticed.
//    * the EM collar's clamp seat stood outboard of the band's
//      register, over the spur teeth, clamping nothing -- and its clamp
//      was narrower than the register, and its anti-rotation feature
//      was a keyway where the band offers a circumferential groove.
//      Both collars had sized their clamps by hand against the band's
//      ECHOED report, because the band published no function to read.
//    * the pin boss band was beta +/- 2 GPIN, a bare factor of two.  On
//      row 1, where it was written, that is small.  On the equator row,
//      whose planets are 154 teeth, the retainer came out as a
//      near-hemispherical shell that cut through the register lands.
//    * the band's register relief was 2 mm, so its spur flange stood
//      only 12 mm above the groove centre.  Both collars publish an
//      envelope taller than that.  Either one's clamp lugs fouled it.
//
//  The pattern in all four is the same and it is worth naming: a
//  number that two files have to agree on was PUBLISHED by one and
//  COPIED by the other, and the copy was never compared with the
//  original.  An assembly is where the comparison happens.
//
//  THIS pass, which is the one that puts slices end to end, found five
//  more.  Four are gaps in what a part publishes rather than errors in
//  what it draws, and the fifth is a stale caveat:
//
//    * polar_cap.scad publishes the ball RADIUS, pc_if_ball(), but
//      neither the ball count nor the groove arc radius.  The balls
//      belong to a joint, not to a cap -- a cap at the end of a stack
//      is drawn without them -- so the member that forms the joint is
//      exactly the member that cannot place them with a computed
//      clearance.  Section 4 states this and draws no balls.
//    * pc_if_crown() publishes the crown's OUTER cone distance and its
//      back cone, but not the inner end of its face.  The crown
//      coupling's flank clearance is an ANGLE and so is independent of
//      cone distance, which is what saves the check; but its arc in mm
//      is tightest at the inner end, and a caller cannot compute it.
//      Section 4 gives the angle, and the arc at the outer end only,
//      and says which is which.
//    * neither collar publishes its torque duty as a function.
//      collar_chain.scad computes its own clamp grip and its own chain
//      pull, collar_em.scad computes its own machine torque, and
//      equator_band.scad computes its own port torque -- all three only
//      echo them.  So this file can compute what each clamp HOLDS from
//      published geometry and declared friction and bolt stress --
//      section 6 does -- and cannot compare either against the duty.
//      That comparison is still missing and it is the one that decides
//      whether the register needs a key.
//    * the chain collar clamps 22 mm of bore where the EM collar clamps
//      20, against a register whose land-groove-land is 20 mm inside a
//      relieved span of 30.5.  Both fit.  But the chain collar overhangs
//      the register proper at each end, onto the relief, and no file
//      says so: the band publishes the span, each collar publishes its
//      width, and nothing subtracts them.  Section 5 does.
//    * STALE, and now false: collar_em.scad's verifier reported that the
//      EM collar had to be MIRRORED in z or its machine fouled the
//      band's spur flange by 4.05 mm.  That was measured against the
//      2 mm register relief.  The relief is now longer, and section 3
//      prints the clearance of the collar drawn the way its own file
//      draws it, unmirrored, as a positive number.  The mirror is no
//      longer needed and this file does not apply it.
//
//  ---------------------------------------------------------------
//  1.  WHERE THE SLICES GO
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
//  in words, and section 8 below evaluates it and shows it cannot be
//  built: a row is a spherical shell of cone distances 0.75L to L about
//  its apex, the equator row's shell is the whole northern hemisphere
//  of the fundamental sphere, and the other rows' rings sit ON the
//  surface of that shell.  The suns collide outright, and section 8
//  finds the colliding points rather than arguing about them.
//
//  So the sphere does the job it can do, which is the one the
//  divisibility gate needs: Nr_i = Dref sin(colat_i) with Nr_i an
//  integer selects each row's ring pitch RADIUS, Nr_i m/2 = R
//  sin(colat_i).  Where the row then sits on the axis is a clearance
//  question, and this file answers it by stacking:
//
//      station 0 is the bottom slice, apex at the origin;
//      above it a pole joint, then station 1, and so on.         (16)
//
//  Each station is bounded by two z planes.  Every member of a slice is
//  a spherical shell sector, so its z extreme is at a corner of its
//  (cone distance, colatitude) rectangle; the band and the collar
//  publish their z extents outright.  (16) makes consecutive stations'
//  intervals disjoint, and two solids separated by a plane cannot
//  touch, whatever their radii.  So the whole between-station clearance
//  argument is one comparison per joint and it is exact.  That is why
//  the EM collar, whose stator lugs stand further out than anything
//  else in the machine, needs no special pleading: it is inside its own
//  station's z interval and that interval is private to the station.
//
//  ---------------------------------------------------------------
//  2.  THE POLE JOINT, WHICH IS WHAT MAKES IT A STACK
//
//  Between two slices sit two polar caps, face to face.  Between them
//  they make a face coupling (two crowns, teeth into spaces), a thrust
//  race, a socket round the floating spline sleeve, and a bolted joint.
//  polar_cap.scad builds all of that; what this file has to supply is
//  the one thing a part file cannot: the second cap.
//
//  The cap is drawn with its parting plane at z = 0, its body below,
//  and its crown teeth standing proud above.  A joint is therefore
//      translate([0,0,P])      polar_cap(false);
//      translate([0,0,P+JG])   rotate([180,0,0]) polar_cap(false);
//  and the two crowns interleave, because polar_cap indexes its crown a
//  QUARTER pitch: with tooth centres at (j + 1/4) p, the mated cap's
//  centres land at -(j + 1/4) p, which is half a pitch away, i.e. on
//  this cap's space centres.  Section 4 prints that identity.
//
//  rotate([180,0,0]), not mirror: its determinant is +1, so the shell's
//  winding is carried through unchanged, which is what this kernel's
//  polyhedron() needs.  It is also what you do to the real part.
//
//  JG is a declared joint gap.  At JG = 0 the two caps' parting planes
//  coincide, which is what a bolted joint actually does, and the two
//  webs and the two bolt rings then share faces.  Shared faces are a
//  boolean, and this file has no boolean anywhere; worse, two shells
//  that share vertices come back from the exporter as ONE connected
//  component, so the component count would stop being a check.  So the
//  joint is drawn with a small declared JG.  The crowns still
//  interleave over nearly their whole height -- section 4 prints the
//  engagement depth -- and the flank clearance computed at JG = 0 is a
//  lower bound for the clearance at any JG > 0, because opening the
//  joint moves each tooth toward the thinner part of the facing space.
//  Section 4 computes that bound from the library's own half-width
//  function and prints it.
//
//  The crown coupling is also what LOCATES the pair radially.  The
//  library builds a sector's flanks as ruled surfaces through the apex,
//  and for a gamma = 90 crown the apex sits on the axis in the parting
//  plane, so the teeth are radial: the engaged pair cannot slide
//  sideways without riding up a flank.  polar_cap says that of itself
//  and this file does not improve on it.
//
//  ---------------------------------------------------------------
//  3.  TWO BANDS, TWO DIFFERENT COLLARS, ONE REGISTER
//
//  Two of the slices are equator-row slices.  The equator row's ring is
//  not drawn by slice.scad at all: it IS the equatorial band, a part of
//  its own, which is why sl_slice() takes a `ring` flag.  Each of those
//  two slices therefore gets an equator_band(), and each band gets a
//  collar on its register -- the chain sprocket on the lower, the
//  axial-flux machine on the upper.
//
//  That is the picture the machine is for: one slice taking a chain,
//  the next taking a motor, on the SAME register.  Section 5 checks
//  that claim number by number instead of asserting it, and prints both
//  what matches and what does not.  The short form, which section 5
//  proves: the two collars are interchangeable on the BAND and are not
//  interchangeable on each other's CLAMP.  Five numbers are genuinely
//  shared, because both collars read them from eb_if_*(): the bore, the
//  rib crest radius, the rib axial width, where in z the rib sits, and
//  how tall a hub the register allows.  Everything else -- bolt
//  circles, split azimuths, clamp wall, clamp width, lugs against pads
//  -- differs, and section 5 prints the differences side by side.
//
//  ---------------------------------------------------------------
//  4.  THE DRIVE TABLE
//
//  Each row is a planetary set governed by Willis,
//
//      (w_sun - w_carrier) / (w_ring - w_carrier) = -Nr / Ns,     (17)
//
//  and section 7 prints each station's Willis ratio as an exact reduced
//  rational in the tooth counts, reduced by the library's own gcd_.
//
//  Hold a slice's ring and (17) gives that slice's reduction
//
//      i = w_sun / w_carrier = (Ns + Nr) / Ns,                   (18)
//
//  an exact rational.  Bolt the slices in series -- each slice's
//  carrier driving the next slice's sun, which is what the 46 spline
//  through the pole joint is for -- and the stack's reduction is the
//  PRODUCT of the (18)s:
//
//      i_stack = prod_i (Ns + Nr_i) / Ns^n.                      (19)
//
//  (19) is computed in section 7 as a pair of integers, numerator and
//  denominator, and reduced once at the end.  Both directions are
//  printed: sun in and carrier out is (19), carrier in and sun out is
//  its reciprocal, and each is given as an exact reduced rational and
//  as a decimal.
//
//  THE ORDER DOES NOT MATTER, and section 7 shows it twice.  The
//  arithmetic half is cheap: section 7 enumerates every permutation of
//  the stations and prints (19)'s numerator for each, all equal, the
//  integers being small enough that double precision carries them
//  exactly.  The physical half is the one worth having: the product can
//  only be order-independent if the slices really can be bolted in any
//  order, and they can because the pole interface is the SAME at every
//  station.  That is (7) of the contract: a member's pitch DIAMETER is
//  N m at every cone distance, so the sun keeps Ns m while its cone
//  angle steps from row to row, and one 46-tooth spline and one cap
//  crown serve the whole stack.  Section 2 of the report prints
//  2 L sin(gamma_s) against Ns m for every station, with the difference.
//
//  ---------------------------------------------------------------
//  5.  WHAT THIS FILE DRAWS, AND WHAT THE EXPORT CAN AND CANNOT SAY
//
//  Members that SHARE NO VOLUME, like every part file here.  No
//  difference(), no intersection(), no boolean of any kind.
//
//  The component count is therefore a NECESSARY check -- the bodies
//  this file places must come back as that many components -- and it is
//  not a sufficient one, for two reasons this file states rather than
//  hides.  Two shells that interpenetrate without sharing a vertex
//  still count as two.  And this model is far past the 25000 triangles
//  at which this kernel's export-time union gives up on a connected
//  group, so the union is skipped and the exporter says so; a scene
//  that DOES overlap comes back with holes = 0 and a volume that counts
//  the overlap twice.  The printed clearances are the statement.  Every
//  one of them is computed here from numbers the parts publish.
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
PHI   = sg_phi();
JT    = sg_jt();
ROWS  = sg_rows();
NROW  = len(ROWS);

// ---- the stations: [row index into sg_rows(), collar code] ----------
// collar code 0 = bare register, 1 = the chain collar, 2 = the EM
// collar.  A station carries a band if and only if its row is the
// equator row, because equator_band.scad IS the equator row's ring; a
// collar needs a band, and section 9 of the report counts them.
// Bottom of the stack first.
STK_ST = [ [NROW-1, 1],      // equator row, chain sprocket collar
           [NROW-1, 2],      // equator row, axial-flux EM collar
           [0,      0] ];    // row 1, the small final set, bare
NST    = len(STK_ST);

// ---- design choices made HERE, and named as choices ------------------
STK_GAP  = 2.0;    // axial clearance, station to facing cap body, mm
STK_JG   = 0.05;   // declared pole-joint gap, section 2, mm
STK_HITN = 12;     // grid per side for section 8's collision search
STK_NCL  = 40;     // samples across the crown tooth for section 4
// declared inputs for the clamp-grip model of section 6.  These are
// NOT read from any part file and are NOT design allowables; they are
// named here so the model's output can be read as what it is.
STK_MU   = 0.15;   // assumed dry steel-on-steel friction coefficient
STK_SIG  = 400;    // assumed bolt assembly stress, N/mm^2
STK_AS3  = 5.03;   // M3 thread stress area, mm^2  (declared, not read)
STK_AS6  = 20.1;   // M6 thread stress area, mm^2  (declared, not read)

function stk_row(n)  = STK_ST[n][0];
function stk_col(n)  = STK_ST[n][1];
// the equator row's ring is the band, so its slice is drawn without one
function stk_ring(i) = i != NROW - 1;
function stk_band(n) = !stk_ring(stk_row(n));

// ===================================================================
//  A STATION'S OWN EXTENT, IN ITS OWN FRAME (APEX AT THE ORIGIN)
// ===================================================================
// A slice member as a (cone distance, colatitude) rectangle.
// z = L cos G and r = L sin G are each monotone in L, and z is monotone
// in G, so z's extremes over the rectangle are at corners.  r is not
// monotone in G -- it peaks at G = 90 -- so its maximum is taken at the
// colatitude in range nearest 90.
function stk_mzlo(m) = cos(m[3]) >= 0 ? m[0]*cos(m[3]) : m[1]*cos(m[3]);
function stk_mzhi(m) = cos(m[2]) >= 0 ? m[1]*cos(m[2]) : m[0]*cos(m[2]);
function stk_mrhi(m) = m[1]*sin(max(m[2], min(m[3], 90)));

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

function stk_slo(i)  = min([ for (m = stk_bodies(i)) stk_mzlo(m) ]);
function stk_shi(i)  = max([ for (m = stk_bodies(i)) stk_mzhi(m) ]);
function stk_srad(i) = max([ for (m = stk_bodies(i)) stk_mrhi(m) ]);

// the band and the collars, read from their own files
EB_RCH = eb_if_reach();            // [r_in, r_out, z_lo, z_hi], band frame
COLZ   = eb_if_groove()[1];        // the register groove centre, band frame
CC_ENV = cc_if_envelope();         // [|z| the bosses reach, bolt-head reach]
EM_SPN = em_if_span();             // [z_lo, z_hi] about the clamp centre

function stk_clo(n) = let (c = stk_col(n))
    c == 1 ? COLZ - CC_ENV[0] : c == 2 ? COLZ + EM_SPN[0] : 0;
function stk_chi(n) = let (c = stk_col(n))
    c == 1 ? COLZ + CC_ENV[0] : c == 2 ? COLZ + EM_SPN[1] : 0;
function stk_crad(n) = let (c = stk_col(n))
    c == 1 ? cc_if_tip() : c == 2 ? em_if_rad() : 0;
function stk_cname(n) = let (c = stk_col(n))
    c == 1 ? "chain" : c == 2 ? "EM" : "none";

// the station's own interval and radius: slice, band if it has one,
// collar if it has one
function stk_zlo(n) = min(concat([ stk_slo(stk_row(n)) ],
                                 stk_band(n) ? [EB_RCH[2]] : [],
                                 stk_col(n) > 0 ? [stk_clo(n)] : []));
function stk_zhi(n) = max(concat([ stk_shi(stk_row(n)) ],
                                 stk_band(n) ? [EB_RCH[3]] : [],
                                 stk_col(n) > 0 ? [stk_chi(n)] : []));
function stk_rad(n) = max(concat([ stk_srad(stk_row(n)) ],
                                 stk_band(n) ? [EB_RCH[1]] : [],
                                 stk_col(n) > 0 ? [stk_crad(n)] : []));

// ===================================================================
//  (16):  THE AXIAL CHAIN, BOTTOM UP
// ===================================================================
// A cap reaches PCL below its parting plane and its crown stands PCH
// above it.  So an as-drawn cap occupies [P - PCL, P + PCH] and a
// turned-over one [P - PCH, P + PCL].
PCL = -pc_if_span()[0];
PCH =  pc_if_span()[1];

// the joint plane above station n -- the lower cap of that joint sits
// on it as drawn, so its body must clear the station below by STK_GAP
function stk_joint(n) = stk_apex(n) + stk_zhi(n) + STK_GAP + PCL;
// station 0 at the origin; each later station clears the turned-over
// upper cap of the joint beneath it by STK_GAP
function stk_apex(n) =
  n <= 0 ? 0
         : stk_joint(n-1) + STK_JG + PCL + STK_GAP - stk_zlo(n);

Z_B    = stk_zlo(0) - STK_GAP - PCL;      // bottom end cap, turned over
Z_T    = stk_joint(NST-1);                // top end cap, as drawn
HEIGHT = (Z_T + PCH) - (Z_B - PCH);
RMAX   = max(concat([ pc_if_rad() ], [ for (n = [0:NST-1]) stk_rad(n) ]));

// ===================================================================
//  THE CROWN COUPLING OF THE POLE JOINT, SECTION 4
// ===================================================================
NCAP = pc_if_crown()[0];          // cap crown tooth count
LCAP = pc_if_crown()[1];          // its outer cone distance
CR_P = 360/NCAP;                  // crown angular pitch
CR_GB= sg_gb(90, PHI);            // base cone of a gamma = 90 crown
CR_HT= sg_ht(NCAP, JT*M, M);      // half tooth angle, backlash included
CR_TA= sg_ta(NCAP);               // addendum angle
CR_TF= sg_tf(NCAP);               // dedendum angle

// At a shared world z and cone distance L the two crowns' colatitudes
// satisfy cos(Gb) = -JG/L - cos(Ga); at JG = 0 that is Gb = 180 - Ga,
// which is the tight case, because opening the joint moves each tooth
// toward the thinner part of the facing space.  Both crowns are
// EXTERNAL sectors, so both half widths come from sg_hw_e at g = 90.
function stk_cfl(Ga) = CR_P/2 - sg_hw_e(Ga, 90, CR_GB, CR_HT)
                              - sg_hw_e(180 - Ga, 90, CR_GB, CR_HT);
// the colatitudes at which BOTH teeth exist: Ga in [90-tf, 90+ta] and
// 180-Ga in the same range, i.e. Ga in [90-ta, 90+ta]
CR_CLS   = [ for (q = [0:STK_NCL])
               stk_cfl(90 - CR_TA + 2*CR_TA*q/STK_NCL) ];
CR_CLMIN = min(CR_CLS);
CR_TIPZ  = LCAP*sin(CR_TA);                  // tip above the pitch plane
CR_ROOTZ = LCAP*sin(CR_TF);                  // root below it
CR_AXC   = CR_ROOTZ - CR_TIPZ + STK_JG;      // axial tip clearance as drawn
CR_ENG   = 2*CR_TIPZ - STK_JG;               // engagement depth as drawn

// ===================================================================
//  (15), THE PLACEMENT THAT CANNOT BE BUILT, SECTION 8
// ===================================================================
// The sun's SOLID hub band: bore cone to root cone, material at every
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
//  EXACT RATIONAL ARITHMETIC FOR THE DRIVE TABLE, SECTION 7
// ===================================================================
// str() falls back to scientific notation past six figures, which would
// print an exact integer as an approximation.  This spells one out digit
// by digit instead, so every integer in (19) is shown in full.
function stk_istr(n) =
  n < 0 ? str("-", stk_istr(-n))
  : n < 10 ? str(floor(n))
  : str(stk_istr(floor(n/10)), floor(n - 10*floor(n/10)));

function stk_prod(v, i = 0) = i >= len(v) ? 1 : v[i]*stk_prod(v, i+1);
function stk_perms(v) =
  len(v) <= 1 ? [v]
  : [ for (i = [0:len(v)-1])
        for (p = stk_perms([ for (j = [0:len(v)-1]) if (j != i) v[j] ]))
          concat([v[i]], p) ];
// (18) per station, as the integer numerator over Ns
STK_NUMS = [ for (n = [0:NST-1]) NS + sl_nr(stk_row(n)) ];
STK_NUM  = stk_prod(STK_NUMS);                      // numerator of (19)
STK_DEN  = stk_prod([ for (n = [0:NST-1]) NS ]);    // Ns^n
STK_G    = gcd_(STK_NUM, STK_DEN);
STK_PERM = [ for (p = stk_perms(STK_NUMS)) stk_prod(p) ];

// ===================================================================
//  THE CLAMP GRIP MODEL, SECTION 6
// ===================================================================
// A DECLARED model, not a standard: a two-piece split clamp with n_pl
// parting planes, each closed by bolt force F_pl, grips the bore by
// friction, and the torque it can pass is taken as
//
//     T = 2 n_pl mu F_pl r_bore.                                 (20)
//
// (20) is the belt-friction form with the clamp treated as two
// tangentially loaded halves.  It is an estimate, it is printed with
// every input shown, and it is not a design allowable.
function stk_grip(npl, Fpl, rb) = 2*npl*STK_MU*Fpl*rb;
CC_NPL = 2;                           // chain collar, splits at 90 and 270
EM_NPL = 2;                           // EM collar, splits at 0 and 180
CC_FPL = (cc_if_bolt()[3]/CC_NPL)*STK_SIG*STK_AS3;   // M3 per parting plane
EM_FPL = 1*STK_SIG*STK_AS6;                          // one pinch M6 per plane
CC_T   = stk_grip(CC_NPL, CC_FPL, cc_if_bore());
EM_T   = stk_grip(EM_NPL, EM_FPL, em_if_seat());

// ===================================================================
//  THE MACHINE
// ===================================================================
module stk_station(n) {
    translate([0, 0, stk_apex(n)]) {
        sl_slice(stk_row(n), stk_ring(stk_row(n)));
        if (stk_band(n)) equator_band();
        if (stk_col(n) == 1) translate([0, 0, COLZ]) collar_chain();
        if (stk_col(n) == 2) translate([0, 0, COLZ]) collar_em();
    }
}

// a pole joint: two caps face to face, STK_JG apart, no balls -- see
// section 4 for why the balls are not this file's to place
module stk_pole_joint(z) {
    translate([0, 0, z]) polar_cap(false);
    translate([0, 0, z + STK_JG]) rotate([180, 0, 0]) polar_cap(false);
}

module stack() {
    for (n = [0:NST-1]) stk_station(n);
    for (n = [0:NST-2]) stk_pole_joint(stk_joint(n));
    // the two end caps, parting planes facing outward: no mate, and so
    // no balls
    translate([0, 0, Z_B]) rotate([180, 0, 0]) polar_cap(false);
    translate([0, 0, Z_T]) polar_cap(false);
}
stack();

// ===================================================================
//  REPORT
// ===================================================================
echo("=== stack: planetary slices bolted end to end ===");

echo("--- 1. read from the contract, not restated ---");
echo(str("   sg_m() = ", M, " mm   sg_r() = ", R, " mm   sg_ns() = ", NS,
         "   sg_dref() = ", DREF, "   sg_phi() = ", PHI,
         "   sg_jt() = ", JT, "   rows in the table = ", NROW));
for (i = [0:NROW-1])
  echo(str("      ", ROWS[i][0], "  Nr = ", sl_nr(i), "  D = ", sl_dd(i),
           "  Np = ", sl_np(i), "  k = ", sl_k(i),
           "   gamma_s = asin(", NS, "/", sl_dd(i), ") = ", sl_gs(i),
           "   L = ", sl_lo(i)));

echo("--- 2. the stations, (16), bottom up ---");
echo(str("   ", NST, " stations: ",
         [ for (n = [0:NST-1])
             str(ROWS[stk_row(n)][0],
                 stk_col(n) > 0 ? str(" / ", stk_cname(n), " collar")
                                : " / no collar") ]));
for (n = [0:NST-1])
  echo(str("   station ", n, "  ", ROWS[stk_row(n)][0],
           "   apex z = ", stk_apex(n),
           "   own interval ", stk_zlo(n), " to ", stk_zhi(n),
           "   world ", stk_apex(n) + stk_zlo(n), " to ",
           stk_apex(n) + stk_zhi(n),
           "   radius ", stk_rad(n),
           "   band ", stk_band(n), "   collar ", stk_cname(n)));
echo(str("   the separating planes, each the mid-plane of a joint: ",
         [ for (n = [0:NST-2]) stk_joint(n) + STK_JG/2 ],
         ".  Two solids on opposite sides of a plane cannot touch, so no",
         " member of one station can reach any member of another whatever",
         " their radii.  That is the whole between-station clearance",
         " argument and it is exact."));
for (n = [0:NST-2])
  echo(str("      station ", n, " top ", stk_apex(n) + stk_zhi(n),
           " -> lower cap body bottom ", stk_joint(n) - PCL, "   gap ",
           (stk_joint(n) - PCL) - (stk_apex(n) + stk_zhi(n)),
           ";   upper cap body top ", stk_joint(n) + STK_JG + PCL,
           " -> station ", n+1, " bottom ",
           stk_apex(n+1) + stk_zlo(n+1), "   gap ",
           (stk_apex(n+1) + stk_zlo(n+1))
             - (stk_joint(n) + STK_JG + PCL)));
echo(str("   bottom end cap parting plane z = ", Z_B,
         ", turned over, so it occupies ", Z_B - PCH, " to ", Z_B + PCL,
         " and its top clears station 0's bottom ", stk_zlo(0), " by ",
         stk_zlo(0) - (Z_B + PCL)));
echo(str("   top end cap parting plane z = ", Z_T, ", as drawn, occupying ",
         Z_T - PCL, " to ", Z_T + PCH, ", its underside clearing station ",
         NST-1, "'s top ", stk_apex(NST-1) + stk_zhi(NST-1), " by ",
         (Z_T - PCL) - (stk_apex(NST-1) + stk_zhi(NST-1))));
echo(str("   overall z ", Z_B - PCH, " to ", Z_T + PCH, " = ", HEIGHT,
         " mm tall over a largest radius of ", RMAX, " mm"));
echo(str("   both end caps face OUTWARD, which is what makes the stack a",
         " module rather than a finished box: an end cap presents exactly",
         " the face an internal joint presents -- the same ", NCAP,
         " tooth crown and the same ", NS,
         " spline -- so a second stack bolts onto either end of this one",
         " and the result is still a stack.  That is also why power enters",
         " and leaves at the poles: the pole is the only interface that",
         " repeats."));
echo(str("   the fundamental sphere is R = ", R,
         ", so a machine inside it would be ", 2*R, " mm tall; this one is ",
         HEIGHT, " mm, a factor of ", HEIGHT/(2*R),
         ".  The contract's section 4 is explicit that a shared apex has no",
         " solution and that a stack is the resolution; a stack of bevel",
         " sets with a bolted pole joint between each pair is long, and",
         " that length is the cost of the exactness, not a modelling",
         " artefact."));
echo("   ONE SPLINE AT EVERY STATION -- (7), pitch diameter is N m at");
echo("   every cone distance, so the sun is Ns m across at every row and");
echo("   one cap fits every joint.  That is what makes the order of");
echo("   section 7 free:");
for (n = [0:NST-1])
  let (i = stk_row(n))
    echo(str("      station ", n, "  ", ROWS[i][0],
             "   2 L sin(gamma_s) = 2*", sl_lo(i), "*sin(", sl_gs(i),
             ") = ", 2*sl_lo(i)*sin(sl_gs(i)), "   Ns m = ", NS*M,
             "   difference ", 2*sl_lo(i)*sin(sl_gs(i)) - NS*M));
echo(str("   the suns' outer ends land at z = ",
         [ for (n = [0:NST-1]) let (i = stk_row(n))
             stk_apex(n) + sl_lo(i)*cos(sl_gs(i)) ],
         " on one axis, every one of them at pitch radius ", NS*M/2));

echo("--- 3. each station's own members: slice, band, collar ---");
for (n = [0:NST-1])
  let (i = stk_row(n))
    echo(str("   station ", n, "  slice ", stk_slo(i), " to ", stk_shi(i),
             " out to r ", stk_srad(i),
             stk_band(n) ? str(";  band ", EB_RCH[2], " to ", EB_RCH[3],
                               ", r ", EB_RCH[0], " to ", EB_RCH[1],
                               ";  slice bottom minus band top = ",
                               stk_slo(i) - EB_RCH[3],
                               " -- NEGATIVE, and it is the MESH, not an",
                               " overlap: the band's crown ring teeth ARE",
                               " this row's ring and the planets run in",
                               " them.  The clearance there is slice.scad's",
                               " own ring/planet figure")
                         : ";  no band -- this row's ring is its own",
             stk_col(n) > 0
               ? str(";  ", stk_cname(n), " collar ", stk_clo(n), " to ",
                     stk_chi(n), " out to r ", stk_crad(n))
               : ";  bare register"));
echo(str("   the register, read from the band: floor r ", eb_if_floor(),
         " against sg_r() = ", R, ", difference ", eb_if_floor() - R,
         ";  lands r ", eb_if_land()[0], " each ", eb_if_land()[1],
         " wide;  groove ", eb_if_groove()[0], " wide centred z = ", COLZ,
         ";  relieved span ", eb_if_span()[0], " to ", eb_if_span()[1],
         " = ", eb_if_span()[1] - eb_if_span()[0], " mm"));
echo(str("   a collar bores ", eb_if_collar()[0], " with a rib ",
         eb_if_collar()[2], " wide whose inner face is at ",
         eb_if_collar()[1], ";  radial clearance ", eb_if_clear()[0],
         ", axial ", eb_if_clear()[1], " per side;  the register allows a",
         " hub ", eb_if_hub(), " mm tall symmetric on the groove"));
echo(str("   chain collar top ", COLZ + CC_ENV[0],
         " against the spur flange underside ", eb_if_span()[1],
         ":  clearance ", eb_if_span()[1] - (COLZ + CC_ENV[0]),
         ", clear = ", eb_if_span()[1] - (COLZ + CC_ENV[0]) > 0));
echo(str("   EM collar top ", COLZ + EM_SPN[1],
         " against the spur flange underside ", eb_if_span()[1],
         ":  clearance ", eb_if_span()[1] - (COLZ + EM_SPN[1]),
         ", clear = ", eb_if_span()[1] - (COLZ + EM_SPN[1]) > 0,
         ".  THAT SETTLES THE STALE CAVEAT OF SECTION 0: collar_em's",
         " verifier reported a 4.05 mm foul here and told the stack to",
         " mirror the collar in z.  That was measured against the old 2 mm",
         " register relief.  The number above is positive, so this file",
         " draws the collar the way collar_em.scad draws it, unmirrored,",
         " and applies no mirror."));
echo(str("   BELOW the groove, in the band's own frame: the EM collar",
         " reaches ", COLZ + EM_SPN[0], " where the band itself stops at ",
         eb_if_span()[0], ", so the collar hangs ",
         eb_if_span()[0] - (COLZ + EM_SPN[0]),
         " mm below the bottom of the band.  That is free air and not a",
         " foul -- the band ends there, and the next thing further down is",
         " the joint below, on the far side of its separating plane.  The",
         " chain collar's corresponding figure is ",
         eb_if_span()[0] - (COLZ - CC_ENV[0]), " mm."));

echo("--- 4. the pole joint ---");
echo(str("   ", NST-1, " joint(s), parting planes at z = ",
         [ for (n = [0:NST-2]) stk_joint(n) ], ", each two caps ", STK_JG,
         " mm apart, plus two end caps at ", Z_B, " and ", Z_T));
echo(str("   a cap reaches ", PCL, " below its parting plane and its crown",
         " stands ", PCH, " above it: pc_if_span() = ", pc_if_span()));
echo(str("   crown: pc_if_crown() = ", pc_if_crown(), ", so N_cap = ", NCAP,
         ", outer cone distance ", LCAP, ", angular pitch p = 360/", NCAP,
         " = ", CR_P, ", quarter pitch ", CR_P/4));
echo(str("   N_cap/Ns = ", NCAP, "/", NS, " = ", NCAP/NS, ", an integer: ",
         NCAP % NS == 0,
         " -- so the crown pattern and the spline pattern share a phase"));
echo("   QUARTER PITCH INDEXING.  This cap's tooth centres sit at");
echo("   (j + 1/4) p; the mated cap's land at -(j + 1/4) p after the");
echo("   reflection, and must coincide with this cap's SPACE centres at");
echo("   (j + 3/4) p.  Reduced modulo p:");
for (j = [0:3])
  let (tc = (j + 0.25)*CR_P, mt = -tc, sc = tc + CR_P/2,
       mm_ = mt - CR_P*floor(mt/CR_P), ss_ = sc - CR_P*floor(sc/CR_P))
    echo(str("      j=", j, "   mated tooth centre mod p = ", mm_,
             "   space centre mod p = ", ss_, "   difference ", mm_ - ss_));
echo(str("   FLANK CLEARANCE.  Both crowns are external sectors at",
         " gamma = 90, so both half widths are sg_hw_e at g = 90, with",
         " base cone sg_gb(90,", PHI, ") = ", CR_GB,
         " and half tooth angle sg_ht(", NCAP, ",", JT*M, ",", M, ") = ",
         CR_HT, " deg"));
echo(str("      clearance(Ga) = p/2 - hw_e(Ga) - hw_e(180-Ga), sampled at ",
         STK_NCL+1, " colatitudes across [90-ta, 90+ta] = [", 90-CR_TA,
         ", ", 90+CR_TA, "]"));
echo(str("      minimum ", CR_CLMIN, " deg;  at the pitch plane itself",
         " p/2 - 2 ht = ", CR_P/2 - 2*CR_HT, " deg, and the two agree to ",
         CR_CLMIN - (CR_P/2 - 2*CR_HT),
         ", so the tight point is the pitch plane, as it must be: away",
         " from it one tooth thins faster than the other thickens"));
echo(str("      and that angle is the library's own backlash: 180 jt m /",
         " (pi m N_cap) = ", 180*JT*M/(PI*M*NCAP), " deg, difference ",
         CR_P/2 - 2*CR_HT - 180*JT*M/(PI*M*NCAP)));
echo(str("      as an arc at the published OUTER cone distance ", LCAP,
         ":  ", rad(CR_CLMIN)*LCAP, " mm, against jt m / 2 = ", JT*M/2,
         ", difference ", rad(CR_CLMIN)*LCAP - JT*M/2));
echo(str("      the INNER end of the crown's face is not published by",
         " pc_if_crown(), and the arc there is smaller in proportion to",
         " cone distance, so this file gives the ANGLE -- which is",
         " independent of cone distance and is the quantity that decides",
         " interference -- and the arc at the outer end only.  Naming that",
         " gap is section 0's second finding."));
echo(str("      the clearance above is computed at JG = 0.  Opening the",
         " joint to the drawn ", STK_JG,
         " mm moves each tooth toward the thinner part of the facing",
         " space, so the figure is a lower bound for what is drawn."));
echo(str("   AXIAL.  A tip stands L sin(ta) = ", LCAP, "*sin(", CR_TA,
         ") = ", CR_TIPZ, " above the pitch plane and the facing root lies",
         " L sin(tf) = ", LCAP, "*sin(", CR_TF, ") = ", CR_ROOTZ,
         " beyond it, so the tip clearance as drawn is ", CR_ROOTZ, " - ",
         CR_TIPZ, " + ", STK_JG, " = ", CR_AXC, " mm"));
echo(str("   ENGAGEMENT as drawn: the two tooth bands overlap over",
         " 2 L sin(ta) - JG = 2*", CR_TIPZ, " - ", STK_JG, " = ", CR_ENG,
         " mm of the ", 2*CR_TIPZ, " mm available, i.e. ",
         100*CR_ENG/(2*CR_TIPZ), " percent of full engagement"));
echo(str("   NO BALLS.  polar_cap publishes pc_if_ball() = ", pc_if_ball(),
         ", the ball radius, but neither the ball count nor the groove arc",
         " radius; and the drawn joint is ", STK_JG,
         " mm open, so the two half grooves do not close into a torus",
         " anyway.  A ball placed here could not be given a computed",
         " clearance, so none is placed.  That is section 0's first",
         " finding, and it is an interface gap, not a defect in the cap."));
echo(str("   WHY THE JOINT IS DRAWN OPEN AT ALL: at JG = 0 the two webs",
         " and the two bolt rings share faces on the parting plane.  This",
         " file has no boolean, and two shells that share vertices come",
         " back from the exporter as ONE component, which would retire the",
         " component count as a check.  ", STK_JG,
         " mm is what the bolts close in service."));

echo("--- 5. the two collars on one register ---");
echo("   WHAT IS SHARED.  Both collars read these from eb_if_*(), so");
echo("   every difference below is exactly zero by construction, and");
echo("   printing them is how an assembly confirms the reading happened:");
echo(str("      bore          chain ", cc_if_bore(), "   EM ", em_if_seat(),
         "   band ", eb_if_collar()[0], "   differences ",
         cc_if_bore() - eb_if_collar()[0], ", ",
         em_if_seat() - eb_if_collar()[0]));
echo(str("      rib crest r   chain ", cc_if_tongue()[0], "   EM ",
         eb_if_floor() + eb_if_clear()[0], "   band ", eb_if_collar()[1],
         "   differences ", cc_if_tongue()[0] - eb_if_collar()[1], ", ",
         eb_if_floor() + eb_if_clear()[0] - eb_if_collar()[1]));
echo(str("      rib width     chain ", cc_if_tongue()[1], "   EM ",
         em_if_rib()[0], "   band ", eb_if_collar()[2], "   differences ",
         cc_if_tongue()[1] - eb_if_collar()[2], ", ",
         em_if_rib()[0] - eb_if_collar()[2]));
echo(str("      rib depth     EM ", em_if_rib()[1], "   band land height ",
         eb_if_land()[0] - eb_if_floor(), "   difference ",
         em_if_rib()[1] - (eb_if_land()[0] - eb_if_floor())));
echo(str("      z datum       both on the groove centre ", COLZ,
         ", which is eb_if_groove()[1]"));
echo(str("      hub allowed   ", eb_if_hub(), ";  chain uses 2*", CC_ENV[0],
         " = ", 2*CC_ENV[0], ", spare ", eb_if_hub() - 2*CC_ENV[0],
         ";  the EM collar is not symmetric about the groove, so only its ",
         EM_SPN[1], " mm above it meets that allowance, spare ",
         eb_if_hub()/2 - EM_SPN[1]));
echo("   WHAT IS NOT SHARED.  These belong to the two collars, and a");
echo("   third collar would have to match the BAND, not either of them:");
echo(str("      clamp width   chain ", cc_if_width(), "   EM ",
         em_if_width(), "   land+groove+land ",
         2*eb_if_land()[1] + eb_if_groove()[0], "   relieved span ",
         eb_if_span()[1] - eb_if_span()[0]));
echo(str("                    so the chain collar overhangs the register",
         " proper by (", cc_if_width(), " - ",
         2*eb_if_land()[1] + eb_if_groove()[0], ")/2 = ",
         (cc_if_width() - (2*eb_if_land()[1] + eb_if_groove()[0]))/2,
         " mm at each end, onto the relief, and the EM collar by ",
         (em_if_width() - (2*eb_if_land()[1] + eb_if_groove()[0]))/2,
         ".  Both are inside the relieved span, with ",
         (eb_if_span()[1] - eb_if_span()[0] - cc_if_width())/2, " and ",
         (eb_if_span()[1] - eb_if_span()[0] - em_if_width())/2,
         " mm to spare per side.  Section 0's fourth finding: no file",
         " subtracts those two numbers, because the band publishes the",
         " span, each collar publishes its width, and nothing compares",
         " them."));
echo(str("      split azimuth chain ", cc_if_split()[0], " and ",
         cc_if_split()[1], ", gap ", cc_if_split()[2],
         " deg   EM 0 and 180, gap ", em_if_split()[0], " mm of arc = ",
         em_if_split()[1], " deg.  NOT the same planes: ",
         cc_if_split()[0], " deg apart."));
echo(str("      bolt circle   chain r ", cc_if_bolt()[0], ", ",
         cc_if_bolt()[3], " bolts of clearance radius ", cc_if_bolt()[2],
         " through pads, axes at |z| = ", cc_if_bolt()[1],
         "   EM r ", em_if_bc()[0], ", ", em_if_bc()[1],
         " lugs first at azimuth ", em_if_bc()[2], ", M", em_if_bolt()[0],
         " clearance ", em_if_bolt()[1], ", boss ", em_if_bolt()[2],
         ".  The two bolt circles differ by ",
         em_if_bc()[0] - cc_if_bolt()[0], " mm in radius."));
echo(str("      clamp wall    EM ", em_if_wall(), " -> clamp outer r ",
         em_if_seat() + em_if_wall(),
         ";  the chain collar has no single wall -- its outermost radius",
         " is the sprocket tip ", cc_if_tip(), ", its ", cc_if_teeth(),
         " teeth at pitch ", cc_if_pitch()[0], " on pitch radius ",
         cc_if_pitch()[1]));
echo(str("      pinch ears    EM ", em_if_pinch(),
         " = [bolt circle r, ear length, stand-off];  the chain collar has",
         " none, it pulls its two arcs together on ", cc_if_bolt()[3],
         " axial bolts through pads instead"));
echo(str("   VERDICT, and it is the one the assembly exists to give: the",
         " two collars ARE interchangeable on the band -- five numbers,",
         " all matching to zero above -- and are NOT interchangeable on",
         " each other's clamp.  Nothing bolted to one will bolt to the",
         " other.  They swap on the BAND, which is the whole point of the",
         " register, and this stack shows it by fitting one of each to two",
         " identical bands."));

echo("--- 6. what holds a collar to its band, and what is missing ---");
echo(str("   The register is a circumferential groove, not a keyway: it",
         " locates the collar axially and radially and transmits torque by",
         " BORE FRICTION only.  (20), a DECLARED model and not a standard:",
         " T = 2 n_pl mu F_pl r_bore, the belt form with the clamp taken",
         " as two tangentially loaded halves."));
echo(str("   declared inputs, none of them read from a part file: mu = ",
         STK_MU, ", bolt assembly stress ", STK_SIG,
         " N/mm^2, M3 stress area ", STK_AS3, " mm^2, M6 ", STK_AS6,
         " mm^2"));
echo(str("   chain collar: ", cc_if_bolt()[3], " bolts over ", CC_NPL,
         " parting planes, so F_pl = (", cc_if_bolt()[3], "/", CC_NPL,
         ")*", STK_SIG, "*", STK_AS3, " = ", CC_FPL, " N;  T = 2*", CC_NPL,
         "*", STK_MU, "*", CC_FPL, "*", cc_if_bore(), " = ", CC_T,
         " Nmm = ", CC_T/1000, " Nm"));
echo(str("   EM collar: one tangential M6 per parting plane over ", EM_NPL,
         " planes, so F_pl = ", STK_SIG, "*", STK_AS6, " = ", EM_FPL,
         " N;  T = 2*", EM_NPL, "*", STK_MU, "*", EM_FPL, "*",
         em_if_seat(), " = ", EM_T, " Nmm = ", EM_T/1000, " Nm"));
echo(str("   EM grip over chain grip ", EM_T/CC_T,
         ", which is the bolt-area ratio ", STK_AS6/STK_AS3,
         " times the bolts-per-plane ratio ", 1/(cc_if_bolt()[3]/CC_NPL),
         " times the bore ratio ", em_if_seat()/cc_if_bore(),
         " = ", (STK_AS6/STK_AS3)*(1/(cc_if_bolt()[3]/CC_NPL))
                *(em_if_seat()/cc_if_bore()),
         ", difference ", EM_T/CC_T - (STK_AS6/STK_AS3)
                *(1/(cc_if_bolt()[3]/CC_NPL))*(em_if_seat()/cc_if_bore())));
echo(str("   WHAT IS MISSING, and it is section 0's third finding: neither",
         " collar publishes its torque DUTY as a function.  collar_chain",
         " computes its own clamp grip and chain pull, collar_em its own",
         " machine torque, equator_band its own port torque -- all of them",
         " only echo it.  So the numbers above are what each clamp HOLDS",
         " under the declared inputs, and there is nothing published to",
         " compare them against.  Until one of those three files publishes",
         " a duty, no file in this directory can say whether the register",
         " needs a key."));

echo("--- 7. the drive table ---");
echo(str("   (17) Willis per station, (w_s - w_c)/(w_r - w_c) = -Nr/Ns,",
         " as an exact reduced rational:"));
for (n = [0:NST-1])
  let (i = stk_row(n), g = gcd_(sl_nr(i), NS))
    echo(str("      station ", n, "  ", ROWS[i][0], "   -", sl_nr(i), "/",
             NS, " = -", sl_nr(i)/g, "/", NS/g, " = ", -sl_nr(i)/NS,
             "   gcd = ", g));
echo("   (18) ring held, sun in, carrier out:  i = (Ns + Nr)/Ns");
for (n = [0:NST-1])
  let (i = stk_row(n), a = NS + sl_nr(i), g = gcd_(a, NS))
    echo(str("      station ", n, "  ", ROWS[i][0], "   (", NS, "+",
             sl_nr(i), ")/", NS, " = ", a, "/", NS, " = ", a/g, "/", NS/g,
             " = ", a/NS, "   assembly gate (Ns+Nr)/k = ", a/sl_k(i),
             ", integer: ", a % sl_k(i) == 0));
echo("   (19) the stack, carriers in series through the 46 spline:");
echo(str("      numerator    prod(Ns + Nr_i) over ", STK_NUMS, " = ",
         stk_istr(STK_NUM)));
echo(str("      denominator  Ns^", NST, " = ", stk_istr(STK_DEN)));
echo(str("      gcd ", stk_istr(STK_G), ", so i_stack = ",
         stk_istr(STK_NUM/STK_G), "/", stk_istr(STK_DEN/STK_G), " = ",
         STK_NUM/STK_DEN, "   (sun in, carrier out: a reduction)"));
echo(str("      the other direction, carrier in and sun out, is the",
         " reciprocal: ", stk_istr(STK_DEN/STK_G), "/",
         stk_istr(STK_NUM/STK_G), " = ", STK_DEN/STK_NUM));
echo(str("      the reduction lost nothing: (num/gcd)*gcd - num = ",
         (STK_NUM/STK_G)*STK_G - STK_NUM, " and (den/gcd)*gcd - den = ",
         (STK_DEN/STK_G)*STK_G - STK_DEN,
         ";  and num/gcd and den/gcd share no factor, gcd(",
         stk_istr(STK_NUM/STK_G), ",", stk_istr(STK_DEN/STK_G), ") = ",
         gcd_(STK_NUM/STK_G, STK_DEN/STK_G)));
echo(str("   ORDER INDEPENDENCE, arithmetic half: all ", len(STK_PERM),
         " permutations of the station list give the numerator ",
         [ for (q = STK_PERM) stk_istr(q) ],
         ", spread max - min = ", max(STK_PERM) - min(STK_PERM)));
echo(str("   ORDER INDEPENDENCE, physical half: the product can only be",
         " free of the order if the slices really can be bolted in any",
         " order, and they can because every station presents the SAME",
         " pole interface -- pc_if_spline() = ", pc_if_spline(),
         " and pc_if_crown() = ", pc_if_crown(),
         " at every one of them, because there is one cap part, and",
         " because 2 L sin(gamma_s) = Ns m at every row, which section 2",
         " prints with its difference."));
echo(str("   so the stack as built is a ", NST, "-slice reduction of ",
         STK_NUM/STK_DEN, ":1.  And because each band is a LIVE ring",
         " rather than a held one, the collars are the control surface:",
         " holding one band while another turns selects, and driving two",
         " at once splits torque by drag current rather than by",
         " kinematics.  The contract says that plainly under its section 5",
         " and this file does not improve on it."));
echo(str("   the ratios the whole table offers, as decimals: ",
         [ for (i = [0:NROW-1]) (NS + sl_nr(i))/NS ],
         ";  sun held instead, w_ring/w_carrier = (Ns+Nr)/Nr = ",
         [ for (i = [0:NROW-1]) (NS + sl_nr(i))/sl_nr(i) ],
         ";  carrier held, w_ring/w_sun = -Ns/Nr = ",
         [ for (i = [0:NROW-1]) -NS/sl_nr(i) ]));

echo("--- 8. (15) evaluated: the sphere placement cannot be built ---");
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
echo("   and the same test at the stations this file actually builds,");
echo("   every pair of them, at their real apexes:");
for (a = [0:NST-2])
  for (b = [a+1:NST-1])
    let( h = stk_hits(stk_row(a), stk_row(b), stk_apex(a), stk_apex(b)) )
      echo(str("      station ", a, " sun into station ", b, " sun: ",
               len(h), " points inside", len(h) == 0 ? "  -- clear" : ""));
echo(str("   the sphere placement also fails for a reason the sun test",
         " does not need to reach: the equator row's own shell runs from",
         " cone distance ", sl_li(NROW-1), " to ", sl_lo(NROW-1),
         " about the sphere centre, and every other row's ring circle",
         " lies at |p| = ", R,
         " -- ON that shell's outer surface.  The equator row fills the",
         " space the other rows' rings need."));

echo("--- 9. not modelled ---");
echo("   the polar shaft and the floating spline sleeve that tie one");
echo("   slice's carrier to the next slice's sun through the joint: the");
echo("   cavity for it is polar_cap's and polar_cap prints the wall the");
echo("   sleeve has to live in, but no sleeve is drawn here;");
echo("   the bolts, at the pole joints and at both collars;");
echo("   the balls of the pole joint, for the reason section 4 gives;");
echo("   the housing, and the EM stator's ground path to it;");
echo("   the chain and its pinion, and the band's spur pinion;");
echo("   the EM collar's phase leads, terminals and bus to its cans;");
echo("   the bearings other than the race a closed cap pair forms;");
echo("   and the brake collar the design table's third collar implies.");
echo(str("   also not modelled, and a real limit rather than an omission:",
         " only the equator row has a band, so only an equator-row station",
         " can take a collar at all.  This stack has ",
         len([ for (n = [0:NST-1]) if (stk_band(n)) 1 ]), " of ", NST,
         " stations with a band, and ",
         len([ for (n = [0:NST-1]) if (stk_col(n) > 0) 1 ]),
         " with a collar.  Rows 1 to 3 would each need a band of their own",
         " at their own ring count, and none exists."));

echo("--- 10. what the export must report ---");
NCAPS = 2*(NST-1) + 2;
NBODY = sum([ for (n = [0:NST-1])
                let (i = stk_row(n))
                  (stk_ring(i) ? 4 : 3) + 2*sl_k(i)
                  + (stk_band(n) ? 2 : 0)
                  + (stk_col(n) == 1 ? 14 : stk_col(n) == 2 ? 108 : 0) ])
      + 3*NCAPS;
echo(str("   per station: ",
         [ for (n = [0:NST-1]) let (i = stk_row(n))
             str(ROWS[i][0], " slice ", (stk_ring(i) ? 4 : 3) + 2*sl_k(i),
                 stk_band(n) ? " + band 2" : "",
                 stk_col(n) == 1 ? " + chain 14"
                                 : stk_col(n) == 2 ? " + EM 108" : "") ]));
echo(str("   caps: ", NCAPS, " of 3 bodies each -- web, crown ring, bolt",
         " ring, no balls -- = ", 3*NCAPS));
echo(str("   TOTAL ", NBODY, " bodies, so validate.py must report ", NBODY,
         " components, 0 holes, 0 inconsistently wound edges and 0",
         " T-junctions.  There is no boolean anywhere in this file."));
echo(str("   and the warning to expect: this model is far past the 25000",
         " triangles at which the export-time union gives up on a",
         " connected group, so the union IS skipped and the exporter says",
         " so.  The component count is therefore necessary and not",
         " sufficient -- sections 2 to 5 are where the non-overlap is",
         " actually argued, one separating plane per joint and one",
         " computed clearance per interface."));
