// ===================================================================
//  collar_chain.scad -- the chain collar: the mechanical input at the
//  equator, and the one port in the machine where a chain is allowed.
//
//  Built on spherical_gear.scad, which is the contract.  Every number
//  it takes from that table is read through a function and never
//  restated, and only ONE of them enters the geometry: sg_r(), the
//  equatorial radius, which is where the register datum goes.  The
//  rest -- sg_m(), sg_ns(), sg_dref(), sg_jt(), sg_rows(), sg_row() --
//  are read so the report can check this collar against the table.
//  Everything else here is either declared at the top of this file --
//  the chain, which is a standard part, the band's register groove,
//  which is declared here as this collar's own interface to the band,
//  and the collar's own widths -- or derived
//  from those and printed with both sides shown, in mm and degrees.
//  Two figures are quoted from the
//  library's own REPORT rather than its table, and are used for nothing
//  but comparison: the 114 tooth chain collar it echoes, and gate 7's
//  17.4 Nm.  The report prints this file's own derivation of the first
//  and the difference against the quoted value.
//
//  ---------------------------------------------------------------
//  0.  WHAT THE COLLAR IS
//
//  The band wraps the slice's widest circle and is one rigid member at
//  one angular velocity, toothed inside as the equator row's crown
//  ring.  Outside it carries a swappable collar.  This is the chain
//  collar: a split sprocket that clamps to the band, takes a chain,
//  and comes off without dismantling the stack.
//
//  It is four things at once and the file builds them in that order:
//  a SPROCKET whose count closes on the chain; a ROLLER SEAT profile,
//  the pocket a roller sits in, swept round that sprocket; a CLAMP,
//  which is what makes it swappable; and a SEAT, the tongue that drops
//  into the band's groove and locates the collar on the equator.
//
//  Because the collar has to come off the band without the stack
//  coming apart, it parts into TWO ARCS rather than opening at a
//  single split: a one-piece collar would have to spring wide enough
//  for its bore to pass the register rib, whose height the report
//  prints, while two arcs simply lift off.  So there are two joints,
//  each with two lugs, one on each arc.  The brief asks for a bolt
//  boss on each lug; each lug here carries two, one above and one
//  below the chain plane, because a boss can only stand clear of the
//  body on one side of that plane and a one-sided clamp twists the arc
//  it pulls.  Four pads and two bolts a joint, eight pads and four
//  bolts in all; the report prints the counts.
//
//  ---------------------------------------------------------------
//  1.  A SPROCKET'S PITCH CIRCLE IS A POLYGON'S CIRCUMCIRCLE
//
//  A chain is not a belt.  Its pitch line is the polygon through the
//  roller centres, and on a sprocket of N teeth those centres sit at
//  the vertices of a regular N-gon of side p.  So
//
//      p = 2 r_p sin(180/N),   r_p = p / (2 sin(180/N))          (1)
//
//  and the closure is by construction, exactly as the contract's rows
//  close by construction: N chords of the chain's own pitch shut the
//  circle with no remainder, for any integer N.  The report prints (1)
//  with its residual, and prints N (360/N) = 360 with its remainder.
//
//  What is NOT closed is the circle: the polygon perimeter N p falls
//  short of 2 pi r_p, and the difference is printed.  That shortfall
//  is the chordal action.  A roller's radius from the axis runs from
//  r_p at a vertex to r_p cos(180/N) at the middle of a chord, so the
//  drive has a speed ripple
//
//      1 - cos(180/N)                                            (2)
//
//  printed as a fraction, as a percentage and as the roller's rise in
//  mm.
//
//  ---------------------------------------------------------------
//  2.  THE STACK-UP, AND WHY THE COUNT IS NOT CHOSEN
//
//  Radii outward from the band, each one the previous plus a declared
//  width, nothing eyeballed and nothing restated:
//
//      R_gf   = sg_r()                 the groove floor               (3)
//      R_rib  = R_gf  + rib height     the register rib crest -- the
//                                      height, the two axial widths and
//                                      the two clearances are READ from
//                                      equator_band.scad's eb_if_*(),
//                                      not restated here
//      R_bore = R_rib + c_r            the collar bore
//      R_hub  = R_bore + hub wall
//      R_wmin = R_hub  + web           the LEAST rim base -- a BOUND,
//                                      not a face: nothing is built at
//                                      this radius.  Section 3 says
//                                      where the web really ends.
//      r_s    = d/2 + c_s              the roller seat's arc radius   (4)
//      r_p   >= R_wmin + rim + r_s                                    (5)
//
//  (5) is the whole of the COUNT's sizing, and only that: the seat
//  floor is r_p - r_s, and it has to stand a declared rim depth
//  outside the least rim base.  r_p rises with N, so (1) turns (5)
//  into a bound on the count:
//
//      N >= 180 / asin( p / (2 r_p_min) )                         (6)
//
//  Two divisibilities then pick the count out of the integers above
//  that bound.  The collar parts in two, so each arc must carry a
//  whole number of seats: N even.  And the sprocket should carry the
//  symmetry the machine already has -- three of the contract's four
//  rows run three planets -- so N divisible by 3.  N is therefore the
//  least multiple of 6 at or above the bound of (6).  The report
//  prints the bound, the rounding, N/2, N/3 and N/k for every row of
//  the table with its gcd, and the margin the rounding bought.
//
//  N/4 is NOT an integer, and that is not a failure but the reason the
//  split works: with N even but not divisible by 4, azimuth 90 and 270
//  fall on tooth CRESTS and never on a roller seat.  The report prints
//  the crest azimuth and its residual against 90, which is zero.
//
//  Against the contract's own counts the collar shares almost nothing:
//  gcd(N, Dref) and gcd(N, Ns) are printed, and both are 2.  What
//  matters there is weaker and is stated as such: N and Dref are both
//  whole counts per revolution of ONE rigid band, so both excitations
//  close on the revolution, which is the same argument gate 5 makes
//  for the EM collar's lcm(S,P).
//
//  ---------------------------------------------------------------
//  3.  THE ROLLER SEAT, BUILT AS A LAND BETWEEN TWO FLANKS
//
//  The pocket is a real seat, not a notch.  Put the origin on the
//  axis, the seat centre on the pitch circle at azimuth 0, and work in
//  the local frame x radial, y tangential.  The seat is the arc of the
//  circle of radius r_s about the pitch point:
//
//      Q(a) = ( r_p - r_s cos a,  r_s sin a )                     (7)
//
//  with a = 0 at the floor.  Its depth is fixed by (4): the bottom
//  diameter comes out at the standard's pitch diameter minus roller
//  diameter, less the 2 c_s the seat is cut oversize, and the report
//  prints that identity with a zero residual.
//
//  The seat cannot run to a = 90: past some point the contact normal,
//  which is the line from the roller centre to the contact and so is
//  the seat circle's own radius, leans too far from the tangential to
//  drive the roller.  The angle it makes with the tangential is 90 - a,
//  so the seat ends at
//
//      a_e = 90 - phi_ch                                          (8)
//
//  for a declared seat pressure angle phi_ch.  Beyond it the flank is
//  the straight line TANGENT to the seat circle at Q(a_e), so flank
//  and seat meet with a common tangent and the roller never climbs a
//  step.  That line has two exact descriptions and the file prints
//  both against each other:
//
//      distance from the pitch point to the line = r_s            (9)
//      foot of the axis normal at azimuth -a_e,
//      at distance  h = r_p sin(phi_ch) - r_s                    (10)
//
//  so in polar form the whole flank is one cosine, r = h/cos(psi+a_e),
//  and because it is a straight line the mesh samples it at its two
//  ends only: the chord between them IS the flank, exactly, and the
//  only faceting on the tooth is on the seat arc and the tip land.
//  The report prints the residuals of (9) and (10), and the seat form
//  and the flank form evaluated at the azimuth where they meet.
//
//  The tip is the standard's topping circle, declared, not derived:
//
//      D_o = p ( 0.6 + cot(180/N) )                              (11)
//
//  and the flank runs out to it.  What is left between the flank and
//  the crest is the tip land; the report prints it in degrees and in
//  mm and checks that it is positive, because with (8), (10) and (11)
//  all fixed independently there is nothing to stop it going negative
//  at some other count.
//
//  The rim carrying all this is thin, because the tooth has to fit
//  between the chain's inner plates; it steps out to the full web
//  thickness a declared clearance inside where those plates reach,
//  at R_plt = r_p - plate depth/2 - c_p.  THAT is where the web ends,
//  not R_wmin, and the report prints the step against both, against
//  the seat floor, and checks that every seat lies in the thin part.
//  The web is taller than the chain's own envelope over the pin heads,
//  so it would foul the chain if it reached it: its clearance from the
//  plates' inner edge is c_p exactly, by the definition of R_plt, and
//  the report prints that residual rather than a larger number taken
//  off the bound R_wmin.  What stands under the seat floor in the THIN
//  section is r_p - r_s - R_plt, which is smaller than the rim depth of
//  (5) and is set by the plate depth, not by (5); it too is printed.
//
//  ---------------------------------------------------------------
//  4.  THE WRAP, AND THE LOOP THAT CLOSES ON IT
//
//  A chain loop closes only on an EVEN number of pitches, because
//  inner and outer links alternate.  Take the loop at a declared
//  nominal centre distance by the usual series, round UP to the next
//  even integer, and then the centre distance is no longer free: it is
//  solved so the loop closes exactly on that many pitches.  With
//  sin(ph) = (r_p - r_p2)/C, the taut band on the two pitch circles is
//
//      L p = 2 C cos(ph) + (pi + 2 ph) r_p + (pi - 2 ph) r_p2    (12)
//
//  and the file bisects (12) for C.  The residual is printed.
//
//  (12) runs on ARCS.  A chain's pitch line is the inscribed polygon,
//  which is shorter, by 2 r (rad(180/N) - sin(180/N)) at every engaged
//  pitch on both wheels; the report prints that deficit over the whole
//  loop.  It is not corrected away here -- it is what the take-up is
//  for -- and saying so is the honest version of "the chain closes".
//
//  The wrap follows from the same angle,
//
//      wrap = 180 + 2 ph                                         (13)
//
//  and the seats under the chain are wrap/(360/N).  Both are printed,
//  with the two tangency azimuths.  The splits at 90 and 270 lie
//  INSIDE that wrap, and that is safe for a printed reason: the split
//  is on a crest, a crest carries no roller, and the seats either side
//  of it are whole.  Their azimuths are printed.
//
//  Hunting is where this count loses something, and the file says so.
//  A roller returns to the same tooth after lcm(N, L) pitches, so it
//  visits N/gcd(N, L) of the N teeth.  L is even and N had to be even
//  to halve the collar, so gcd is at least 2 and a roller sees half
//  the teeth.  An odd count would hunt; an odd count cannot be halved.
//  The report prints gcd(N, L) and the number of teeth visited.
//
//  ---------------------------------------------------------------
//  5.  THE CLAMP
//
//  A hoop tension F in a ring of radius R gripping over an axial
//  length L presses with p = F/(R L): cut the ring in half and the
//  pressure on the half, integrated, is 2 p R L against the two cut
//  faces carrying 2 F.  The friction torque is then
//
//      T = mu p (2 pi R L) R = 2 pi mu F R                       (14)
//
//  and L cancels, so the grip does not depend on how long the bore is
//  -- only the pressure does, and both are printed.  F is the bolts of
//  one joint at a declared preload.  The result is printed against the
//  chain's own working tension at r_p, and of the two torques this file
//  computes the report names the limit: the chain, with the ratio.
//  Gate 7's 17.4 Nm is printed beside them and is NOT adjudicated
//  against them, because it is a per-tooth-pair figure at the POLAR
//  crown and not a torque at this port; the report says so.
//
//  The gap is declared as a width at the bolt radius and is therefore
//  an angle; between two radial faces it measures differently at the
//  bore, at the bolt and over the tips, and all three are printed.
//  The gap is bounded by how much of the end tooth may be lost: the
//  report prints the first seat's start azimuth against the arc's end,
//  and the radius at which the half tooth at the split is cut off
//  against the radius where the seat's contact arc ends -- the cut
//  takes clearance flank and tip land, not contact.
//
//  ---------------------------------------------------------------
//  6.  BACKLASH, WHICH IS GATE 3's WHOLE POINT
//
//  Gate 3 of the contract says chain backlash lives only on the chain
//  collar.  This file computes what that is worth.  Turn the collar
//  back with the chain held: the engaging roller moves until it meets
//  the other flank, which is 2 c_s of seat clearance, and the joint at
//  that roller takes up its own pin-bushing clearance c_j:
//
//      backlash = 2 c_s + c_j   mm at the pitch radius           (15)
//
//  The collar is clamped to the band, so the angle
//
//      backlash / r_p                                            (16)
//
//  is the angle AT THE BAND as well; it is printed in degrees and as
//  an arc at the register radius sg_r().  The band is the equator
//  row's ring, so with the carrier held Willis refers it to the sun as
//  (Nr/Ns) times that, and the report prints it beside gate 6's own
//  4 jt/(Ns m), recomputed from the table, and their ratio.  The chain
//  alone is several times the whole gear train's lost motion, and that
//  number is the argument for gate 3, not a decoration on it.
//
//  Chain pitch tolerance is deliberately NOT in (15).  It is a length
//  error the take-up absorbs, not lost motion at a tooth, and the
//  report says so where it prints the figure.
//
//  ---------------------------------------------------------------
//  7.  VOLUME, AND WHAT IS DRAWN
//
//  The members share no volume, so the file can print the exact volume
//  of the mesh it writes rather than a bound on it.  For the two arcs
//  that is the divergence theorem summed over the very faces cc_sweep
//  lists; a quad whose corners are not coplanar has two triangulations
//  and both are computed, because the exporter picks one and the
//  difference between them is the whole uncertainty.  It is printed.
//  The pads and the bolts have no curved faces at all -- a pad is a
//  box with an n-gon bore, a bolt two n-gon prisms -- so their volumes
//  are closed forms and exact.
//
//  ONE MODELLING COMPROMISE, and it is declared.  A lug pad is the
//  same piece as its arc in the part.  Here it stands off the hub's
//  face by a declared clearance, and each bolt head stands off its
//  seat by the same, because this kernel's export-time union welds
//  shells that touch and leaves T junctions in the weld.  Nothing else
//  in the file is separated that is not separate in the machine.
//
//  Not modelled, and not claimed: the bolt thread and its nut -- the
//  shank is drawn at its major diameter, and BOTH pad bores are
//  clearance bores, so the joint takes a nut, which is neither drawn
//  nor counted in any volume here -- and the band itself: its register groove is declared here, as this
//  collar's own interface to it, with the floor put on sg_r() because
//  that is the one radius the contract fixes.  The report prints
//  sg_r() against Dref m/2 with a zero residual.
//
//  Every hole in this part is walled, never subtracted: the bolt bore
//  is an inner path in the pad's own section, so the pad is extruded
//  around it.  There is no difference() and no intersection() in this
//  file.
//
//  ---------------------------------------------------------------
//  8.  WHAT AN ASSEMBLY CALLS
//
//  collar_chain() is the whole part, drawn about its own axis with the
//  origin on the register groove's MID-PLANE and the splits at azimuth
//  90 and 270.  `use` imports modules and functions but never
//  variables, so every radius, width and azimuth an assembly needs is
//  published as a cc_if_*() function and echoed in section 12; an
//  assembly reads them there and restates none of them.  Section 11
//  prints the envelope the collar occupies, including the bosses that
//  stand outside the clamped bore, because the band is not modelled
//  here and whether it leaves that space is the assembly's check.
// ===================================================================

