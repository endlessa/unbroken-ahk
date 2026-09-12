include <gears.scad>
// ===================================================================
//  Spiral bevel pair, 2:1, 90 degree shafts
//
//  Phasing: the two members touch along the shared cone element, which
//  lies at azimuth 0 in the wheel's own frame and -- after
//  rotate([0,90,0]) swings the pinion's axis onto +x, mapping its
//  local +x to world -z -- at azimuth 180 in the pinion's.  So the
//  wheel wants a tooth centreline there and the pinion a space, which
//  is half a pitch further round.  The spiral offset is zero at
//  mid-face by construction, so those two conditions are enough.
// ===================================================================

M    = 3.0;
Z1   = 34;           // wheel
Z2   = 17;           // pinion
PSI  = 35;           // mean spiral angle -- the Gleason standard
AL   = 20;

G1   = bev_gamma(Z1,Z2);          // 63.435
G2   = bev_gamma(Z2,Z1);          // 26.565
LO   = bev_Lo(M,Z1,G1);           // both members share this
LI   = 0.68*LO;                   // face width 0.32 Lo, inside the Lo/3 rule
LM   = (LO+LI)/2;
RC   = 0.99*LM;                   // face-mill cutter radius
LH1  = 0.32*LO;                   // where the wheel dish meets its hub
LH2  = 0.30*LO;
T1   = 6.0;                       // dish thickness
T2   = 6.0;
ZB1  = 9;                         // wheel back face -- kept clear of the
ZB2  = 8;                         // pinion shaft, which runs through the
RH1  = 13; RH2 = 10;              // apex on its way out the far side

PH1  = 0;                         // ph places a TOOTH centreline, so the
PH2  = 180 + 180/Z2;              // wheel gets a tooth on the common cone
                                  // element and the pinion a space -- half
                                  // a pitch past its own azimuth 180

echo("cone angles", G1, G2, " sum =", G1+G2);
echo("Lo", LO, "Li", LI, "face width", LO-LI, "=", (LO-LI)/LO, "of Lo");
echo("cutter radius", RC, "rho =", sb_rho(LM,RC,PSI));
echo("face contact ratio wheel  =",
     sb_face_ratio(LO,LI,LM,sb_rho(LM,RC,PSI),RC,G1,Z1));
echo("face contact ratio pinion =",
     sb_face_ratio(LO,LI,LM,sb_rho(LM,RC,PSI),RC,G2,Z2));
echo("ratio =", Z1/Z2);

WHEEL = [0.82, 0.68, 0.34];
PIN   = [0.46, 0.62, 0.72];
BODY  = [0.46, 0.49, 0.54];
SHAFT = [0.34, 0.37, 0.41];

// ---- wheel: axis +z, left hand -------------------------------------
color(WHEEL) bevel_spiral(M, Z1, G1, LO, LI, RC, PSI, AL, PH1,  1);
color(BODY)  bev_blank(M, Z1, G1, LO, LH1, T1, RH1, ZB1, 4.5);
// the wheel is driven from ABOVE: its shaft cannot come down through
// the apex, because that is where the pinion's shaft goes
color(SHAFT) translate([0,0,ZB1]) cylinder(h=46, r=8.5, $fn=56);

// ---- pinion: axis +x, right hand -----------------------------------
rotate([0,90,0]) {
    color(PIN)   bevel_spiral(M, Z2, G2, LO, LI, RC, PSI, AL, PH2, -1);
    color(BODY)  bev_blank(M, Z2, G2, LO, LH2, T2, RH2, ZB2, 3.5);
    color(SHAFT) translate([0,0,-46]) cylinder(h=46+ZB2, r=6.5, $fn=56);
}
