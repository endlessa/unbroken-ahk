// PLACEHOLDER HEADER -- rewritten at the end from what the file prints.

use <spherical_gear.scad>

// ===================================================================
//  DECLARED INPUTS -- everything else in this file is derived
// ===================================================================
EB_PORT  = 10;      // outer port ratio, declared: band teeth / pinion teeth
EB_NPIN  = 28;      // pinion teeth, declared; undercut checked below
EB_F     = 0.25;    // crown face as a fraction of cone distance.  This is
                    // sg_sector's own default; it is written here so the
                    // seat this file cuts and the sector it seats agree,
                    // and it is passed to sg_sector explicitly.
EB_SPURB = 12;      // spur face width, mm
EB_REL   = 0.75;    // relief: how far the wall's top face sits below the
                    // crown's root plane
EB_LIP   = 2.00;    // seat lip inboard of the crown ring's own bore
EB_WEBT  = 5.00;    // web thickness under the seat's inner end
EB_SKW   = 8.00;    // register skirt wall

// -- the register the collars clamp on --------------------------------
EB_LANDH = 2.50;    // land height above the band's outer face
EB_LANDW = 7.00;    // axial width of one land
EB_GRW   = 6.00;    // axial width of the groove between the lands
EB_CR    = 0.30;    // radial clearance the collar is to use, tongue/floor
EB_CA    = 0.20;    // axial clearance the collar is to use, per side

// -- Lewis inputs for the port, declared ------------------------------
EB_SIGMA = 30;      // allowable bending, N/mm^2, printed polyamide
EB_LEWY  = 0.40;    // Lewis form factor at phi = 25, high tooth count

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
NR    = ROW[1];                 // ring teeth
D     = ROW[2];                 // cone-distance scale
NP    = ROW[3];                 // planet teeth
KP    = ROW[4];                 // planets

// ===================================================================
//  DERIVED -- the crown ring
// ===================================================================
GS    = asin(NS/D);
GP    = asin(NP/D);
GR    = asin(NR/D);             // = 90 exactly, because NR == D
GB    = sg_gb(GR, PHI);         // = 90 - phi exactly
TA    = sg_ta(D);
TF    = sg_tf(D);
LO    = D*M_/2;                 // cone distance = sphere radius
LI    = LO*(1 - EB_F);
ADD   = LO*tan(TA);             // addendum, mm  (= m)
DED   = LO*tan(TF);             // dedendum, mm  (= 1.25 m)

// sg_sector's INTERNAL hub rule, rebuilt from the published sg_ta/sg_tf
// so that the seat this file cuts follows the sector's own back cone.
// If the library's rule moves, the seat moves with it.
function eb_ghub_i(g, D) = g + sg_tf(D) + 2.5*(sg_ta(D) + sg_tf(D));
GHUB  = eb_ghub_i(GR, D);
COTH  = cos(GHUB)/sin(GHUB);    // the seat cone, z = rho * COTH

CR_RMAX = LO*sin(GR - TA);      // crown ring's widest point (at the tip cone)
CR_RROOT= LO*sin(GR + TF);      // its radius at the root cone
CR_ZTIP = LO*cos(GR - TA);      // tooth tip above the equatorial plane
CR_ZROOT= LO*cos(GR + TF);      // root below it
CR_HUBO = LO*sin(GHUB);         // back cone, outer end
CR_HUBI = LI*sin(GHUB);         // back cone, inner end

// ===================================================================
//  DERIVED -- the planar spur ring on the outer face
// ===================================================================
NOUT  = EB_PORT*EB_NPIN;        // 280
RP2   = NOUT*M_/2;              // pitch radius
RB2   = RP2*cos(PHI);           // r_b / r_p = cos(phi), the surviving relation
RA2   = RP2 + ADD;
RF2   = RP2 - DED;
HT2   = sg_ht(NOUT, JT_*M_, M_);          // half tooth angle at the pitch circle
INVP  = sg_invp(PHI);
function eb_h(r) = HT2 + INVP - sg_invp(acos(RB2/r));   // half width at r
HRF   = eb_h(RF2);
HRA   = eb_h(RA2);
CDIST = (NOUT + EB_NPIN)*M_/2;            // centre distance to the pinion
WALL  = RP2 - R;                          // = (NOUT - NR) m / 2
METAL = RF2 - CR_RROOT;                   // metal, spur root to crown root

