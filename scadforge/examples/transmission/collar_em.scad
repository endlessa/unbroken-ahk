// ===================================================================
//  collar_em.scad -- THE REGENERATIVE EM COLLAR, the electrical
//  equatorial input of one transmission slice.
//
//  An axial-flux machine that clamps on the same equatorial band the
//  chain collar clamps: 24 slots and 28 poles, three phase, from
//  sg_em(); a 12 F supercapacitor bank over 12-24 V, from sg_cap().
//  Every quantity this collar shares with the rest of the machine is
//  read from spherical_gear.scad -- sg_r(), sg_m(), sg_em(), sg_cap(),
//  sg_rows(), sg_ns(), gcd_(), lcm_() -- and none of them is restated
//  here.  Everything else is a collar-side choice, named once in the
//  DESIGN block, and printed in the report beside the identity or the
//  clearance it has to satisfy.
//
//  Nothing below is eyeballed and nothing below is asserted without
//  being printed.  Where a number is an ASSUMPTION rather than a
//  consequence -- the airgap shear stress, the volumetric energy
//  density of a supercapacitor can -- it is labelled as one in the
//  report, with where the figure comes from.
//
//  ---------------------------------------------------------------
//  1.  WHERE IT CLAMPS, AND WHAT THE INTERFACE IS
//
//  The contract publishes the sphere radius R = sg_r() as a function.
//  It does NOT publish the equatorial band's own outer diameter: its
//  equatorial-band report prints the outer spur face radius and the
//  radial wall between the two faces, but as echoed text, not as a
//  function, so this file cannot read them -- and a restated constant
//  is a constant that can drift, so it does not copy them either.
//  The clamp seat is therefore placed by one collar-side offset,
//
//      R_seat = R + off                                          (1)
//
//  off = 24 mm, chosen to stand outboard of the band's toothed outer
//  face as the contract's own equatorial-band report prints it.  That
//  choice, and the seven dimensions that go with it, are the whole
//  interface; they are published as em_if_*() so a second collar can
//  READ them rather than copy them, and the report prints all eight.
//
//  The clamp is genuinely split: two halves, each spanning
//
//      a_half = 180 - deg(gap / R_seat)                          (2)
//
//  of azimuth, so the parting gap is a stated 1.0 mm of ARC at the
//  seat rather than a stated angle, and one tongue per half drops
//  2.5 mm into a keyway in the band.  Two tangential pinch bolts
//  close the halves; six axial bolts on one circle carry whatever
//  collar is fitted.  A collar that repeats those eight numbers
//  interchanges with this one.  THE CHAIN COLLAR IS NOT PRESENT IN
//  THIS DIRECTORY as this file is written, so interchangeability is
//  a published contract here, not a checked one, and the report says
//  so in those words.
//
//  ---------------------------------------------------------------
//  2.  THE WINDING ARITHMETIC
//
//  S = 24 slots, P = 28 poles from sg_em().  Slots per pole per
//  phase is the defining fraction of the winding:
//
//      q = S / (3 P) = 24/84 = 2/7                               (3)
//
//  reduced by gcd(24,84) = 12, which the file prints as the reduction
//  it is.  A three phase winding on S slots and P poles is BALANCED --
//  the three phase belts are congruent under rotation -- when
//
//      S / (3 gcd(S,P))  is an integer                           (4)
//
//  and here gcd(24,28) = 4 gives 24/12 = 2.  Each phase owns
//  S/3 = 8 of the 24 concentrated coils.
//
//  The winding factor is not quoted, it is computed from the star of
//  slots.  Slot i sits at electrical angle
//
//      th_i = (P/2)(360/S) i = 210 i  degrees                    (5)
//
//  Phase A takes the slots whose phasor falls in the half open belt
//  [-30, 30) with sign +1 and in [150, 210) with sign -1; the file
//  prints that this selects exactly S/3 slots.  Then
//
//      kd = | SUM s_i exp(j th_i) | / (S/3)                      (6)
//      kp = sin( (P/2)(360/S) / 2 )                              (7)
//      kw = kp kd                                                (8)
//
//  because a concentrated coil spans exactly one slot pitch, which is
//  (P/2)(360/S) = 210 electrical degrees.  The file prints kd, kp and
//  kw.  That kw = 0.933 is the familiar figure for a 24/28
//  concentrated winding is a MEMORY and is flagged as one; the 0.933
//  printed here is equation (8) evaluated, not a recalled number.
//
//  Cogging.  A slot and a pole return to the same relative position
//  after lcm(S,P) events in a mechanical revolution:
//
//      N_cog = lcm(S,P) = lcm(24,28) = 168 = 24*7 = 28*6          (9)
//
//  and three phase torque ripple, six pulses per electrical cycle,
//  lands at
//
//      N_rip = 6 (P/2) = 84                                     (10)
//
//  so N_cog = 2 N_rip exactly: the two electrical excitations are not
//  independent, they share the factor 84, and the file prints
//  gcd(168,84) = 84 rather than leaving it implied.
//
//  THE COINCIDENCE WITH THE GEAR MESHES is the part worth stating
//  plainly.  Two periodic excitations of orders p and q per
//  revolution add to a pattern whose period is 360/gcd(p,q) degrees,
//  that is, one that repeats gcd(p,q) times per revolution, and their
//  intermodulation |i p - j q| can reach any multiple of gcd(p,q) and
//  nothing smaller.  With 168 = 2^3 * 3 * 7:
//
//      row 1    Nr =  59 = 59            gcd = 1
//      row 2    Nr = 143 = 11*13         gcd = 1
//      row 3    Nr = 188 = 2^2 * 47      gcd = 4
//      equator  Nr = 242 = 2 * 11^2      gcd = 2
//      sun      Ns =  46 = 2 * 23        gcd = 2
//
//  The file prints every one of those gcds and lcms with gcd_ and
//  lcm_ from the contract, prints the beat order |168 - N|, and says
//  which ones share a factor: rows 1 and 2 share NOTHING with the
//  cogging, so their combination with it repeats only once per
//  revolution -- the long slow beat is the one a rider feels -- while
//  row 3 shares 4 and the equator row and the sun share 2, so those
//  combinations repeat 4 and 2 times a revolution instead.  None of
//  this is hidden behind a "closes exactly": the closure the contract
//  proves under its gate 5 is that lcm(S,P) is an integer, which is
//  about the collar alone.
//
//  ---------------------------------------------------------------
//  3.  THE MAGNETIC GEOMETRY
//
//  Stator and rotor are flat annuli normal to the stack axis, which
//  is what makes the machine axial flux: 24 teeth stand up off a
//  back iron ring, each tooth a body 6.5 degrees wide carrying a shoe
//  12.6 degrees wide, so the slot opening is 15 - 12.6 = 2.4 degrees.
//  Facing them across a stated airgap of 1.0 mm are 28 magnets, each
//  an arc of 0.85 of the pole pitch 360/28, of real thickness on a
//  rotor back iron disc.  The active annulus runs from R_ai to R_ao;
//  the report prints the pole pitch, the magnet arc, the slot pitch,
//  the tooth and shoe widths and the slot opening all as MILLIMETRES
//  at the mean radius, because degrees hide whether a thing can be
//  wound.
//
//  Torque.  An airgap of mean shear stress sigma over an annulus
//  gives
//
//      T = sigma INT r dA = sigma 2 pi (R_ao^3 - R_ai^3)/3       (11)
//
//  and sigma is an ASSUMPTION: 0.012 N/mm^2, the low end of the
//  usual 10-30 kPa band for an air cooled machine with no liquid
//  path, taken from memory of machine design practice and NOT
//  computed here.  Everything that follows from (11) -- the torque,
//  the per mesh share, the braking power, the seconds of buffer --
//  inherits that assumption and the report says so on the same line.
//
//  The collar torque enters the slice at the equator row's RING,
//  because the band is the ring, and Willis (10) of the contract then
//  fixes the split with the sun:
//
//      (w_sun - w_carrier)/(w_ring - w_carrier) = -Nr/Ns        (12)
//
//  which for the equator row is -242/46 = -121/23 exactly.  The file
//  prints that reduction with gcd_.  The MECHANICAL ceiling on this
//  port is not computed here: it is the contract's gate 7, printed
//  there, and this file does not restate it.
//
//  ---------------------------------------------------------------
//  4.  THE CAPACITOR BANK, AS CANS
//
//  sg_cap() = [C, Vlo, Vhi] = [12 F, 12 V, 24 V].  Usable energy
//  between the two rails:
//
//      E_use = C (Vhi^2 - Vlo^2)/2 = 6*(576-144) = 2592 J        (13)
//      E_full = C Vhi^2 / 2 = 3456 J                             (14)
//
//  so 75 percent of the stored energy is usable over a 2:1 voltage
//  band -- the file prints (13), and it is the same 2592 J the
//  contract prints under its gate 5, because both evaluate sg_cap().
//
//  The cans are sized from (14), not from (13), because a can has to
//  hold the charge it is charged to.  A series string of
//
//      n = ceil(Vhi / Vcell) = ceil(24/2.7) = 9 cells           (15)
//
//  at the EDLC cell voltage 2.7 V gives a string rated 24.3 V, and
//  n cells in series of C_cell each make C = C_cell/n, so
//
//      C_cell = n C = 108 F                                     (16)
//
//  The volumetric energy density is an ASSUMPTION, and here is where
//  it comes from: a commercial 2.7 V, 3000 F cell in a 60 x 138 mm
//  can holds 0.5*3000*2.7^2 = 10935 J in pi*30^2*138 = 390150 mm^3,
//  which is 0.0280 J/mm^3 = 7.8 Wh/L.  Derated to
//
//      e_v = 0.0216 J/mm^3 = 6.0 Wh/L                           (17)
//
//  for packaging, terminals and can wall, (14) needs
//
//      V_bank = E_full / e_v = 3456/0.0216 = 160000 mm^3        (18)
//
//  which is 17778 mm^3 per cell; at a 22 mm can diameter that is a
//  can 46.8 mm long, and the nine cans in this model are exactly
//  that size, computed, not drawn.  The 3000 F figure and the
//  derating are recalled catalogue practice, not measurements, and
//  the report labels both.
//
//  ---------------------------------------------------------------
//  5.  HOW THE SOLID IS BUILT
//
//  Everything is a swept shell.  A profile is a closed loop in the
//  (r, z) half plane listed counterclockwise; a station places that
//  profile at an azimuth, or at a point along a path.  em_shell then
//  writes one polyhedron over the grid.  The winding rule, once, for
//  all of it: build the stations and the profile so that
//  cross(d_station, d_profile) points OUT of the solid, then list
//  every side quad as (i,j) (i,j+1) (i+1,j+1) (i+1,j), whose right
//  hand normal is cross(d_profile, d_station) and therefore points
//  IN, which is what this kernel's polyhedron wants.  Two faces
//  sharing an edge then traverse it in opposite directions, and the
//  exported volume comes out positive.  End caps: the cap at station
//  0 is listed in REVERSED profile order and the cap at the last
//  station in forward order, for the same reason.
//
//  Two station lists at the SAME azimuth with different profiles
//  give a vertical step face, and that is how the tongue on the
//  clamp bore and the shoe on the stator tooth are steps rather than
//  ramps.
//
//  ADDITIVE ONLY, and no boolean at all.  Every hole in this part is
//  a bore built as a wall around it: a lug is an annular tube whose
//  inner cylinder IS the bolt hole, so nothing is ever subtracted.
//  And nothing overlaps: every body stands a stated 0.05 mm clear of
//  its neighbours, the same bookkeeping gap slice.scad uses.  In the
//  real part the clamp, its lugs and its ears are one casting; here
//  they are separate shells, because this kernel skips its export
//  time union past 25000 triangles and an overlap would then be
//  counted twice in the exported volume while still validating with
//  zero holes.  With no overlap anywhere, the exported volume is the
//  SUM of the bodies either side of that threshold, and the report
//  prints that sum to compare against.
//
//  ---------------------------------------------------------------
//  6.  WHAT THE FILE CHECKS, AND WHAT IT LEAVES OPEN
//
//  Printed with both sides shown: (1) through (18); the balance gate
//  (4); that the phase belt holds exactly S/3 slots; every gcd and
//  lcm of the cogging order against every mesh order; the eight
//  interface numbers; each of the eleven clearances that keep the
//  bodies apart, as a subtraction; the predicted volume of every
//  family of bodies and their total; and the body count, which must
//  equal the component count validate.py reports.
//
//  Left open and NOT modelled: the bolts themselves; the stator's
//  ground path past three mounting lugs; the phase leads, the
//  terminals and the bus to the cans; the slice housing.  The rotor
//  is carried by the clamp and the stator is grounded to the housing,
//  and only the first of those two is geometry in this file.
// ===================================================================

