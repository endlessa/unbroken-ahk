// ===================================================================
//  equator_band.scad -- the equatorial band.  ONE rigid ring at ONE
//  angular velocity, toothed on both faces with two different tooth
//  systems, and grooved on its outer cylinder for a swappable collar.
//
//  Built on spherical_gear.scad, which is the contract.  Every number
//  that belongs to the design table is READ from that file through
//  `use`; nothing from the table is restated here.  Four numbers come
//  from outside it and are named as such at the head of the file: the
//  port ratio and the pinion count (from which the outer tooth count
//  is a product, not a written-down integer), and the two Lewis
//  inputs sigma and Y.  The library echoes an outer count and a pinion
//  count of its own; both are quoted at the bottom of the file ONLY so
//  that the report can print the difference against this file's
//  product, which is the drift check.
//
//  ---------------------------------------------------------------
//  0.  WHAT THE BAND IS
//
//  The equator row is the fundamental slice: its ring pitch circle is
//  a great circle of the bevel sphere, so gamma_r = 90, its cone
//  distance L equals the sphere radius R, and its apex sits on the
//  sphere centre.  The band is that row's ring.  It carries
//
//    INSIDE   the crown ring, Nr spherical-involute teeth on a pitch
//             cone of half angle 90 -- the equatorial PLANE.  It meshes
//             the row's planets, and through them the common sun.
//    OUTSIDE  an ordinary planar involute spur ring on a cylinder about
//             the polar axis, taking a pinion on a PARALLEL axis.  That
//             is the second input, the one that comes in at the
//             equator.
//    BETWEEN  19 mm of radial wall, which is the whole structural job
//             of the part: it ties the two tooth systems into one rigid
//             member so that they cannot move relative to each other.
//    BELOW    a register: a groove between two raised lands on the
//             band's outer face, at the sphere's own equatorial radius
//             sg_r(), which is where a swappable collar clamps.
//
//  Three ports, one body, one angular velocity.  The crown mesh and
//  the spur mesh therefore stand in a fixed integer relation, and the
//  collar is a third path onto the same member.
//
//  ---------------------------------------------------------------
//  1.  WHY gamma = 90 IS EXACT, AND WHAT MAKES IT EXACT
//
//  From the contract's (7), sin(gamma) = N/D for every member of a
//  row, so the equator row's ring has
//
//      sin(gamma_r) = Nr / D = 242/242 = 1,   gamma_r = 90.         (1)
//
//  That is not a rounding.  It holds because the row was SELECTED with
//  Nr = D, and the price of that selection is the contract's closure
//  condition (8), gamma_r = gamma_s + 2 gamma_p.  Put gamma_r = 90 in
//  (8) and the condition becomes a Diophantine one:
//
//      2 gamma_p = 90 - gamma_s
//   => cos(2 gamma_p) = cos(90 - gamma_s) = sin(gamma_s)
//   => 1 - 2 sin^2(gamma_p) = sin(gamma_s)
//   => 1 - 2 Np^2/D^2 = Ns/D
//   => 2 Np^2 = D (D - Ns).                                        (2)
//
//  (2) is an identity between integers with no square root in it, and
//  the table's equator row satisfies it: 2*154^2 = 47432 = 242*196.
//  The file prints both sides, and prints the contract's own general
//  closure (9) for the same row beside it, so the two agree.
//
//  With gamma_r = 90 the base cone follows from the contract's (1):
//
//      sin(gamma_b) = sin(90) cos(phi) = cos(phi)
//   => gamma_b = 90 - phi   exactly.                               (3)
//
//  At phi = 25 that is 65 degrees, and the file prints sg_gb(90,phi)
//  against 90 - phi.
//
//  ---------------------------------------------------------------
//  2.  THE SPEC'S EQUATOR DEGENERACY IS FALSE.  THE REAL BRIDGE.
//
//  The specification this machine came from says the pitch cone
//  "degenerates to a cylinder" at the equator, so that the equatorial
//  teeth may be cut as ordinary planar involutes.  It does not.  A
//  pitch cone of half angle 90 is the equatorial PLANE; a gear with a
//  plane pitch surface is a CROWN gear; and the exact conjugate of a
//  crown gear is still a spherical involute.  The library prints the
//  disproof and this file prints it again, because the equator is the
//  one place in the machine where somebody would act on it:
//
//      INVS(90; 65) = 9.30401 deg      the spherical involute function
//      inv(25)      = 1.71746 deg      the planar one
//      ratio        = 5.417
//
//  A factor of 5.4 is not a degeneracy.  The crown's flank half width
//  is ht +/- (INVS(Gamma) - INVS(90)), and the file prints how much
//  azimuth that sweeps across the working depth, against the tooth
//  pitch 360/Nr, so the size of the thing being got wrong is on the
//  record next to the ratio.
//
//  What DOES survive, intact, at every latitude including this one, is
//  the RATIO of base circle to pitch circle.  Divide the contract's (1)
//  through by sin(gamma):
//
//      sin(gamma_b)/sin(gamma) = cos(phi)                          (4)
//
//  and the left side is r_b/r_p, because a cone of half angle gamma
//  meets the sphere of radius L in a circle of radius L sin(gamma).
//  So r_b/r_p = cos(phi) on the sphere exactly as in the plane.  The
//  file prints (4) at gamma = 10, 30, 60, 90 against cos(phi), and
//  then prints r_b/r_p for the OUTER spur ring, which is the same
//  number to the last digit.  THAT is the bridge between the band's
//  two tooth systems: not the involute function, which differs by
//  5.4x, but the base-to-pitch ratio, which is identical.
//
//  The limit that does hold is gamma_b -> 0, the apex receding to
//  infinity at fixed base radius, which is the cylinder -- and the
//  outer ring is exactly that case, honestly arrived at: its mesh is
//  parallel-axis, its pitch surface IS a cylinder, its apex IS at
//  infinity.  It is a planar involute because it is planar, not
//  because a spherical one degenerated into it.
//
//  ---------------------------------------------------------------
//  3.  THE OUTER PORT
//
//  Two integers are declared: the port ratio and the pinion count.
//  The band's outer count is their product, so the ratio is exact by
//  construction and not by rounding a diameter:
//
//      Nout = ratio * Npin = 10 * 28 = 280                         (5)
//      r_p  = Nout m / 2 = 140 mm
//      centre distance = (Nout + Npin) m / 2 = 154 mm
//
//  The pinion's tooth count is above the undercut limit 2/sin^2(phi)
//  = 11.21 at phi = 25, which is printed.  The flank is the planar
//  involute of the base circle r_b = r_p cos(phi), built with the
//  library's sg_ptp / sg_invp branch of the same function family: the
//  half tooth width at radius r is
//
//      h(r) = ht + inv(phi) - inv(alpha_r),   cos(alpha_r) = r_b/r  (6)
//
//  with ht the library's own half tooth angle sg_ht(N, jt m, m), so
//  the outer mesh carries the same circumferential backlash per mesh
//  as every mesh in the machine, which the file prints as
//  pi m - 2 t = jt m at the pitch circle.
//
//  The flank is not built from (6).  It is built from the library's
//  sg_ptp, the planar involute POINT of base radius r_b at roll
//  parameter th, taken at th = deg(tan(alpha_r)) and swung onto the
//  tooth centre line; (6) is then the CHECK, and the file prints the
//  largest disagreement between the two over a whole flank, in mm and
//  in degrees.  Every flank sample therefore lies ON the involute and
//  the only faceting is the chord between samples, whose deviation
//  from the curve the file also measures and prints.
//
//  The addendum and dedendum are not written down either.  The
//  contract gives them as ANGLES, sg_ta(D) = atan(2/D) and
//  sg_tf(D) = atan(2.5/D), measured at the cone distance L = D m/2, so
//
//      a = L tan(sg_ta(D)) = m,   b = L tan(sg_tf(D)) = 1.25 m      (7)
//
//  and the file evaluates (7) rather than restating 1 and 1.25.
//
//  ---------------------------------------------------------------
//  4.  THE RADIAL STACK-UP AND THE WALL
//
//  The two pitch surfaces are a plane of radius R and a cylinder of
//  radius r_p, so the wall between them is a difference of two pitch
//  radii and is therefore an integer multiple of m/2:
//
//      wall = r_p - R = (Nout - Nr) m / 2 = (280 - 242)/2 = 19 mm   (8)
//
//  That is the PITCH-surface wall, and it is not metal.  The metal is
//  measured between the two root surfaces: the crown ring's material
//  ends at the sphere, whose radius at the crown's root cone is
//  L sin(90 + tf), and the spur ring's root circle is at r_p - b.  The
//  file prints both numbers and their difference, 17.7565 mm, next to
//  the nominal 19.
//
//  ---------------------------------------------------------------
//  5.  THE REGISTER
//
//  The collars clamp on a groove between two raised lands, and the
//  groove floor is the band's outer face at radius sg_r() exactly --
//  the sphere's own equatorial radius, continued downward as a
//  cylinder.  The lands stand proud of that floor, so the register is
//  for a radially split collar that closes onto it; a one-piece collar
//  could not pass the lands.  The chosen dimensions are printed in full,
//  together with the collar bore and tongue radii they imply, so a
//  collar part can be checked against them without reading this file.
//
//  The register also reacts the port's overturning couple.  The spur
//  ring's mid-plane sits below the crown mesh plane, because nothing
//  of the band may stand above the equatorial plane outside the sphere
//  -- that space belongs to the carrier.  The file prints the moment
//  arm and the register's own axial length.
//
//  ---------------------------------------------------------------
//  6.  HOW IT IS DRAWN: TWO MEMBERS, NO BOOLEAN
//
//  The file contains no union(), no difference() and no
//  intersection().  It writes two closed shells that SHARE NO VOLUME:
//
//      the crown ring   sg_sector(Nr, Nr, D, ext = false, F)
//      the body         one closed meridian swept through one closed
//                       ring of azimuths -- a torus, so no end walls
//
//  and the crown ring seats in the body: its back cone lands on a seat
//  cut to the same cone, and its sphere stands in a bore cut at
//  sg_r().  The two seats are computed, not assumed, and their
//  clearances are printed.
//
//  This is what the parts are -- a fine-pitch crown ring at m = 1 is a
//  wear part and is made separately from the member that carries the
//  power port -- and it is also what keeps the export off the boolean
//  path, so that the exported mesh is face for face what this file
//  writes.  What the export CANNOT do is confirm the two members do
//  not overlap: past 25000 triangles this kernel skips its export-time
//  union and says so, and a scene past that threshold that DOES
//  overlap comes back with holes = 0 and a volume that counts the
//  overlap twice.  So a clean leak count is not by itself evidence.
//  The printed seat clearances are the statement.
//
//  The body's shell is a torus in index space: the meridian loop is
//  cyclic and the azimuth ring is cyclic, so every edge is used
//  exactly twice and there are no end walls to get wrong.  Every
//  azimuth station is written once -- each segment of a tooth is half
//  open -- because a duplicated station makes a zero-area quad and
//  this kernel drops zero-area quads out of a shell silently, taking
//  twelve boundary edges with each one.
//
//  ---------------------------------------------------------------
//  7.  WHAT THE FILE CHECKS
//
//  Volume, twice over and from two directions.  For the crown ring the
//  library's sg_vol is an ARC prediction, so the mesh must come in
//  slightly under it.  For the body the file computes the exact volume
//  of the mesh it is about to write -- the divergence theorem summed
//  over the two triangles the writer fans out of each quad -- and also
//  brackets it between two exact solids of revolution, the same
//  meridian with the spur stations pinned at the root circle and at
//  the tip circle.  The mesh volume must lie strictly between them.
//
//  References: the contract, spherical_gear.scad, for the spherical
//  involute, the closure arithmetic and the design table.  Figliolini
//  and Angeles, ASME JMD 2005.  Willis for the epicyclic relation.
// ===================================================================