use <spherical_gear.scad>
use <equator_band.scad>

// ===================================================================
//  DECLARED INPUTS
//  Six groups.  Nothing else in this file is a written-down DIMENSION
//  or standard constant; every other number below is a pure geometric
//  one (2, 90, 180, 360), a unit conversion, an integer count, a mesh
//  index or a colour.  The groups are:
//  the chain, which is a standard part; the band's register groove,
//  declared here as this collar's own interface to the band, with its
//  floor put on the one radius the contract fixes, sg_r(); the
//  collar's own widths; the two clamp inputs, friction and preload;
//  two figures quoted from the library's REPORT for comparison and
//  used for nothing else; and the mesh density.  Everything with a
//  dimension that is not in this list is derived below and printed.
// ===================================================================

// -- the chain: ISO 606 05B-1, declared, not derived ------------------
CC_P     = 8.00;    // chain pitch, mm
CC_DR    = 5.00;    // roller diameter, mm
CC_BI    = 3.00;    // inner width between the inner link plates, mm
CC_HP    = 7.10;    // link plate depth, mm
CC_WO    = 8.20;    // overall chain width over the pin heads, mm
CC_CJ    = 0.10;    // pin-bushing clearance at one joint, mm
CC_FW    = 630;     // working tension, N
CC_N2    = 19;      // the pinion this collar drives against
CC_C0    = 400;     // nominal centre distance, mm
CC_TOP   = 0.60;    // the standard's topping constant: D_o = p(c + cot(180/N))

