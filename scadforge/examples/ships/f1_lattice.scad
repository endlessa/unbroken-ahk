include <ship_lib.scad>
// ===================================================================
//  FACTION I -- THE LATTICE            (radiolarian geodesic frames)
//
//  Radiolaria are single-celled marine protists that build silica
//  skeletons, and some of them build them on exact polyhedral
//  symmetry: Circogonia icosahedra is icosahedral, and Haeckel's
//  Aulonia hexagona is the near-hexagonal net that Fuller later kept
//  pointing at.  The mathematics is therefore not a metaphor -- it is
//  icosahedral geodesic subdivision, the same construction the
//  library builds by FINDING the faces (any vertex triple whose three
//  pairwise distances equal the edge length) rather than transcribing
//  them.  Frequency 2 gives V=42, E=120, F=80; V-E+F=2.
//
//  TECHNOLOGY follows from the skeleton.  The Lattice does not build
//  hulls.  It builds frames, and slings its pressure vessels inside
//  them, because a geodesic frame carries load in pure tension and
//  compression along its struts and needs no skin to do it.  Their
//  weapon is the same fact used offensively: drive a strut network at
//  an eigenfrequency of the target's own frame and it tears itself
//  apart.  Hence the spines -- those are radiators and emitters, not
//  decoration, and every ship is bristling with them.
//
//  SILHOUETTE RULES
//    1. Open frame. You can see stars through every ship.
//    2. Straight struts only; no curved structural member anywhere.
//    3. Pressure vessels are spheres, always fully enclosed by frame.
//    4. Spines radiate from frame VERTICES, never from face centres.
//    5. Bigger ship = higher subdivision frequency, not thicker struts.
// ===================================================================

SHOW = 0;          // 0 = fleet line-up, 1..5 = single ship

// Hoisted once. geo_edges is an O(n^2) positional dedup, so calling it
// per ship is what turns a two-second build into a minute -- frequency
// 3 alone (540 raw edges) costs 62s, which is why no ship here uses
// it. Density comes from CONCENTRIC shells instead, which is what
// Actinomma actually does: two or three nested lattice spheres joined
// by radial beams.
E1 = geo_edges(1);  P1 = geo_pts(1);
E2 = geo_edges(2);  P2 = geo_pts(2);

STRUT = [0.78, 0.79, 0.74];    // bone silica
NODE  = [0.34, 0.36, 0.39];    // dark sintered joints
VESL  = [0.85, 0.62, 0.26];    // amber pressure spheres
SPINE = [0.62, 0.64, 0.68];
GLOW  = [0.42, 0.88, 0.92];    // resonance cyan
DARK  = [0.24, 0.26, 0.29];

// A geodesic frame scaled anisotropically. Struts stay straight, which
// is the whole point: scaling a polyhedron cannot bend a member.
module frame(E, P, R, sx, sy, sz, rs, rn) {
    color(STRUT) for (e=E) rod([e[0][0]*R*sx, e[0][1]*R*sy, e[0][2]*R*sz],
                               [e[1][0]*R*sx, e[1][1]*R*sy, e[1][2]*R*sz], rs, 7);
    color(NODE)  for (p=P) translate([p[0]*R*sx, p[1]*R*sy, p[2]*R*sz])
                     sphere(r=rn, $fn=10);
}

// Radial beams tying two concentric shells together.
module shell_ties(P, R0, R1, sx, sy, sz, r) {
    color(SPINE) for (p=P) rod([p[0]*R0*sx, p[1]*R0*sy, p[2]*R0*sz],
                               [p[0]*R1*sx, p[1]*R1*sy, p[2]*R1*sz], r, 5);
}

// Spines on the frame vertices whose outward direction has a positive
// component along `dir` -- so a bow array, a stern array, or a full
// halo, chosen by where the emitters need to point.
module spines(P, R, sx, sy, sz, dir, len, r0, thresh=0.30) {
    for (p=P) {
        v = [p[0]*sx, p[1]*sy, p[2]*sz];
        u = unit3(v);
        if (u*dir > thresh) {
            A = [p[0]*R*sx, p[1]*R*sy, p[2]*R*sz];
            color(SPINE) rod(A, A + u*len, r0, 6);
            color(GLOW)  translate(A + u*len) sphere(r=r0*1.15, $fn=8);
        }
    }
}

