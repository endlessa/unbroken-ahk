include <char_lib.scad>
// ===================================================================
//  GNOME -- player character
//
//  The head is the whole design.  4.2 heads tall against a human's 7.5
//  means the skull is very nearly twice the share of the figure, and no
//  amount of costume will argue with that -- it is what makes a gnome
//  read as clever-and-small rather than merely short.  A dwarf at 5.2
//  is stocky; a gnome at 4.2 is top-heavy, and the ears and nose are
//  sized to lean into it rather than apologise for it.
//
//  Distinguishing it from the dwarf is the real problem, since both are
//  short.  Three numbers do it, not the costume:
//    heads  4.2 against 5.2 -- bigger head, by a lot.
//    girth  0.88 against 1.15 -- slight where the dwarf is massive.
//    shld   0.124 against 0.172 -- narrow-shouldered, so the figure
//           tapers upward instead of squaring off.
//  The dwarf is a block; the gnome is a lightbulb.
//
//  The hat is counted separately from the profile: the BODY is 1.05 m
//  and the hat carries the silhouette to about 1.28.  Worth stating,
//  because a height figure that quietly includes headgear is not a
//  height figure.
// ===================================================================

P = prof(H     = 1.050,
         heads = 4.20,
         shld  = 0.124,
         hipf  = 0.440,
         girth = 0.88,
         armf  = 0.330,
         lean  = 2,
         dep   = 0.66);

SKIN = [0.780, 0.575, 0.460];
SKN2 = [0.650, 0.455, 0.360];
HAIR = [0.760, 0.745, 0.715];   // white, and the second-brightest thing here
BEARD= [0.800, 0.788, 0.760];
MAIL = [0.430, 0.400, 0.360];
PLATE= [0.585, 0.600, 0.625];
STEEL= [0.720, 0.735, 0.760];
LTHR = [0.400, 0.285, 0.190];
LTH2 = [0.285, 0.200, 0.135];
CLOTH= [0.620, 0.225, 0.180];   // the hat, and it runs through the tunic
CLT2 = [0.330, 0.300, 0.255];
BRASS= [0.830, 0.640, 0.235];
GLASS= [0.420, 0.620, 0.600];
EYEW = [0.895, 0.885, 0.855];
EYED = [0.130, 0.110, 0.095];

JAW = 1.04; BROW = 1.02; SLIM = 1.02;
MUZ = 0;
NP  = 0.26;   // nose projection, in head-heights
HH  = pHH(P);

module face() {
    // Features are placed against the skull surface INTERPOLATED to their
    // own height, never against one station's figure.  On the orc
    // stations 3 and 4 are 4 cm apart, which is the whole difference
    // between an eye that reads and one buried in the head.
    ZB = head_z(P, 4); ZJ = head_z(P, 3); ZC = head_z(P, 2);
    FJ = head_front(P, 3, JAW, BROW, MUZ);
    WJ = head_half(P, 3, JAW, BROW, SLIM);
    FE = face_x(P, face_t(-0.060), JAW, BROW, MUZ);
    WE = face_w(P, face_t(-0.060), JAW, BROW, SLIM);
    color(SKIN) smesh(head_G(P, 0, JAW, BROW, MUZ, SLIM));
    // No brow ridge on any species.  It was there because this renderer
    // has no shadows, so overhanging the socket is the only way to sink
    // an eye -- but a lens on a curved skull always reads as a separate
    // object under flat shading, and on every one of these faces it came
    // out as a bar laid across the forehead.  The skull's own BROW knob
    // still widens the supraorbital station, which was the part actually
    // doing work.
    //
    // Nose.  The spine descends MONOTONICALLY -- an earlier version had
    // station 1 below station 2, so the sweep doubled back and collapsed
    // into a stub between the eyes.  NP is how far the tip stands proud
    // of the skull, in head-heights.
    color(SKIN) smesh(limb_G(
        [ tipH(P,[face_x(P,face_t(-0.03),JAW,BROW,MUZ) - 0.02*HH,    0, ZB - 0.03*HH]),
          tipH(P,[face_x(P,face_t(-0.11),JAW,BROW,MUZ) + 0.35*NP*HH, 0, ZB - 0.11*HH]),
          tipH(P,[face_x(P,face_t(-0.19),JAW,BROW,MUZ) + 1.00*NP*HH, 0, ZB - 0.19*HH]),
          tipH(P,[face_x(P,face_t(-0.27),JAW,BROW,MUZ) + 0.55*NP*HH, 0, ZB - 0.27*HH]) ],
        [ 0.055*HH, 0.088*HH, 0.125*HH, 0.112*HH ], [ 0.058*HH, 0.098*HH, 0.140*HH, 0.126*HH ],
        [ 2.6, 2.7, 2.8, 2.8 ], [0,1,0], 14));
    both_y() {
        // Level with the TOP QUARTER of the nose.  The nose runs ZB-0.03
        // to ZB-0.27, so its top quarter centres on ZB-0.06; at ZB-0.20
        // the eyes sat level with the nose TIP and the whole bridge stood
        // above them, which reads as a nose mounted over the eyes rather
        // than between them.  The eye sits just proud of the surface and
        // the pupil just proud of the eye.
        color(EYEW) translate(tipH(P, [FE*0.99, WE*0.40, ZB - 0.06*HH]))
            scale([0.55,1,1]) sphere(r = 0.074*HH, $fn = 14);
        color(EYED) translate(tipH(P, [FE*1.06, WE*0.42, ZB - 0.065*HH]))
            scale([0.45,1,1]) sphere(r = 0.042*HH, $fn = 12);
        // Ears swept UP and out.  The elf's are the same part with the
        // rise and sweep reversed -- there the point travels back along
        // the skull, here it goes up and away from it.
        color(SKIN) smesh(ear_G(P, ZJ + 0.10*HH, 0.36, 0.30, 0.30, 0.10, 0.085, 0.022));
    }
}

