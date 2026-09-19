include <char_kit.scad>
// ===================================================================
//  DWARF -- player character                                     v4
//
//  Faces +x, up +z, the character's LEFT is +y.  Scene units metres,
//  feet on z = 0, so the figure drops straight into a scene at scale.
//
//  The include above resolves against THIS file's directory, so the kit
//  it names sits beside it.  (An older note here said includes resolved
//  against the server's working directory and told you to build a flat
//  copy by hand; that stopped being true, and the recipe no longer
//  applies.)
//
//  WHY A FIGURE IS HARDER THAN A SHIP HERE
//  A hull is one sweep along one spine.  A body is a dozen sweeps that
//  have to meet, and every meeting is a chance to leave the knife cut
//  the cobra's neck had.  Two rules do most of the work:
//
//    1. Every limb STARTS INSIDE its parent.  The arm's first station
//       sits inside the ribcage, the head's inside the chest, the
//       boot's inside the shin.  A buried cap is interior geometry and
//       cannot show a seam, so no join needs hiding.
//    2. Limbs sweep with pt_N (parallel transport), not a fixed up
//       vector.  frame_up(S,i,[0,0,1]) is DEGENERATE on a leg: the
//       tangent is vertical, cross(up,T) is zero, and the whole section
//       collapses to a line.  Parallel transport carries the normal by
//       the minimum rotation instead and never has that failure.
//
//  MIRRORING IS SAFE HERE -- measured, not assumed.  mirror() reverses
//  face order in this kernel, so a mirrored sweep keeps a positive
//  signed volume and stays correctly lit.  Probed before relying on it,
//  because the cobra shipped nine revisions with an inside-out wing.
//  So every paired limb is built once, on the +y side, and mirrored.
//
//  PROPORTION.  A fantasy dwarf is not a small human: it is a human
//  compressed vertically and widened, keeping adult mass.  The read is
//  the height-to-breadth ratio.  MEASURED off the built mesh, not
//  estimated: 1.358 m tall, 0.719 m across the deltoids, so 0.53 --
//  where a tall human runs about 0.23.  The torso alone is 0.468 wide;
//  it is the arms hung off it that carry the figure out to 0.72.
//  Short legs do the rest: hip at 42% of standing height against about
//  50% human.  Skull is 0.26 chin to crown, so 5.2 heads by the bone,
//  but hair and beard read as head mass and the eye sees nearer four.
//
//  LIGHTING is the same single directional light as the ships, with no
//  specular and a 0.25 ambient floor, so this palette sits mid-value
//  throughout.  A character has to read against any background, which
//  is the opposite problem to the near-black ships.
// ===================================================================

// ---- landmarks -------------------------------------------------------
Z_ANKLE =  0.135;
Z_KNEE  =  0.395;
Z_HIP   =  0.570;
Z_BELT  =  0.730;
Z_CHEST =  0.920;
Z_SHLD  =  1.010;
Z_CHIN  =  1.105;
Z_BROW  =  1.245;
Z_CROWN =  1.350;

Y_HIP   =  0.112;    // leg centreline offset from the median plane
Y_SHLD  =  0.238;

// ---- palette ---------------------------------------------------------
SKIN = [0.745, 0.535, 0.425];
SKN2 = [0.615, 0.420, 0.330];
BEARD= [0.600, 0.280, 0.150];   // copper, the loudest thing on the figure
BRD2 = [0.450, 0.205, 0.110];
HAIR = [0.530, 0.245, 0.132];
MAIL = [0.395, 0.420, 0.460];
PLATE= [0.600, 0.625, 0.665];
STEEL= [0.735, 0.755, 0.790];
LTHR = [0.330, 0.240, 0.180];
LTH2 = [0.240, 0.170, 0.130];
CLOTH= [0.195, 0.330, 0.300];   // deep teal, the faction colour
CLT2 = [0.145, 0.250, 0.230];
BRASS= [0.800, 0.610, 0.220];
EYEW = [0.880, 0.870, 0.840];
EYED = [0.120, 0.100, 0.090];