use <spherical_gear.scad>

// ===================================================================
//  READ FROM THE CONTRACT.  Nothing in this block is a literal.
// ===================================================================
M      = sg_m();                 // module, mm
RSPH   = sg_r();                 // sphere radius, mm
EMV    = sg_em();                // [slots, poles]
SLOTS  = EMV[0];
POLES  = EMV[1];
CAPV   = sg_cap();               // [C farad, Vlo, Vhi]
CAPF   = CAPV[0];
VLO    = CAPV[1];
VHI    = CAPV[2];
ROWS   = sg_rows();
ROWEQ  = sg_row(len(ROWS)-1);    // the equator row: the band IS its ring
NSUN   = sg_ns();

// ===================================================================
//  DESIGN -- collar-side choices.  Each one is printed in the report
//  next to the identity or clearance it has to satisfy.
// ===================================================================
EM_OFF   = 24;      // (1) seat offset outboard of R, mm
EM_WC    = 18;      // clamp axial width, mm
EM_TC    = 8;       // clamp radial wall, mm
EM_SPLIT = 1.0;     // parting gap, mm of ARC at the seat
EM_TD    = 2.5;     // tongue depth, mm
EM_TW    = 10;      // tongue crest width, mm of arc at the seat
EM_TCH   = 0.6;     // tongue flank chamfer, mm of arc at the seat
EM_NLUG  = 6;       // axial bolt lugs, 3 per half
EM_LUG0  = 30;      // azimuth of the first lug, deg
EM_BOLT  = 6;       // M6 everywhere on the interface
EM_RBORE = 3.2;     // M6 clearance bore radius, mm
EM_RLUG  = 6.0;     // lug/ear/tab outer radius, mm
EM_TTAB  = 6;       // collar-side tab thickness, mm
EM_EARL  = 8;       // pinch ear length along the bolt, mm
EM_EARS  = 0.5;     // pinch ear standoff from the parting plane, mm
EM_JG    = 0.05;    // bookkeeping gap between bodies, mm