use <spherical_gear.scad>

// ===================================================================
//  DECLARED INPUTS -- everything else in this file is derived
// ===================================================================
EB_PORT  = 10;      // outer port ratio, declared
EB_NPIN  = 28;      // pinion teeth, declared; undercut checked below
EB_F     = 0.25;    // crown face as a fraction of cone distance.  This is
                    // sg_sector's own default, written here so the seat
                    // this file cuts and the sector it seats agree; it is
                    // passed to sg_sector explicitly.
EB_SPURB = 12;      // spur face width, mm
EB_REL   = 0.75;    // relief: how far the wall's top face clears the
                    // crown's root plane
EB_LIP   = 2.00;    // seat lip inboard of the crown ring's own bore
EB_WEBT  = 5.00;    // web thickness under the seat's inner end
EB_SKW   = 8.00;    // register skirt wall

// -- the register the collars clamp on --------------------------------
EB_LANDH = 2.50;    // land height above the band's outer face
EB_LANDW = 7.00;    // axial width of one land
EB_GRW   = 6.00;    // axial width of the groove between the lands
EB_RELG  = 2.00;    // relief between the spur flange and the upper land
EB_CR    = 0.30;    // radial clearance the collar is to use
EB_CA    = 0.20;    // axial clearance the collar is to use, per side