// -- the band's register groove, READ from the band -------------------
// These five were literals here, restated from the band's echoed report,
// and they happened to agree with it.  The EM collar restated the same
// register from the same text and got it wrong by 21 mm, which is what
// made the band publish eb_if_*().  A number a part must match is read.
EB_LND   = eb_if_land();     // [land crest radius, one land's axial width]
EB_GRV   = eb_if_groove();   // [groove axial width, groove centre in z]
EB_CLR   = eb_if_clear();    // [radial, axial per side]
CC_RIBH  = EB_LND[0] - eb_if_floor();   // rib height above sg_r()
CC_RIBW  = EB_LND[1];                   // axial width of one rib
CC_GW    = EB_GRV[0];                   // axial width of the groove
CC_CR    = EB_CLR[0];                   // radial clearance
CC_CA    = EB_CLR[1];                   // axial clearance, per side

// -- the collar's own widths ------------------------------------------
CC_THUB  = 9.00;    // hub wall, bore to hub outside
CC_TWEB  = 5.00;    // web, hub outside to the LEAST rim base R_WMIN, a
                    // bound on the count only; the built web runs out
                    // to R_PLT, which is further
CC_TRIM  = 3.20;    // least rim depth from the least rim base R_WMIN up to
                    // the seat floor.  NOT the thin rim under the seat
                    // floor, which is r_p - r_s - R_PLT and is printed.
CC_HHUB  = 11.00;   // hub half height
CC_HWEB  = 5.00;    // web half height
CC_CP    = 0.40;    // radial clearance to the chain plate's inner edge
CC_CW    = 0.20;    // axial clearance, rim to inner link plate, per side
CC_CS    = 0.15;    // roller seat clearance (seat radius over roller radius)
CC_PA    = 22.0;    // seat pressure angle limit: the seat arc ends where the
                    // contact normal makes this angle with the tangential
CC_GAPMM = 2.00;    // clamp gap, mm, measured at the bolt radius
CC_PADL  = 9.00;    // lug pad length along the bolt axis, mm
CC_PADH  = 8.00;    // lug pad height above the hub, mm
CC_BHOLE = 1.70;    // bolt bore radius (M3 clearance)
CC_BSH   = 1.50;    // bolt shank radius
CC_BHR   = 2.75;    // bolt head radius
CC_BHH   = 3.00;    // bolt head height
CC_NBJ   = 2;       // bolts per joint
CC_PART  = 0.20;    // modelling parting clearance under a lug, mm

// -- clamp inputs, declared -------------------------------------------
CC_MU    = 0.25;    // friction coefficient at the bore, declared
CC_FB    = 1000;    // bolt preload, N, declared

// -- quoted only for comparison, never used as design data ------------
CC_ECHO_N  = 114;   // the chain collar count the library's report echoes
CC_ECHO_G7 = 17.4;  // gate 7's echoed ceiling, Nm per tooth pair

// -- mesh density ------------------------------------------------------
CC_NH    = 4;       // samples on each half of a roller seat
CC_NPAD  = 24;      // samples round a pad section and its bore
CC_FNB   = 24;      // facets on a bolt

// ===================================================================
//  THE CONTRACT, READ, NEVER RESTATED
// ===================================================================
M_    = sg_m();
NS    = sg_ns();
DREF  = sg_dref();
RSPH  = sg_r();
JT_   = sg_jt();
ROWS_ = sg_rows();

// ===================================================================
//  DERIVED
// ===================================================================
// 1. the seat stack-up, from the band out
R_GF    = RSPH;                     // groove floor = the equatorial cylinder
R_RIB   = R_GF + CC_RIBH;           // rib crest
R_TIN   = R_GF + CC_CR;             // tongue inner face
R_BORE  = R_RIB + CC_CR;            // collar bore
R_HUB   = R_BORE + CC_THUB;
R_WMIN  = R_HUB  + CC_TWEB;         // LEAST rim base: a bound, not a face
RS      = CC_DR/2 + CC_CS;          // roller seat arc radius
RP_MIN  = R_WMIN + CC_TRIM + RS;    // least admissible pitch radius

// 2. the count: least multiple of 6 whose pitch circle clears RP_MIN
N_MIN   = 180/asin(CC_P/(2*RP_MIN));
NDIV    = lcm_(2,3);                // even to halve, /3 for the k=3 rows
NCH     = NDIV*ceil(ceil(N_MIN)/NDIV);
BETA    = 180/NCH;                  // half pitch angle
RP      = CC_P/(2*sin(BETA));       // pitch radius
R_FLOOR = RP - RS;                  // seat floor
R_PLT   = RP - CC_HP/2 - CC_CP;     // rim thins here: the plate's inner edge
H_RIM   = CC_BI/2 - CC_CW;          // rim half thickness
H_TON   = CC_GW/2 - CC_CA;          // tongue half height

// 3. the roller seat and the tooth
AE      = 90 - CC_PA;               // seat arc half extent, from the floor
QX      = RP - RS*cos(AE);          // seat arc end, radial
QY      = RS*sin(AE);               // seat arc end, tangential
RQ      = sqrt(QX*QX + QY*QY);      // its radius
DQ      = atan2(QY, QX);            // its azimuth off the seat centre
UX      = sin(AE);  UY = cos(AE);   // flank direction, tangent to the seat
QU      = QX*UX + QY*UY;
HFL     = abs(QX*UY - QY*UX);       // flank line's distance from the axis
PSI0    = atan2(QY - QU*UY, QX - QU*UX);   // azimuth of that foot
RTIP    = CC_P*(CC_TOP + cos(BETA)/sin(BETA))/2;       // topping circle
SFL     = -QU + sqrt(QU*QU + RTIP*RTIP - QX*QX - QY*QY);   // flank length
TX      = QX + SFL*UX;  TY = QY + SFL*UY;
DT      = atan2(TY, TX);            // azimuth where the flank meets the tip
LAND    = BETA - DT;                // tip land half width, degrees

// 4. the clamp geometry
R_BOLT  = (R_BORE + R_HUB)/2;
GAPD    = CC_GAPMM/R_BOLT*180/PI;   // clamp gap as an angle
A0      = 90 + GAPD/2;              // arc A, first azimuth
A1      = 270 - GAPD/2;             // arc A, last azimuth
J0      = ceil(A0/(2*BETA));        // first whole seat in arc A
J1      = floor(A1/(2*BETA));       // last whole seat in arc A
Z_PAD0  = CC_HHUB + CC_PART;
Z_BOLT  = Z_PAD0 + CC_PADH/2;      // bolt axis height
PAD_RO  = sqrt(R_HUB*R_HUB - pow(CC_GAPMM/2 + CC_PADL, 2));  // pad outer face
// 5. the chain loop and the wrap
RP2     = CC_P/(2*sin(180/CC_N2));                  // pinion pitch radius
L_NOM   = 2*CC_C0/CC_P + (NCH + CC_N2)/2
          + pow((NCH - CC_N2)/(2*PI), 2)*CC_P/CC_C0;