EM_RAI   = 177;     // active annulus, inner radius, mm
EM_RAO   = 203;     // active annulus, outer radius, mm
EM_WT    = 6.5;     // stator tooth body width, deg
EM_WSH   = 12.6;    // stator shoe width, deg
EM_HT    = 12;      // tooth body height, mm
EM_TSH   = 4;       // shoe thickness, mm
EM_FIL   = 1.5;     // shoe-to-body fillet, mm of arc at the mean radius
EM_TBI   = 6;       // stator back iron thickness, mm
EM_BIX   = 11;      // back iron radial overhang each side, mm
EM_RW    = 5.0;     // coil bundle radius, mm
EM_LIN   = 1.0;     // slot liner clearance, tooth to bundle, mm
EM_AG    = 1.0;     // AIRGAP, shoe face to magnet face, mm
EM_TM    = 4;       // magnet thickness, mm
EM_PA    = 0.85;    // magnet arc as a fraction of the pole pitch
EM_TROT  = 6;       // rotor back iron disc thickness, mm
EM_RIM   = 6;       // rotor disc rim beyond the magnets, mm
EM_RMNT  = 7.0;     // stator mounting lug outer radius, mm
EM_RMB   = 4.2;     // stator mounting lug bore radius, M8 clearance
EM_NMNT  = 3;       // stator mounting lugs

EM_VCELL = 2.7;     // EDLC cell voltage, V           ASSUMPTION
EM_EV    = 0.0216;  // J/mm^3 packaged                ASSUMPTION, see (17)
EM_DCAN  = 22;      // can diameter, mm -- length follows from (18)
EM_RCAN0 = 150;     // inner end of the cans, mm
EM_SIG   = 0.012;   // N/mm^2 mean airgap shear        ASSUMPTION, see (11)
EM_RPM   = [60, 150, 300, 600];
EM_NRPM  = 300;     // the speed the buffer figure is quoted at, rpm

// mesh densities
NAZ  = 2;           // degrees of azimuth per station on a full ring
NSEC = 14;          // points round a coil bundle section
NPL  = 10;          // stations on a coil straight
NPC  = 6;           // stations on a coil corner quarter
NPR  = 14;          // stations on a coil arc
NTUB = 24;          // stations round a bolt tube

// ===================================================================
//  DERIVED GEOMETRY.  Everything here follows from the two blocks
//  above by arithmetic; the report prints each one with its identity.
// ===================================================================
RSEAT = RSPH + EM_OFF;            // (1) clamp bore
RCO   = RSEAT + EM_TC;            // clamp outer radius
RTNG  = RSEAT - EM_TD;            // tongue crest radius
GSPL  = deg(EM_SPLIT/RSEAT);      // (2) parting gap as an angle
GTNG  = deg(EM_TW/RSEAT);         // tongue crest width as an angle
GTCH  = deg(EM_TCH/RSEAT);        // tongue flank chamfer as an angle
RBC   = RCO + EM_JG + EM_RLUG;    // bolt circle radius
RLGX  = RBC + EM_RLUG;            // outermost radius of the clamp group

ZC1   =  EM_WC/2;                 // clamp top    z
ZC0   = -EM_WC/2;                 // clamp bottom z
ZTB0  = ZC1 + EM_JG;              // collar-side tab, bottom
ZTB1  = ZTB0 + EM_TTAB;           // collar-side tab, top   = rotor disc top
ZROT1 = ZTB1;                     // rotor back iron disc, top
ZROT0 = ZTB0;                     // rotor back iron disc, bottom
ZMG1  = ZROT0 - EM_JG;            // magnet face on the disc
ZMG0  = ZMG1 - EM_TM;             // magnet face on the airgap
ZSH1  = ZMG0 - EM_AG;             // shoe face on the airgap  <- THE AIRGAP
ZSH0  = ZSH1 - EM_TSH;            // shoe underside
ZTT0  = ZSH0 - EM_HT;             // tooth body root
ZBI1  = ZTT0 - EM_JG;             // stator back iron, top
ZBI0  = ZBI1 - EM_TBI;            // stator back iron, bottom

RDI   = RLGX + EM_JG;             // rotor disc bore
RDO   = EM_RAO + EM_RIM;          // rotor disc outer radius
RBI   = EM_RAI - EM_BIX;          // stator back iron inner radius
RBO   = EM_RAO + EM_BIX;          // stator back iron outer radius
RMNT  = RBO + EM_JG + EM_RMNT;    // stator mounting lug circle

RHO   = EM_RW + EM_LIN;           // tooth surface to coil centreline
ZCOIL = (ZBI1 + ZSH0)/2;          // coil bundle axis: centred in the slot
CLSLT = (ZSH0 - ZBI1)/2 - EM_RW;  // slot clearance above and below the bundle
RCIN  = EM_RAI - RHO - EM_RW;     // innermost radius the coil reaches
// The tightest clearance in the part: the shoe-to-body fillet leans into
// the slot exactly where the coil bundle is fattest.  Measured, not
// asserted -- the perpendicular distance from the tooth flank to the
// bundle surface is RHO - sqrt(rw^2 - (z-ZCOIL)^2) and the fillet face
// stands at rad(GFIL) r (1 + (z-ZSH0)/EM_HT), worst at r = EM_RAO; the
// minimum of the difference over the bundle profile is what counts.
CFIL = min([ for (i = [0:40])
               let( z = ZCOIL - EM_RW + 2*EM_RW*i/40,
                    xb = RHO - sqrt(max(0, EM_RW*EM_RW - (z-ZCOIL)*(z-ZCOIL))),
                    xf = rad(GFIL)*EM_RAO*(1 + (z-ZSH0)/EM_HT) )
                 xb - xf ]);

GMAG  = EM_PA*360/POLES;          // magnet arc, deg
GSLOT = 360/SLOTS;                // slot pitch, deg
GPOLE = 360/POLES;                // pole pitch, deg
RMEAN = (EM_RAI + EM_RAO)/2;
GFIL  = deg(EM_FIL/RMEAN);        // shoe-to-body fillet as an angle

// capacitor bank, (13) to (18)
EUSE  = CAPF*(VHI*VHI - VLO*VLO)/2;
EFULL = CAPF*VHI*VHI/2;
NCELL = ceil(VHI/EM_VCELL);
CCELL = NCELL*CAPF;
VBANK = EFULL/EM_EV;
VCAN  = VBANK/NCELL;
RCAN  = EM_DCAN/2;
HCAN  = VCAN/(PI*RCAN*RCAN);
ZCAN  = ZBI0 - EM_JG - RCAN;      // can axis: tucked under the back iron

// torque, (11)
TNM   = EM_SIG*2*PI*(pow(EM_RAO,3) - pow(EM_RAI,3))/3/1000;
WMECH = 2*PI*EM_NRPM/60;
PWATT = TNM*WMECH;
TBUF  = EUSE/PWATT;

// the star of slots, (5) to (8)
function em_wrap(a) = a - 360*floor(a/360);
function em_belt(a) = let(w = em_wrap(a))
                        (w >= 330 || w < 30) ? 1 : ((w >= 150 && w < 210) ? -1 : 0);
