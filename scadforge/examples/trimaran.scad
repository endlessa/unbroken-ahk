include <naut_lib.scad>
// ===================================================================
//   "SOUNDER"  --  75 ft organic tri-hull hydroplaning yacht
//   Exterior only.  +X forward (transom 0, stem 75), +Y stbd, Z=0 DWL.
//   Entirely additive: parametric surfaces emitted as polyhedron quad
//   meshes.  Not one difference() or intersection() in the file.
// ===================================================================
L = 75;

VENTRAL = [0.900,0.905,0.893];   // below the chine
TOPSIDE = [0.455,0.495,0.545];   // chine to sheer
HOUSE   = [0.800,0.808,0.800];
WINGC   = [0.755,0.768,0.778];
SEA     = [0.088,0.152,0.182];
GLASS   = [0.170,0.202,0.245];
RIBC    = [0.660,0.678,0.700];
HRIB    = [0.560,0.578,0.600];
BRONZE  = [0.720,0.525,0.320];

function fwd(t,t0) = max(0,(t-t0)/(1-t0));
function stp(v,a,b) = v<=a ? 0 : v>=b ? 1 : 0.5-0.5*cos(180*(v-a)/(b-a));
function lens(u,n) = pow(max(0, 1 - u*u), 1/n);

// ===================================================================
//  1. CENTRE HULL
//     THE FULLNESS INVERSION: maximum chine beam sits AFT of midships
//     (a planing hull), maximum sheer beam sits FORWARD of midships
//     (a whale).  From below it is a racing bottom; from above it is a
//     body.  One hull, two correct answers.
//     THE DISSOLVING CHINE: the chine fillet radius runs 0.08 ft at the
//     transom -- knife-hard, which is what makes it plane -- to 3.1 ft
//     at the bow, a full round bilge.  An edge with a lifecycle.
//     TWO VENTILATED STEPS at x=19 and x=34 break the wetted length.
// ===================================================================
CHI = 14;  NHALF = 48;  CHIP = 2*NHALF - 2 - CHI;
XS = concat([ for (i=[0:18]) i*1.0 ], [18.98, 19.02],
            [ for (i=[1:14]) 19 + i ], [33.98, 34.02],
            [ for (i=[1:20]) 34 + i*1.30 ], [ for (i=[1:24]) 60.0 + i*0.625 ]);
NU = len(XS) - 1;
function bc_(x) = max(0.06, 6.55*lens((x-26)/(x<26 ? 30 : 46), x<26 ? 2.6 : 1.45));
function bs_(x) = max(0.09, 7.80*lens((x-44)/(x<44 ? 48 : 31), x<44 ? 2.9 : 2.35));
function zstep(x) = (x>19 ? 0.45 : 0) + (x>34 ? 0.35 : 0);
function zk_(x)  = -3.30 + 0.0148*min(x,19) + 0.0192*max(0,min(x,34)-19)
                   - zstep(x) + 3.53*pow(fwd(x/L,0.453),1.45);
function zch_(x) = zk_(x) + bc_(x)*tan(13 + 44*pow(x/L,1.70));
function zs_(x)  = 4.95 + 4.55*pow(x/L,2.10) - 0.70*sin(180*x/L);
function hull_sec(x) = full_section(half_section(
    [ [0, zs_(x)+0.80+0.60*(1-x/L)], [bs_(x), zs_(x)], [bc_(x), zch_(x)], [0, zk_(x)] ],
    [ 3.4, 0.40+1.55*x/L, 0.08+3.10*pow(fwd(x/L,0.20),1.30),
      0.05+2.20*pow(fwd(x/L,0.34),1.55) ] ));
HSEC = [ for (u=[0:NU]) hull_sec(XS[u]) ];
function lift(sec,x,sy) = [ for (p=sec) [x, sy*p[0], p[1]] ];
HTOP = [ for (u=[0:NU]) lift(part(HSEC[u],CHIP,CHI,10), XS[u], 1) ];
HBOT = [ for (u=[0:NU]) lift(part(HSEC[u],CHI,CHIP,10), XS[u], 1) ];