NL      = 2*ceil(L_NOM/2);                       // a chain closes even
// taut-band length on the two pitch circles, less the loop asked for
function cc_flen(C) =
  let (ph = asin((RP - RP2)/C))
    2*C*cos(ph) + rad(180 + 2*ph)*RP + rad(180 - 2*ph)*RP2 - NL*CC_P;
function cc_bis(lo, hi, n) =
  n <= 0 ? (lo + hi)/2
         : let (mid = (lo + hi)/2)
             cc_flen(mid) > 0 ? cc_bis(lo, mid, n-1) : cc_bis(mid, hi, n-1);
CDIST   = cc_bis(CC_P*NL/8, CC_P*NL/2, 60);      // exact centre distance
PHIW    = asin((RP - RP2)/CDIST);
WRAP    = 180 + 2*PHIW;                             // wrap on the collar
WRAP2   = 180 - 2*PHIW;                             // wrap on the pinion
TANG0   = 90 - PHIW;                                // first tangency azimuth
TANG1   = 270 + PHIW;                            // the second one
NENG    = WRAP/(2*BETA);                            // seats under the chain
// arc minus chord over the wrapped runs: the band formula uses the arcs
DEFC    = NENG*2*RP*(rad(BETA) - sin(BETA))
          + (WRAP2/(360/CC_N2))*2*RP2*(rad(180/CC_N2) - sin(180/CC_N2));

// 6. backlash, gate 3
BL_MM   = 2*CC_CS + CC_CJ;                          // at the pitch radius
BL_DEG  = deg(BL_MM/RP);                     // at the collar = the band
BL_BAND = rad(BL_DEG)*RSPH;                  // as an arc at the register
NR_EQ   = sg_row(3)[1];                      // the equator ring count
BL_SUN  = BL_DEG*NR_EQ/NS;                          // referred to the sun
BL_GEAR = deg(4*JT_*M_/(NS*M_));             // gate 6, recomputed

// 7. the clamp
L_GRIP  = 2*CC_RIBW;                                // bore length on the ribs
F_HOOP  = CC_NBJ*CC_FB;                             // hoop tension per joint
P_BORE  = F_HOOP/(R_BORE*L_GRIP);                   // contact pressure
T_CLAMP = 2*PI*CC_MU*F_HOOP*R_BORE/1000;            // friction torque, Nm
T_CHAIN = CC_FW*RP/1000;                            // chain's own capacity, Nm

// ===================================================================
//  THE INTERFACE, PUBLISHED AS FUNCTIONS
//  `use` imports modules and functions but NOT variables, so an
//  assembly that reads a radius out of this file must read it HERE.
//  A restated constant is a constant that can drift, which is the
//  contract's own argument for publishing its table this way.  Every
//  one of these is echoed in section 12 below, so the numbers an
//  assembly gets are the numbers this file prints.
// ===================================================================
function cc_if_groove()   = [R_GF, CC_RIBH, CC_RIBW, CC_GW, CC_CR, CC_CA];
      // what the BAND must provide: groove floor radius, rib height,
      // rib axial width, groove axial width, radial and axial clearance
function cc_if_bore()     = R_BORE;              // collar bore, over the rib crests
function cc_if_tongue()   = [R_TIN, 2*H_TON];    // tongue inner face radius, axial width
function cc_if_width()    = 2*CC_HHUB;           // clamped bore axial length
function cc_if_tip()      = RTIP;                // outermost radius of the part
function cc_if_teeth()    = NCH;                 // sprocket count
function cc_if_pitch()    = [CC_P, RP];          // chain pitch, pitch radius
function cc_if_split()    = [90, 270, GAPD];     // the two split azimuths, gap angle
function cc_if_arc()      = [A0, A1, J0, J1];    // the arguments cc_arc() takes
function cc_if_bolt()     = [R_BOLT, Z_BOLT, CC_BHOLE, 2*CC_NBJ];
      // bolt circle radius, bolt axis |z|, clearance bore radius, bolts in all
function cc_if_envelope() = [Z_PAD0 + CC_PADH,
                             CC_GAPMM/2 + CC_PADL + CC_BHH + CC_PART];
      // |z| the bosses reach about the register mid-plane, and how far
      // a bolt head reaches either side of a split plane

// ===================================================================
//  THE PROFILE
//  r(psi) for the rim, in three exact pieces per pitch.
// ===================================================================
// the seat: the circle of radius RS about the pitch point, inner branch
function cc_seat_r(a) = RP*cos(a) - sqrt(max(0, RS*RS - RP*RP*sin(a)*sin(a)));
// the flank: the straight line tangent to that circle at the seat's end.
// Its normal stands at azimuth -AE and its distance from the axis is
// HFL, so in polar form it is one cosine.
function cc_flank_r(a) = HFL/cos(a - PSI0);
// the whole rim radius, folded into one pitch about the nearest seat
function cc_tooth_r(psi) =
  let (d = psi - 2*BETA*round(psi/(2*BETA)), a = abs(d))
    a <= DQ ? cc_seat_r(a) : a <= DT ? cc_flank_r(a) : RTIP;
// the seat sampled by its own arc parameter, so the samples are even
// along the seat and land exactly on its two ends
function cc_seat_d(a) = atan2(RS*sin(a), RP - RS*cos(a));

// azimuth samples for one arc: the arc's own end, then every seat's arc
// and the tooth between it and the next, then the arc's other end
function cc_az(a0, a1, j0, j1) =
  concat([a0],
    [ for (j = [j0:j1]) each
        concat([ for (i = [0:2*CC_NH])
                   2*BETA*j + cc_seat_d(-AE + 2*AE*i/(2*CC_NH)) ],
               j < j1 ? [ 2*BETA*j + DT, 2*BETA*j + BETA,
                          2*BETA*j + 2*BETA - DT ] : []) ],
    [a1]);

// the collar's section: a closed loop in (r, z), counter-clockwise, as a
// staircase of four bands -- tongue, hub, web, rim.  Stations flagged
// true take their radius from the rim profile instead.
CC_PROF = [
  [R_TIN , -H_TON ], [R_BORE, -H_TON ], [R_BORE, -CC_HHUB], [R_HUB , -CC_HHUB],
  [R_HUB , -CC_HWEB], [R_PLT , -CC_HWEB], [R_PLT , -H_RIM ], [RTIP  , -H_RIM ],
  [RTIP  ,  H_RIM ], [R_PLT ,  H_RIM ], [R_PLT ,  CC_HWEB], [R_HUB ,  CC_HWEB],
  [R_HUB ,  CC_HHUB], [R_BORE,  CC_HHUB], [R_BORE,  H_TON ],
  [R_TIN ,  H_TON ] ];
CC_FLG  = [ false,false,false,false, false,false,false,true,
            true ,false,false,false, false,false,false,false ];

// the point grid of one arc, [station][azimuth]
function cc_grid(a0, a1, j0, j1) =
  let (A = cc_az(a0, a1, j0, j1))
    [ for (t = [0:len(CC_PROF)-1])
        [ for (i = [0:len(A)-1])
            let (rr = CC_FLG[t] ? cc_tooth_r(A[i]) : CC_PROF[t][0])
              [ rr*cos(A[i]), rr*sin(A[i]), CC_PROF[t][1] ] ] ];

