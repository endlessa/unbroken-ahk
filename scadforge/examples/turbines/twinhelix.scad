include <turbine.scad>
// ===================================================================
//  Counter-rotating twin helical rotor
//
//  Two Gorlov rotors on one mast, wrung opposite ways and geared to
//  turn against each other.  Two things fall out of that:
//
//    * The reaction torque cancels.  A single vertical-axis rotor
//      tries to wring its own mast round, and the foundation has to
//      eat it.  Two rotors of equal solidity turning opposite ways
//      hand the mast nothing but their weight.
//
//    * The generator sees the sum of both speeds.  Drive the stator
//      from one rotor and the rotor from the other and the relative
//      speed doubles, which is the whole reason to build the machine
//      tall and thin rather than short and fat.
//
//  The lower rotor is set half a blade pitch out of phase with the
//  upper one (60 degrees for three blades), so the two torque ripples
//  interleave rather than add.
// ===================================================================

R     = 26;
H     = 60;          // per rotor
N     = 3;
C     = 9.2;
TC    = 0.18;
PITCH = -3;
NU    = 84;
WRAP  = 360/N;

Z1    = 44;          // lower rotor floor
GAP   = 13;          // gearbox between the two
Z2    = Z1 + H + GAP;
RS    = 3.8;

echo("rotor radius", R, "per-rotor height", H, "total swept =", 2*2*R*H);
echo("phase offset between rotors =", WRAP/2);

UP    = [0.88, 0.74, 0.39];
DN    = [0.42, 0.62, 0.72];
STEEL = [0.60, 0.63, 0.68];
SHAFT = [0.44, 0.47, 0.52];
CAN   = [0.50, 0.54, 0.58];
DARK  = [0.28, 0.30, 0.34];
PAD   = [0.33, 0.35, 0.33];

// sgn = +1 wraps one way, -1 the other; ph is the rotor's own phase
module rotor(z0, sgn, ph, col) {
    translate([0,0,z0]) rotate([0,0,ph]) {
        for (k=[0:N-1]) color(col)
            tsweep(vawt_blade(R, H, NU, 360*k/N, sgn*WRAP, C, TC, sgn*PITCH, 0.08));
        for (k=[0:N-1]) {
            color(STEEL) arm(5.4, R, 3.0,   9.4, 0.26, 360*k/N, 0.38);
            color(STEEL) arm(5.4, R, H-3.0, 9.4, 0.26, 360*k/N + sgn*WRAP, 0.38);
        }
        for (z=[3.0, H-3.0]) color(STEEL) translate([0,0,z-5])
            cylinder(h=10, r=6.0, $fn=48);
    }
}

rotor(Z1,       1,  0,        UP);
rotor(Z1+H+GAP, -1, WRAP/2,   DN);

// ---- shaft, gearbox, mast ------------------------------------------
color(SHAFT) translate([0,0,Z1-9]) cylinder(h=2*H+GAP+18, r=RS, $fn=56);
color(SHAFT) translate([0,0,Z2+H+9]) cylinder(h=3, r1=6.0, r2=3.6, $fn=48);
// the reversing box that ties the two rotors together
color(CAN)   translate([0,0,Z1+H+1.5]) cylinder(h=GAP-3, r=8.0, $fn=64);
color(DARK)  translate([0,0,Z1+H+0.6]) cylinder(h=1.4, r=8.8, $fn=64);
color(DARK)  translate([0,0,Z1+H+GAP-2.0]) cylinder(h=1.4, r=8.8, $fn=64);

color(CAN)   translate([0,0,Z1-20]) cylinder(h=16, r=8.6, $fn=64);
color(DARK)  translate([0,0,Z1-22]) cylinder(h=2.2, r=9.4, $fn=64);
color(STEEL) cylinder(h=Z1-20, r1=10.6, r2=7.6, $fn=72);

for (k=[0:2]) color(STEEL) rotate([0,0,50+120*k]) {
    leg();
    translate([34,0,0]) cylinder(h=3.2, r=6.2, $fn=40);
}
module leg() {
    P = [ for (u=[0:14]) let(s=u/14) [ 8.6 + 26*s, 0, 24 - 22*pow(s, 1.4) ] ];
    D = [ for (u=[0:14]) let(s=u/14) 3.2 - 1.0*s ];
    tsweep([ for (u=[0:14])
        let( t = u==14 ? P[14]-P[13] : P[u+1]-P[u], tn = t/norm(t),
             sx = [ -tn[2], 0, tn[0] ] )
        [ for (i=[0:21]) let(a = -360*i/22)
            P[u] + D[u]*(cos(a)*sx + sin(a)*[0,1,0]) ] ]);
}

color(PAD) translate([0,0,-2.5]) cylinder(h=2.5, r=86, $fn=120);