// ===================================================================
//  2. AMAS -- hulls, not foils.  Bowed convex-outboard in plan and
//     toed in at the bow, so the boat looks like it is already moving.
// ===================================================================
AX0 = 18; AX1 = 58; NA = 52;
function ax(u) = AX0 + (AX1-AX0)*u/NA;
function ayc(x) = 15.20 - 0.0028*pow(x-38, 2) - 0.028*max(0,x-38);
function abc_(x) = max(0.05, 2.15*lens((x-38)/(x<38 ? 21 : 21.5), 2.45));
function abs_(x) = max(0.06, 2.60*lens((x-36)/(x<36 ? 19 : 23), 2.70));
function azk_(x) = -1.55 + 2.40*pow(max(0,(38-x)/22),1.85) + 6.90*pow(max(0,(x-38)/23),1.75);
function azc_(x) = azk_(x) + abc_(x)*tan(21 + 34*pow((x-AX0)/(AX1-AX0),1.30));
function azs_(x) = 1.95 + 2.85*pow(x/L,2.10) - 0.35*sin(180*(x-AX0)/(AX1-AX0));
function ama_deck(x) = azs_(x) + 0.55;
function ama_sec(x) = full_section(half_section(
    [ [0, ama_deck(x)], [abs_(x), azs_(x)], [abc_(x), azc_(x)], [0, azk_(x)] ],
    [ 1.7, 0.35+0.70*x/L, 0.07+1.70*pow(fwd(x/L,0.20),1.3),
      0.05+1.30*pow(fwd(x/L,0.30),1.5) ] ));
function AG(sy, lo, hi) = [ for (u=[0:NA]) let(x=ax(u))
    [ for (p = part(ama_sec(x), lo, hi, 10)) [x, sy*(ayc(x) + p[0]), p[1]] ] ];

// ===================================================================
//  3. THE WING AND ITS CELLS
//     "Wing cells" = a spanwise run of aerofoil-sectioned arch blades
//     spanning hull to ama under one continuous skin, with the bays
//     between consecutive blades opened into lens-shaped through-voids.
//     Structure, wing and ribcage at once.
//
//     THE ARCH LAW, used for every arch on the vessel:
//        z(s) = z0 + (z1-z0)s + H*[sin(180 s)]^p ,  p = 0.78
//     p < 1 makes the tangent VERTICAL at both feet, so an arch leaves
//     the hull and lands on the ama with no visible springing joint --
//     it grows out of the surface the way a rib grows off a sternum.
// ===================================================================
PARCH = 0.78;
XB = [22.0, 32.0, 42.0, 52.0];
function y_root(x) = bs_(x)*0.97;
function z_root(x) = zs_(x) + 0.35;
function y_tip(x)  = ayc(x);
function z_tip(x)  = ama_deck(x) - 0.95;
function arch_H(x) = 4.20 + 5.20*sin(180*pow((x-18)/40, 1.10));
function wz(x, s) = z_root(x) + (z_tip(x)-z_root(x))*s
                    + arch_H(x)*pow(sin(180*s), PARCH);
function wy(x, s) = y_root(x) + (y_tip(x)-y_root(x))*s;

// bay parameter: 0 at a blade, 1 at mid-bay
function bay(x) = let( k = x<XB[1] ? 0 : x<XB[2] ? 1 : 2 )
    pow(sin(180*(x-XB[k])/(XB[k+1]-XB[k])), 0.75);
function endf(x) = pow(sin(180*(x-XB[0])/(XB[3]-XB[0])), 0.35);
function gap(x) = max(0.0, 0.165*endf(x) - 0.020) + 0.200*bay(x)*endf(x);
NW = 88; NSP = 15; WT = 0.52;
function wx(u) = XB[0] + (XB[3]-XB[0])*u/NW;
// one strip as a closed section: out along the top, back along the bottom
function strip_sec(x, s0, s1) =
  concat( [ for (i=[0:NSP]) let(s = s0+(s1-s0)*i/NSP) [wy(x,s), wz(x,s)] ],
          [ for (i=[NSP:-1:0]) let(s = s0+(s1-s0)*i/NSP) [wy(x,s), wz(x,s)-WT] ] );
function WGRID(sy, inner) = [ for (u=[0:NW]) let(x = wx(u), g = gap(x))
    [ for (p = strip_sec(x, inner ? -0.04 : 0.5+g, inner ? 0.5-g : 1.04))
        [x, sy*p[0], p[1]] ] ];

function blade_spine(k, sy) = [ for (i=[0:40]) let(s=i/40)
    [ XB[k] + 1.8*sin(180*s), sy*wy(XB[k],s), wz(XB[k],s) ] ];
function blade_sec(i) = let(f = pow(abs(2*i/40-1), 2.0))
    [ for (q = foil(2.6 + 2.4*f, 0.62 + 0.70*f, 0.85, 12)) [q[1], q[0]] ];

// The catwalk that rides the arch crowns.  It closes the tops of the
// bays, so the air between two ribs reads as a cell rather than a gap.
NCW = 56; SCW = 0.475;
function cw_sp(sy) = [ for (i=[0:NCW]) let(x = XB[0]-1.2 + (XB[3]-XB[0]+2.4)*i/NCW)
    [ x, sy*wy(x,SCW), wz(x,SCW) + 0.30 ] ];
function cw_sec(i) = let(f = pow(abs(2*i/NCW-1), 2.2))
    lame(1.30 - 0.85*f, 0.34 - 0.20*f, 2.6, 20);