// ---- sweeping a limb -------------------------------------------------
// S, W, D, E are parallel lists: spine point, lateral half-width,
// fore-aft half-depth, and Lame exponent at that station.  Carrying the
// exponent per station is what lets one sweep run from a round thigh
// (e=2.6) into a blocky boot (e=3.6) without a seam between them.
//
// N0 is orthogonalised against the first tangent before use.  pt_N
// takes N0 verbatim at station 0, so an N0 that is not perpendicular to
// the spine shears every section along the whole limb.
function ortho(N0, T) = unit3(N0 - T*(T*N0));

function limb_G(S, W, D, E, N0, n=20, d=0) =
  let( T0 = tang(S,0), M0 = ortho(N0, T0) )
    [ for (i=[0:len(S)-1])
        let( T = tang(S,i), N = pt_N(S,i,M0), B = cross(T,N) )
          [ for (p = lame(W[i]+d, D[i]+d, E[i], n)) S[i] + N*p[0] + B*p[1] ] ];

// ---- torso -----------------------------------------------------------
// The spine carries a real lumbar curve: the waist sits slightly
// forward of the hips and the chest slightly back of the waist.  It is
// two centimetres of offset and it is the difference between a figure
// standing and a figure stacked.
TOR_S = [ [ 0.000, 0, 0.535],
          [ 0.008, 0, 0.660],
          [ 0.010, 0, 0.730],
          [ 0.004, 0, 0.825],
          [-0.004, 0, 0.920],
          [-0.006, 0, 0.985],
          [-0.004, 0, 1.040] ];
TOR_W = [ 0.150, 0.138, 0.142, 0.178, 0.216, 0.234, 0.190 ];
TOR_D = [ 0.108, 0.100, 0.104, 0.126, 0.142, 0.132, 0.108 ];
TOR_E = [ 2.60,  2.80,  3.00,  3.00,  2.90,  2.80,  2.60  ];

function tor_G(d=0) = limb_G(TOR_S, TOR_W, TOR_D, TOR_E, [0,1,0], 28, d);

module torso() {
    G = tor_G();
    color(CLOTH) smesh(G);
    // Mail shirt over the tunic, standing proud so it is a garment and
    // not a painted band.
    color(MAIL)  sband(tor_G(0.010), 2, 6);
    color(LTHR)  sband(tor_G(0.018), 1, 2);      // the belt
    color(BRASS) translate([0.118, 0, Z_BELT - 0.005])
        scale([0.5, 1, 1]) sphere(r = 0.046, $fn = 20);
}

// ---- legs ------------------------------------------------------------
// Station 0 sits above the hip, inside the pelvis, so the thigh's cap
// is interior geometry and the hip needs no fairing.
LEG_S = [ [ 0.004, Y_HIP + 0.006, 0.640],
          [ 0.004, Y_HIP + 0.002, 0.500],
          [ 0.010, Y_HIP + 0.004, Z_KNEE],
          [ 0.004, Y_HIP,         0.300],
          [ 0.000, Y_HIP - 0.010, 0.200],
          [ 0.000, Y_HIP - 0.012, Z_ANKLE] ];
LEG_W = [ 0.098, 0.092, 0.079, 0.079, 0.059, 0.051 ];
LEG_D = [ 0.100, 0.096, 0.086, 0.089, 0.063, 0.056 ];
LEG_E = [ 2.60,  2.60,  2.70,  2.70,  2.80,  2.90  ];

function leg_G(d=0) = limb_G(LEG_S, LEG_W, LEG_D, LEG_E, [0,1,0], 18, d);

// The boot sweeps with N0 = +x so its local x runs fore-aft, which is
// what lame4 needs: the toe grows forward while the heel stays put.
BOOT_S = [ [0.000, Y_HIP - 0.012, 0.205],
           [0.000, Y_HIP - 0.012, 0.140],
           [0.004, Y_HIP - 0.012, 0.080],
           [0.008, Y_HIP - 0.012, 0.032],
           [0.008, Y_HIP - 0.012, 0.008] ];
BOOT_F = [ 0.066, 0.086, 0.112, 0.138, 0.132 ];   // forward
BOOT_A = [ 0.066, 0.076, 0.086, 0.092, 0.088 ];   // aft
BOOT_H = [ 0.064, 0.078, 0.086, 0.084, 0.077 ];   // half-width
BOOT_E = [ 3.00,  3.20,  3.40,  3.60,  3.60  ];