// -- Lewis inputs for the port, declared ------------------------------
EB_SIGMA = 30;      // allowable bending, N/mm^2, printed polyamide
EB_LEWY  = 0.40;    // Lewis form factor at phi = 25, high tooth count
EB_RHO   = 1.01e-3; // polyamide density, g/mm^3, declared

// -- quoted only for comparison, never used as design data ------------
EB_ECHO_NOUT = 280; // the outer count the library's own report echoes
EB_ECHO_NPIN = 28;  // and the pinion it echoes with it

// -- mesh density ------------------------------------------------------
EB_NRA = 4;         // samples on each half of a spur root arc
EB_NFL = 10;        // samples on a spur flank
EB_NTA = 4;         // samples on a spur tip arc

// ===================================================================
//  THE CONTRACT, READ, NEVER RESTATED
// ===================================================================
M_    = sg_m();
PHI   = sg_phi();
JT_   = sg_jt();
NS    = sg_ns();
DREF  = sg_dref();
R     = sg_r();
ROW   = sg_row(3);              // the equator row
RNAME = ROW[0];
NR    = ROW[1];                 // ring teeth
D     = ROW[2];                 // cone-distance scale
NP    = ROW[3];                 // planet teeth
KP    = ROW[4];                 // planets

// ===================================================================
//  DERIVED -- the crown ring
// ===================================================================
GS    = asin(NS/D);
GP    = asin(NP/D);
GR    = asin(NR/D);             // (1): = 90 exactly, because NR == D
GB    = sg_gb(GR, PHI);         // (3): = 90 - phi exactly
TA    = sg_ta(D);
TF    = sg_tf(D);
LO    = D*M_/2;                 // cone distance = sphere radius
LI    = LO*(1 - EB_F);
FACE  = LO - LI;
ADD   = LO*tan(TA);             // (7) addendum, mm
DED   = LO*tan(TF);             // (7) dedendum, mm

// sg_sector's INTERNAL hub rule, rebuilt from the published sg_ta/sg_tf
// so that the seat this file cuts follows the sector's own back cone.
// If the library's rule moves, the seat moves with it.
function eb_ghub_i(g, D) = g + sg_tf(D) + 2.5*(sg_ta(D) + sg_tf(D));
GHUB  = eb_ghub_i(GR, D);
COTH  = cos(GHUB)/sin(GHUB);    // the seat cone is z = rho * COTH

CR_RMAX = LO*sin(GR - TA);      // crown ring's widest MESH vertex
CR_RROOT= LO*sin(GR + TF);      // its radius at the root cone
CR_ZTIP = LO*cos(GR - TA);      // tooth tip above the equatorial plane
CR_ZROOT= LO*cos(GR + TF);      // root below it
CR_HUBO = LO*sin(GHUB);         // back cone, outer end
CR_HUBI = LI*sin(GHUB);         // back cone, inner end

// ===================================================================
//  DERIVED -- the planar spur ring on the outer face
// ===================================================================
NOUT  = EB_PORT*EB_NPIN;                  // (5)
RP2   = NOUT*M_/2;
RB2   = RP2*cos(PHI);                     // (4) r_b/r_p = cos(phi)
RA2   = RP2 + ADD;
RF2   = RP2 - DED;
HT2   = sg_ht(NOUT, JT_*M_, M_);          // half tooth angle at the pitch circle
INVP  = sg_invp(PHI);
function eb_h(r) = HT2 + INVP - sg_invp(acos(RB2/r));        // (6)
HRF   = eb_h(RF2);
HRA   = eb_h(RA2);
CDIST = (NOUT + EB_NPIN)*M_/2;
WALL  = RP2 - R;                          // (8)
METAL = RF2 - CR_RROOT;                   // metal, spur root to crown root
ZMIN  = 2/(sin(PHI)*sin(PHI));            // undercut limit