// Hair only at the nape and temples: the hat takes the crown, so a full
// shell would just be buried under it and cost triangles for nothing.
module hair() {
    color(HAIR) hair_shell(P, 2, 5, 0.011*HH, 10, 2, JAW, BROW, 0, SLIM);
    color(HAIR) translate(tipH(P, [-0.22*HH, 0, pCHIN(P) + 0.22*HH]))
        scale([0.70, 1.00, 1.05]) sphere(r = 0.42*HH, $fn = 22);
}

// The pointed hat.  A cone alone reads as a traffic marker, so it gets a
// brim and a slump: the tip translates forward as well as up, which is
// the difference between a hat that is worn and a hat that is balanced.
module hat() {
    b = pCHIN(P) + 0.86*HH;
    color(CLOTH) translate(tipH(P, [-0.02*HH, 0, b]))
        rotate([0, 8, 0]) cylinder(h = 0.92*HH, r1 = 0.50*HH, r2 = 0.045*HH, $fn = 26);
    color(CLOTH) translate(tipH(P, [-0.02*HH, 0, b - 0.03*HH]))
        scale([1.12, 1.12, 0.30]) sphere(r = 0.52*HH, $fn = 26);
    color(BRASS) translate(tipH(P, [-0.02*HH, 0, b + 0.05*HH]))
        scale([1.06, 1.06, 0.22]) sphere(r = 0.50*HH, $fn = 26);
}

// A short spade beard -- present, but a fraction of the dwarf's drop, so
// the two short species never read as the same silhouette.
module beard() {
    both_y() {
        color(BEARD) smesh(brd_G(P, 0.78, 0.34, 0.20));
        color(BEARD) translate(tipH(P, [0.44*HH, 0.145*HH, pCHIN(P) + 0.16*HH]))
            rotate([0, 14, -18]) scale([0.70, 1.30, 0.48]) sphere(r = 0.180*HH, $fn = 16);
    }
}

// Tinker's apron and a lens on a headband: the costume says what the
// proportion cannot, which is what this one does for a living.
module apron() {
    color(LTHR) sband(tor_G(P, 0.024*pH(P)/1.358), 1, 3);
}
module lens() {
    color(BRASS) translate(tipH(P, [0.20*HH, 0.30*HH, pCHIN(P) + 0.60*HH]))
        rotate([0, 78, 0]) cylinder(h = 0.16*HH, r = 0.145*HH, center = true, $fn = 18);
    color(GLASS) translate(tipH(P, [0.27*HH, 0.30*HH, pCHIN(P) + 0.60*HH]))
        rotate([0, 78, 0]) cylinder(h = 0.03*HH, r = 0.120*HH, center = true, $fn = 18);
}

body(P, SKIN, CLOTH, CLT2, MAIL, LTHR, LTH2, PLATE, STEEL, BRASS);
apron();
face();
hair();
hat();
beard();
lens();
