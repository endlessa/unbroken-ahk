include <turbine.scad>
// ===================================================================
//  Horizontal-axis turbine -- the twist that matters
//
//  Here the wring is not for smoothness, it is for incidence.  A
//  section at radius r meets the wind at phi = atan(V / (w r)).  The
//  wind speed V is the same all down the blade but w*r is not, so phi
//  collapses from tens of degrees at the root to a few at the tip.
//  To hold one useful angle of attack the whole way out, the blade
//  must be set coarse inboard and nearly flat outboard -- 24 degrees
//  at the root here, 1.2 at the tip.  That is why a turbine blade
//  looks wrung out, and why an untwisted one stalls inboard and
//  windmills uselessly outboard at the same time.
//
//  Chord falls as s^0.72: the inboard sections have a short torque
//  arm and little speed, so they need area to contribute at all.
//  The first sixth of the span blends from a round spar stub into
//  the aerofoil, which is how the load actually gets into the hub.
// ===================================================================

R     = 64;          // tip radius
R0    = 4.0;         // blade root station
C0    = 13.0;        // root chord
CT    = 3.6;         // tip chord
B0    = 24;          // root twist, degrees from the plane of rotation
BT    = 1.2;         // tip twist
TC    = 0.21;        // root thickness ratio (halves toward the tip)
NU    = 104;
NB    = 3;

HUB   = 126;         // hub height
YAW   = 22;          // where the machine is looking
TILT  = 5;           // rotor axis nose-up, so a flexing blade clears the tower
AZ0   = 18;          // rotor position

echo("tip radius", R, "hub height", HUB, "swept area =", PI*R*R);
echo("tip clearance above ground =", HUB - R);
echo("solidity =", NB*0.5*(C0+CT)*(R-R0)/(PI*R*R));

BLADE = [0.90, 0.89, 0.85];
NAC   = [0.80, 0.80, 0.78];
TOWER = [0.72, 0.73, 0.74];
DARK  = [0.36, 0.38, 0.41];
PAD   = [0.33, 0.35, 0.33];

// nacelle: rounded tail, straight body, slight shoulder at the front
NP = concat( [[0,0]],
             [ for (i=[1:8]) let(s=i/8) [ 7.6*pow(sin(90*s), 0.60), 12*s ] ],
             [ [7.6, 32], [7.0, 41], [0, 41] ] );
// spinner: an ogive that swallows the blade roots
SP = concat( [[0,0], [7.6,0]],
             [ for (i=[1:10]) let(s=i/10) [ 7.6*pow(cos(89*s), 0.62), 21*s ] ] );

module rev_x(P, dx, n=80) translate([dx,0,0]) rotate([0,90,0])
    rotate_extrude($fn=n) polygon(P);

translate([0,0,HUB]) rotate([0,0,YAW]) rotate([0,-TILT,0]) {
    color(NAC) rev_x(NP, -28);
    color(NAC) rev_x(SP,  11);
    translate([17,0,0]) for (k=[0:NB-1]) rotate([AZ0 + 360*k/NB, 0, 0])
        color(BLADE) tsweep(hawt_blade(R0, R, NU, C0, CT, B0, BT, TC, 1.35, 0.30, 0.17, 6.6));
    // met mast on the nacelle roof, because every one of them has one
    color(DARK) translate([-20,0,6.9]) cylinder(h=11, r=0.5, $fn=16);
    color(DARK) translate([-22.5,0,17.4]) rotate([0,90,0]) cylinder(h=5, r=0.42, $fn=16);
}

// ---- yaw bearing and tower -----------------------------------------
color(DARK)  translate([0,0,HUB-11.5]) cylinder(h=3.0, r=7.2, $fn=64);
color(TOWER) cylinder(h=HUB-9.5, r1=9.8, r2=6.2, $fn=80);
color(DARK)  cylinder(h=3.2, r=11.6, $fn=80);
color(PAD)   translate([0,0,-2.5]) cylinder(h=2.5, r=96, $fn=140);
