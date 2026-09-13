include <ship_lib.scad>
// ===================================================================
//  FACTION II -- THE SOUNDING              (cetacean laminar bodies)
//
//  Every hull is a Myring (1976) body of revolution -- the form fitted
//  to fast-swimming fish and cetaceans and used for AUVs ever since --
//  with the nose exponent doing the species work: n=2 is a plain
//  semi-ellipsoid, and pushing n up blunts the nose into a melon the
//  way a beluga or a sperm whale carries one.
//
//  Two real cetacean features do the styling, and neither is invented:
//
//    COUNTERSHADING.  Dark above, pale below.  Here it is not paint
//    over a hull, it is two swept solids sharing a chord, so the seam
//    is interior geometry and there is no coplanar pair anywhere.
//
//    TUBERCLES.  The humpback's flipper has a scalloped leading edge
//    (Fish & Battle 1995) that keeps flow attached past the angle a
//    smooth edge stalls at.  Every lifting surface in this fleet is
//    scalloped, and at thumbnail size that serration is the faction.
//
//  The caudal peduncle is compressed side to side, which is what a
//  fast cetacean actually does: the tail stock goes narrow in width
//  and stays tall, so the fluke has a stiff blade to beat against.
//
//  TECHNOLOGY.  The Sounding has no engine plume anywhere -- look for
//  one and there isn't one, which is the fastest way to identify them.
//  They beat a gravitic fluke, which means no exhaust, no thermal
//  bloom and nothing to see coming.  They hunt by sound: the melon is
//  a phased emitter, and the weapon is a focused pressure pulse, the
//  same trick a sperm whale's nose is already built for.
//
//  SILHOUETTE RULES
//    1. One continuous body. No hard edge anywhere on the hull.
//    2. Fineness ratio between 4.5 and 7. Never stubby, never a needle.
//    3. Every lifting surface has a scalloped leading edge.
//    4. Stern ends in a horizontal fluke. Never a nozzle.
//    5. Dark dorsal, pale ventral, split on the waterline.
// ===================================================================

SHOW = 0;          // 0 = fleet line-up, 1..5 = single ship

// Countershading is a VALUE split, and the lighting here has no
// specular to recover a too-dark hull with -- anything under about
// 0.35 goes to mud at the 0.25 ambient floor. So the dorsal is a mid
// slate, not a true dark, and the contrast is carried by the ventral
// being genuinely pale.
DORS = [0.40, 0.46, 0.54];    // slate dorsal
VENT = [0.84, 0.83, 0.77];    // pale ventral
// Lifting surfaces are LIGHTER than the dorsal, not darker. A whale is
// recognised at distance by its fluke and its flipper, and a fluke a
// shade under the hull disappears into it at thumbnail size -- which
// is exactly what the first render of this fleet did.
FIN  = [0.66, 0.70, 0.75];
FLUK = [0.72, 0.75, 0.79];
TRIM = [0.90, 0.62, 0.22];    // sensor amber
EYE  = [0.40, 0.84, 0.92];

NV = 48;                       // section points; split at 0 and NV/2

// Countershaded Myring body.  fr/mid/af are the Myring a/b/c as
// fractions of length; nn is the nose exponent; flat is how far the
// peduncle is squeezed in width.
module cet_body(L, d, fr, mid, af, nn, tth, flat, NU=60) {
    SEC = function(i)
        let( u = i/NU, x = u*L,
             r = max(0.015, myring_r(x, d, fr*L, mid*L, af*L, nn, tth)),
             kw = 1 - flat*sstep((u-0.60)/0.34),
             kh = 1 + 0.10*sstep((u-0.60)/0.34) )
          lame(r*kw, r*kh, 2.35, NV);
    S = [ for (i=[0:NU]) [L/2 - u_x(i,NU,L), 0, 0] ];
    G = [ for (i=[0:NU]) let(F = frame_up(S,i,[0,0,1]))
            [ for (p = SEC(i)) S[i] + F[1]*p[0] + F[2]*p[1] ] ];
    color(DORS) smesh([ for (g=G) part_of(g, NV/2, 0, 3) ]);
    color(VENT) smesh([ for (g=G) part_of(g, 0, NV/2, 3) ]);
}
function u_x(i,NU,L) = L*i/NU;

// A scalloped flipper, swept and mirrored to both flanks.
module flippers(at, root_c, tip_c, span, sweep, dih, amp, k) {
    color(FIN) both_y() translate([at,0,0])
        tfin(root_c, tip_c, span, sweep, 0.13, dih, amp, k, 34);
}

// Dorsal fin: the same aerofoil stood on edge.
module dorsal(at, root_c, tip_c, h, sweep) {
    color(FIN) translate([at,0,0]) rotate([-90,0,0])
        tfin(root_c, tip_c, h, sweep, 0.13, 0, 0.05, 5, 26);
}

// Fluke: horizontal, swept, notched in the middle the way a real one
// is -- two blades meeting at a narrow stock, not one plate.
module fluke(at, root_c, tip_c, span, sweep, amp) {
    color(FLUK) both_y() translate([at,0,0])
        tfin(root_c, tip_c, span, sweep, 0.10, -4, amp, 6, 30);
}

