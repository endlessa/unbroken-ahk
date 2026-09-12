include <turbine.scad>
// ===================================================================
//  Helical Savonius -- the twisted drum
//
//  Two scoops, each a half-cylinder, wrung through a full half turn
//  over the height of the rotor.  A flat Savonius has a dead angle:
//  twice per revolution both scoops present their edges to the wind
//  and the torque falls through zero, so the thing sits there in a
//  breeze and sulks.  Wring the pair through 180 degrees and every
//  rotor angle has some slice of scoop square to the wind.  Torque
//  never crosses zero, and it will start from any position at all.
//
//  Drag-driven, so it cannot beat Betz and does not try: it runs at a
//  tip-speed ratio near 1, makes its torque at a standstill, and does
//  not care which way the wind is coming from.
// ===================================================================

RSC   = 15;          // scoop radius
OFF   = 10.5;        // scoop centre offset from the axis
WALL  = 1.2;         // sheet thickness
H     = 94;          // height
TWIST = 180;         // total wring
NU    = 120;
ROT   = OFF + RSC;   // rotor radius, 25.5
OVL   = 2*(RSC-OFF); // overlap slot the downwind scoop breathes through
RP    = ROT*1.06;    // end plate radius
ZR    = 33;          // rotor floor

echo("rotor radius", ROT, "height", H, "aspect H/D =", H/(2*ROT));
echo("overlap", OVL, "ratio e/d =", OVL/(2*RSC));
echo("swept area =", 2*ROT*H);
// The scoop's inner face never comes nearer the axis than this, which
// is what sets the largest shaft that will fit through the middle.
echo("clear bore radius =", abs(RSC-WALL-OFF));

SCOOP = [0.74, 0.30, 0.22];
BACK  = [0.40, 0.44, 0.50];
PLATE = [0.62, 0.65, 0.70];
STEEL = [0.58, 0.61, 0.66];
CAN   = [0.46, 0.50, 0.55];
DARK  = [0.30, 0.33, 0.37];
PAD   = [0.33, 0.35, 0.33];

translate([0,0,ZR]) {
    color(SCOOP) tsweep(sav_blade(RSC, OFF, WALL, H, NU, 0,   TWIST));
    color(BACK)  tsweep(sav_blade(RSC, OFF, WALL, H, NU, 180, TWIST));

    // end plates and a mid diaphragm -- a Savonius without end plates
    // spills its scoops out of the top and bottom and loses a fifth
    // of its power to it
    for (z=[-1.8, H]) color(PLATE) translate([0,0,z])
        cylinder(h=1.8, r=RP, $fn=110);
    color(STEEL) translate([0,0,-8]) cylinder(h=H+18, r=2.4, $fn=40);
    color(STEEL) translate([0,0,H+10]) cylinder(h=2.6, r1=4.2, r2=2.8, $fn=40);
}

// ---- plinth ---------------------------------------------------------
// A squat rotor wants a squat pedestal: the bottom end plate is wide
// enough to be the table top, so the base only has to be a can.
color(CAN)   translate([0,0,3.2]) cylinder(h=ZR-6.0, r1=13.0, r2=8.4, $fn=72);
color(STEEL) translate([0,0,ZR-2.8]) cylinder(h=2.8, r1=8.4, r2=6.0, $fn=64);
color(STEEL) translate([0,0,0.8]) cylinder(h=2.6, r=15.0, $fn=72);
color(DARK)  cylinder(h=1.0, r=17.5, $fn=72);
// inspection hatch, so the can reads as a machine and not a bollard
color(DARK)  translate([10.6,0,6]) rotate([90,0,90])
    linear_extrude(height=1.4) polygon([[-3.4,0],[3.4,0],[3.4,9],[-3.4,9]]);

color(PAD) translate([0,0,-2.5]) cylinder(h=2.5, r=64, $fn=120);