SLOTE = [ for (i = [0:SLOTS-1]) (POLES/2)*(360/SLOTS)*i ];
SLOTS_= [ for (i = [0:SLOTS-1]) em_belt(SLOTE[i]) ];
NBELT = sum([ for (i = [0:SLOTS-1]) SLOTS_[i]*SLOTS_[i] ]);
KDX   = sum([ for (i = [0:SLOTS-1]) SLOTS_[i]*cos(SLOTE[i]) ]);
KDY   = sum([ for (i = [0:SLOTS-1]) SLOTS_[i]*sin(SLOTE[i]) ]);
KD    = sqrt(KDX*KDX + KDY*KDY)/NBELT;
KP    = sin((POLES/2)*(360/SLOTS)/2);
KW    = KP*KD;

// the ripple orders, (9) and (10)
NCOG  = lcm_(SLOTS, POLES);
NRIP  = 6*(POLES/2);

// ---- the interface, published so a second collar can READ it -------
function em_if_seat()   = RSEAT;   // 1 clamp bore radius, mm
function em_if_width()  = EM_WC;   // 2 clamp axial width, mm
function em_if_wall()   = EM_TC;   // 3 clamp radial wall, mm
function em_if_tongue() = [EM_TW, EM_TD, GTNG];   // 4 width mm, depth mm, arc deg
function em_if_split()  = [EM_SPLIT, GSPL];       // 5 gap mm, gap deg
function em_if_bc()     = [RBC, EM_NLUG, EM_LUG0];// 6 bolt circle r, lugs, first
function em_if_bolt()   = [EM_BOLT, EM_RBORE, EM_RLUG];  // 7 M size, bore, boss
function em_if_pinch()  = [RBC, EM_EARL, EM_EARS];       // 8 pinch bolt seat

// ===================================================================
//  THE SWEPT SHELL.  One polyhedron over a station x profile grid.
//  See header section 5 for the winding rule; it is not repeated.
// ===================================================================
module em_shell(P, cap = false) {
    S = len(P); n = len(P[0]);
    pts = [ for (i = [0:S-1]) for (j = [0:n-1]) P[i][j] ];
    id  = function (i,j) (i % S)*n + (j % n);
    side = [ for (i = [0 : cap ? S-2 : S-1]) for (j = [0:n-1])
               [ id(i,j), id(i,j+1), id(i+1,j+1), id(i+1,j) ] ];
    c0 = cap ? [ for (j = [1:n-2]) [ id(0,0),   id(0,j+1),   id(0,j)   ] ] : [];
    c1 = cap ? [ for (j = [1:n-2]) [ id(S-1,0), id(S-1,j), id(S-1,j+1) ] ] : [];
    polyhedron(points = pts, faces = concat(side, c0, c1), convexity = 8);
}
// a rectangle in (r,z), counterclockwise
function em_rect(r0, r1, z0, z1) = [[r0,z0],[r1,z0],[r1,z1],[r0,z1]];
// place an (r,z) profile at azimuth a about the z axis
function em_at(prof, a) = [ for (p = prof) [p[0]*cos(a), p[0]*sin(a), p[1]] ];

// a full annulus: bore r0, outer r1, between z0 and z1.  With r0 > 0
// the bore is a real hole, which is how every bolt hole here is made.
module em_annulus(r0, r1, z0, z1, n) {
    em_shell([ for (i = [0:n-1]) em_at(em_rect(r0,r1,z0,z1), 360*i/n) ], false);
}
// an annular sector prism from azimuth a0 to a1, with end caps
module em_prism(r0, r1, z0, z1, a0, a1, n) {
    em_shell([ for (i = [0:n]) em_at(em_rect(r0,r1,z0,z1), a0 + (a1-a0)*i/n) ], true);
}

// ---- the coil bundle: a circular section swept along a closed path --
// Q is a closed planar path in z = zc traversed counterclockwise; the
// section is a circle of radius rw in the plane normal to the path.
// For a CLOSED PLANAR path the swept volume is exactly pi rw^2 L by
// Pappus, because the curvature correction integrates to zero, so the
// report can predict it without integrating anything.
function em_tube(Q, zc, rw, ns) =
  let( S = len(Q) )
  [ for (i = [0:S-1])
      let( p = Q[i], q = Q[(i+1)%S], o = Q[(i+S-1)%S],
           t = [q[0]-o[0], q[1]-o[1]],
           T = t/norm(t), N = [T[1], -T[0]] )   // N = T x zhat: outward for ccw
        [ for (j = [0:ns-1]) let (th = 360*j/ns)
            [ p[0] + rw*cos(th)*N[0], p[1] + rw*cos(th)*N[1], zc + rw*sin(th) ] ] ];

function em_pol(r, a) = [r*cos(a), r*sin(a)];
function em_arc(C, rho, th0, sweep, n) =
  [ for (i = [0:n-1]) let (t = th0 + sweep*i/n)
      [C[0] + rho*cos(t), C[1] + rho*sin(t)] ];
function em_lin(P, Q, n) =
  [ for (i = [0:n-1]) [P[0] + (Q[0]-P[0])*i/n, P[1] + (Q[1]-P[1])*i/n] ];

// The offset of the tooth footprint -- the annular sector r in [a,b],
// azimuth in [-ph,ph] -- at perpendicular distance rho.  Four straight
// or circular runs and four quarter turns of radius rho about the
// footprint's corners, which is the EXACT offset curve, so the path
// length is 2(b-a) + 2 pi rho + 2 rad(ph)(a+b) with nothing left over.
function em_coilpath(a, b, ph, rho, nl, nc, nr) =
  let( n1 = [-sin(ph), -cos(ph)], n3 = [-sin(ph), cos(ph)],
       Ca = em_pol(a,-ph), Cb = em_pol(b,-ph),
       Cc = em_pol(b, ph), Cd = em_pol(a, ph) )
    concat( em_lin([Ca[0]+rho*n1[0], Ca[1]+rho*n1[1]],
                   [Cb[0]+rho*n1[0], Cb[1]+rho*n1[1]], nl),
            em_arc(Cb, rho, -90-ph, 90, nc),
            [ for (i = [0:nr-1]) em_pol(b+rho, -ph + 2*ph*i/nr) ],
            em_arc(Cc, rho, ph, 90, nc),
            em_lin([Cc[0]+rho*n3[0], Cc[1]+rho*n3[1]],
                   [Cd[0]+rho*n3[0], Cd[1]+rho*n3[1]], nl),
            em_arc(Cd, rho, 90+ph, 90, nc),
            [ for (i = [0:nr-1]) em_pol(a-rho, ph - 2*ph*i/nr) ],
            em_arc(Ca, rho, 180-ph, 90, nc) );
LCOIL = 2*(EM_RAO-EM_RAI) + 2*PI*RHO + 2*rad(EM_WT/2)*(EM_RAI+EM_RAO);

// ===================================================================
//  THE BODIES
// ===================================================================

// One half of the split clamp.  The station list is [azimuth, bore
// radius], in five runs: bore, chamfer in, tongue crest, chamfer out,
// bore.  The chamfer is EM_TCH mm of arc, a real lead-in on the tongue
// flank -- and it is also what keeps the flank off a duplicated
// azimuth, which would put a zero-area quad in the shell (see the
// kernel finding beside this file).  Every run is generated half open
// so no two stations ever share an azimuth.
module em_clamp_half(a0, a1, ac) {
    at0 = ac - GTNG/2;  at1 = ac + GTNG/2;
    b0 = at0 - GTCH;    b1 = at1 + GTCH;
    n1 = max(2, ceil((b0 - a0)/NAZ));
    nc = 3;
    n3 = max(2, ceil(GTNG/NAZ));
    n5 = max(2, ceil((a1 - b1)/NAZ));
    S = concat( [ for (i = [0:n1-1]) [a0  + (b0-a0 )*i/n1, RSEAT] ],
                [ for (i = [0:nc-1]) [b0  + GTCH*i/nc,
                                      RSEAT + (RTNG-RSEAT)*i/nc] ],
                [ for (i = [0:n3-1]) [at0 + GTNG*i/n3, RTNG] ],
                [ for (i = [0:nc-1]) [at1 + GTCH*i/nc,
                                      RTNG + (RSEAT-RTNG)*i/nc] ],
                [ for (i = [0:n5  ]) [b1  + (a1-b1)*i/n5, RSEAT] ] );
    em_shell([ for (s = S) em_at(em_rect(s[1], RCO, ZC0, ZC1), s[0]) ], true);
}

