include <gears.scad>
// ===================================================================
//  GEARED SPHERE -- a spherical differential.
//  Two hemispheres, four mitre-family pinions on radial axes round the
//  equator.  Turn either hemisphere and the pinions walk the other one
//  the opposite way: the halves rotate separately, through each other.
// ===================================================================
R   = 37;                       // sphere radius: set just clear of the
                                // outer tooth tips, so the gears sit AT the
                                // surface instead of sunk behind a rim
ZS  = 20; ZP = 14;              // side gear / pinion teeth
GS  = bev_gamma(ZS,ZP);         // side gear cone angle
GP  = 90 - GS;                  // pinion cone angle -- complementary
LO  = 34;                       // outer cone distance, shared by all
LI  = 0.60*LO;                  // inner cone distance of the TEETH
LIB = 0.50*LO;                  // the body runs deeper than the teeth do:
                                // if both stop at the same cone distance
                                // their end faces are coplanar and the
                                // gear centre fills with z-fighting
M   = 2*LO*sin(GS)/ZS;          // module follows from the cone distance
NP  = 4;                        // pinions

echo("side gear cone", GS, "pinion cone", GP, "sum", GS+GP);
echo("module", M, "Lo(side)", bev_Lo(M,ZS,GS), "Lo(pinion)", bev_Lo(M,ZP,GP));
echo("pinion angular half width", GP, "< half spacing", 180/NP);

TOP = [0.88,0.73,0.36]; BOT = [0.70,0.76,0.83];
PIN = [0.93,0.88,0.74]; CAGE= [0.44,0.47,0.53];

THC = 54;                       // cap reaches this colatitude
EXPLODE = 0;                    // hemispheres lifted apart when > 0
// -- upper hemisphere
translate([0,0,EXPLODE]) color(TOP)
    { bev_body(R, THC, GS, LO, LIB); bevel(M, ZS, GS, LO, LI); }
// -- lower hemisphere: the mirror image, which is what makes it mesh
translate([0,0,-EXPLODE]) color(BOT) mirror([0,0,1])
    { bev_body(R, THC, GS, LO, LIB); bevel(M, ZS, GS, LO, LI); }
// -- pinions on radial axes
for (k=[0:NP-1]) rotate([0,0,360*k/NP]) rotate([0,90,0]) color(PIN) {
    bev_body(R, 33, GP, LO, LIB);
    bevel(M, ZP, GP, LO, LI);
}
// -- carrier: four meridian ribs, passing between the pinions
if (EXPLODE == 0) for (k=[0:NP-1]) color(CAGE)
    rotate([0,0,45 + 360*k/NP]) rotate([90,0,0]) rotate([0,0,-46])
        rotate_extrude(angle=92, $fn=120) translate([R-0.6,0]) circle(r=2.3, $fn=24);