// the pinion, for the contact ratio of the port mesh
PN_RP = EB_NPIN*M_/2;
PN_RB = PN_RP*cos(PHI);
PN_RA = PN_RP + ADD;
CRAT  = (sqrt(RA2*RA2 - RB2*RB2) + sqrt(PN_RA*PN_RA - PN_RB*PN_RB)
         - CDIST*sin(PHI)) / (PI*M_*cos(PHI));

// ===================================================================
//  DERIVED -- the band's meridian, from the crown's root plane down
// ===================================================================
R_BORE = R;                     // the crown ring's bore seat = sg_r()
R_LAND = R + EB_LANDH;
R_SKIN = R - EB_SKW;
R_BIN  = CR_HUBI - EB_LIP;
Z_SH   = CR_ZROOT - EB_REL;     // wall top face, clear of the crown root
Z_SB   = Z_SH - EB_SPURB;       // spur flange underside
Z_RT   = Z_SB - EB_RELG;        // top of the upper land
Z_L1   = Z_RT - EB_LANDW;       // upper land bottom = groove top
Z_L2   = Z_L1 - EB_GRW;         // groove bottom
Z_L3   = Z_L2 - EB_LANDW;       // skirt bottom face
Z_WEB  = LI*cos(GHUB) - EB_WEBT;
ZPORT  = (Z_SH + Z_SB)/2;       // spur mid-plane: the port's line of action
REGL   = Z_SB - Z_L3;           // register length, flange underside to skirt end

MER = [ [R_BIN,  R_BIN*COTH],   [R_BORE, R_BORE*COTH], [R_BORE, Z_SH],
        [0,      Z_SH],         [0,      Z_SB],
        [R,      Z_SB],         [R,      Z_RT],
        [R_LAND, Z_RT],         [R_LAND, Z_L1],        [R,      Z_L1],
        [R,      Z_L2],         [R_LAND, Z_L2],        [R_LAND, Z_L3],
        [R_SKIN, Z_L3],         [R_SKIN, Z_WEB],       [R_BIN,  Z_WEB] ];
FLG = [ false, false, false, true,  true,
        false, false, false, false, false,
        false, false, false, false, false, false ];

// what the collar must be built to
COL_BORE = R_LAND + EB_CR;      // collar bore, over the land crests
COL_TONG = R + EB_CR;           // collar tongue inner face, over the floor
COL_TW   = EB_GRW - 2*EB_CA;    // collar tongue axial width

// ---- the interface, published so a collar can READ it ----------------
// These three numbers were computed here and printed in the report, and
// nothing could read them, so each collar sized its clamp by hand
// against the report.  The chain collar landed on the register; the EM
// collar landed 21 mm outboard of it, on nothing.  A number a part must
// match is published as a function or it is not published.
function eb_if_floor()  = R;                       // 1 groove floor r = sg_r()
function eb_if_land()   = [R_LAND, EB_LANDW];      // 2 land crest r, one land's width
function eb_if_groove() = [EB_GRW, (Z_L1+Z_L2)/2]; // 3 groove width, its centre in z
function eb_if_clear()  = [EB_CR, EB_CA];          // 4 radial, axial per side
function eb_if_collar() = [COL_BORE, COL_TONG, COL_TW];  // 5 bore, tongue face, width
function eb_if_span()   = [Z_L3, Z_SB];            // 6 skirt end, flange underside
function eb_if_hub()    = 2*(Z_SB - (Z_L1+Z_L2)/2);// 7 tallest hub symmetric on the groove
function eb_if_reach()  = [R_BIN, RA2, Z_L3, CR_ZTIP];  // 8 the band's own envelope
function eb_if_port()   = ZPORT;                   // 9 the port's line of action

// ===================================================================
//  THE SPUR BOUNDARY
//  One tooth, from the space centreline before it to just short of the
//  space centreline after it, azimuth strictly increasing.  Every
//  segment is half open, so no azimuth is ever written twice.
// ===================================================================
function eb_tooth(j) =
  let( pc = 360*j/NOUT, hp = 180/NOUT,
       A = [ for (i=[0:EB_NRA-1]) let(t=i/EB_NRA)
               [RF2, (pc-hp) + ((pc-HRF)-(pc-hp))*t] ],
       C = [ for (i=[0:EB_NFL-1]) let(t=i/EB_NFL, r=RF2+(RA2-RF2)*t)
               [r, pc - eb_h(r)] ],
       Dd= [ for (i=[0:EB_NTA-1]) let(t=i/EB_NTA)
               [RA2, (pc-HRA) + 2*HRA*t] ],
       E = [ for (i=[0:EB_NFL-1]) let(t=i/EB_NFL, r=RA2+(RF2-RA2)*t)
               [r, pc + eb_h(r)] ],
       G = [ for (i=[0:EB_NRA-1]) let(t=i/EB_NRA)
               [RF2, (pc+HRF) + ((pc+hp)-(pc+HRF))*t] ] )
    concat(A,C,Dd,E,G);

BND = [ for (j=[0:NOUT-1]) each eb_tooth(j) ];
AZ  = [ for (p = BND) p[1] ];
RSP = [ for (p = BND) p[0] ];