// An axial bolt lug: a tube on the bolt circle whose bore IS the hole.
module em_lug(psi, z0, z1) {
    translate([RBC*cos(psi), RBC*sin(psi), 0])
      em_annulus(EM_RBORE, EM_RLUG, z0, z1, NTUB);
}

// A tangential pinch ear at a parting plane.  sgn picks which half it
// belongs to; the bolt axis is tangential at the bolt circle radius.
module em_ear(psi, sgn) {
    rotate([0,0,psi]) translate([RBC,0,0]) rotate([-90,0,0])
      em_annulus(EM_RBORE, EM_RLUG,
                 sgn > 0 ? EM_EARS : -EM_EARS - EM_EARL,
                 sgn > 0 ? EM_EARS + EM_EARL : -EM_EARS, NTUB);
}

// The rotor: a back iron disc carried by the clamp, and 28 magnets on
// its airgap face, each an arc of EM_PA of the pole pitch.
module em_rotor_disc() { em_annulus(RDI, RDO, ZROT0, ZROT1, ceil(360/NAZ)); }
module em_magnet(j) {
    c = GPOLE*j;
    em_prism(EM_RAI, EM_RAO, ZMG0, ZMG1, c - GMAG/2, c + GMAG/2,
             max(3, ceil(GMAG/NAZ)));
}

// The stator: a back iron ring, and 24 teeth each with a shoe.  The
// station list is [azimuth, root z]: the shoe runs the full width at
// the shoe's own root, the body only over the narrower window, and the
// two duplicated azimuths between them are the underside of the shoe.
module em_backiron() { em_annulus(RBI, RBO, ZBI0, ZBI1, ceil(360/NAZ)); }
// The station list is [azimuth, root z] in five runs: shoe alone, the
// fillet where the root drops from the shoe underside to the tooth
// root, the tooth body, the fillet back up, shoe alone.  The fillet is
// EM_FIL mm of arc at the mean radius and sits OUTSIDE the body window,
// so the body keeps its full width EM_WT at the root; it is a real
// lamination fillet and it also keeps the shell free of the zero-area
// quads a duplicated azimuth would give.
module em_tooth(i) {
    w0 = EM_WT/2 + GFIL;
    n1 = max(2, ceil((EM_WSH/2 - w0)/NAZ*4));
    nf = 3;
    n3 = max(4, ceil(EM_WT/NAZ*2));
    S = concat( [ for (j = [0:n1-1]) [-EM_WSH/2 + (EM_WSH/2-w0)*j/n1, ZSH0] ],
                [ for (j = [0:nf-1]) [-w0 + GFIL*j/nf,
                                      ZSH0 + (ZTT0-ZSH0)*j/nf] ],
                [ for (j = [0:n3-1]) [-EM_WT/2 + EM_WT*j/n3, ZTT0] ],
                [ for (j = [0:nf-1]) [ EM_WT/2 + GFIL*j/nf,
                                      ZTT0 + (ZSH0-ZTT0)*j/nf] ],
                [ for (j = [0:n1  ]) [ w0 + (EM_WSH/2-w0)*j/n1, ZSH0] ] );
    rotate([0,0,GSLOT*i])
      em_shell([ for (s = S) em_at(em_rect(EM_RAI, EM_RAO, s[1], ZSH1), s[0]) ],
               true);
}
// The coil on tooth i: a bundle wound round the tooth body, its
// centreline the exact offset of the tooth footprint at distance RHO.
module em_coil(i) {
    rotate([0,0,GSLOT*i])
      em_shell(em_tube(em_coilpath(EM_RAI, EM_RAO, EM_WT/2, RHO, NPL, NPC, NPR),
                       ZCOIL, EM_RW, NSEC), false);
}
// Three lugs that ground the stator to the slice housing, which is not
// in this file.
module em_mount(j) {
    psi = 360*j/EM_NMNT;
    translate([RMNT*cos(psi), RMNT*sin(psi), 0])
      em_annulus(EM_RMB, EM_RMNT, ZBI0, ZBI1, NTUB);
}
// A supercapacitor can, lying radially under the stator back iron, its
// length computed from (18) and not chosen.
module em_can(j) {
    rotate([0,0,360*j/NCELL]) translate([EM_RCAN0, 0, ZCAN]) rotate([0,90,0])
      cylinder(h = HCAN, r = RCAN, $fn = 32);
}

// ===================================================================
//  REPORT
// ===================================================================
echo("=== collar_em: the regenerative EM collar ===");
echo("--- read from the contract, not restated ---");
echo(str("   sg_m() = ", M, " mm    sg_r() = ", RSPH, " mm    sg_ns() = ", NSUN));
echo(str("   sg_em() = [", SLOTS, ", ", POLES, "]  slots, poles"));
echo(str("   sg_cap() = [", CAPF, " F, ", VLO, " V, ", VHI, " V]"));
echo(str("   sg_row(", len(ROWS)-1, ") = the equator row, ", ROWEQ[0],
         ": Nr = ", ROWEQ[1], " D = ", ROWEQ[2], " Np = ", ROWEQ[3],
         " k = ", ROWEQ[4], " -- the band IS this row's ring"));

echo("--- 1. the split clamp interface, DEFINED here, published as em_if_*() ---");
echo(str("   1 seat bore      R_seat = R + off = ", RSPH, " + ", EM_OFF, " = ",
         RSEAT, " mm radius, diameter ", 2*RSEAT, " mm   em_if_seat()"));
echo(str("   2 clamp width    ", EM_WC, " mm, z from ", ZC0, " to ", ZC1,
         "   em_if_width()"));
echo(str("   3 clamp wall     ", EM_TC, " mm, so the clamp outer radius is ",
         RCO, " mm   em_if_wall()"));
echo(str("   4 tongue         ", EM_TW, " mm wide and ", EM_TD,
         " mm deep, crest radius ", RTNG, " mm; as an angle deg(", EM_TW, "/",
         RSEAT, ") = ", GTNG, " deg, with a ", EM_TCH,
         " mm = ", GTCH, " deg chamfer on each flank; one per half, at",
         " azimuth 90 and 270   em_if_tongue()"));
echo(str("   5 parting gap    ", EM_SPLIT, " mm of arc at the seat = deg(",
         EM_SPLIT, "/", RSEAT, ") = ", GSPL,
         " deg, so each half spans 180 - ", GSPL, " = ", 180-GSPL,
         " deg (2); parting planes at azimuth 0 and 180   em_if_split()"));
echo(str("   6 bolt circle    radius ", RBC, " mm = ", RCO, " + ", EM_JG, " + ",
         EM_RLUG, ", diameter ", 2*RBC, " mm; ", EM_NLUG, " lugs at ",
         360/EM_NLUG, " deg from azimuth ", EM_LUG0,
         ", 3 per half   em_if_bc()"));
