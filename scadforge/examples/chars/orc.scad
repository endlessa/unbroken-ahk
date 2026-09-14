include <char_lib.scad>
// ===================================================================
//  ORC -- player character
//
//  The tallest of the five and the only one that does not stand up
//  straight.  lean = 9 degrees pivots the trunk, arms and head about
//  the hip while the legs stay plumb, which is why the profile carries
//  a lean term at all: leaning by translating the torso just puts the
//  figure in a hole.
//
//  armf = 0.265 is the other half of it.  That is fingertip height as a
//  fraction of standing height, and at 0.265 against a human's 0.350
//  the knuckles hang near the knee.  Long arms plus a forward lean is
//  the whole posture, and it does more than the tusks do.
//
//  Mass: girth 1.25 on shld 0.147 puts this figure 0.94 m across the
//  arms, by far the widest here in absolute terms.  But the DWARF is
//  still the broader of the two relative to its own height -- 0.53
//  against 0.46 -- and that is the right way round.  Broad-for-its-size
//  is the dwarf's signature and the orc must not steal it; the orc's
//  claim is absolute scale, a whole dwarf's breadth again on a figure
//  half as tall once more.  The first pass had shld 0.176 and girth
//  1.32, which measured 1.14 m across, beat the dwarf's ratio outright,
//  and made the two read as the same idea at different sizes.
//
//  The skull uses head_G's shape knobs rather than its own geometry:
//  jaw 1.34 widens the mandible stations, brow 1.30 the supraorbital
//  one, and muzzle 0.40 pushes the lower face forward into a
//  prognathic jaw.  Same eight stations as every other species.
// ===================================================================

P = prof(H     = 2.050,
         heads = 6.50,
         shld  = 0.147,
         hipf  = 0.470,
         girth = 1.25,
         armf  = 0.265,
         lean  = 9,
         dep   = 0.72);

SKIN = [0.425, 0.520, 0.395];   // grey-green
SKN2 = [0.330, 0.415, 0.310];
HAIR = [0.155, 0.140, 0.130];
MAIL = [0.395, 0.400, 0.420];
PLATE= [0.545, 0.550, 0.575];
STEEL= [0.600, 0.605, 0.625];
LTHR = [0.400, 0.300, 0.215];
LTH2 = [0.235, 0.175, 0.130];
CLOTH= [0.255, 0.205, 0.160];   // dark hide, well below the plate
CLT2 = [0.195, 0.160, 0.130];
BRASS= [0.720, 0.545, 0.215];
BONE = [0.880, 0.860, 0.790];
EYEW = [0.880, 0.820, 0.560];
EYED = [0.140, 0.110, 0.090];

JAW = 1.34; BROW = 1.30; MUZ = 0.40; SLIM = 1.00;
HH  = pHH(P);

module face() {
    FB = head_front(P, 4, JAW, BROW, MUZ);   // brow line, front surface
    FJ = head_front(P, 3, JAW, BROW, MUZ);   // mid-face, front surface
    ZB = head_z(P, 4); ZJ = head_z(P, 3); ZC = head_z(P, 2);
    WJ = head_half(P, 3, JAW, BROW, SLIM);
    color(SKIN) smesh(head_G(P, 0, JAW, BROW, MUZ, SLIM));
    // Brow ridge, sat ON the surface.  With no shadows in this renderer
    // a deep-set eye can only be made by physically overhanging it.
    color(SKIN) translate(tipH(P, [FB*0.86, 0, ZB]))
        scale([0.64, 1.30, 0.36]) sphere(r = 0.285*HH, $fn = 20);
    // Nose: swept from the brow down to the lip so it keeps an edge
    // along the bridge instead of going soft.
    color(SKIN) smesh(limb_G(
        [ tipH(P,[FB*0.74, 0, ZB - 0.04*HH]), tipH(P,[FB*0.92, 0, ZB - 0.30*HH]),
          tipH(P,[FJ*0.96, 0, ZJ + 0.16*HH]), tipH(P,[FJ*0.90, 0, ZJ + 0.02*HH]) ],
        [ 0.060*HH, 0.090*HH, 0.115*HH, 0.105*HH ], [ 0.052*HH, 0.076*HH, 0.096*HH, 0.088*HH ],
        [ 2.6, 2.7, 2.8, 2.8 ], [0,1,0], 14));
    both_y() {
        color(EYEW) translate(tipH(P, [FB*0.86, WJ*0.40, ZB - 0.20*HH]))
            scale([0.55,1,0.85]) sphere(r = 0.085*HH, $fn = 14);
        color(EYED) translate(tipH(P, [FB*0.99, WJ*0.43, ZB - 0.21*HH]))
            scale([0.45,1,1]) sphere(r = 0.046*HH, $fn = 12);
        color(SKN2) smesh(ear_G(P, ZJ + 0.06*HH, 0.40, 0.16, 0.14, 0.20, 0.060, 0.018));
        // Lower tusks, rising past the upper lip.  Short and thick --
        // long thin ones read as a boar, and the jaw already carries the
        // weight of this face.
        // Sized against the skull, not against a fraction of head
        // height: at muzzle 0.40 the chin is 5 cm proud of the brow, so
        // a tusk that looked right on a flat face disappears here.
        color(BONE) translate(tipH(P, [head_front(P,2,JAW,BROW,MUZ)*0.80, WJ*0.44, ZC + 0.10*HH]))
            rotate([0, -22, -8]) cylinder(h = 0.62*HH, r1 = 0.090*HH, r2 = 0.028*HH, $fn = 12);
    }
}

// Topknot: the hair shell is cut short at the sides so the skull stays
// massive, and the height goes into a bound tail instead.
module hair() {
    color(HAIR) hair_shell(P, 4, 7, 0.011*HH, 10, 2, JAW, BROW, MUZ, SLIM);
    color(LTH2) translate(tipH(P, [-0.10*HH, 0, pCHIN(P) + 0.95*HH]))
        rotate([0,0,0]) cylinder(h = 0.10*HH, r = 0.115*HH, center = true, $fn = 14);
    for (i = [0:5]) let( t = i/5 )
        color(HAIR) translate(tipH(P, [ (-0.12 - 0.34*t)*HH, 0,
                                       pCHIN(P) + (1.02 - 0.16*t*t)*HH ]))
            scale([1.0, 0.72, 0.72]) sphere(r = (0.105 - 0.055*t)*HH, $fn = 12);
}

// Trapezius: a slab bridging neck to shoulder on each side.  On a figure
// leaning this far forward the neck would otherwise read as a gap, and
// filling it is most of what makes the posture look powerful rather
// than merely stooped.
module traps() {
    both_y() color(SKIN) translate(tip(P, [-0.06*HH, 0.42*HH, pSHZ(P) + 0.10*HH]))
        rotate([0, -14, 0]) scale([0.90, 1.25, 0.52]) sphere(r = 0.40*HH, $fn = 18);
}

module harness() {
    S = pH(P)/1.358;
    color(LTHR) sband(tor_G(P, 0.026*S), 3, 5);
    color(STEEL) translate(tip(P, [pSH(P)*0.62, 0, pSHZ(P) - 0.30*(pSHZ(P)-pHIP(P))]))
        scale([0.4, 1, 1]) sphere(r = 0.20*HH, $fn = 20);
}

body(P, SKIN, CLOTH, CLT2, MAIL, LTHR, LTH2, PLATE, STEEL, BRASS);
harness();
pauldrons(P, PLATE, STEEL, BRASS, 0.078, 0.098, 0.016);
traps();
face();
hair();