// how far the flank chords fall inside the involute: for each chord,
// the exact curve point at the parameter midpoint, measured off the
// chord line.  This is the faceting, measured, not asserted.
function eb_fp(r) = let (a = r*cos(eb_h(r)), b = r*sin(eb_h(r))) [a, b];
function eb_sag(i) =
  let( r0 = RF2 + (RA2-RF2)*i/EB_NFL, r1 = RF2 + (RA2-RF2)*(i+1)/EB_NFL,
       P0 = eb_fp(r0), P1 = eb_fp(r1), Pm = eb_fp((r0+r1)/2),
       dx = P1[0]-P0[0], dy = P1[1]-P0[1],
       ex = Pm[0]-P0[0], ey = Pm[1]-P0[1] )
    abs(dx*ey - dy*ex)/sqrt(dx*dx + dy*dy);
FLK_SAG = max([ for (i=[0:EB_NFL-1]) eb_sag(i) ]);
FLK_CHD = max([ for (i=[0:EB_NFL-1])
                  let( r0 = RF2 + (RA2-RF2)*i/EB_NFL,
                       r1 = RF2 + (RA2-RF2)*(i+1)/EB_NFL,
                       P0 = eb_fp(r0), P1 = eb_fp(r1) )
                    norm(P1-P0) ]);
TIP_SAG = RA2*(1 - cos(HRA/EB_NTA));
RT_SAG  = RF2*(1 - cos((180/NOUT - HRF)/(2*EB_NRA)));

// ===================================================================
//  THE SHELL
//  A closed meridian loop swept through a closed ring of azimuths is a
//  TORUS: both indices are cyclic, so there are no end walls at all
//  and every edge is used exactly twice.  The face word is sg_shell's,
//  so the right-hand normal of every face points INTO the solid and
//  two faces sharing an edge traverse it in opposite directions --
//  which is the orientation-blind test, and the exported volume comes
//  out positive.  The meridian is written CLOCKWISE in (rho, z); that
//  handedness is what the face word below is matched to, and it is the
//  leading minus in eb_meshvol and eb_rev.
// ===================================================================
module eb_lathe(MER, FLG, AZ, RSP) {
    P = len(MER); M = len(AZ);
    pts = [ for (i=[0:M-1])
              let (a = AZ[i], rs = RSP[i], ca = cos(a), sa = sin(a))
                for (t = [0:P-1])
                  let (q = MER[t], r = FLG[t] ? rs : q[0])
                    [r*ca, r*sa, q[1]] ];
    id = function (i,t) (i % M)*P + (t % P);
    polyhedron(points = pts,
      faces = [ for (i=[0:M-1]) for (t=[0:P-1])
                  [ id(i,t), id(i+1,t), id(i+1,t+1), id(i,t+1) ] ],
      convexity = 10);
}

// The exact volume of the mesh eb_lathe writes: the divergence theorem
// over the two triangles the writer fans out of each quad.  Not a
// bound -- the exported number itself, to the f32 the writer rounds to.
function eb_det(a,b,c) = a[0]*(b[1]*c[2]-b[2]*c[1])
                       + a[1]*(b[2]*c[0]-b[0]*c[2])
                       + a[2]*(b[0]*c[1]-b[1]*c[0]);
function eb_meshvol(MER, FLG, AZ, RSP) =
  let (P = len(MER), M = len(AZ))
    -sum([ for (i=[0:M-1])
             let (i1 = (i+1)%M,
                  c0 = cos(AZ[i]),  s0 = sin(AZ[i]),
                  c1 = cos(AZ[i1]), s1 = sin(AZ[i1]),
                  r0 = RSP[i], r1 = RSP[i1])
               sum([ for (t=[0:P-1])
                       let (t1 = (t+1)%P, q = MER[t], u = MER[t1],
                            qa = FLG[t]  ? r0 : q[0], qb = FLG[t]  ? r1 : q[0],
                            ua = FLG[t1] ? r0 : u[0], ub = FLG[t1] ? r1 : u[0],
                            P0 = [qa*c0, qa*s0, q[1]], P1 = [qb*c1, qb*s1, q[1]],
                            P2 = [ub*c1, ub*s1, u[1]], P3 = [ua*c0, ua*s0, u[1]])
                         (eb_det(P0,P1,P2) + eb_det(P0,P2,P3))/6 ]) ]);

// Exact volume of the solid of revolution of the meridian, with the
// flagged stations pinned at a fixed radius: V = 2 pi \oint r^2/2 dz.
function eb_rev(MER, FLG, rpin) =
  let (n = len(MER))
    -2*PI*sum([ for (i=[0:n-1]) let (j = (i+1)%n,
                     a = FLG[i] ? rpin : MER[i][0],
                     b = FLG[j] ? rpin : MER[j][0])
                  (MER[j][1]-MER[i][1])*(a*a + a*b + b*b)/6 ]);
// and its polar second moment about z: J/rho = 2 pi \oint r^4/4 dz.
function eb_polar(MER, FLG, rpin) =
  let (n = len(MER))
    -2*PI*sum([ for (i=[0:n-1]) let (j = (i+1)%n,
                     a = FLG[i] ? rpin : MER[i][0],
                     b = FLG[j] ? rpin : MER[j][0])
                  (MER[j][1]-MER[i][1])*(a*a*a*a + a*a*a*b + a*a*b*b
                                         + a*b*b*b + b*b*b*b)/20 ]);