function boot_G(d=0) =
  let( T0 = tang(BOOT_S,0), M0 = ortho([1,0,0], T0) )
    [ for (i=[0:len(BOOT_S)-1])
        let( T = tang(BOOT_S,i), N = pt_N(BOOT_S,i,M0), B = cross(T,N) )
          [ for (p = lame4(BOOT_F[i]+d, BOOT_A[i]+d, BOOT_H[i]+d, BOOT_E[i], 20))
              BOOT_S[i] + N*p[0] + B*p[1] ] ];

module legs() {
    both_y() {
        color(CLT2)  smesh(leg_G());
        color(LTH2)  sband(leg_G(0.008), 2, 3);      // knee pad
        color(LTHR)  smesh(boot_G());
        color(PLATE) sband(boot_G(0.009), 1, 2);     // boot band
        color(LTH2)  sband(boot_G(0.004), 3, 4);     // sole
    }
}

// ---- arms ------------------------------------------------------------
// Rhythm matters more than length here: upper arm slim, elbow pinched,
// forearm swollen past the upper arm.  That reversal is what makes a
// dwarf's arm read as a dwarf's rather than a short human's.
ARM_S = [ [ 0.000, 0.205, 1.030],
          [ 0.008, 0.252, 0.975],
          [ 0.018, 0.268, 0.880],
          [ 0.034, 0.273, 0.775],
          [ 0.054, 0.271, 0.672],
          [ 0.070, 0.264, 0.585] ];
ARM_W = [ 0.086, 0.084, 0.071, 0.066, 0.074, 0.060 ];
ARM_D = [ 0.086, 0.084, 0.073, 0.070, 0.078, 0.064 ];
ARM_E = [ 2.60,  2.60,  2.60,  2.60,  2.70,  2.80  ];

function arm_G(d=0) = limb_G(ARM_S, ARM_W, ARM_D, ARM_E, [1,0,0], 18, d);

HAND_S = [ [0.072, 0.263, 0.600],
           [0.084, 0.259, 0.535],
           [0.094, 0.255, 0.483],
           [0.100, 0.251, 0.445],
           [0.100, 0.249, 0.420] ];
HAND_W = [ 0.057, 0.059, 0.053, 0.042, 0.029 ];
HAND_D = [ 0.062, 0.072, 0.070, 0.057, 0.040 ];
HAND_E = [ 2.80,  3.00,  3.00,  2.90,  2.80  ];

function hand_G(d=0) = limb_G(HAND_S, HAND_W, HAND_D, HAND_E, [1,0,0], 16, d);

module arms() {
    both_y() {
        color(CLOTH) smesh(arm_G());
        color(MAIL)  sband(arm_G(0.008), 1, 2);      // mail sleeve
        color(LTHR)  sband(arm_G(0.010), 4, 5);      // bracer
        color(SKIN)  smesh(hand_G());
    }
}

// Pauldrons: a plate swept over the deltoid on an arc in the y-z plane,
// long fore-aft and thin radially, so it reads as metal BENT over the
// shoulder.  The single biggest silhouette win on the figure -- a
// squared shoulder on a short figure is most of the dwarf read.
// v1 stacked five rotated spheres here and they read as feathers.
PLD_R = 0.084;
function pld_S(n=7) =
  [ for (i=[0:n]) let( a = 10 + 108*i/n )
      [ 0, Y_SHLD + PLD_R*sin(a), Z_SHLD + 0.014 + PLD_R*cos(a) ] ];
// The fore-aft length has to fall away at BOTH ends of the arc, or the
// plate is a constant-width band: hooks from the front and a bucket from
// the side, which is exactly what the first two attempts looked like.
// A half-sine gives a blunt-ended lens -- widest over the point of the
// shoulder, narrowing where it meets the neck and again where it ends
// on the arm.  pow(t,0.8) biases the widest point slightly inboard,
// where the deltoid actually is.
function pld_G(d=0, n=7) =
  let( S = pld_S(n), M0 = ortho([1,0,0], tang(S,0)) )
    [ for (i=[0:n])
        let( T = tang(S,i), N = pt_N(S,i,M0), B = cross(T,N), t = i/n )
          [ for (p = lame(0.042 + 0.054*sin(180*pow(t,0.8)) + d,
                          0.014 + 0.006*sstep(t) + d, 3.0, 18))
              S[i] + N*p[0] + B*p[1] ] ];