// ===================================================================
//  GEOMETRY
// ===================================================================
// Sweep a closed section round the axis between two azimuths and cap
// both ends.  The section runs counter-clockwise in (r, z) and the
// azimuth increases, so the quad (t,i)->(t+1,i)->(t+1,i+1)->(t,i+1)
// carries its right-hand normal INTO the solid, which is what this
// kernel's polyhedron wants: the face before it and the face after it
// traverse every shared edge the other way round.  The cap at the first
// azimuth is therefore the section listed backwards and the cap at the
// last azimuth the section listed forwards.
module cc_sweep(G) {
    T = len(G); MM = len(G[0]);
    id = function (t,i) (t*MM + i);
    polyhedron(
      points = [ for (t = [0:T-1]) for (i = [0:MM-1]) G[t][i] ],
      faces = concat(
        [ for (t = [0:T-1]) for (i = [0:MM-2])
            [ id(t,i), id((t+1)%T,i), id((t+1)%T,i+1), id(t,i+1) ] ],
        [ [ for (t = [T-1:-1:0]) id(t,0) ] ],
        [ [ for (t = [0:T-1]) id(t,MM-1) ] ] ),
      convexity = 12 );
}

// one half collar: tongue, clamp hub, web and the toothed rim
module cc_arc(a0, a1, j0, j1) { cc_sweep(cc_grid(a0, a1, j0, j1)); }

// A lug pad: a box on the hub's top face with the bolt bore through it,
// extruded along the bolt axis so both its faces are square to the bolt.
// The bore is walled, not subtracted: the section is one polygon with an
// inner path.  dir = +1 puts it on the +x side of the split plane.
module cc_pad(dir, zlo, zhi, zb) {
    n = CC_NPAD;
    // sketch x is the axial coordinate, sketch y the radial one; the
    // rotate below sends sketch x to world z with dir's sign on it
    sk = concat(
      [ [-dir*zlo, R_BORE], [-dir*zhi, R_BORE],
        [-dir*zhi, PAD_RO], [-dir*zlo, PAD_RO] ],
      [ for (i = [0:n-1])
          [ -dir*zb + CC_BHOLE*sin(360*i/n),
            R_BOLT  + CC_BHOLE*cos(360*i/n) ] ] );
    translate([dir*CC_GAPMM/2, 0, 0])
      rotate([0, dir*90, 0])
        linear_extrude(height = CC_PADL)
          polygon(points = sk, paths = [[0,1,2,3], [for (i=[0:n-1]) 4+i]]);
}

// the clamp bolt: head and shank, drawn at the shank's major diameter
// with a clearance in both bores, so it shares no volume with the pads
module cc_bolt(z) {
    translate([-(CC_GAPMM/2 + CC_PADL + CC_BHH + CC_PART), R_BOLT, z])
      rotate([0, 90, 0]) {
        cylinder(r = CC_BHR, h = CC_BHH, $fn = CC_FNB);
        translate([0, 0, CC_BHH])
          cylinder(r = CC_BSH, h = CC_GAPMM + 2*CC_PADL, $fn = CC_FNB);
    }
}

// one joint: two lugs, one on each arc, with a bolt boss above and
// below the chain plane on each, and a bolt through each pair
module cc_joint() {
    for (s = [1, -1]) {
      cc_pad(s,  Z_PAD0,  Z_PAD0 + CC_PADH,  Z_BOLT);
      cc_pad(s, -Z_PAD0 - CC_PADH, -Z_PAD0, -Z_BOLT);
    }
    cc_bolt( Z_BOLT);
    cc_bolt(-Z_BOLT);
}

module collar_chain() {
    color([0.62,0.66,0.70]) cc_arc(A0, A1, J0, J1);
    color([0.58,0.62,0.66]) rotate([0,0,180]) cc_arc(A0, A1, J0, J1);
    color([0.72,0.60,0.38]) cc_joint();
    color([0.72,0.60,0.38]) rotate([0,0,180]) cc_joint();
}

// ---- the exact volume of the mesh this file writes --------------------
// The divergence theorem over the very faces cc_sweep lists.  The faces
// carry their normals INTO the solid, so the sum comes out negative and
// is negated.  A quad whose four corners are not coplanar has two
// triangulations; both are computed and both are printed, because the
// exporter picks one of them and the difference is the whole uncertainty.
function cc_det(a,b,c) = a[0]*(b[1]*c[2] - b[2]*c[1])
                       + a[1]*(b[2]*c[0] - b[0]*c[2])
                       + a[2]*(b[0]*c[1] - b[1]*c[0]);
function cc_mesh_vol(G) =
  let (T = len(G), MM = len(G[0]),
       side = sum([ for (t = [0:T-1]) for (i = [0:MM-2])
                      let (p = G[t][i], q = G[(t+1)%T][i],
                           u = G[(t+1)%T][i+1], v = G[t][i+1])
                        [ cc_det(p,q,u) + cc_det(p,u,v),
                          cc_det(p,q,v) + cc_det(q,u,v) ] ]),
       // both caps are planar, so any fan of them carries the same volume
       caps = sum([ for (t = [1:T-2])
                      cc_det(G[T-1][0], G[T-1-t][0], G[T-2-t][0]) ])
            + sum([ for (t = [1:T-2])
                      cc_det(G[0][MM-1], G[t][MM-1], G[t+1][MM-1]) ]) )
    -(side + [caps, caps])/6;
// a regular n-gon of circumradius R, inscribed: area
function cc_ngon(n, R) = n/2*R*R*sin(360/n);

V_ARC  = cc_mesh_vol(cc_grid(A0, A1, J0, J1));
V_PAD  = CC_PADL*CC_PADH*(PAD_RO - R_BORE)
       - cc_ngon(CC_NPAD, CC_BHOLE)*CC_PADL;
V_BOLT = cc_ngon(CC_FNB, CC_BHR)*CC_BHH
       + cc_ngon(CC_FNB, CC_BSH)*(CC_GAPMM + 2*CC_PADL);
V_TOT  = 2*V_ARC[0] + 8*V_PAD + 4*V_BOLT;

// ===================================================================
//  REPORT
//  Every line prints both sides of whatever it claims.
// ===================================================================
echo("=== 0. what the chain collar is ===");
echo(str("two arcs, ", 4*CC_NBJ, " lug pads and ", 2*CC_NBJ,
         " bolts, drawn as members that share no volume.  The lug pads",
         " are the same piece as their arc in the part; here each stands",
         " off the hub's face by ", CC_PART, " mm and each bolt head stands",
         " off its seat by the same, because this kernel's export-time",
         " union welds shells that touch and leaves T junctions in the",
         " weld.  That clearance is the one place the model differs from",
         " the part, and it is declared."));

echo("=== 1. the chain, declared, not derived ===");
echo(str("   pitch p = ", CC_P, " mm   roller diameter d = ", CC_DR,
         " mm   inner width ", CC_BI, " mm"));
echo(str("   plate depth ", CC_HP, " mm   width over the pin heads ", CC_WO,
         " mm   joint clearance ", CC_CJ, " mm   working tension ",
         CC_FW, " N"));

echo("=== 2. the pitch polygon closes ===");
echo(str("   (1) p = 2 r_p sin(180/N): 2*", RP, "*sin(", BETA, ") = ",
         2*RP*sin(BETA), " = p = ", CC_P,
         "   residual ", 2*RP*sin(BETA) - CC_P));
echo(str("   N*(360/N) = ", NCH, "*", 360/NCH, " = ", NCH*(360/NCH),
         " = 360   remainder ", NCH*(360/NCH) - 360));
echo(str("   pitch diameter 2 r_p = ", 2*RP,
         " mm   polygon perimeter N p = ", NCH*CC_P,
         " mm   pitch circumference 2 pi r_p = ", 2*PI*RP,
         " mm   the polygon is short by ", 2*PI*RP - NCH*CC_P, " mm"));
echo(str("   chordal rise 1 - cos(180/N) = ", 1 - cos(BETA), " = ",
         100*(1 - cos(BETA)), " percent speed ripple, ",
         RP*(1 - cos(BETA)), " mm of roller rise"));

