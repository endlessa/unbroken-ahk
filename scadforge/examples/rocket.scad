// ============================================================
//  "Kármán"  --  sounding-rocket outer mould line
//  Exterior surfaces only: nose, body, boat-tail, fins, nozzle.
//  Every profile is an aerodynamic one, not a decorative one.
// ============================================================

// ---- master dimensions (cm) --------------------------------
R        = 10;                 // body radius        (1 caliber = 20)
Z_AFT    = 0;                  // aft end of the airframe
BT_L     = 24;                 // boat-tail length
RT       = 7;                  // boat-tail exit radius
Z_TUBE0  = Z_AFT + BT_L;       // 24  start of the cylinder
Z_TUBE1  = 144;                // end of the cylinder
NL       = 60;                 // nose length = 3 calibers
Z_TIP    = Z_TUBE1 + NL;       // 204   -> fineness ratio 10.2

// ---- livery ------------------------------------------------
WHITE  = [0.945, 0.945, 0.930];
CHAR   = [0.145, 0.155, 0.175];
COPPER = [0.800, 0.500, 0.255];
STEEL  = [0.600, 0.625, 0.660];

NSEG = 96;                     // circumferential resolution

// ============================================================
//  1. NOSE  --  LD-Haack (von Karman) series, C = 0
//     minimum wave drag for a given length & base radius
// ============================================================
function clamp1(v) = min(1, max(-1, v));
function haack(x) =
    x <= 0  ? 0 :
    x >= NL ? R :
    let (th = acos(clamp1(1 - 2*x/NL)))
        R/sqrt(PI) * sqrt(max(0, th*PI/180 - sin(2*th)/2));

function nose_r(z) = haack(Z_TIP - z);

NS = 44;                       // samples, packed toward the tip
NP = [ for (i = [0:NS])
         let (x = NL*pow(i/NS, 1.8))
           [Z_TIP - x, haack(x)] ];          // [z, r], tip first

// one colour band of the nose, revolved
module nose_band(z0, z1) {
    mid = [ for (i = [NS:-1:0]) if (NP[i][0] > z0 && NP[i][0] < z1) NP[i] ];
    r0  = nose_r(z0);
    r1  = nose_r(z1);
    pts = concat(
            [[0, z0], [r0, z0]],
            [ for (p = mid) [p[1], p[0]] ],
            r1 > 1e-9 ? [[r1, z1], [0, z1]] : [[0, z1]] );
    rotate_extrude($fn = NSEG) polygon(pts);
}

// ============================================================
//  2. BOAT-TAIL  --  cosine blend, tangent to the tube at the
//     shoulder and to the exit: peak slope 11.1 deg, so the
//     flow stays attached all the way to the base.
// ============================================================
BS = 26;
module boat_tail() {
    pts = concat(
        [[0, Z_AFT]],
        [ for (i = [0:BS])
            let (u = i/BS)
              [ RT + (R - RT)*(0.5 - 0.5*cos(180*u)), Z_AFT + BT_L*u ] ],
        [[0, Z_AFT + BT_L]] );
    rotate_extrude($fn = NSEG) polygon(pts);
}

// ============================================================
//  3. FINS  --  clipped delta, 49.7 deg leading-edge sweep,
//     lofted from a symmetric NACA 00xx section (8.5% t/c):
//     round leading edge, cusped trailing edge, no flat plate.
// ============================================================
FIN_CR    = 46;    // root chord
FIN_CT    = 18;    // tip chord
FIN_SPAN  = 22;    // exposed semi-span
FIN_SWEEP = 26;    // leading-edge offset at the tip
FIN_T     = 0.085; // thickness / chord
FIN_ZLE   = 70;    // root leading edge (root TE lands on the boat-tail shoulder)
FIN_R0    = R - 1.2;

function naca(u) =
    5*FIN_T*(  0.2969*sqrt(u) - 0.1260*u
             - 0.3516*u*u + 0.2843*u*u*u - 0.1036*u*u*u*u );