module pauldrons() {
    both_y() {
        color(PLATE) smesh(pld_G());
        color(STEEL) sband(pld_G(0.005), 5, 7);
        color(BRASS) translate([0, Y_SHLD + PLD_R*sin(56), Z_SHLD + 0.014 + PLD_R*cos(56)])
            scale([1.7, 1, 1]) sphere(r = 0.011, $fn = 12);
    }
}

// ---- head ------------------------------------------------------------
// Starts at z = 0.95, inside the chest, so the neck emerges instead of
// being stuck on.  The skull is widest at the cheekbone and narrows to
// the crown; the jaw is heavy because the beard has to hang off it.
HEAD_S = [ [-0.004, 0, 0.950],
           [-0.002, 0, 1.045],
           [ 0.004, 0, 1.105],
           [ 0.008, 0, 1.170],
           [ 0.006, 0, Z_BROW],
           [ 0.000, 0, 1.310],
           [-0.004, 0, Z_CROWN],
           [-0.006, 0, 1.362] ];
HEAD_W = [ 0.072, 0.076, 0.101, 0.113, 0.114, 0.101, 0.064, 0.024 ];
HEAD_D = [ 0.074, 0.080, 0.110, 0.119, 0.116, 0.102, 0.066, 0.025 ];
HEAD_E = [ 2.60,  2.60,  2.80,  2.90,  2.90,  2.80,  2.60,  2.40  ];

function head_G(d=0) = limb_G(HEAD_S, HEAD_W, HEAD_D, HEAD_E, [0,1,0], 24, d);

NOSE_S = [ [0.078, 0, 1.246], [0.098, 0, 1.214], [0.116, 0, 1.186], [0.121, 0, 1.166] ];
NOSE_W = [ 0.016, 0.023, 0.030, 0.026 ];
NOSE_D = [ 0.018, 0.026, 0.034, 0.030 ];
NOSE_E = [ 2.60,  2.70,  2.80,  2.80  ];
function nose_G(d=0) = limb_G(NOSE_S, NOSE_W, NOSE_D, NOSE_E, [0,1,0], 14, d);

module head() {
    color(SKIN) smesh(head_G());
    // Brow. Heavy, and set proud, because with no shadows the only way
    // to get a deep-set eye is to physically overhang it.
    color(SKIN) translate([0.072, 0, Z_BROW - 0.004])
        scale([0.62, 1.55, 0.40]) sphere(r = 0.072, $fn = 22);
    // Nose: swept, not a squashed sphere, and deliberately sized to
    // reach FURTHER forward than the beard does (0.150 against 0.122).
    // It is the one facial feature that survives to thumbnail size, and
    // in v1 the beard buried it.
    color(SKIN) smesh(nose_G());
    both_y() {
        color(EYEW) translate([0.082, 0.043, 1.208])
            scale([0.55, 1, 1]) sphere(r = 0.019, $fn = 14);
        color(EYED) translate([0.093, 0.046, 1.205])
            scale([0.45, 1, 1]) sphere(r = 0.011, $fn = 12);
        color(SKN2) translate([-0.002, 0.108, 1.175])
            scale([0.70, 0.42, 1.10]) sphere(r = 0.035, $fn = 14);   // ear
    }
}

// Hair: a cap lifted straight off the head's OWN grid, inflated.  Built
// that way it cannot float or sink however the skull is retuned, which
// a separately-positioned shell always eventually does.  Stations 4..7
// run brow to crown, so the hairline lands on the brow where a dwarf's
// belongs, and the nape mass and sideburns carry it down to meet the
// beard so the head is framed rather than capped.
// Hair, not a hat.  sband() takes a whole ring, so a band that stops at
// one station leaves a clean horizontal edge across the forehead, and a
// clean horizontal edge across a forehead is the brim of a knitted cap
// -- which is what v3 and v4 both looked like, however the colour was
// tuned.  part_of() takes an ARC of each section instead, so this is
// the rear three-quarters of the head only: it runs from the jaw up and
// leaves the face open, and its front edges fall vertically beside the
// temples the way a hairline does.  The chord that closes each arc
// passes through the skull's interior, so it is never seen.
//
// head_G's sections index 0 = left, 6 = front, 12 = right, 18 = back,
// so 9..3 is the wrap that excludes the face.
module hair() {
    color(HAIR) sband([ for (g = head_G(0.010)) part_of(g, 9, 3, 3) ], 2, 7);
    color(HAIR) translate([-0.050, 0, 1.174])
        scale([0.76, 1.06, 1.28]) sphere(r = 0.099, $fn = 22);
    both_y() color(HAIR) translate([0.012, 0.098, 1.190])
        scale([0.80, 0.50, 1.18]) sphere(r = 0.064, $fn = 16);
    // A widow's peak.  The cap band ends on a single station, so its
    // lower edge is a clean horizontal ring -- and a clean horizontal
    // ring across a forehead reads as the brim of a knitted hat, not as
    // hair.  One point of hair dipping over the brow breaks the line,
    // and that is the whole difference between a hat and a hairline.
    color(HAIR) translate([0.082, 0, 1.234])
        rotate([0, 34, 0]) scale([0.60, 1.05, 0.34]) sphere(r = 0.056, $fn = 16);
}

