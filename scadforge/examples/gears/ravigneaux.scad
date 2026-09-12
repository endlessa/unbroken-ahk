include <gears.scad>
// ===================================================================
//  Ravigneaux compound planetary
//
//  One ring, one carrier, TWO suns and TWO sets of pinions, all on one
//  axis.  It is the gearset in most four-speed automatics, and it is
//  compound in the strict sense: no single planet carries the whole
//  path from input to output.
//
//      large sun  <->  LONG pinions  <->  ring
//      small sun  <->  SHORT pinions <->  long pinions
//
//  "Long" and "short" are AXIAL lengths, not diameters.  The long
//  pinions run the full face and are present in both gear planes; the
//  short ones live only in the small sun's plane.  That is what makes
//  the packing work: the large sun would foul the short pinions, so it
//  is put in the plane where they are not.
//
//  Four meshes have to be clocked at once, and the last one does not
//  come free.  Walk the chain -- large sun fixes each long pinion, each
//  long pinion fixes the ring, and also fixes its own short pinion,
//  and the short pinion fixes the small sun -- and the small sun comes
//  out wanting a DIFFERENT phase from each of the three sets unless
//  the tooth counts are right.  With 30/18/21/18 the three answers
//  agree; change the short pinion to 18 teeth and they land exactly a
//  third of a pitch apart, and the third set will not go in.
//
//  Meshing conditions, as one scalar each.  For two externals whose
//  centres lie along azimuth b, with PhA and PhB the azimuths of a
//  tooth centreline on each:
//
//      zA(PhA-b)/360 + zB(PhB-b-180)/360  ==  1/2   (mod 1)
//
//  and for a planet inside a ring, with no 180 because the contact is
//  on the same side:
//
//      zR(PhR-b)/360 + zP(PhP-b)/360      ==  1/2   (mod 1)
//
//  Both say the same thing: a tooth of one lands on a space of the
//  other. Solve each for the unknown phase and the whole train falls
//  out of four lines of arithmetic.
// ===================================================================

M    = 2.0;
ZSL  = 30;      // large sun   -- meshes the long pinions
ZPL  = 18;      // long pinions
ZPS  = 21;      // short pinions
ZSS  = 18;      // small sun   -- meshes the short pinions
ZR   = ZSL + 2*ZPL;             // 66 -- forced, so the long pinion fits both
NP   = 3;

A1   = M*(ZSL+ZPL)/2;           // long pinion orbit   48
A2   = M*(ZSS+ZPS)/2;           // short pinion orbit  39
A4   = M*(ZPL+ZPS)/2;           // pinion to pinion    39
// the three centres form a triangle, which fixes the angle between a
// long pinion and its own short one
DEL  = acos((A1*A1 + A2*A2 - A4*A4)/(2*A1*A2));
G0   = atan2(A2*sin(DEL), A2*cos(DEL) - A1);   // long pinion -> short pinion

HF   = 26;      // full face (long pinions, ring)
HH   = 12;      // half face
// The small sun and its short pinions go in the UPPER plane and the
// large sun in the lower one. Either way round works mechanically --
// what matters is only that the large sun and the short pinions are
// never in the same plane, because the large sun's tip reaches well
// inside where the short pinions orbit.

echo("ring teeth", ZR, " (forced: zSL + 2*zPL)");
echo("orbits: long", A1, " short", A2, " pinion centres", A4, " delta", DEL);
echo("hold ring, drive small sun -> carrier :", 1 + ZR/ZSS);
echo("hold ring, drive large sun -> carrier :", 1 + ZR/ZSL);
echo("hold carrier, small sun in, ring out  :", -ZR/ZSS);
echo("small sun tip", M*ZSS/2+M, "< long pinion orbit inner",
     A1 - M*ZPL/2 - M);
echo("short pinion tip reach", A2 + M*ZPS/2 + M, "< ring tip", M*ZR/2 - M);

// ---- phases --------------------------------------------------------
// Ph is the azimuth of a tooth CENTRELINE. gear_poly and ring_poly both
// put one at 90-180/z, so ph = Ph - 90 + 180/z turns it into the
// rotation those modules take.
function to_ph(Ph, z) = Ph - 90 + 180/z;

PHSL = 0;                                   // datum: free choice
function ph_pl(psi) = psi + 180 + (360/ZPL)*(0.5 - ZPL*0 - ZSL*(PHSL-psi)/360);
function ph_r(psi)  = psi       + (360/ZR )*(0.5 - ZPL*(ph_pl(psi)-psi)/360);
function ph_ps(psi) = let(g = psi + G0)
                      g + 180   + (360/ZPS)*(0.5 - ZPL*(ph_pl(psi)-g)/360);
function ph_ss(psi) = let(b = psi + DEL)
                      b         + (360/ZSS)*(0.5 - ZPS*(ph_ps(psi)-b-180)/360);

echo("ring phase from each set :", [ for (i=[0:NP-1]) ph_r(360*i/NP) % (360/ZR) ]);
echo("small sun from each set  :", [ for (i=[0:NP-1]) ph_ss(360*i/NP) % (360/ZSS) ]);

RING  = [0.54, 0.57, 0.62];
LONG  = [0.86, 0.70, 0.34];
SHORT = [0.40, 0.62, 0.70];
SUNS  = [0.80, 0.42, 0.30];
SUNL  = [0.55, 0.44, 0.68];
CARR  = [0.30, 0.33, 0.38];
PIN   = [0.38, 0.41, 0.46];

// ---- the train -----------------------------------------------------
color(RING) ring(M, ZR, HF, M*ZR/2 + 8, 20, 0, to_ph(ph_r(0), ZR));

color(SUNL) spur(M, ZSL, HH, 20, 0, to_ph(PHSL, ZSL));
color(SUNS) translate([0,0,HF-HH]) spur(M, ZSS, HH, 20, 0, to_ph(ph_ss(0), ZSS));

for (i=[0:NP-1]) {
    psi = 360*i/NP;
    translate(A1*[cos(psi), sin(psi), 0])
        color(LONG) spur(M, ZPL, HF, 20, 0, to_ph(ph_pl(psi), ZPL));
    translate(A2*[cos(psi+DEL), sin(psi+DEL), 0] + [0,0,HF-HH])
        color(SHORT) spur(M, ZPS, HH, 20, 0, to_ph(ph_ps(psi), ZPS));
}

// ---- carrier and shafts --------------------------------------------
color(CARR) translate([0,0,-7]) cylinder(h=5, r=A2 + M*ZPS/2 + 3, $fn=120);
for (i=[0:NP-1]) {
    psi = 360*i/NP;
    color(PIN) translate(A1*[cos(psi), sin(psi), 0] + [0,0,-2])
        cylinder(h=HF+7, r=4.6, $fn=32);
    color(PIN) translate(A2*[cos(psi+DEL), sin(psi+DEL), 0] + [0,0,HF-HH-3])
        cylinder(h=HH+8, r=4.6, $fn=32);
}
color(CARR) translate([0,0,-26]) cylinder(h=19, r=10, $fn=64);  // carrier out
color(SUNL) translate([0,0,-26]) cylinder(h=26, r=5.4, $fn=48); // large sun in
color(SUNS) translate([0,0,HF]) cylinder(h=16, r=7.0, $fn=64);  // small sun in