echo(str("   7 bolt           M", EM_BOLT, ", bore radius ", EM_RBORE,
         " mm, boss radius ", EM_RLUG, " mm, wall ", EM_RLUG-EM_RBORE,
         " mm; grip ", EM_WC, " + ", EM_JG, " + ", EM_TTAB, " = ",
         EM_WC+EM_JG+EM_TTAB, " mm   em_if_bolt()"));
echo(str("   8 pinch bolts    M", EM_BOLT, " tangential at radius ", RBC,
         " mm and z = 0, one per parting plane, ears ", EM_EARL,
         " mm long standing ", EM_EARS, " mm off the plane   em_if_pinch()"));
echo("   A collar that repeats 1 to 8 interchanges with this one.  THE CHAIN");
echo("   COLLAR IS NOT IN THIS DIRECTORY as this file is written, so that");
echo("   interchange is a published contract here and NOT a checked one:");
echo("   nothing in this file compares itself against another collar.");
echo(str("   The contract does not publish the band's outer diameter as a",
         " function -- only R = ", RSPH, " -- so the ", EM_OFF,
         " mm offset is a collar-side choice, made to stand outboard of the",
         " band's toothed outer face as the contract's own equatorial-band",
         " report prints it.  This file does not restate that radius."));

echo("--- 2. the winding arithmetic ---");
echo(str("   (3) q = S/(3P) = ", SLOTS, "/", 3*POLES, " = ",
         SLOTS/gcd_(SLOTS,3*POLES), "/", 3*POLES/gcd_(SLOTS,3*POLES),
         "  reduced by gcd = ", gcd_(SLOTS,3*POLES), ", value ", SLOTS/(3*POLES)));
echo(str("   (4) balance gate S/(3 gcd(S,P)) = ", SLOTS, "/(3*",
         gcd_(SLOTS,POLES), ") = ", SLOTS/(3*gcd_(SLOTS,POLES)),
         "   integer: ", SLOTS % (3*gcd_(SLOTS,POLES)) == 0));
echo(str("   coils = S = ", SLOTS, ", per phase S/3 = ", SLOTS/3,
         "; the belt [-30,30) with [150,210) reversed selects ", NBELT,
         " slots, and S/3 = ", SLOTS/3, "   equal: ", NBELT == SLOTS/3));
echo(str("   (5) slot pitch in electrical degrees (P/2)(360/S) = ",
         (POLES/2)*(360/SLOTS)));
echo(str("   (6) kd = |", KDX, ", ", KDY, "|/", NBELT, " = ", KD,
         "   (7) kp = sin(", (POLES/2)*(360/SLOTS)/2, ") = ", KP,
         "   (8) kw = kp kd = ", KW));
echo("   that kw = 0.933 is the familiar figure for a 24/28 concentrated");
echo("   winding is a MEMORY, flagged as one; the value above is (8) evaluated.");
echo(str("   (9)  cogging order  lcm(S,P) = lcm(", SLOTS, ",", POLES, ") = ",
         NCOG, " = ", SLOTS, "*", NCOG/SLOTS, " = ", POLES, "*", NCOG/POLES,
         " events per mechanical revolution"));
echo(str("   (10) torque ripple order 6*(P/2) = ", NRIP,
         "   gcd(", NCOG, ",", NRIP, ") = ", gcd_(NCOG,NRIP),
         " = ", NRIP, ", so the cogging order is exactly ", NCOG/NRIP,
         " times the ripple order: the two are NOT independent"));
echo("   coincidence with the gear mesh orders.  Two excitations of orders p");
echo("   and q add to a pattern of period 360/gcd(p,q) degrees, so it repeats");
echo("   gcd(p,q) times per revolution, and |i p - j q| reaches multiples of");
echo("   gcd(p,q) and nothing smaller:");
for (r = ROWS)
  echo(str("      ", r[0], "  N = ", r[1], "   gcd(", NCOG, ",", r[1], ") = ",
           gcd_(NCOG, r[1]), "   lcm = ", lcm_(NCOG, r[1]),
           "   beat |", NCOG, "-", r[1], "| = ", abs(NCOG - r[1]),
           gcd_(NCOG, r[1]) == 1
             ? "   SHARES NO FACTOR: repeats once per revolution"
             : str("   shares ", gcd_(NCOG, r[1]), ": repeats ",
                   gcd_(NCOG, r[1]), " times per revolution")));
echo(str("      sun      N = ", NSUN, "   gcd(", NCOG, ",", NSUN, ") = ",
         gcd_(NCOG, NSUN), "   lcm = ", lcm_(NCOG, NSUN),
         "   beat |", NCOG, "-", NSUN, "| = ", abs(NCOG - NSUN),
         "   shares ", gcd_(NCOG, NSUN), ": repeats ", gcd_(NCOG, NSUN),
         " times per revolution"));
echo(str("      ", NCOG, " = 2^3*3*7 = ", 8*3*7, ", so a mesh order shares a",
         " factor with the cogging exactly when it is even or divisible by 3",
         " or 7.  Rows 1 and 2 (", ROWS[0][1], " prime, ", ROWS[1][1], " = 11*13)",
         " share nothing; row 3 (", ROWS[2][1], " = 4*47) shares 4; the equator",
         " row (", ROWS[3][1], " = 2*11^2) and the sun (", NSUN,
         " = 2*23) share 2."));
echo(str("   electrical frequency f = (P/2) rpm/60, cogging f = ", NCOG,
         " rpm/60:"));
for (n = EM_RPM)
  echo(str("      ", n, " rpm   f_elec = ", (POLES/2)*n/60, " Hz",
           "   f_cog = ", NCOG*n/60, " Hz   f_ripple = ", NRIP*n/60, " Hz"));

echo("--- 3. the magnetic geometry ---");
echo(str("   active annulus r = ", EM_RAI, " to ", EM_RAO, " mm, radial length ",
         EM_RAO-EM_RAI, " mm, mean radius ", RMEAN, " mm"));
echo(str("   THE AIRGAP is ", EM_AG, " mm: shoe face z = ", ZSH1,
         ", magnet face z = ", ZMG0, ", ", ZMG0, " - ", ZSH1, " = ", ZMG0-ZSH1));
echo(str("   airgap annulus area pi(Rao^2-Rai^2) = ",
         PI*(EM_RAO*EM_RAO - EM_RAI*EM_RAI), " mm^2, of which the magnets cover ",
         EM_PA, " = ", EM_PA*PI*(EM_RAO*EM_RAO - EM_RAI*EM_RAI), " mm^2"));
echo(str("   pole pitch 360/", POLES, " = ", GPOLE, " deg = ",
         rad(GPOLE)*RMEAN, " mm at the mean radius; magnet arc ", EM_PA,
         " of it = ", GMAG, " deg = ", rad(GMAG)*RMEAN,
         " mm, gap between magnets ", rad(GPOLE-GMAG)*RMEAN, " mm"));
echo(str("   slot pitch 360/", SLOTS, " = ", GSLOT, " deg = ", rad(GSLOT)*RMEAN,
         " mm at the mean radius; tooth body ", EM_WT, " deg = ",
         rad(EM_WT)*RMEAN, " mm, shoe ", EM_WSH, " deg = ", rad(EM_WSH)*RMEAN,
         " mm, slot opening ", GSLOT, " - ", EM_WSH, " = ", GSLOT-EM_WSH,
         " deg = ", rad(GSLOT-EM_WSH)*RMEAN, " mm"));