// ---- beard -----------------------------------------------------------
// The load-bearing silhouette element, and the thing v1 got most wrong:
// one mass swallowed the whole face and read as a featureless orange
// bib.  Built now as ONE LOBE, mirrored, so the halves meet on the
// median plane.  That buys three things free: a central parting where
// they meet, a forked tip where they separate low down, and a beard
// wider than the skull (0.144 against 0.114) that still leaves the face
// readable -- at eye and nose height it only reaches x = 0.114, where
// the nose reaches 0.150, so the nose stays proud of it.
//
// The spine travels FORWARD as it descends, because the chest it lies
// on is moving forward too: the torso's front face reaches x = 0.138 at
// the sternum.  v2 held the beard at 0.100 all the way down and the
// bottom half was swallowed by the ribcage, which is why it looked
// sheared off at the collarbone.  A beard rests ON a chest.
BRD_S = [ [ 0.030, 0.052, 1.162],
          [ 0.052, 0.062, 1.108],
          [ 0.078, 0.066, 1.030],
          [ 0.098, 0.062, 0.948],
          [ 0.106, 0.056, 0.866],
          [ 0.104, 0.050, 0.800] ];
BRD_W = [ 0.062, 0.070, 0.072, 0.066, 0.054, 0.036 ];
BRD_D = [ 0.052, 0.062, 0.066, 0.060, 0.050, 0.034 ];
BRD_E = [ 2.60,  2.70,  2.80,  2.80,  2.70,  2.60  ];

function brd_G(d=0) = limb_G(BRD_S, BRD_W, BRD_D, BRD_E, [0,1,0], 20, d);

// A braid: beads on a gentle helix under a brass clasp, hung off the
// lobe tip.  It has to hang FORWARD of the torso to be seen at all --
// the chest reaches x = 0.13 and the belt 0.132, so a braid plumbed
// straight down from the beard's tip vanishes inside the figure.  Two
// earlier attempts did exactly that and never saw daylight.
module braid(len, r0, turns) {
    N = 8;
    color(BRASS) translate([0.126, 0.050, 0.792])
        rotate([0, 22, 0]) cylinder(h = 0.019, r = r0*1.35, center = true, $fn = 14);
    for (i = [0:N-1]) let( t = i/(N-1), a = turns*360*t, r = r0*(1 - 0.42*t) )
        color(i % 2 == 0 ? BEARD : BRD2)
            translate([0.130 + 0.009*cos(a) + 0.022*t,
                       0.050 + 0.009*sin(a),
                       0.778 - len*t])
                scale([0.88, 1, 0.74]) sphere(r = r, $fn = 12);
}

module beard() {
    both_y() {
        color(BEARD) smesh(brd_G());
        color(BRD2)  sband(brd_G(0.004), 3, 5);     // underside, in shade
        braid(0.092, 0.024, 1.1);
        // Moustache: sweeping out and down past the jawline, so the
        // mouth is implied rather than modelled.
        color(BEARD) translate([0.092, 0.036, 1.158])
            rotate([0, 16, -18]) scale([0.70, 1.40, 0.52]) sphere(r = 0.048, $fn = 16);
    }
}

// ---- assembly --------------------------------------------------------
torso();
legs();
arms();
pauldrons();
head();
hair();
beard();
