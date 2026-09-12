include <turbine.scad>
// ===================================================================
//  Gorlov helical vertical-axis turbine
//
//  Three aerofoil blades, each wrapped through 120 degrees, so that
//  the three of them together occupy every azimuth at every instant.
//  That is the trick: a straight-bladed Darrieus has to be spun up by
//  its generator and hammers its bearings once per blade per turn,
//  while this one starts in a breeze and pulls smoothly.
//
//  Blades are set 3 degrees nose-in.  A vertical-axis blade sees the
//  apparent wind from inboard through most of the upwind pass, and a
//  little negative pitch puts the useful part of the lift curve where
//  the blade actually spends its time.
// ===================================================================

R     = 30;          // radius to the quarter-chord
H     = 84;          // swept height
N     = 3;           // blades
C     = 10.6;        // chord
TC    = 0.18;        // thickness / chord  (NACA 0018, a stiff section)
PITCH = -3;          // preset, degrees nose-in
NU    = 108;         // sections per blade
WRAP  = 360/N;       // each blade covers one Nth of a turn

ZR    = 58;          // height of the rotor's bottom rim
RS    = 4.0;         // shaft radius
SPIN  = 8;          // rotor position, degrees -- purely for the picture

echo("R", R, "H", H, "blades", N, "wrap/blade", WRAP);
echo("solidity Nc/2piR =", N*C/(2*PI*R));
echo("swept area =", 2*R*H);
echo("helix angle from vertical =", atan(2*PI*R*(WRAP/360)/H));

BLADE = [0.88, 0.74, 0.39];
STEEL = [0.60, 0.63, 0.68];
SHAFT = [0.44, 0.47, 0.52];
DARK  = [0.28, 0.30, 0.34];
CAN   = [0.50, 0.54, 0.58];
PAD   = [0.33, 0.35, 0.33];

// ---- rotor ---------------------------------------------------------
translate([0,0,ZR]) rotate([0,0,SPIN]) {
    for (k=[0:N-1]) color(BLADE)
        tsweep(vawt_blade(R, H, NU, 360*k/N, WRAP, C, TC, PITCH, 0.07));

    // arms, at the azimuth each blade end actually reaches
    for (k=[0:N-1]) {
        color(STEEL) arm(6, R, 3.5,   10.5, 0.26, 360*k/N, 0.38);
        color(STEEL) arm(6, R, H-3.5, 10.5, 0.26, 360*k/N + WRAP, 0.38);
    }

    // shaft and its hub collars
    color(SHAFT) translate([0,0,-12]) cylinder(h=H+18, r=RS, $fn=56);
    for (z=[3.5, H-3.5]) color(STEEL) translate([0,0,z-5.5])
        cylinder(h=11, r=6.6, $fn=48);
    color(STEEL) translate([0,0,H+6]) cylinder(h=3, r1=6.6, r2=4.2, $fn=48);
}

// ---- mast, machine can, foot ---------------------------------------
color(CAN)   translate([0,0,ZR-24]) cylinder(h=17, r=8.8, $fn=64);
color(DARK)  translate([0,0,ZR-7]) cylinder(h=3.0, r=7.6, $fn=64);
color(DARK)  translate([0,0,ZR-26.5]) cylinder(h=2.6, r=9.6, $fn=64);
color(STEEL) cylinder(h=ZR-24, r1=10.2, r2=7.6, $fn=64);

// tripod feet: a leg is just a thin cone laid along its own line
for (k=[0:2]) {
    az = 60 + 120*k;
    color(STEEL) rotate([0,0,az]) {
        hull_free_leg();
        translate([36,0,0]) cylinder(h=3.2, r=6.5, $fn=40);
    }
}
module hull_free_leg() {
    // from the mast at z=27 out to the pad at z=2 -- built as a swept
    // round section so there is no boolean anywhere in the model
    P = [ for (u=[0:14]) let(s=u/14)
            [ 8.2 + 28*s, 0, 27 - 25*pow(s, 1.4) ] ];
    D = [ for (u=[0:14]) let(s=u/14) 3.4 - 1.1*s ];
    tsweep([ for (u=[0:14])
        let( t = u==14 ? P[14]-P[13] : P[u+1]-P[u],
             tn = t/norm(t),
             sx = [ -tn[2], 0, tn[0] ] )     // in-plane normal, y is free
        [ for (i=[0:23]) let(a = -360*i/24)
            P[u] + D[u]*(cos(a)*sx + sin(a)*[0,1,0]) ] ]);
}

// ---- pad -----------------------------------------------------------
color(PAD) translate([0,0,-2.5]) cylinder(h=2.5, r=78, $fn=120);