V_LO  = eb_rev(MER, FLG, RF2);
V_HI  = eb_rev(MER, FLG, RA2);
V_BODY= eb_meshvol(MER, FLG, AZ, RSP);
V_CR  = sg_vol(NR, NR, D, false, EB_F, M_);
J_LO  = eb_polar(MER, FLG, RF2);
J_HI  = eb_polar(MER, FLG, RA2);

// ===================================================================
//  REPORT
// ===================================================================
echo("=== equator_band: gates ===");
echo(str("A  gamma_r = 90 exactly      ", GR == 90 ? "PASS" : "FAIL",
         ", Nr == D == ", NR, " so sin(gamma_r) = ", NR/D));
echo(str("B  Diophantine 2Np^2=D(D-Ns) ",
         2*NP*NP == D*(D - NS) ? "PASS" : "FAIL",
         ", ", 2*NP*NP, " = ", D*(D-NS)));
echo(str("C  contract closure (9)      ",
         sg_closes(NS, NP, NR, D) ? "PASS" : "FAIL"));
echo(str("D  base cone 90 - phi        ", GB == 90 - PHI ? "PASS" : "FAIL",
         ", sg_gb(90,phi) = ", GB, " = 90 - ", PHI, " = ", 90-PHI));
echo(str("E  port ratio exact          ",
         NOUT % EB_NPIN == 0 ? "PASS" : "FAIL", ", ", NOUT, "/", EB_NPIN,
         " = ", NOUT/EB_NPIN, " = declared ", EB_PORT));
echo(str("F  outer closure 2 r_p/m     ",
         2*RP2/M_ == NOUT ? "PASS" : "FAIL", ", 2*", RP2, "/", M_,
         " = ", 2*RP2/M_, " = Nout = ", NOUT));
echo(str("G  pinion clears undercut    ", EB_NPIN > ZMIN ? "PASS" : "FAIL",
         ", Npin = ", EB_NPIN, " > 2/sin^2(phi) = ", ZMIN));
echo(str("H  port contact ratio > 1.2  ", CRAT > 1.2 ? "PASS" : "FAIL",
         ", CR = ", CRAT));
echo(str("I  crown face within L/3     ", FACE <= LO/3 ? "PASS" : "FAIL",
         ", face = ", FACE, " <= L/3 = ", LO/3));
echo(str("J  crown ring clears the bore ",
         LO - sqrt(LO*LO - Z_SH*Z_SH) > 0 ? "PASS" : "FAIL",
         ", least radial gap = ", LO - sqrt(LO*LO - Z_SH*Z_SH), " mm"));
echo(str("K  body mesh volume bracketed ",
         V_BODY > V_LO && V_BODY < V_HI ? "PASS" : "FAIL",
         ", ", V_LO, " < ", V_BODY, " < ", V_HI));
echo("SPEC EQUATOR DEGENERACY     FAIL as stated, and this is the one");
echo("  place in the machine where it would be acted on.  Numbers below.");

echo("=== 1. the crown ring: gamma = 90 is exact ===");
echo(str("row = ", RNAME, "   Nr = ", NR, "  D = ", NR == D ? "Nr" : "?",
         " = ", D, "  Np = ", NP, "  Ns = ", NS, "  planets k = ", KP));
echo(str("   sin(gamma_r) = Nr/D = ", NR, "/", D, " = ", NR/D,
         "   gamma_r = ", GR, " deg"));
echo(str("   closure (8) gamma_s + 2 gamma_p = ", GS, " + 2*", GP,
         " = ", GS + 2*GP, "   gamma_r = ", GR));
echo(str("   which at gamma_r = 90 is cos(2 gamma_p) = sin(gamma_s): ",
         cos(2*GP), " = ", sin(GS)));
echo(str("   and that is the integer identity 2 Np^2 = D(D - Ns):  2*",
         NP, "^2 = ", 2*NP*NP, "   D(D-Ns) = ", D, "*", D-NS, " = ",
         D*(D-NS)));
echo(str("   contract (9): Ns(D^2-2Np^2)+2Np q = ", sg_rhs(NS,NP,D)[0],
         "   Nr D^2 = ", NR, "*", D*D, " = ", NR*D*D,
         "   closes: ", sg_closes(NS,NP,NR,D)));
echo(str("   base cone (3): sg_gb(90,", PHI, ") = ", GB,
         "   90 - phi = ", 90 - PHI,
         "   sin(gamma_b) = ", sin(GB), "  cos(phi) = ", cos(PHI)));
echo(str("   L = D m/2 = ", LO, " mm = R = sg_r() = ", R,
         "   the equator row's apex is the sphere centre"));
echo(str("   latitude circle: R sin(colat 90) = ", R*sin(90),
         " = Nr m/2 = ", NR*M_/2, "   teeth wrap it ", NR, " times"));
echo(str("   addendum (7) L tan(sg_ta(D)) = ", ADD, " = m = ", M_,
         "   dedendum L tan(sg_tf(D)) = ", DED, " = 1.25 m = ", 1.25*M_));
echo(str("   face = L*", EB_F, " = ", FACE, " mm, from L_i = ", LI,
         " to L_o = ", LO, "   bound L/3 = ", LO/3));
echo(str("   tooth tip stands ", CR_ZTIP, " mm above the equatorial plane",
         ", root ", -CR_ZROOT, " mm below it"));

echo("=== 2. the spec's degeneracy, disproved again here ===");
echo(str("   INVS(90; ", GB, ") = ", sg_invs(90, GB),
         "   90(sec phi - 1) = ", 90*(1/cos(PHI) - 1),
         "   planar inv(phi) = ", INVP,
         "   ratio = ", sg_invs(90, GB)/INVP));