echo(str("   shoe-to-body fillet ", EM_FIL, " mm of arc at the mean radius = ",
         GFIL, " deg, sitting outside the body window so the body keeps ",
         EM_WT, " deg at the root"));
echo(str("   slot depth (tooth body height) ", EM_HT,
         " mm holds a bundle of diameter ", 2*EM_RW, " mm with ", CLSLT,
         " mm above and below; bundle centreline offset from the tooth ",
         RHO, " mm, so the liner clearance is ", RHO, " - ", EM_RW, " = ",
         RHO-EM_RW, " mm"));
echo(str("   coil path length 2(Rao-Rai) + 2 pi rho + 2 rad(", EM_WT/2,
         ")(Rai+Rao) = ", 2*(EM_RAO-EM_RAI), " + ", 2*PI*RHO, " + ",
         2*rad(EM_WT/2)*(EM_RAI+EM_RAO), " = ", LCOIL, " mm per coil, ",
         24*LCOIL/1000, " m of bundle over ", SLOTS, " coils"));
echo(str("   (11) T = sigma 2 pi (Rao^3 - Rai^3)/3 = ", EM_SIG, "*2pi*(",
         pow(EM_RAO,3), " - ", pow(EM_RAI,3), ")/3 = ", TNM*1000,
         " Nmm = ", TNM, " Nm.  sigma = ", EM_SIG*1000,
         " kPa is an ASSUMPTION, the low end of the 10-30 kPa band quoted for",
         " air cooled machines, from memory of machine design practice and",
         " NOT computed here; every figure below inherits it."));
echo(str("   at ", EM_NRPM, " rpm: w = 2pi*", EM_NRPM, "/60 = ", WMECH,
         " rad/s, P = T w = ", PWATT, " W = ", PWATT/1000, " kW"));
echo(str("   the equator row carries it through k = ", ROWEQ[4],
         " planet meshes, so ", TNM, "/", ROWEQ[4], " = ", TNM/ROWEQ[4],
         " Nm per mesh.  The MECHANICAL ceiling on this port is the contract's",
         " gate 7 and is printed there; this file does not restate it."));
echo(str("   (12) Willis at the equator row: (w_s-w_c)/(w_r-w_c) = -Nr/Ns = -",
         ROWEQ[1], "/", NSUN, " = -", ROWEQ[1]/gcd_(ROWEQ[1],NSUN), "/",
         NSUN/gcd_(ROWEQ[1],NSUN), " exactly, reduced by gcd = ",
         gcd_(ROWEQ[1],NSUN), ".  The collar drives w_r; assist and regen are",
         " the two signs of the same term."));

echo("--- 4. the capacitor bank ---");
echo(str("   (13) E_use = C(Vhi^2-Vlo^2)/2 = ", CAPF, "*(", VHI*VHI, "-",
         VLO*VLO, ")/2 = ", EUSE,
         " J, the same figure the contract prints under its gate 5, because",
         " both evaluate sg_cap()"));
echo(str("   (14) E_full = C Vhi^2/2 = ", CAPF, "*", VHI*VHI, "/2 = ", EFULL,
         " J, so the usable fraction over a 2:1 rail band is ", EUSE, "/",
         EFULL, " = ", EUSE/EFULL));
echo(str("   (15) n = ceil(Vhi/Vcell) = ceil(", VHI, "/", EM_VCELL, ") = ",
         NCELL, " cells in series, rated ", NCELL*EM_VCELL, " V >= ", VHI,
         " V: ", NCELL*EM_VCELL >= VHI));
echo(str("   (16) C_cell = n C = ", NCELL, "*", CAPF, " = ", CCELL,
         " F per cell, and n in series give C = C_cell/n = ", CCELL/NCELL,
         " F = ", CAPF, " F: ", CCELL/NCELL == CAPF));
echo(str("   (17) e_v = ", EM_EV, " J/mm^3 = ", EM_EV*1e6/3600,
         " Wh/L, an ASSUMPTION.  Where it comes from: a 2.7 V 3000 F cell in a",
         " 60 x 138 mm can holds 0.5*3000*2.7^2 = ", 0.5*3000*2.7*2.7,
         " J in pi*30^2*138 = ", PI*900*138, " mm^3, which is ",
         0.5*3000*2.7*2.7/(PI*900*138), " J/mm^3 = ",
         0.5*3000*2.7*2.7/(PI*900*138)*1e6/3600,
         " Wh/L.  Both that cell and the derating to ", EM_EV*1e6/3600,
         " Wh/L are recalled catalogue practice, not measurements."));
echo(str("   (18) V_bank = E_full/e_v = ", EFULL, "/", EM_EV, " = ", VBANK,
         " mm^3 = ", VBANK/1000, " cm^3, over ", NCELL, " cans = ", VCAN,
         " mm^3 each; at diameter ", EM_DCAN, " mm that is a can ", HCAN,
         " mm long, which is the can this file builds: pi*", RCAN, "^2*", HCAN,
         " = ", PI*RCAN*RCAN*HCAN, " mm^3"));
echo(str("   the cans lie radially from r = ", EM_RCAN0, " to ",
         EM_RCAN0+HCAN, " mm on axis z = ", ZCAN,
         ", tucked under the stator back iron"));
echo(str("   buffer: E_use/P at ", EM_NRPM, " rpm = ", EUSE, "/", PWATT, " = ",
         TBUF, " s of braking at full assumed shear, and the current at the",
         " rails is P/V = ", PWATT/VHI, " A at ", VHI, " V and ", PWATT/VLO,
         " A at ", VLO, " V"));

echo("--- 5. clearances: nothing overlaps, every gap is a subtraction ---");
echo(str("    1 clamp group outer ", RLGX, " to innermost coil reach ", RCIN,
         ": ", RCIN, " - ", RLGX, " = ", RCIN-RLGX, " mm"));
echo(str("    2 tab outer ", RLGX, " to rotor disc bore ", RDI, ": ", RDI-RLGX,
         " mm"));
echo(str("    3 magnet top ", ZMG1, " to rotor disc underside ", ZROT0, ": ",
         ZROT0-ZMG1, " mm"));
echo(str("    4 shoe face ", ZSH1, " to magnet face ", ZMG0, ": ", ZMG0-ZSH1,
         " mm  == the airgap"));
echo(str("    5 coil bundle to back iron top and to shoe underside: ", CLSLT,
         " mm each"));
echo(str("    6 coil bundle to tooth flank: ", RHO-EM_RW, " mm"));
echo(str("    7 tooth root ", ZTT0, " to back iron top ", ZBI1, ": ", ZTT0-ZBI1,
         " mm"));
echo(str("    8 back iron bottom ", ZBI0, " to can top ", ZCAN+RCAN, ": ",
         ZBI0-(ZCAN+RCAN), " mm"));
echo(str("    9 bolt boss inner ", RBC-EM_RLUG, " to clamp outer ", RCO, ": ",
         RBC-EM_RLUG-RCO, " mm"));
echo(str("   10 mount boss inner ", RMNT-EM_RMNT, " to back iron outer ", RBO,
         ": ", RMNT-EM_RMNT-RBO, " mm"));
echo(str("   11 pinch ear to its parting plane: ", EM_EARS,
         " mm, and the two halves part by ", EM_SPLIT, " mm of arc"));