module vessel(pos, r) {
    color(VESL) translate(pos) sphere(r=r, $fn=26);
    color(DARK) translate(pos) rotate([0,90,0]) cylinder(h=r*2.3, r=r*0.28, center=true, $fn=14);
}

module drives(pos, r, n, len) {
    for (i=[0:n-1]) {
        a = 360*i/n;
        p = pos + [0, r*cos(a), r*sin(a)];
        color(DARK) translate(p) rotate([0,-90,0]) bell(r*0.32, r*0.52, len*0.55, 0.10, 18, 8);
        color(GLOW) translate(p + [-len*0.5,0,0]) plume(r*0.44, len, GLOW, 6);
    }
}

// ---- 1. SPICULE -- interceptor, 14 m --------------------------------
// One octahedral cell stretched three to one. The smallest frame that
// is still a closed polyhedron, which is the Lattice's whole argument
// about why it needs no skin.
module spicule() {
    V = [[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]];
    P = [ for (v=V) [v[0]*7.0, v[1]*1.9, v[2]*1.7] ];
    // i stops at 4: [6:5] is a REVERSED range here, not an empty one,
    // and it feeds undef into abs().
    color(STRUT) for (i=[0:4]) for (j=[i+1:5])
        if (abs(V[i]*V[j]) < 0.5) rod(P[i], P[j], 0.17, 6);
    color(NODE) for (p=P) translate(p) sphere(r=0.34, $fn=9);
    vessel([0.4,0,0], 1.15);
    color(SPINE) rod([7.0,0,0],[11.6,0,0], 0.16, 6);
    color(GLOW)  translate([11.6,0,0]) sphere(r=0.24, $fn=8);
    drives([-6.6,0,0], 1.15, 2, 3.4);
}

// ---- 2. AULONIA -- corvette, 26 m -----------------------------------
module aulonia() {
    frame(E1, P1, 6.2, 1.85, 1.0, 0.92, 0.20, 0.40);
    vessel([1.2,0,0], 2.2);
    spines(P1, 6.2, 1.85, 1.0, 0.92, [1,0,0], 3.6, 0.19, 0.45);
    spines(P1, 6.2, 1.85, 1.0, 0.92, [-1,0,0], 2.2, 0.16, 0.55);
    color(SPINE) rod([11.5,0,0],[16.0,0,0], 0.22, 7);
    color(GLOW)  translate([16.0,0,0]) sphere(r=0.36, $fn=9);
    drives([-11.0,0,0], 2.0, 3, 5.0);
}

// ---- 3. CIRCOGONIA -- frigate, 42 m ---------------------------------
// Named for the radiolarian that actually is an icosahedron. Frequency
// 2: 120 struts, 42 nodes, and a spine on every node that faces out.
module circogonia() {
    frame(E2, P2, 8.6, 1.95, 1.0, 0.95, 0.17, 0.32);
    vessel([2.6,0,0], 2.6);
    vessel([-3.4,0,0], 2.0);
    spines(P2, 8.6, 1.95, 1.0, 0.95, [1,0,0], 3.2, 0.15, 0.55);
    spines(P2, 8.6, 1.95, 1.0, 0.95, [0,0,1], 2.4, 0.13, 0.72);
    spines(P2, 8.6, 1.95, 1.0, 0.95, [0,0,-1], 2.4, 0.13, 0.72);
    color(SPINE) rod([16.8,0,0],[24.0,0,0], 0.26, 8);
    color(GLOW)  translate([24.0,0,0]) sphere(r=0.44, $fn=10);
    drives([-16.2,0,0], 3.0, 4, 6.4);
}