echo(str("   a planar flank on this crown would be ",
         sg_invs(90, GB) - INVP, " deg of azimuth wrong at the tip, = ",
         (sg_invs(90, GB) - INVP)/(360/NR), " tooth pitches at Nr = ", NR));
echo("   what survives is (4), r_b/r_p = cos(phi), at EVERY gamma:");
for (g = [10, 30, 60, 90])
  echo(str("      gamma = ", g, "   sin(gamma_b)/sin(gamma) = ",
           sin(sg_gb(g,PHI))/sin(g), "   cos(phi) = ", cos(PHI)));
echo(str("      the OUTER ring: r_b/r_p = ", RB2, "/", RP2, " = ", RB2/RP2,
         "   cos(phi) = ", cos(PHI),
         "   difference = ", RB2/RP2 - cos(PHI)));
echo("   that identity, not the involute function, is the bridge between");
echo("   the band's two tooth systems.  The outer ring is a planar");
echo("   involute because its mesh is parallel-axis and its pitch surface");
echo("   is a cylinder -- the gamma_b -> 0 limit -- not because a");
echo("   spherical involute degenerated into one at gamma = 90.");

echo("=== 3. the outer port ===");
echo(str("   Nout = ratio * Npin = ", EB_PORT, " * ", EB_NPIN, " = ", NOUT,
         "   ratio ", NOUT, "/", EB_NPIN, " = ", NOUT/EB_NPIN, " exactly"));
echo(str("   the library's report echoes Nout = ", EB_ECHO_NOUT,
         " and Npin = ", EB_ECHO_NPIN, "; this file's product differs by ",
         NOUT - EB_ECHO_NOUT, " and ", EB_NPIN - EB_ECHO_NPIN));
echo(str("   r_p = Nout m/2 = ", RP2, "   r_b = r_p cos(phi) = ", RB2,
         "   r_a = r_p + a = ", RA2, "   r_f = r_p - b = ", RF2));
echo(str("   closure: 2 r_p/m = ", 2*RP2/M_, " = Nout = ", NOUT,
         "   circumference/(pi m) = ", 2*PI*RP2/(PI*M_), " = ", NOUT));
echo(str("   centre distance (Nout+Npin) m/2 = ", CDIST,
         " mm; pinion r_p = ", PN_RP, " r_b = ", PN_RB, " r_a = ", PN_RA));
echo(str("   (6) at the pitch circle: h(r_p) = ", eb_h(RP2),
         "   ht = sg_ht(Nout, jt m, m) = ", HT2,
         "   difference = ", eb_h(RP2) - HT2));
echo(str("   h(r_f) = ", HRF, "   h(r_a) = ", HRA,
         "   tip is not pointed: h(r_a) > 0 is ", HRA > 0));
echo(str("   tooth thickness at r_p = ", 2*rad(HT2)*RP2,
         " mm; circular pitch pi m = ", PI*M_,
         "; backlash pi m - 2 t = ", PI*M_ - 2*(2*rad(HT2)*RP2)/2,
         " = jt m = ", JT_*M_));
echo(str("   space width at the root circle = ",
         2*rad(180/NOUT - HRF)*RF2, " mm; tip land = ", 2*rad(HRA)*RA2,
         " mm.  The root space is the printability limit on this face."));
echo(str("   contact ratio with the ", EB_NPIN, " tooth pinion = ", CRAT));
echo(str("   faceting, measured: ", EB_NFL, " chords on a flank, longest ",
         FLK_CHD, " mm, greatest deviation from the involute ", FLK_SAG,
         " mm; tip arc sagitta ", TIP_SAG, " mm; root arc sagitta ",
         RT_SAG, " mm"));
echo(str("   Lewis F = sigma b m Y = ", EB_SIGMA, "*", EB_SPURB, "*", M_,
         "*", EB_LEWY, " = ", EB_SIGMA*EB_SPURB*M_*EB_LEWY,
         " N per tooth pair, so the port carries ",
         EB_SIGMA*EB_SPURB*M_*EB_LEWY*RP2/1000,
         " Nm at the band and ",
         EB_SIGMA*EB_SPURB*M_*EB_LEWY*PN_RP/1000, " Nm at the pinion"));

echo("=== 4. the wall between the two tooth systems ===");
echo(str("   pitch surfaces: crown plane at R = ", R,
         ", spur cylinder at r_p = ", RP2));
echo(str("   (8) wall = r_p - R = ", WALL, " = (Nout - Nr) m/2 = (", NOUT,
         " - ", NR, ")*", M_, "/2 = ", (NOUT-NR)*M_/2));
echo(str("   metal: crown material ends at L sin(90+tf) = ", CR_RROOT,
         ", spur root circle at ", RF2, ", so the wall is ", METAL,
         " mm of metal, = wall - b + (L - L sin(90+tf)) = ",
         WALL - DED + (LO - CR_RROOT)));
echo(str("   the bore is cut at sg_r() = ", R_BORE,
         " and the crown ring is a sphere of the same radius, so the",
         " register gap is set by how far the bore lip sits off the",
         " equator: L - sqrt(L^2 - z^2) at z = ", Z_SH, " is ",
         LO - sqrt(LO*LO - Z_SH*Z_SH), " mm, opening to ",
         LO - CR_HUBO, " mm at the seat"));
echo(str("   seat: both the sector's back cone and this file's seat are",
         " cut at GHUB = ", GHUB, " deg from the same rule, cot = ", COTH,
         "; the seat runs from r = ", R_BIN, " to ", R_BORE,
         " and the crown ring's back cone from ", CR_HUBI, " to ", CR_HUBO,
         ", so the lip stands ", CR_HUBI - R_BIN, " mm inboard"));