echo(str("   13 shoe-to-body fillet to coil bundle, the tightest clearance in",
         " the part, minimised over the bundle profile at r = ", EM_RAO, ": ",
         CFIL, " mm  (fillet ", EM_FIL, " mm of arc at r = ", RMEAN, " = ",
         GFIL, " deg, so ", rad(GFIL)*EM_RAO,
         " mm of lean at the shoe underside against a liner clearance of ",
         EM_LIN, " mm)"));
echo(str("   12 coil to neighbouring coil at the inner end: 2*", EM_RAI,
         "*sin((", GSLOT, "-", EM_WT, ")/2) - 2*(", RHO, "+", EM_RW, ") = ",
         2*EM_RAI*sin((GSLOT-EM_WT)/2), " - ", 2*(RHO+EM_RW), " = ",
         2*EM_RAI*sin((GSLOT-EM_WT)/2) - 2*(RHO+EM_RW), " mm"));
echo(str("   envelope: radius ", RMNT+EM_RMNT, " mm (diameter ",
         2*(RMNT+EM_RMNT), "), z from ", ZCAN-RCAN, " to ", ZROT1, " = ",
         ZROT1-(ZCAN-RCAN), " mm of stack"));

echo("--- 6. predicted volumes; the mesh must come in UNDER, because every");
echo("       chord cuts inside its arc and every bundle section is a polygon ---");
// clamp half = the plain ring sector, plus the tongue crest, plus the two
// flank chamfers: for a bore ramping linearly from RSEAT to RTNG over
// rad(GTCH), the extra area integrates to rad(GTCH)(RSEAT td - td^2/3)/1
// over the pair, exactly.
VHALF = rad(180-GSPL)/2*(RCO*RCO - RSEAT*RSEAT)*EM_WC
      + rad(GTNG)/2*(RSEAT*RSEAT - RTNG*RTNG)*EM_WC
      + rad(GTCH)*(RSEAT*EM_TD - EM_TD*EM_TD/3)*EM_WC;
VTUB  = function (ri, ro, L) PI*(ro*ro - ri*ri)*L;
VLUG  = VTUB(EM_RBORE, EM_RLUG, EM_WC);
VTAB  = VTUB(EM_RBORE, EM_RLUG, EM_TTAB);
VEAR  = VTUB(EM_RBORE, EM_RLUG, EM_EARL);
VDISC = VTUB(RDI, RDO, EM_TROT);
VMAG  = rad(GMAG)/2*(EM_RAO*EM_RAO - EM_RAI*EM_RAI)*EM_TM;
VBI   = VTUB(RBI, RBO, EM_TBI);
// tooth = body + shoe + the pair of fillets, whose extra height falls
// linearly from EM_HT to 0 across rad(GFIL), so the pair contributes
// rad(GFIL) EM_HT (Rao^2-Rai^2)/2 exactly.
VTOOTH= rad(EM_WT)/2*(EM_RAO*EM_RAO - EM_RAI*EM_RAI)*EM_HT
      + rad(EM_WSH)/2*(EM_RAO*EM_RAO - EM_RAI*EM_RAI)*EM_TSH
      + rad(GFIL)*EM_HT*(EM_RAO*EM_RAO - EM_RAI*EM_RAI)/2;
VCOIL = PI*EM_RW*EM_RW*LCOIL;
VCOILP= NSEC/2*sin(360/NSEC)*EM_RW*EM_RW*LCOIL;
VMNT  = VTUB(EM_RMB, EM_RMNT, EM_TBI);
VCANM = PI*RCAN*RCAN*HCAN;
NBODY = 2 + EM_NLUG + EM_NLUG + 4 + 1 + POLES + 1 + SLOTS + SLOTS + EM_NMNT + NCELL;
VTOT  = 2*VHALF + EM_NLUG*VLUG + EM_NLUG*VTAB + 4*VEAR + VDISC + POLES*VMAG
      + VBI + SLOTS*VTOOTH + SLOTS*VCOIL + EM_NMNT*VMNT + NCELL*VCANM;
echo(str("   clamp half      2 x ", VHALF, " = ", 2*VHALF, " mm^3"));
echo(str("   bolt lug        ", EM_NLUG, " x ", VLUG, " = ", EM_NLUG*VLUG));
echo(str("   collar tab      ", EM_NLUG, " x ", VTAB, " = ", EM_NLUG*VTAB));
echo(str("   pinch ear       4 x ", VEAR, " = ", 4*VEAR));
echo(str("   rotor disc      1 x ", VDISC, " = ", VDISC));
echo(str("   magnet          ", POLES, " x ", VMAG, " = ", POLES*VMAG));
echo(str("   stator back iron 1 x ", VBI, " = ", VBI));
echo(str("   tooth with shoe ", SLOTS, " x ", VTOOTH, " = ", SLOTS*VTOOTH));
echo(str("   coil bundle     ", SLOTS, " x ", VCOIL, " = ", SLOTS*VCOIL,
         "  (pi rw^2 L by Pappus; with the ", NSEC,
         "-gon section actually meshed it is ", SLOTS*VCOILP, ")"));
echo(str("   stator mount    ", EM_NMNT, " x ", VMNT, " = ", EM_NMNT*VMNT));
echo(str("   capacitor can   ", NCELL, " x ", VCANM, " = ", NCELL*VCANM));
echo(str("   predicted total = ", VTOT, " mm^3; with the polygon-section coils ",
         VTOT - SLOTS*(VCOIL-VCOILP)));
echo(str("   bodies = 2 halves + ", EM_NLUG, " lugs + ", EM_NLUG, " tabs + 4 ears",
         " + 1 rotor disc + ", POLES, " magnets + 1 back iron + ", SLOTS,
         " teeth + ", SLOTS, " coils + ", EM_NMNT, " mounts + ", NCELL,
         " cans = ", NBODY, ", and none of them overlaps, so validate.py must",
         " report ", NBODY, " components and a volume just under the total"));
echo("   not modelled: the bolts, the phase leads and terminals, the bus to");
echo("   the cans, the stator's ground path past its three lugs, the housing.");

// ===================================================================
//  ASSEMBLY
// ===================================================================
CLAMPC = [0.62,0.64,0.68];   // clamp group
ROTC   = [0.42,0.46,0.52];   // rotor back iron
MAGC   = [0.76,0.34,0.36];   // magnets
STATC  = [0.50,0.54,0.60];   // stator iron
COILC  = [0.80,0.52,0.24];   // copper
CANC   = [0.30,0.44,0.56];   // capacitor cans

color(CLAMPC) {
    em_clamp_half(GSPL/2, 180 - GSPL/2, 90);
    em_clamp_half(180 + GSPL/2, 360 - GSPL/2, 270);
    for (i = [0:EM_NLUG-1]) em_lug(EM_LUG0 + 360*i/EM_NLUG, ZC0, ZC1);
    for (s = [0, 180]) { em_ear(s, 1); em_ear(s, -1); }
}
color(ROTC) {
    em_rotor_disc();
    for (i = [0:EM_NLUG-1]) em_lug(EM_LUG0 + 360*i/EM_NLUG, ZTB0, ZTB1);
}
color(MAGC)  for (j = [0:POLES-1]) em_magnet(j);
color(STATC) { em_backiron();
               for (i = [0:SLOTS-1]) em_tooth(i);
               for (j = [0:EM_NMNT-1]) em_mount(j); }
color(COILC) for (i = [0:SLOTS-1]) em_coil(i);
color(CANC)  for (j = [0:NCELL-1]) em_can(j);