// ===================================================================
//  4. SUPERSTRUCTURE
//     Not a deckhouse on a deck -- a mass that grows out of it.  Swept
//     fore and aft; its height rises from the deck aft, holds, and
//     falls away forward into a 15 ft raked screen, so there is no
//     hard horizontal anywhere.  Three stories read as three glazing
//     ribbons that taper out with the mass.
// ===================================================================
HFIL = 6; HSEG = 4; NPT = 9;
HN  = 2 + (NPT-2)*(HFIL+1) + (NPT-1)*HSEG;
function nodemid(k) = 1 + k*HSEG + (k-1)*(HFIL+1) + HFIL/2;
NHF = 2*HN - 2;
function prt(i) = NHF - i;
HXC = 27.0;
function hgt(x) = max(1.02, 17.5 * lens((x-HXC)/(x<HXC ? 22.0 : 34.0), x<HXC ? 4.2 : 1.72)
                              * stp(x, 5.0, 17.0));
FZ  = [1.000, 0.800, 0.675, 0.590, 0.470, 0.395, 0.285, 0.000];
FW  = [0.000, 0.330, 0.520, 0.640, 0.755, 0.820, 0.885, 1.000];
GIN = [0, 1, 1, 1, 1, 1, 1, 0];
function hbase(x) = bs_(x)*0.720;
function hsec(x, groove) =
  let( w = hbase(x), zd = zs_(x) - 0.25, h = hgt(x),
       P = concat( [ for (k=[0:7]) [ max(0.20, w*FW[k] - groove*GIN[k]*0.40),
                                     zd + h*FZ[k] ] ], [ [0.15, zd] ] ),
       R = [ 0, 2.4, 1.4, 1.4, 1.4, 1.4, 1.4, 2.0, 0 ] )
    full_section(chain(P, R, HFIL, HSEG));
HX0 = 5.6; HX1 = 59.5; NH = 108;
function hx(u) = HX0 + (HX1-HX0)*u/NH;
// glazing runs only where the mass is full, so no lamellae at the ends
GX0 = 13.5; GX1 = 52.0; NG = 84;
function gx(u) = GX0 + (GX1-GX0)*u/NG;

// DORSAL SPINE -- the arch law rotated into the fore-and-aft plane.
// One continuous swell along the crown instead of ribs across it, so
// the roof gains a line without gaining a cage.
NSPN = 70; SPX0 = 16.0; SPX1 = 53.0;
function spn_sp() = [ for (i=[0:NSPN]) let(x = SPX0 + (SPX1-SPX0)*i/NSPN)
    [ x, 0, zs_(x) - 0.25 + hgt(x) + 0.18 ] ];
function spn_sec(i) = let(f = pow(abs(2*i/NSPN-1), 1.7))
    lame(1.55 - 1.20*f, 0.62 - 0.50*f, 2.3, 18);

// ===================================================================
//  5. SHEER STRAKE -- one bronze line, the counter-shading boundary
// ===================================================================
function ss_sp(sy) = [ for (i=[0:72]) let(x = 0.6 + 73.6*i/72)
    [ x, sy*(bs_(x)+0.09), zs_(x)-0.28 ] ];

// ===================================================================
//  assembly
// ===================================================================
SEA_ON = 1;   // set to 0 to see the underwater body
if (SEA_ON) color(SEA) translate([-320, -320, -10]) cube([760, 640, 10.0]);
color(TOPSIDE) sweep_mesh(HTOP);
color(VENTRAL) sweep_mesh(HBOT);
for (sy=[1,-1]) {
    color(TOPSIDE) sweep_mesh(AG(sy, CHIP, CHI));
    color(VENTRAL) sweep_mesh(AG(sy, CHI, CHIP));

    color(BRONZE)  sweep_mesh(sweep_grid(ss_sp(sy), function(i) lame(0.40,0.23,2.4,14), [0,0,1]));
    color(RIBC) sweep_mesh(sweep_grid(cw_sp(sy), function(i) cw_sec(i), [0,0,1]));
    color(RIBC) for (k=[0:3])
        sweep_mesh(sweep_grid(blade_spine(k,sy), function(i) blade_sec(i), [1,0,0]));
}
color(HOUSE) sweep_mesh([ for (u=[0:NH]) lift(hsec(hx(u), 1), hx(u), 1) ]);
BI = [ for (k=[1:7]) nodemid(k) ];
for (g = [0,2,4]) color(GLASS) {
    sweep_mesh([ for (u=[0:NG]) lift(part_shell(hsec(gx(u),0), BI[g],        BI[g+1],      0.10, 0.24), gx(u), 1) ]);
    sweep_mesh([ for (u=[0:NG]) lift(part_shell(hsec(gx(u),0), prt(BI[g+1]), prt(BI[g]),   0.10, 0.24), gx(u), 1) ]);
}
color(HOUSE) sweep_mesh(sweep_grid(spn_sp(), function(i) spn_sec(i), [0,0,1]));