echo(str("   the crown ring's widest mesh vertex is at r = ", CR_RMAX,
         ", which is ", R_BORE - CR_RMAX, " mm inside the bore; the exact",
         " sphere reaches ", LO, " at z = 0, which is ", -Z_SH,
         " mm above the bore's lip and so outside it"));

echo("=== 5. the register the collars clamp on ===");
echo(str("   groove floor radius = sg_r() = ", R, " exactly: ", R == sg_r()));
echo(str("   lands ", EB_LANDH, " mm proud at r = ", R_LAND, ", each ",
         EB_LANDW, " mm wide; groove ", EB_GRW, " mm wide between them"));
echo(str("   axial stations: flange underside z = ", Z_SB, ", relief to ",
         Z_RT, ", upper land to ", Z_L1, ", groove floor to ", Z_L2,
         ", lower land to ", Z_L3));
echo(str("   register length = ", REGL, " mm = relief ", EB_RELG,
         " + land ", EB_LANDW, " + groove ", EB_GRW, " + land ", EB_LANDW,
         " = ", EB_RELG + 2*EB_LANDW + EB_GRW));
echo(str("   a collar built to these takes bore ", COL_BORE, " (= r_land + ",
         EB_CR, "), tongue inner face ", COL_TONG, " (= sg_r() + ", EB_CR,
         "), tongue width ", COL_TW, " (= groove - 2*", EB_CA, ")"));
echo(str("   groove centre at z = ", (Z_L1 + Z_L2)/2,
         "; a collar hub symmetric about it may be up to ",
         2*(Z_SB - (Z_L1+Z_L2)/2), " mm tall before it fouls the flange"));
echo(str("   the port's line of action is the spur mid-plane z = ", ZPORT,
         ", so the port pulls with a ", -ZPORT,
         " mm arm about the crown mesh plane and the register reacts it",
         " over ", REGL, " mm, an arm ratio of ", REGL/(-ZPORT)));
echo(str("   nothing of the band stands above z = ", Z_SH,
         " outside the sphere: the only band material above that plane is",
         " the crown ring's teeth, whose widest point is r = ", CR_RMAX,
         " < R = ", R, ".  That space belongs to the carrier."));

echo("=== 6. the band as a member of the row ===");
echo(str("   Willis (w_s - w_c)/(w_r - w_c) = -Nr/Ns = -", NR, "/", NS,
         " = -", NR/gcd_(NR,NS), "/", NS/gcd_(NR,NS)));
echo(str("   sun held: w_band/w_carrier = (Ns+Nr)/Nr = ", NS+NR, "/", NR,
         " = ", (NS+NR)/gcd_(NS+NR,NR), "/", NR/gcd_(NS+NR,NR),
         " = ", (NS+NR)/NR, ", so the pinion turns ",
         EB_PORT*(NS+NR)/NR, " times per carrier turn"));
echo(str("   band held: w_c/w_s = Ns/(Ns+Nr) = ", NS, "/", NS+NR, " = ",
         NS/gcd_(NS,NS+NR), "/", (NS+NR)/gcd_(NS,NS+NR), " = ", NS/(NS+NR)));
echo(str("   assembly (Ns+Nr)/k = ", NS+NR, "/", KP, " = ", (NS+NR)/KP,
         "   integer: ", (NS+NR) % KP == 0));
echo(str("   backlash: jt m = ", JT_*M_,
         " mm at the crown mesh and the same at the port, which is ",
         deg(JT_*M_/RP2), " deg at the band and ", deg(JT_*M_/PN_RP),
         " deg at the pinion"));

echo("=== 7. mass, inertia and the mesh ===");
echo(str("   body: exact mesh volume = ", V_BODY, " mm^3, bracketed by the",
         " same meridian revolved with the spur stations pinned at the",
         " root circle ", V_LO, " and at the tip circle ", V_HI));
echo(str("   crown ring: sg_vol predicts ", V_CR,
         " mm^3 over arcs, so the mesh must come in under it"));
echo(str("   predicted total = ", V_BODY + V_CR, " mm^3"));
echo(str("   mass at rho = ", EB_RHO, " g/mm^3 is between ",
         (V_LO + V_CR)*EB_RHO, " and ", (V_HI + V_CR)*EB_RHO, " g"));
echo(str("   polar second moment of the body alone, J = rho * ",
         J_LO, " to ", J_HI, " mm^5, i.e. ", J_LO*EB_RHO*1e-3,
         " to ", J_HI*EB_RHO*1e-3, " kg mm^2"));
echo(str("   envelope: r from ", R_BIN, " to ", RA2, " mm, z from ", Z_L3,
         " to ", CR_ZTIP, " mm"));
echo(str("   mesh: body ", len(AZ), " azimuth stations * ", len(MER),
         " meridian points = ", len(AZ)*len(MER), " quads = ",
         2*len(AZ)*len(MER), " triangles, no end walls (a torus in both",
         " indices); crown ring as sg_sector writes it"));
echo(str("   the two members share no volume.  Past 25000 triangles this",
         " kernel skips the export-time union, so the export cannot",
         " confirm that; the seat clearances above are the statement."));

// ===================================================================
//  THE BAND
//  Two members that share no volume: the body, and the crown ring
//  seated in it.  Wrapped in a module so the stack can place one.
// ===================================================================
module equator_band() {
    color([0.55, 0.62, 0.70]) eb_lathe(MER, FLG, AZ, RSP);
    color([0.82, 0.68, 0.34]) sg_sector(NR, NR, D, false, EB_F, M_, PHI, JT_);
}
equator_band();