// ---- 4. HEXACTIN -- cruiser, 68 m -----------------------------------
// Two cells on one spar. A hexactin is the six-rayed spicule of a
// glass sponge, and the spar is exactly that: one member through both
// frames, carrying the whole ship in compression.
module hexactin() {
    color(SPINE) rod([-30,0,0],[26,0,0], 0.5, 10);
    for (dx=[9.5,-11.5]) translate([dx,0,0])
        frame(E2, P2, 7.4, 1.25, 1.05, 1.0, 0.16, 0.30);
    translate([9.5,0,0])  spines(P2, 7.4, 1.25, 1.05, 1.0, [1,0,0], 3.0, 0.15, 0.6);
    translate([-11.5,0,0]) spines(P2, 7.4, 1.25, 1.05, 1.0, [0,0,1], 2.6, 0.14, 0.7);
    translate([-11.5,0,0]) spines(P2, 7.4, 1.25, 1.05, 1.0, [0,0,-1], 2.6, 0.14, 0.7);
    vessel([9.5,0,0], 2.5);
    vessel([-11.5,0,0], 2.9);
    color(NODE) for (dx=[0,-2.4,2.4]) translate([dx,0,0]) rotate([0,90,0])
        cylinder(h=0.9, r=1.5, center=true, $fn=18);
    color(SPINE) rod([26,0,0],[34,0,0], 0.30, 8);
    color(GLOW)  translate([34,0,0]) sphere(r=0.5, $fn=10);
    drives([-30,0,0], 3.4, 5, 7.0);
}

// ---- 5. HAECKEL -- dreadnought, 110 m -------------------------------
// The resonance lance is nine spines on a common frame, phase-locked.
// Everything else on the ship exists to hold them rigid relative to
// one another, which is why the frame is the ship.
module haeckel() {
    // Two concentric lattice shells tied by radial beams -- the
    // Actinomma plan. Denser than a frigate without a single strut
    // being thicker, which is the faction's own rule.
    frame(E2, P2, 14.0, 2.35, 1.0, 1.0, 0.24, 0.46);
    frame(E2, P2,  8.6, 2.35, 1.0, 1.0, 0.17, 0.30);
    shell_ties(P2, 8.6, 14.0, 2.35, 1.0, 1.0, 0.11);
    for (p=[[10,0,0],[0,0,0],[-10,0,0]]) vessel(p, 3.0);
    spines(P2, 14.0, 2.35, 1.0, 1.0, [1,0,0], 6.0, 0.22, 0.52);
    spines(P2, 14.0, 2.35, 1.0, 1.0, [0,1,0], 3.4, 0.18, 0.80);
    spines(P2, 14.0, 2.35, 1.0, 1.0, [0,-1,0], 3.4, 0.18, 0.80);
    spines(P2, 14.0, 2.35, 1.0, 1.0, [0,0,1], 3.4, 0.18, 0.80);
    spines(P2, 14.0, 2.35, 1.0, 1.0, [0,0,-1], 3.4, 0.18, 0.80);
    // the lance: nine emitters on a forward ring, all pointing +x
    for (i=[0:8]) let(a=360*i/9, rr=4.6)
        translate([33, rr*cos(a), rr*sin(a)]) {
            color(SPINE) rotate([0,90,0]) cylinder(h=9, r=0.26, $fn=7);
            color(GLOW)  translate([9,0,0]) sphere(r=0.40, $fn=9);
        }
    color(SPINE) for (i=[0:8]) let(a=360*i/9, rr=4.6)
        rod([33, rr*cos(a), rr*sin(a)], [33, 0,0], 0.13, 5);
    color(NODE) translate([33,0,0]) rotate([0,90,0]) cylinder(h=1.6, r=1.4, center=true, $fn=20);
    drives([-33.5,0,0], 5.2, 6, 9.5);
}

// ---- staging ---------------------------------------------------------
module ship(i) {
    if (i==1) spicule();
    if (i==2) aulonia();
    if (i==3) circogonia();
    if (i==4) hexactin();
    if (i==5) haeckel();
}
// The line-up is centred on the origin so the renderer's own bounding
// box fit frames it without a camera override.
if (SHOW == 0) {
    OFF = [0, 19, 44, 82, 138];
    for (i=[1:5]) translate([0, OFF[4]/2 - OFF[i-1], 0]) ship(i);
} else ship(SHOW);