AN = 34;
function rib(c) =
    concat(
      [ for (i = [0:AN])  let (u = 0.5 - 0.5*cos(180*i/AN)) [c*u, -c*naca(u)] ],
      [ for (i = [AN-1:-1:1]) let (u = 0.5 - 0.5*cos(180*i/AN)) [c*u,  c*naca(u)] ] );

module fin_blade() {
    hull() {
        translate([0, 0, FIN_R0])            linear_extrude(0.02) polygon(rib(FIN_CR));
        translate([FIN_SWEEP, 0, FIN_R0 + FIN_SPAN])
                                             linear_extrude(0.02) polygon(rib(FIN_CT));
    }
}

// streamlined root fairing: blunt forward, tapered aft -- kills
// the fin/body interference vortex.
module fin_shoe() {
    hull() {
        translate([R - 1.4, 0, FIN_ZLE + 6])                sphere(2.5, $fn = 32);
        translate([R - 1.4, 0, FIN_ZLE - 26])               sphere(2.5, $fn = 32);
        translate([R - 1.4, 0, FIN_ZLE - FIN_CR - 4])       sphere(1.0, $fn = 32);
    }
}

// ============================================================
//  4. RACEWAY  --  external cable conduit, faired at both ends
// ============================================================
module raceway(rf, ra, z0, z1) {
    hull() {
        translate([R - 1.0, 0, z1]) sphere(rf, $fn = 28);
        translate([R - 1.0, 0, z0]) sphere(ra, $fn = 28);
    }
}

// ============================================================
//  5. NOZZLE  --  bell contour, revolved as a shell so the
//     expansion surface is visible from below
// ============================================================
BL = 20; BR0 = 6.2; BR1 = 11.6; BW = 0.75; BN = 22;
module bell() {
    outer = [ for (i = [BN:-1:0]) let (u = i/BN)
                [ BR0 + (BR1-BR0)*pow(u, 0.6), -1 - (BL-1)*u ] ];
    inner = [ for (i = [0:BN]) let (u = i/BN)
                [ BR0 - BW + (BR1-BR0)*pow(u, 0.6), -1 - (BL-1)*u ] ];
    rotate_extrude($fn = NSEG) polygon(concat(outer, inner));
}

// ============================================================
//  assembly
// ============================================================

// -- airframe, flush colour bands (no step, no drag) --
color(CHAR)   translate([0,0,Z_TUBE0]) cylinder(h = 14, r = R, $fn = NSEG);
color(COPPER) translate([0,0,38])      cylinder(h =  3, r = R, $fn = NSEG);
color(WHITE)  translate([0,0,41])      cylinder(h = 59, r = R, $fn = NSEG);
color(CHAR)   translate([0,0,100])     cylinder(h =  3, r = R, $fn = NSEG);
color(WHITE)  translate([0,0,103])     cylinder(h = 41, r = R, $fn = NSEG);

color(STEEL)  boat_tail();

// -- nose, banded the same way --
color(COPPER) nose_band(Z_TUBE1, Z_TUBE1 + 3);
color(WHITE)  nose_band(Z_TUBE1 + 3, 192);
color(CHAR)   nose_band(192, Z_TIP);

// -- four fins, cruciform --
for (a = [0, 90, 180, 270]) rotate([0, 0, a]) {
    color(CHAR)  translate([0, 0, FIN_ZLE]) rotate([0, 90, 0]) fin_blade();
    color(STEEL) fin_shoe();
}

// -- raceways, tucked between the fin planes --
color(WHITE) rotate([0, 0,  45]) raceway(2.0, 1.5, 30, 139);
color(WHITE) rotate([0, 0, 225]) raceway(1.4, 1.1, 44, 131);

// -- engine --
color(STEEL)  translate([0, 0, -1.8]) cylinder(h = 1.9, r1 = 6.6, r2 = 7.0, $fn = NSEG);
color(COPPER) bell();
