include <char_lib.scad>
// ===================================================================
//  ELF -- player character
//
//  Everything here is the human's numbers pushed one way and held.
//  8.5 heads against 7.5 is the classical heroic canon taken a step
//  past heroic; hip at 0.545 against 0.50 puts the legs well over half
//  the figure; girth 0.80 and shld 0.112 take the mass out.  The result
//  is that an elf and a human at the same height are still obviously
//  different people, which is the test this profile has to pass --
//  height alone was never the distinguishing thing.
//
//  No beard, deliberately: with the dwarf, gnome and human all bearded,
//  the bare jaw is doing real work in a line-up.  The ears take the
//  gnome's swept-ear part with the rise and sweep reversed, so the
//  point travels BACK along the skull instead of up away from it.
// ===================================================================

P = prof(H     = 1.880,
         heads = 8.50,
         shld  = 0.112,
         hipf  = 0.545,
         girth = 0.80,
         armf  = 0.355,
         lean  = -1,
         dep   = 0.56);

SKIN = [0.815, 0.700, 0.625];
SKN2 = [0.695, 0.580, 0.510];
HAIR = [0.780, 0.720, 0.545];   // pale gold
HAI2 = [0.640, 0.585, 0.430];
MAIL = [0.505, 0.545, 0.520];
PLATE= [0.640, 0.680, 0.650];
STEEL= [0.780, 0.810, 0.785];
LTHR = [0.330, 0.330, 0.290];
LTH2 = [0.245, 0.250, 0.220];
CLOTH= [0.235, 0.360, 0.310];   // deep forest
CLT2 = [0.180, 0.275, 0.240];
BRASS= [0.775, 0.700, 0.430];   // pale gold, not the dwarf's brass
EYEW = [0.900, 0.895, 0.875];
EYED = [0.135, 0.145, 0.130];

JAW = 0.82; BROW = 0.88; SLIM = 0.86;
MUZ = 0;
HH  = pHH(P);

module face() {
    FB = head_front(P, 4, JAW, BROW, MUZ);   // brow line, front surface
    FJ = head_front(P, 3, JAW, BROW, MUZ);   // mid-face, front surface
    ZB = head_z(P, 4); ZJ = head_z(P, 3); ZC = head_z(P, 2);
    WJ = head_half(P, 3, JAW, BROW, SLIM);
    color(SKIN) smesh(head_G(P, 0, JAW, BROW, MUZ, SLIM));
    // Brow ridge, sat ON the surface.  With no shadows in this renderer
    // a deep-set eye can only be made by physically overhanging it.
    color(SKIN) translate(tipH(P, [FB*0.88, 0, ZB]))
        scale([0.48, 0.92, 0.24]) sphere(r = 0.150*HH, $fn = 20);
    // Nose: swept from the brow down to the lip so it keeps an edge
    // along the bridge instead of going soft.
    color(SKIN) smesh(limb_G(
        [ tipH(P,[FB*0.74, 0, ZB - 0.04*HH]), tipH(P,[FB*0.92, 0, ZB - 0.30*HH]),
          tipH(P,[FJ*0.98, 0, ZJ + 0.16*HH]), tipH(P,[FJ*0.94, 0, ZJ + 0.02*HH]) ],
        [ 0.034*HH, 0.046*HH, 0.058*HH, 0.049*HH ], [ 0.038*HH, 0.054*HH, 0.070*HH, 0.060*HH ],
        [ 2.6, 2.7, 2.8, 2.8 ], [0,1,0], 14));
    both_y() {
        color(EYEW) translate(tipH(P, [FB*0.86, WJ*0.40, ZB - 0.20*HH]))
            scale([0.55,1,0.88]) sphere(r = 0.072*HH, $fn = 14);
        color(EYED) translate(tipH(P, [FB*0.99, WJ*0.43, ZB - 0.21*HH]))
            scale([0.45,1,1]) sphere(r = 0.038*HH, $fn = 12);
        // Cheekbones set high and wide, doing the job the jaw is not.
        color(SKIN) translate(tipH(P, [FJ*0.72, WJ*0.62, ZB - 0.34*HH]))
            scale([0.80, 0.55, 0.42]) sphere(r = 0.165*HH, $fn = 16);
        // Swept back and slightly down: the gnome's ear with rise near
        // zero and sweep large.
        color(SKIN) smesh(ear_G(P, ZB - 0.14*HH, 0.30, 0.26, 0.06, 0.46, 0.070, 0.016));
    }
}

// Long hair: the shell carried further down the stations than any other
// species here, plus a mass behind the shoulders.  It is the only thing
// giving this figure width, which a 0.112 shoulder badly needs.
module hair() {
    color(HAIR) hair_shell(P, 1, 7, 0.013*HH, 10, 2, JAW, BROW, 0, SLIM);
    // The hair is load-bearing here: at shld 0.112 this is the narrowest
    // figure in the set, and the mass falling past the shoulders is most
    // of what stops it reading as a pole.
    color(HAIR) translate(tipH(P, [-0.30*HH, 0, pCHIN(P) - 0.34*HH]))
        scale([0.60, 1.72, 2.30]) sphere(r = 0.46*HH, $fn = 24);
    color(HAI2) translate(tipH(P, [-0.34*HH, 0, pCHIN(P) - 1.30*HH]))
        scale([0.48, 1.40, 1.60]) sphere(r = 0.40*HH, $fn = 20);
}

module circlet() {
    color(BRASS) translate(tipH(P, [0, 0, pCHIN(P) + 0.70*HH]))
        rotate([0, 0, 0]) cylinder(h = 0.055*HH, r = 0.475*HH, center = true, $fn = 30);
    color(BRASS) translate(tipH(P, [0.40*HH, 0, pCHIN(P) + 0.68*HH]))
        scale([0.5, 1, 1.7]) sphere(r = 0.065*HH, $fn = 14);
}

// A shoulder mantle, not a full cloak.  Inflating the whole trunk from
// the hip up gave a barrel that swallowed the figure -- and swallowing
// the figure is the one thing this species cannot afford, because the
// slenderness IS the design.  Confined to the top three stations it
// broadens the shoulder line and leaves the narrow waist showing under
// it, which is the shape worth having.
// Station 6 is the trapezius, where the trunk is already flaring toward
// the neck -- inflating it by 5 cm turned the mantle into a bucket
// standing up around the head.  A light cuirass over stations 3 to 5
// only, and thin, keeps the shoulder line and leaves the head clear.
// The silhouette this species needs comes from the hair, not the kit.
module cuirass() {
    S = pH(P)/1.358;
    color(MAIL)  sband(tor_G(P, 0.016*S), 3, 5);
    color(PLATE) sband(tor_G(P, 0.022*S), 4, 5);
}

body(P, SKIN, CLOTH, CLT2, MAIL, LTHR, LTH2, PLATE, STEEL, BRASS);
cuirass();
pauldrons(P, PLATE, STEEL, BRASS, 0.048, 0.070, 0.009);
face();
hair();
circlet();