echo("=== 3. the seat stack-up, from the band out ===");
echo(str("   the register is READ: eb_if_floor() = ", eb_if_floor(),
         "   eb_if_land() = ", EB_LND, "   eb_if_groove() = ", EB_GRV,
         "   eb_if_clear() = ", EB_CLR,
         ";  the bore this file derives from them is ", R_BORE,
         " and the band's own eb_if_collar() says ", eb_if_collar()[0],
         ", difference ", R_BORE - eb_if_collar()[0],
         ";  tongue face ", R_TIN, " against ", eb_if_collar()[1],
         ", difference ", R_TIN - eb_if_collar()[1],
         ";  tongue width ", 2*H_TON, " against ", eb_if_collar()[2],
         ", difference ", 2*H_TON - eb_if_collar()[2]));
echo(str("   the hub is ", 2*CC_HHUB, " mm long on a register that allows ",
         eb_if_hub(), " mm symmetric on the groove: fits ",
         2*CC_HHUB <= eb_if_hub()));
echo(str("   groove floor = the equatorial cylinder sg_r() = ", R_GF,
         " mm from the contract; the rib height, the two widths and the",
         " two clearances are read from the band, and the rest of this",
         " chain is this file's own"));
echo(str("   rib crest    floor + ", CC_RIBH, " = ", R_RIB));
echo(str("   tongue face  floor + ", CC_CR, " = ", R_TIN));
echo(str("   collar bore  crest + ", CC_CR, " = ", R_BORE));
echo(str("   hub outside  bore  + ", CC_THUB, " = ", R_HUB));
echo(str("   LEAST rim base  hub   + ", CC_TWEB, " = ", R_WMIN,
         " -- a bound on the count only.  Nothing is built at this",
         " radius; the web really ends at R_plt, section 5."));
echo(str("   seat arc radius r_s = d/2 + c_s = ", CC_DR/2, " + ", CC_CS,
         " = ", RS));
echo(str("   least pitch radius r_p >= least rim base + ", CC_TRIM,
         " + r_s = ", RP_MIN, " mm"));

echo("=== 4. the tooth count, and every divisibility it is asked for ===");
echo(str("   r_p(N) = p/(2 sin(180/N)) rises with N, so r_p >= ", RP_MIN,
         " means N >= 180/asin(p/(2 r_p_min)) = ", N_MIN,
         ", that is N >= ", ceil(N_MIN)));
echo(str("   N must also be even, so the collar parts into two arcs with a",
         " whole number of seats each, and divisible by 3, so the sprocket",
         " carries the three-planet rows' own symmetry.  lcm(2,3) = ",
         NDIV, ", so N is the least multiple of ", NDIV, " at or above ",
         ceil(N_MIN), ":"));
echo(str("   N = ", NDIV, "*ceil(", ceil(N_MIN), "/", NDIV, ") = ", NDIV,
         "*", ceil(ceil(N_MIN)/NDIV),
         " = ", NCH, "   and ", NCH, " = 2*", NCH/2, " = 3*", NCH/3,
         " = 6*", NCH/6));
echo(str("   N/2 = ", NCH/2, "   integer: ", NCH%2 == 0,
         "   so each arc carries ", NCH/2, " whole seats"));
echo(str("   N/3 = ", NCH/3, "   integer: ", NCH%3 == 0));
echo(str("   N/4 = ", NCH/4, "   integer: ", NCH%4 == 0,
         "   -- it is NOT, and that is what puts a tooth CREST at 90 and",
         " 270 rather than a roller seat, so the split misses every seat"));
for (r_ = ROWS_)
  echo(str("      against ", r_[0], " k = ", r_[4], ":  N/k = ", NCH, "/",
           r_[4], " = ", NCH/r_[4], "   integer: ", NCH%r_[4] == 0,
           "   gcd(N,k) = ", gcd_(NCH, r_[4])));
echo(str("      against the crown ring Dref = ", DREF, ":  gcd = ",
         gcd_(NCH, DREF), "   lcm = ", lcm_(NCH, DREF),
         "   both N and Dref are whole counts per revolution of one rigid",
         " band, so both excitations close on the revolution"));
echo(str("      against the sun Ns = ", NS, ":  gcd = ", gcd_(NCH, NS),
         "   N/Ns = ", NCH/NS));
echo(str("   the library's report echoes a ", CC_ECHO_N,
         " tooth chain collar; this file derives ", NCH,
         "   difference ", NCH - CC_ECHO_N));
echo(str("   margin: r_p - r_p_min = ", RP - RP_MIN,
         " mm, so the depth from the least rim base to the seat floor is ",
         R_FLOOR - R_WMIN, " mm against the ", CC_TRIM,
         " mm asked for -- satisfied: ", R_FLOOR - R_WMIN >= CC_TRIM,
         ".  That surplus IS the rounding up to the next multiple of ",
         NDIV, ".  The THIN rim under the seat floor is a different and",
         " smaller number, printed in section 5."));

echo("=== 5. the roller seat and the tooth it sits between ===");
echo(str("   seat floor r_p - r_s = ", R_FLOOR,
         "   bottom diameter ", 2*R_FLOOR,
         " = pitch diameter - roller diameter - 2 c_s = ", 2*RP - CC_DR,
         " - ", 2*CC_CS, " = ", 2*RP - CC_DR - 2*CC_CS,
         "   residual ", 2*R_FLOOR - (2*RP - CC_DR - 2*CC_CS)));
echo(str("   the seat arc ends where the contact normal makes the declared ",
         CC_PA, " deg with the tangential, so its half extent from the",
         " floor is AE = 90 - ", CC_PA, " = ", AE, " deg"));
echo(str("   flank = the straight line tangent to the seat circle there.",
         "  distance from the pitch point to that line = ",
         abs((RP - QX)*UY - (-QY)*UX), " = r_s = ", RS,
         "   residual ", abs((RP - QX)*UY + QY*UX) - RS));
echo(str("   its foot stands at azimuth ", PSI0, " = -AE = ", -AE,
         "   residual ", PSI0 + AE));
echo(str("   its distance from the axis ", HFL,
         " = r_p sin(phi_ch) - r_s = ",
         RP*sin(CC_PA), " - ", RS, " = ", RP*sin(CC_PA) - RS,
         "   residual ", HFL - (RP*sin(CC_PA) - RS)));
echo(str("   seat meets flank at azimuth ", DQ, " deg, radius ", RQ,
         ";  seat form gives ", cc_seat_r(DQ), " residual ",
         cc_seat_r(DQ) - RQ, ";  flank form gives ", cc_flank_r(DQ),
         " residual ", cc_flank_r(DQ) - RQ));
echo(str("   topping circle r_tip = p(", CC_TOP, " + cot(180/N))/2 = ", RTIP,
         ";  flank form at the tip azimuth ", DT, " deg gives ",
         cc_flank_r(DT), " residual ", cc_flank_r(DT) - RTIP));
echo(str("   tooth height above the pitch circle ", RTIP - RP, " mm = ",
         (RTIP - RP)/CC_P, " p;  tip land 2*(180/N - ", DT, ") = ",
         2*LAND, " deg = ", rad(2*LAND)*RTIP,
         " mm of arc   positive: ", LAND > 0));
echo(str("   rim thickness 2*", H_RIM, " = ", 2*H_RIM,
         " mm inside the chain's ", CC_BI, " mm, clearance ", CC_CW,
         " mm a side"));
echo(str("   the web ENDS and the rim thins at R_plt = r_p - plate/2 - ",
         CC_CP, " = ", R_PLT, ", which is ", R_PLT - R_WMIN,
         " mm outside the least rim base (so (5) is satisfied with room: ",
         R_PLT > R_WMIN, ") and ", R_FLOOR - R_PLT,
         " mm inside the seat floor, so every seat lies in the thin rim: ",
         R_FLOOR > R_PLT));
echo(str("   that ", R_FLOOR - R_PLT, " mm IS the thin rim under the seat",
         " floor, and it is NOT the ", CC_TRIM, " mm of (5): (5) bounds",
         " r_p - r_s - R_wmin = ", R_FLOOR - R_WMIN,
         ", while the thin section is r_p - r_s - R_plt = plate/2 + c_p",
         " - r_s = ", CC_HP/2 + CC_CP - RS, "   residual ",
         (R_FLOOR - R_PLT) - (CC_HP/2 + CC_CP - RS)));
