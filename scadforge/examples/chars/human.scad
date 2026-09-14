include <char_lib.scad>
// ===================================================================
//  HUMAN -- player character
//
//  The baseline, and deliberately so: every other species in this set
//  is described as a departure from these numbers, so this one has to
//  be the unremarkable middle.  7.5 heads, hip at half of standing
//  height, shoulder half-breadth 0.126 of height, girth 1.0.  Those are
//  the figures the gnome, dwarf, elf and orc are all measured against.
//
//  With nothing exaggerated to carry it, a human has to be read through
//  costume and stance instead -- so the silhouette work goes into the
//  tabard's hem, the pauldrons and the boot line rather than the body.
// ===================================================================

P = prof(H     = 1.780,
         heads = 7.50,
         shld  = 0.126,
         hipf  = 0.500,
         girth = 1.00,
         armf  = 0.350,
         lean  = 0,
         dep   = 0.62);

SKIN = [0.735, 0.545, 0.440];
SKN2 = [0.610, 0.430, 0.340];
HAIR = [0.300, 0.200, 0.135];
BEARD= [0.335, 0.225, 0.150];
MAIL = [0.430, 0.450, 0.490];
PLATE= [0.600, 0.625, 0.665];
STEEL= [0.735, 0.755, 0.790];
LTHR = [0.345, 0.255, 0.190];
LTH2 = [0.250, 0.180, 0.135];
CLOTH= [0.310, 0.225, 0.255];   // oxblood tabard
CLT2 = [0.235, 0.230, 0.245];   // grey hose
BRASS= [0.790, 0.615, 0.240];
EYEW = [0.880, 0.870, 0.840];
EYED = [0.130, 0.110, 0.095];

JAW = 0.94; BROW = 0.96; SLIM = 0.93;
MUZ = 0;
NP  = 0.16;   // nose projection, in head-heights
HH  = pHH(P);

module face() {
    FB = head_front(P, 4, JAW, BROW, MUZ);   // brow line, front surface
    FJ = head_front(P, 3, JAW, BROW, MUZ);   // mid-face, front surface
    ZB = head_z(P, 4); ZJ = head_z(P, 3); ZC = head_z(P, 2);
    WJ = head_half(P, 3, JAW, BROW, SLIM);
    color(SKIN) smesh(head_G(P, 0, JAW, BROW, MUZ, SLIM));
    // Brow ridge, sat ON the surface.  With no shadows in this renderer
    // a deep-set eye can only be made by physically overhanging it.
    color(SKIN) translate(tipH(P, [FB*0.84, 0, ZB - 0.07*HH]))
        scale([0.52, 1.00, 0.30]) sphere(r = 0.165*HH, $fn = 20);
    // Nose.  The spine must descend MONOTONICALLY.  The first version ran
    // brow -> ZB-0.30 -> ZJ+0.16 -> ZJ+0.02, and ZJ+0.16 is HIGHER than
    // ZB-0.30, so the sweep doubled back on itself and collapsed into a
    // crumpled stub sitting between the eyes.  On the elf that was enough
    // to make the whole face read upside down: the brow ridge became a
    // mouth above the eyes and the cheekbones became a moustache below.
    // NP is how far the tip stands proud of the skull, in head-heights.
    color(SKIN) smesh(limb_G(
        [ tipH(P,[FB*0.86,              0, ZB - 0.03*HH]),
          tipH(P,[FB*0.95 + 0.35*NP*HH, 0, ZB - 0.11*HH]),
          tipH(P,[FJ*0.98 + 1.00*NP*HH, 0, ZB - 0.19*HH]),
          tipH(P,[FJ*0.92 + 0.55*NP*HH, 0, ZB - 0.27*HH]) ],
        [ 0.042*HH, 0.058*HH, 0.072*HH, 0.062*HH ], [ 0.046*HH, 0.066*HH, 0.086*HH, 0.076*HH ],
        [ 2.6, 2.7, 2.8, 2.8 ], [0,1,0], 14));
    both_y() {
        // The eye has to sit PROUD of the skull, and the pupil proud of
        // the eye.  At FB*0.86 the whole white was inside the head and
        // only slivers showed; at FB*0.99 the pupil stood clear in front
        // of the white and hid it, leaving two black dots.  The white's
        // half-depth in x is about a hundredth of a head-height, so the
        // two have to be stacked within that.
        color(EYEW) translate(tipH(P, [FB*0.97, WJ*0.40, ZB - 0.20*HH]))
            scale([0.55,1,1]) sphere(r = 0.070*HH, $fn = 14);
        color(EYED) translate(tipH(P, [FB*1.05, WJ*0.42, ZB - 0.21*HH]))
            scale([0.45,1,1]) sphere(r = 0.040*HH, $fn = 12);
        // Round ear: the one place a human is defined by what it is NOT.
        // No point, no sweep, no lobe past the jawline.
        color(SKN2) translate(tipH(P, [-0.02*HH, WJ*0.94, ZJ + 0.03*HH]))
            scale([0.66, 0.34, 1.05]) sphere(r = 0.135*HH, $fn = 14);
    }
}

module hair() {
    color(HAIR) hair_shell(P, 3, 7, 0.012*HH, 9, 3, JAW, BROW, 0, SLIM);
    color(HAIR) translate(tipH(P, [-0.20*HH, 0, pCHIN(P) + 0.27*HH]))
        scale([0.72, 1.02, 1.12]) sphere(r = 0.40*HH, $fn = 22);
}

// A short cropped beard, not a dwarf's: one third the drop, and it
// stays inside the jawline instead of spilling past it.
module beard() {
    both_y() {
        color(BEARD) smesh(brd_G(P, 0.62, 0.30, 0.16));
        color(BEARD) translate(tipH(P, [0.40*HH, 0.135*HH, pCHIN(P) + 0.175*HH]))
            rotate([0, 14, -16]) scale([0.70, 1.30, 0.46]) sphere(r = 0.175*HH, $fn = 16);
    }
}

// Tabard hem: a skirt of cloth below the belt.  On a figure with no
// proportional gimmick this is where the silhouette has to come from.
module tabard() {
    S = pH(P)/1.358;
    G = tor_G(P, 0.022*S);
    color(CLOTH) sband(G, 0, 2);
}

body(P, SKIN, CLOTH, CLT2, MAIL, LTHR, LTH2, PLATE, STEEL, BRASS);
tabard();
pauldrons(P, PLATE, STEEL, BRASS, 0.058, 0.082, 0.012);
face();
hair();
beard();