module sensor_band(at, r, w) {
    color(TRIM) translate([at,0,0]) rotate([0,90,0])
        cylinder(h=w, r=r, center=true, $fn=40);
}
module eyes(at, r, z) {
    both_y() color(EYE) translate([at, r*0.80, z]) sphere(r=r*0.16, $fn=12);
}

// ---- 1. PORPOISE -- scout, 16 m -------------------------------------
module porpoise() {
    cet_body(16, 2.9, 0.26, 0.30, 0.44, 2.2, 22, 0.50, 48);
    flippers(1.2, 3.2, 1.0, 4.2, 2.0, -8, 0.075, 5);
    dorsal(-0.6, 2.6, 0.7, 3.0, 1.6);
    fluke(-7.0, 2.6, 0.6, 5.4, 2.4, 0.07);
    eyes(6.0, 1.35, 0.25);
}

// ---- 2. PILOT -- corvette, 30 m -------------------------------------
module pilot() {
    cet_body(30, 5.4, 0.30, 0.28, 0.42, 2.8, 21, 0.52, 56);
    flippers(3.4, 6.0, 1.6, 8.4, 4.0, -10, 0.07, 6);
    dorsal(-2.0, 4.6, 1.1, 5.6, 3.0);
    fluke(-13.2, 4.8, 1.0, 10.4, 4.4, 0.065);
    sensor_band(9.4, 2.35, 0.55);
    eyes(11.0, 2.5, 0.5);
}

// ---- 3. BELUGA -- frigate, 52 m -------------------------------------
// Nose exponent 4.2: the melon. A beluga's is a real acoustic lens and
// this one is the ship's main emitter, which is why it leads.
module beluga() {
    cet_body(52, 10.6, 0.34, 0.26, 0.40, 4.2, 20, 0.55, 64);
    flippers(4.6, 9.4, 2.4, 14.0, 6.4, -9, 0.065, 7);
    dorsal(-4.0, 7.0, 1.6, 8.0, 4.4);
    fluke(-22.8, 8.0, 1.6, 17.5, 7.2, 0.06);
    sensor_band(16.0, 4.7, 0.9);
    sensor_band(12.6, 5.1, 0.6);
    eyes(18.4, 4.6, 0.9);
}

// ---- 4. RORQUAL -- cruiser, 80 m ------------------------------------
// Long, slender, fineness near 7, with the throat pleats a rorqual
// uses to distend. Here they are structural ribs down the ventral.
module rorqual() {
    cet_body(80, 11.6, 0.28, 0.34, 0.38, 2.4, 19, 0.56, 68);
    flippers(9.0, 12.0, 2.4, 22.0, 10.0, -6, 0.06, 9);
    dorsal(-16.0, 8.0, 1.8, 9.0, 5.0);
    fluke(-35.2, 11.0, 2.0, 27.0, 11.0, 0.055);
    color(VENT) for (i=[-4:4]) let(y=i*1.05)
        for (j=[0:22]) let(x = 26 - j*1.35, t = j/22,
                           r = max(0.05, myring_r((40-x), 11.6, 0.28*80, 0.34*80, 0.38*80, 2.4, 19)))
            translate([x, y, -r*0.93]) sphere(r=0.30, $fn=6);
    sensor_band(24.0, 4.6, 1.0);
    eyes(27.0, 4.9, 1.2);
}

// ---- 5. PHYSETER -- capital, 132 m ----------------------------------
// Nose exponent 5.5. A third of the ship is emitter, which is very
// nearly the proportion a sperm whale gives its spermaceti organ, and
// for the same reason: it is a sound gun.
module physeter() {
    cet_body(132, 21.0, 0.40, 0.24, 0.36, 5.5, 18, 0.58, 74);
    flippers(18.0, 18.0, 3.6, 34.0, 15.0, -7, 0.055, 10);
    dorsal(-32.0, 12.0, 2.6, 12.0, 7.0);
    for (dx=[-40,-46,-52]) color(FIN) translate([dx,0,0]) rotate([-90,0,0])
        tfin(4.4, 1.6, 2.6, 1.6, 0.14, 0, 0.05, 3, 14);
    fluke(-58.0, 17.0, 3.0, 44.0, 18.0, 0.05);
    // the emitter array rides the flat forward face of the melon
    for (i=[0:5]) let(a=360*i/6, rr=5.6)
        color(TRIM) translate([64.0, rr*cos(a), rr*sin(a)])
            rotate([0,90,0]) cylinder(h=1.4, r=1.5, $fn=16);
    color(EYE) translate([65.2,0,0]) rotate([0,90,0]) cylinder(h=1.6, r=2.2, $fn=24);
    sensor_band(40.0, 8.6, 1.6);
    eyes(46.0, 9.4, 2.0);
}

// ---- staging ---------------------------------------------------------
module ship(i) {
    if (i==1) porpoise();
    if (i==2) pilot();
    if (i==3) beluga();
    if (i==4) rorqual();
    if (i==5) physeter();
}
if (SHOW == 0) {
    OFF = [0, 26, 64, 122, 208];
    for (i=[1:5]) translate([0, OFF[4]/2 - OFF[i-1], 0]) ship(i);
} else ship(SHOW);