echo(str("   samples: ", 2*CC_NH+1, " on each seat arc, 3 on each tip land",
         ", and the flank sampled at its two ends ONLY, so the chord",
         " between them is the straight flank itself; the faceting on a",
         " tooth is therefore all on the seat arc and the land"));
echo(str("   web half height ", CC_HWEB,
         " mm is more than the chain's own half envelope over the pin",
         " heads, ", CC_WO/2, " mm, so the web would foul the chain if it",
         " reached it: the plates' inner edge is at r = ", RP - CC_HP/2,
         " and the web stops at R_plt = ", R_PLT, ", short by ",
         RP - CC_HP/2 - R_PLT, " mm   clear: ", R_PLT < RP - CC_HP/2));
echo(str("      that margin is c_p = ", CC_CP,
         " by the definition of R_plt, not a larger figure taken off the",
         " bound R_wmin = ", R_WMIN, "   residual ",
         (RP - CC_HP/2 - R_PLT) - CC_CP,
         ".  It is the tightest clearance in the part and it is declared."));

echo("=== 6. the wrap, and the loop that closes on it ===");
echo(str("   pinion ", CC_N2, " teeth, pitch radius ", RP2,
         ";  port ratio N/N2 = ", NCH, "/", CC_N2, " = ", NCH/CC_N2,
         "   integer: ", NCH%CC_N2 == 0,
         " -- exact, and the cost of exact is that a pinion tooth meets the",
         " same ", NCH/CC_N2, " collar teeth every turn"));
echo(str("   loop at the nominal centre ", CC_C0, " mm: L = 2C/p + (N+N2)/2",
         " + ((N-N2)/2pi)^2 p/C = ", L_NOM,
         " pitches;  a roller chain closes only on an EVEN number of",
         " pitches, so L = ", NL, "   L/2 = ", NL/2,
         "   even: ", NL%2 == 0));
echo(str("   the centre distance is then solved, not chosen: the taut band",
         " on the two pitch circles is 2C cos(ph) + (pi+2ph) r_p +",
         " (pi-2ph) r_p2 with sin(ph) = (r_p-r_p2)/C.  C = ", CDIST,
         " mm gives ", NL*CC_P + cc_flen(CDIST), " mm against L p = ",
         NL*CC_P, " mm   residual ", cc_flen(CDIST)));
echo(str("   that band runs on ARCS; a chain's pitch line is the inscribed",
         " polygon, shorter by 2 r (rad(180/N) - sin(180/N)) a pitch: ",
         DEFC, " mm over the whole loop, which the take-up absorbs"));
echo(str("   wrap on the collar 180 + 2*", PHIW, " = ", WRAP,
         " deg, from azimuth ", TANG0, " through 180 to ", TANG1,
         ";  on the pinion ", WRAP2, " deg"));
echo(str("   seats under the chain = wrap/(360/N) = ", WRAP, "/", 360/NCH,
         " = ", NENG, ", of the ", NCH, " the collar has"));
echo(str("   hunting: a roller meets the same tooth again after lcm(N,L)",
         " pitches, so it visits N/gcd(N,L) = ", NCH, "/", gcd_(NCH,NL),
         " = ", NCH/gcd_(NCH,NL), " of the ", NCH,
         " teeth.  An odd N would visit all of them; N had to be even to",
         " halve the collar, and this is what that costs."));

echo("=== 7. the split, the lugs and the clamp ===");
echo(str("   crest number ", (NCH/4 - 0.5), " sits at (360/N)*", NCH/4 - 0.5,
         " + 180/N = ", 2*BETA*(NCH/4 - 0.5) + BETA, " = 90   residual ",
         2*BETA*(NCH/4 - 0.5) + BETA - 90, ";  the splits are at 90 and 270"));
echo(str("   gap ", CC_GAPMM, " mm at the bolt radius ", R_BOLT,
         " is ", GAPD, " deg, so between radial faces it measures ",
         rad(GAPD)*R_BORE, " mm at the bore, ", CC_GAPMM, " mm at the bolt",
         " and ", rad(GAPD)*RTIP, " mm over the tips"));
echo(str("   arc A runs ", A0, " to ", A1, " = ", A1 - A0,
         " deg and carries seats ", J0, " to ", J1, " = ", J1 - J0 + 1,
         " of them = N/2 = ", NCH/2, "   equal: ", J1 - J0 + 1 == NCH/2));
echo(str("   its first seat starts at ", 2*BETA*J0 - DQ, " deg against the",
         " arc's end at ", A0, ": clear by ", 2*BETA*J0 - DQ - A0, " deg = ",
         rad(2*BETA*J0 - DQ - A0)*RQ, " mm, so no seat is cut"));
echo(str("   the half tooth at the split is cut off at radius ",
         cc_flank_r(2*BETA*J0 - A0), " and the seat's contact ends at ",
         RQ, ": the cut removes ", cc_flank_r(2*BETA*J0 - A0) - RQ,
         " mm of clearance flank and the tip land, not contact surface"));
echo(str("   seats either side of the 90 split: ", 2*BETA*(NCH/4 - 0.5),
         " on one arc and ", 2*BETA*(NCH/4 + 0.5),
         " on the other, both whole, and the crest between them carries no",
         " roller, which is why the chain crosses the split without",
         " meeting it (the wrap covers it: ", TANG0, " to ", TANG1, ")"));
echo(str("   cc_joint draws one boss above and one below the chain plane on",
         " each arc, so the bolts per joint are 2 BY CONSTRUCTION and",
         " CC_NBJ only names it: CC_NBJ = ", CC_NBJ, "   equals 2: ",
         CC_NBJ == 2, " -- every count and every hoop tension below is",
         " wrong if that is ever false"));
echo(str("   lugs: ", 2*CC_NBJ, " pads a joint, ", CC_NBJ,
         " on each arc's end, one above and one below the chain plane,",
         " because a one-sided clamp twists the arc.  Pad ", CC_PADL, " x ",
         CC_PADH, " x ", PAD_RO - R_BORE, " mm, its outer face a plane at ",
         PAD_RO, " whose far corners touch the hub's own ", R_HUB,
         "   residual ", sqrt(PAD_RO*PAD_RO + pow(CC_GAPMM/2 + CC_PADL,2))
                          - R_HUB));
echo(str("   bolt: bore radius ", CC_BHOLE, " a ", CC_NPAD,
         "-gon, least radius ", CC_BHOLE*cos(180/CC_NPAD),
         ", shank radius ", CC_BSH, ", clearance ",
         CC_BHOLE*cos(180/CC_NPAD) - CC_BSH,
         " mm.  Neither the thread NOR THE NUT is modelled: both pad bores",
         " are clearance bores at ", CC_BHOLE,
         ", so the joint takes a nut, and no volume below counts one."));
echo(str("   wrench access: the bolt axis is at z = ", Z_BOLT,
         " and the hub's top is at z = ", CC_HHUB, ", so the axis clears",
         " the collar by ", Z_BOLT - CC_HHUB,
         " mm and a key runs in along it; the chain is outside r = ",
         R_PLT, " and the bolt is at r = ", R_BOLT, ", clear by ",
         R_PLT - R_BOLT - CC_BHR, " mm"));
echo(str("   clamp: a hoop tension F on a ring of radius R gripping over",
         " a length L gives pressure p = F/(R L), and the friction torque",
         " is mu p (2 pi R L) R = 2 pi mu F R -- L cancels."));
echo(str("      F = ", CC_NBJ, " bolts * ", CC_FB, " N = ", F_HOOP,
         " N;  grip length = 2 ribs * ", CC_RIBW, " = ", L_GRIP,
         " mm;  p = ", F_HOOP, "/(", R_BORE, "*", L_GRIP, ") = ", P_BORE,
         " N/mm^2"));