// ===================================================================
//  DERIVED -- the band's meridian
// ===================================================================
R_BORE = R;                     // the crown ring's bore seat = sg_r()
R_LAND = R + EB_LANDH;
R_SKIN = R - EB_SKW;
R_BIN  = CR_HUBI - EB_LIP;
Z_SH   = CR_ZROOT - EB_REL;     // wall top, below the crown's root plane
Z_SB   = Z_SH - EB_SPURB;
Z_L1   = Z_SB - EB_LANDW;
Z_L2   = Z_L1 - EB_GRW;
Z_L3   = Z_L2 - EB_LANDW;
Z_WEB  = LI*cos(GHUB) - EB_WEBT;

MER = [ [R_BIN,  R_BIN*COTH],   [R_BORE, R_BORE*COTH], [R_BORE, Z_SH],
        [0,      Z_SH],         [0,      Z_SB],
        [R_LAND, Z_SB],         [R_LAND, Z_L1],        [R,      Z_L1],
        [R,      Z_L2],         [R_LAND, Z_L2],        [R_LAND, Z_L3],
        [R_SKIN, Z_L3],         [R_SKIN, Z_WEB],       [R_BIN,  Z_WEB] ];
FLG = [ false, false, false, true, true,
        false, false, false, false, false, false, false, false, false ];

// ===================================================================
//  THE SPUR BOUNDARY
//  One tooth, from the space centreline before it to just short of the
//  space centreline after it, azimuth strictly increasing.  Every
//  segment is half open, so no azimuth is ever written twice: a
//  duplicated station makes a zero-area quad and this kernel drops
//  zero-area quads out of the shell without a word.
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

// ===================================================================
//  THE SHELL
//  A closed meridian loop swept through a closed ring of azimuths is a
//  TORUS: every index is cyclic in both directions, so there are no
//  end walls at all and every edge is used exactly twice.  The face
//  word is sg_shell's j/k word, so the right-hand normal of every face
//  points INTO the solid and two faces sharing an edge traverse it in
//  opposite directions.
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

// exact volume of the mesh eb_lathe writes: the divergence theorem over
// the two triangles the writer fans out of each quad, with the sign the
// kernel's inward face word gives.
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

// exact solid-of-revolution volume of a closed meridian polygon, with
// the flagged stations pinned at a fixed radius
function eb_pappus(MER, FLG, rpin) =
  let (n = len(MER))
    2*PI*sum([ for (i=[0:n-1]) let (j = (i+1)%n,
                    a = FLG[i] ? rpin : MER[i][0],
                    b = FLG[j] ? rpin : MER[j][0])
                 (MER[j][1]-MER[i][1])*(a*a + a*b + b*b)/6 ]);

// ===================================================================
//  PROBE
// ===================================================================
echo(str("GR=",GR," GB=",GB," TA=",TA," TF=",TF," ADD=",ADD," DED=",DED));
echo(str("GHUB=",GHUB," COTH=",COTH," LO=",LO," LI=",LI));
echo(str("CR_RMAX=",CR_RMAX," CR_RROOT=",CR_RROOT," CR_ZTIP=",CR_ZTIP,
         " CR_ZROOT=",CR_ZROOT," CR_HUBO=",CR_HUBO," CR_HUBI=",CR_HUBI));
echo(str("NOUT=",NOUT," RP2=",RP2," RB2=",RB2," RA2=",RA2," RF2=",RF2,
         " HT2=",HT2," HRF=",HRF," HRA=",HRA));
echo(str("WALL=",WALL," METAL=",METAL," CDIST=",CDIST));
echo(str("MER=",MER));
echo(str("stations=",len(AZ)," first=",AZ[0]," last=",AZ[len(AZ)-1]));
echo(str("Pappus at root ",eb_pappus(MER,FLG,RF2),
         "  at tip ",eb_pappus(MER,FLG,RA2)));
echo(str("body mesh volume ", eb_meshvol(MER,FLG,AZ,RSP)));
echo(str("crown predicted ", sg_vol(NR, NR, D, false, EB_F, M_)));

eb_lathe(MER, FLG, AZ, RSP);
sg_sector(NR, NR, D, false, EB_F, M_, PHI, JT_);