echo(str("      T = 2 pi * ", CC_MU, " * ", F_HOOP, " * ", R_BORE,
         " = ", T_CLAMP, " Nm, against the chain's own ", CC_FW, " N * ",
         RP, " mm = ", T_CHAIN, " Nm"));
echo(str("      of the two torques this file computes the CHAIN is the",
         " limit, not the clamp: T_chain/T_clamp = ", T_CHAIN/T_CLAMP,
         ", chain is the smaller: ", T_CHAIN < T_CLAMP,
         ".  So the port's own ceiling is ", T_CHAIN, " Nm."));
echo(str("      gate 7's echoed ", CC_ECHO_G7, " Nm is printed beside them",
         " and is NOT adjudicated against them: it is a per-tooth-pair",
         " figure at the POLAR crown, a different member and a different",
         " quantity.  Their quotient ", T_CHAIN/CC_ECHO_G7,
         " is a ratio of two unlike things and is offered as nothing more."));

echo("=== 8. backlash, gate 3 ===");
echo("   the chain's lost motion is the freedom of the engaging roller:");
echo(str("      2 c_s from the seat, which is cut ", CC_CS,
         " mm over the roller all round, plus ", CC_CJ,
         " mm at that one joint = ", BL_MM, " mm at the pitch radius"));
echo(str("      = ", BL_MM, "/", RP, " rad = ", BL_DEG,
         " deg.  The collar is clamped to the band, so that is the angle",
         " AT THE BAND too, and it is ", BL_BAND,
         " mm of arc at the register radius ", RSPH));
echo(str("      the band is the equator row's ring, Nr = ", NR_EQ,
         ", so with the carrier held it refers to the sun as ", BL_DEG,
         "*", NR_EQ, "/", NS, " = ", BL_SUN, " deg"));
echo(str("      gate 6's gear lost motion, recomputed from the table, is",
         " 4 jt/(Ns m) = 4*", JT_*M_, "/", NS*M_, " = ", BL_GEAR,
         " deg per slice; the chain is ", BL_SUN/BL_GEAR,
         " times that on its own"));
echo(str("      chain pitch tolerance is NOT counted above: it is a length",
         " error the take-up absorbs, not lost motion at the tooth"));
echo("   that ratio is gate 3: the chain is admitted here and nowhere else,");
echo("   and nothing downstream of this port is a precision path.");

echo("=== 9. the seat that registers the collar to the band ===");
echo(str("   the groove floor is put on sg_r() = ", RSPH,
         " mm because that is the one radius the contract fixes: 2R/m = ",
         DREF, " is the equator row's ring count, so R = ", DREF, "*", M_,
         "/2 = ", DREF*M_/2, "   residual ", RSPH - DREF*M_/2));
echo(str("   tongue: ", 2*H_TON, " mm wide in a ", CC_GW,
         " mm groove, ", CC_CA, " mm a side; its face at ", R_TIN,
         " clears the floor at ", R_GF, " by ", R_TIN - R_GF, " mm"));
echo(str("   bore at ", R_BORE, " over rib crests at ", R_RIB, ": ",
         R_BORE - R_RIB, " mm, which is what the clamp takes up"));
echo(str("   the bore covers both ribs: hub half height ", CC_HHUB,
         " against groove half width + rib width = ", CC_GW/2, " + ",
         CC_RIBW, " = ", CC_GW/2 + CC_RIBW, ", overhang ",
         CC_HHUB - CC_GW/2 - CC_RIBW, " mm a side   covers: ",
         CC_HHUB >= CC_GW/2 + CC_RIBW));
echo(str("   the chain plane and the register plane are both z = 0, so the",
         " chain pull passes through the tongue and puts no overturning",
         " couple on the collar"));

echo("=== 10. volumes: the exact volume of the mesh this file writes ===");
echo(str("   one arc: ", floor(V_ARC[0]), " + ", V_ARC[0] - floor(V_ARC[0]),
         " mm^3 on one quad diagonal, and the other diagonal differs by ",
         V_ARC[0] - V_ARC[1], " mm^3"));
echo(str("   one lug pad: ", CC_PADL, "*", CC_PADH, "*", PAD_RO - R_BORE,
         " - ", cc_ngon(CC_NPAD, CC_BHOLE), "*", CC_PADL, " = ", V_PAD,
         " mm^3, exact: every face of it is planar"));
echo(str("   one bolt: ", cc_ngon(CC_FNB, CC_BHR), "*", CC_BHH, " + ",
         cc_ngon(CC_FNB, CC_BSH), "*", CC_GAPMM + 2*CC_PADL, " = ", V_BOLT,
         " mm^3, exact: an n-gon prism is (n/2) R^2 sin(360/n) h"));
echo(str("   total 2*arc + ", 4*CC_NBJ, "*pad + ", 2*CC_NBJ, "*bolt = ",
         floor(V_TOT), " + ", V_TOT - floor(V_TOT), " mm^3"));
echo(str("   the members share no volume, so that total is the volume of",
         " the mesh itself, not a bound on it.  It is exact for the mesh as",
         " this file holds it, in f64.  An exporter writes coordinates at",
         " its format's own resolution -- f32 in a binary STL, six decimals",
         " in an OFF -- so a validator measuring the FILE will differ from",
         " the figure above at about that resolution.  This file cannot",
         " measure the written file, and does not claim to: it claims only",
         " the f64 figure, and where the two disagree the file is the",
         " coarser of the two."));
echo(str("   mesh: ", len(CC_PROF), " stations * ",
         len(cc_az(A0,A1,J0,J1)), " azimuth samples an arc"));

echo("=== 11. the envelope the collar occupies, for an assembly ===");
echo(str("   everywhere: r from the tongue face ", R_TIN, " out to r_tip ",
         RTIP, ", |z| <= ", CC_HHUB, " on the bore and ", H_RIM,
         " on the teeth; the register mid-plane is z = 0"));
echo(str("   AT EACH JOINT, beyond that: a boss from r = ", R_BORE, " to ",
         PAD_RO, " over |z| = ", Z_PAD0, " to ", Z_PAD0 + CC_PADH,
         ", and a bolt head reaching ",
         CC_GAPMM/2 + CC_PADL + CC_BHH + CC_PART,
         " mm either side of the split plane at r = ", R_BOLT - CC_BHR,
         " to ", R_BOLT + CC_BHR, ", |z| = ", Z_BOLT - CC_BHR, " to ",
         Z_BOLT + CC_BHR));
echo(str("   so the collar's full axial envelope is |z| <= ",
         Z_PAD0 + CC_PADH, " mm about the register mid-plane, against ",
         CC_HHUB, " mm for the clamped bore alone.  The band is not",
         " modelled here, so whether it leaves that space above and below",
         " its groove is the ASSEMBLY's check and this file does not make",
         " it."));

echo("=== 12. the interface, as cc_if_*() publishes it ===");
echo(str("   cc_if_groove()   = ", cc_if_groove(),
         "  [floor r, rib h, rib w, groove w, c_r, c_a] -- the BAND's side"));
echo(str("   cc_if_bore()     = ", cc_if_bore(),
         "   cc_if_tongue() = ", cc_if_tongue(),
         "   cc_if_width() = ", cc_if_width()));
echo(str("   cc_if_tip()      = ", cc_if_tip(), "   cc_if_teeth() = ",
         cc_if_teeth(), "   cc_if_pitch() = ", cc_if_pitch()));
echo(str("   cc_if_split()    = ", cc_if_split(),
         "   cc_if_arc() = ", cc_if_arc()));
echo(str("   cc_if_bolt()     = ", cc_if_bolt(),
         "   cc_if_envelope() = ", cc_if_envelope()));
echo(str("   an assembly places collar_chain() with its origin on the",
         " band's groove MID-PLANE and its +z on the stack axis; the two",
         " splits then lie at azimuth 90 and 270 of the assembly frame",
         " unless it rotates the collar."));

// ===================================================================
//  THE PART
// ===================================================================
collar_chain();
