include <ship_lib.scad>
// ===================================================================
//  COBRA -- strike fighter                                    v7
//
//  Bow +x, up +z, port +y.
//
//  v1 read as a rodent: the hood sat behind the skull like a delta,
//  was far too narrow, and the head was too long.
//  v2 fixed the anatomy and read as a SNAKE -- correct, and useless,
//  because a spread hood drawn faithfully is an oval with two eye
//  spots on it.
//  v3 keeps the anatomy as the ARMATURE and builds an aircraft on it:
//
//    - The trailing edge stays the cobra ellipse. The leading edge is
//      straightened into a swept chine, so the planform reads delta
//      from the front and hood from behind. That one change is what
//      turns the outline from animal into aircraft.
//    - The spectacle drops to insignia scale. A marking that is 30%
//      of the wing is a face; at 8% it is a squadron badge.
//    - Cervical ribs become spanwise spars that break the skin into
//      panels, which is what they structurally are.
//    - The jaw gets an intake, the braincase gets a canopy, and the
//      hood margin gets tip pods. Hardware is what separates a machine
//      from an organism at silhouette range.
//
//  v6 painted it king cobra: black ground, gold chevrons, banded
//  throat.  Two things were wrong with that pass and v7 fixes them:
//    - The chevrons ran ~12% of chord each, so a third of the wing was
//      gold and the ship read gold-with-black-stripes.  Four bands at
//      ~5% each put the black back as the ground colour, which is also
//      the juvenile animal's actual proportion.
//    - The fuselage was a bare slab beside a busy wing.  It now carries
//      raised frames, a shoulder chine continuing the jawline, and the
//      cervical rib spars -- the hood is ribs 3-8 swung out under the
//      skin, so the spars are load path, not decoration.
//
//  Anatomy kept honest: elapid heads are DEPRESSED (wider than tall)
//  and the fangs are proteroglyphous -- short, fixed, at the FRONT of
//  the jaw -- so the cannon are stubby and forward, never slung back.
// ===================================================================

HXC  = -2.0;     // hood ellipse centre, fore-aft
HA   =  9.2;     // ellipse semi-axis fore-aft (governs the TRAILING edge)
HB   = 15.0;     // semi-axis lateral -- wider than long, as a hood is
XLE0 =  7.0;     // leading edge at the root
LESW =  9.0;     // leading-edge sweep to the tip
HTC  =  0.070;
CURL =  2.4;
NB   =  32;

XB   =  5.4;     // back of skull
XN   = 13.6;     // snout
HWD  =  3.40;    // half width
HHT  =  2.05;    // half height (< HWD: depressed)
HRZ  =  1.70;
HTIL =  7;
XA   = -12.5;    // stern

// KING COBRA (Ophiophagus hannah). The juvenile is jet black with
// bright chevrons; the adult keeps the banded throat. Near-black needs
// care under this lighting -- there is no specular to rescue it and
// the ambient floor is 0.25 -- so the "black" is 0.14, which reads as
// black against the 0.05 background while still taking a gradient.
DORS = [0.145, 0.145, 0.165];  // hood and dorsal
PANL = [0.205, 0.205, 0.225];  // panel steps, one value up
CHIN = [0.085, 0.085, 0.100];  // chines and shadow lines
VENT = [0.80, 0.755, 0.625];   // cream throat
GOLD = [0.88, 0.68, 0.17];     // chevrons
GOLD2= [0.60, 0.44, 0.11];     // deeper gold, for the shadowed side
MARK = [0.06, 0.06, 0.07];
BONE = [0.93, 0.91, 0.85];
EYE  = [0.99, 0.74, 0.10];
GLASS= [0.10, 0.145, 0.185];
MISL = [0.56, 0.57, 0.58];
DARK = [0.115, 0.115, 0.130];
GLOW = [0.55, 0.96, 0.42];
SPAR = [0.215, 0.215, 0.235];  // structure, ONE value above the skin

// Straight swept leading edge, elliptical cobra trailing edge. They
// meet at the tip on their own, so the outline closes without a clip.
function hood_grid(sgn) =
  [ for (j=[0:NB])
      let( s   = j/NB,
           e   = sqrt(max(0, 1 - s*s)),
           y   = sgn*HB*s,
           xle = XLE0 - LESW*s,
           xte = HXC - HA*e,
           c   = max(0.05, xle - xte),
           tc  = HTC*(1 - 0.22*s),
           z   = CURL*s*s )
        [ for (p = foil(c, tc, 22)) [ xle - p[0], y, z + p[1] ] ] ];

// A band at constant CHORD FRACTION runs spanwise; because the leading
// edge is swept, that band sweeps aft as it goes outboard, so across
// both wings it forms a V with its apex FORWARD. That is a chevron --
// and it is the king cobra's marking, pointing toward the head, for
// free out of the wing's own geometry. A band at constant SPAN would
// have given stripes parallel to the centreline instead, which is what
// sband() does and why it is used for panels, not for markings.
//
// foil(c,tc,22) lays out 23 upper-surface points (0 = leading edge,
// 22 = trailing) then 21 lower ones, so v indices below index chord.
module chevron(G, v0, v1, h=0.10) {
    smesh([ for (g=G)
        concat( [ for (v=[v0:v1])    g[v] + [0,0,h] ],
                [ for (v=[v1:-1:v0]) g[v] + [0,0,0.012] ] ) ]);
}

module hood() {
    GP = hood_grid(1); GS = hood_grid(-1);
    color(DORS) smesh(GP);            color(DORS) smesh(GS);
    color(PANL) sband(GP, 9, 13);     color(PANL) sband(GS, 9, 13);
    color(PANL) sband(GP, 21, 25);    color(PANL) sband(GS, 21, 25);
    // Four narrow chevrons, not three fat ones.  foil() spaces its
    // points by cosine, so a two-index band is ~5% of chord near
    // mid-chord and less near the edges; v6's three-index bands came
    // to 32% of the wing between them and the gold stopped being a
    // marking and became the paint.  The aftmost band fades to GOLD2
    // the way the juvenile's bands fade down the body.
    color(GOLD)  { chevron(GP,  4,  5); chevron(GS,  4,  5); }
    color(GOLD)  { chevron(GP,  8,  9); chevron(GS,  8,  9); }
    color(GOLD)  { chevron(GP, 12, 13); chevron(GS, 12, 13); }
    color(GOLD2) { chevron(GP, 16, 17); chevron(GS, 16, 17); }
}

// Leading-edge chine: a hard bright strake down the swept edge, which
// is the single strongest "this is an aircraft" cue at thumbnail size.
module chine() {
    for (sgn=[1,-1])
        color(CHIN) for (j=[0:NB-1]) let(s=j/NB, s2=(j+1)/NB)
            rod([XLE0 - LESW*s,  sgn*HB*s,  CURL*s*s + 0.04],
                [XLE0 - LESW*s2, sgn*HB*s2, CURL*s2*s2 + 0.04], 0.13, 6);
}

// Where the hood's upper skin actually is, so things can be laid ON
// it instead of near it.  s is span fraction, f is chord fraction.
function hood_x(s, f) = let( e  = sqrt(max(0, 1 - s*s)),
                             xl = XLE0 - LESW*s,
                             xt = HXC - HA*e )
    xl - f*(xl - xt);
function hood_z(s, f) = let( e  = sqrt(max(0, 1 - s*s)),
                             xl = XLE0 - LESW*s,
                             xt = HXC - HA*e,
                             c  = max(0.05, xl - xt),
                             tc = HTC*(1 - 0.22*s) )
    CURL*s*s + c*naca4(f, tc);

// A flat raised strip following a path across the skin.  A round rod
// was the obvious thing and it was wrong: seen from above, a thin
// cylinder's sides fall straight to the ambient floor, so every spar
// read as a dark scratch rather than a raised member.  A ribbon keeps
// a flat top facing the same way the wing's top faces, so it takes the
// same light and differs only by its own colour, which is the point.
//
// The section is laid out P-Dw -> P+Dw across the top, then back along
// the bottom, where D is the path tangent turned +90 about z.  Listed
// that way round, (section tangent) x (sweep direction) points DOWN
// through the top face -- into the solid -- which is the winding
// polyhedron() wants.  Reverse the two and the strip renders as flat
// ambient grey with no gradient, which is always the tell.
module ribbon(P, w, t) {
    n = len(P)-1;
    D = [ for (i=[0:n]) let(
            d = (i==0 ? P[1]-P[0] : (i==n ? P[n]-P[n-1] : P[i+1]-P[i-1])) )
          unit3([-d[1], d[0], 0]) ];
    smesh([ for (i=[0:n]) [ P[i]-D[i]*w,             P[i]+D[i]*w,
                            P[i]+D[i]*w-[0,0,t],     P[i]-D[i]*w-[0,0,t] ] ]);
}

// Cervical ribs.  A cobra's hood is not a wing that grew -- it is ribs
// 3-8 swung out sideways under the skin, which is why the animal can
// furl it.  Drawn here as the wing's root spars: converging at the
// spine, fanning outboard, following the skin rather than chording
// across it, and closed off by a root rib at their tips so the fan
// ends on a line instead of trailing away.  They sit above the
// chevrons because structure shows through paint, not the other way
// round.
// Held to one value above the skin and a sixth of a unit wide.  At two
// values and a third of a unit the fan stopped being structure under
// the paint and became a pale trapezoid sitting on top of it, fighting
// the insignia for the same piece of wing.  SPAR_Z puts the strips
// UNDER the chevrons, so the gold runs unbroken across them.
SPAR_S0 = 0.13; SPAR_S1 = 0.40; SPAR_Z = 0.06;
function rib_path(sgn, t, NS=12) =
  [ for (i=[0:NS]) let( q  = i/NS,
                        ss = SPAR_S0 + (SPAR_S1-SPAR_S0)*q,
                        ff = lerp(0.32 + 0.08*t, 0.12 + 0.62*t, q) )
      [ hood_x(ss,ff), sgn*HB*ss, hood_z(ss,ff) + SPAR_Z ] ];

module ribs() {
    NR = 5;
    // No root rib closing the tips.  One was drawn and it turned the fan
    // into an outlined triangle sitting on the wing -- the eye reads a
    // closed shape as a decal and an open fan as structure.
    for (sgn=[1,-1]) color(SPAR)
        for (k=[0:NR-1]) ribbon(rib_path(sgn, k/(NR-1)), 0.16, 0.11);
}

// Insignia scale, not face scale.
module spectacle() {
    for (sgn=[1,-1]) {
        s = 0.46; z = CURL*s*s + 0.62;
        translate([HXC - 0.6, sgn*HB*s, z]) {
            color(GOLD) cylinder(h=0.13, r=1.05, $fn=22);
            color(DORS) translate([0,0,0.13]) cylinder(h=0.12, r=0.60, $fn=18);
        }
    }
}

// Tip pods on the hood margin -- sensor forward, countermeasure aft.
// Centre the pod on the tip CHORD midpoint. Hanging it off the leading
// edge point is what made v3's pods float forward of the wing.
module tip_pods() {
    st = 0.975;
    et = sqrt(1 - st*st);
    xl = XLE0 - LESW*st;
    xt = HXC - HA*et;
    both_y() translate([(xl+xt)/2, HB*st, CURL*st*st]) {
        color(PANL) rotate([0,90,0]) cylinder(h=(xl-xt)+3.4, r=0.66, center=true, $fn=18);
        color(EYE)  translate([((xl-xt)+3.4)/2, 0, 0]) sphere(r=0.58, $fn=14);
        color(GLOW) translate([-((xl-xt)+3.4)/2, 0, 0]) sphere(r=0.40, $fn=12);
    }
}

// Underwing hardpoints.
// Two per side, not three, and a whole missile in a contrasting grey.
// In v3 the body was wing-coloured and only the pale nose showed, so
// the stores read as saw teeth along the leading edge.
module hardpoints() {
    both_y() for (i=[0:1]) let(yy = 6.2 + i*4.2, ss = yy/HB,
                               e2 = sqrt(max(0,1-ss*ss)),
                               xl = XLE0 - LESW*ss,
                               xt = HXC - HA*e2,
                               xx = (xl+xt)/2 + 1.2,
                               zw = CURL*ss*ss - 0.5*HTC*(xl-xt),
                               zz = zw - 1.18) {
        color(DARK) translate([xx, yy, (zw+zz)/2 + 0.1])
            cube([1.30, 0.34, zw-zz], center=true);
        color(MISL) translate([xx, yy, zz]) rotate([0,90,0])
            cylinder(h=5.6, r=0.42, center=true, $fn=14);
        color(MISL) translate([xx+2.8, yy, zz]) rotate([0,90,0])
            cylinder(h=1.3, r1=0.42, r2=0.07, $fn=14);
        color(DARK) translate([xx-2.6, yy, zz]) rotate([0,90,0])
            cylinder(h=0.5, r=0.46, $fn=14);
    }
}

// ---- skull -----------------------------------------------------------
NH = 26;
function sk_w(u) = HWD*(1 - 0.72*pow(u,2.1));
function sk_h(u) = HHT*(1 - 0.62*pow(u,1.8));

module skull() {
    S = [ for (i=[0:NH]) let(u=i/NH) [ XB + (XN-XB)*u, 0, -1.05*pow(u,2.3) ] ];
    G = [ for (i=[0:NH]) let(u=i/NH, F=frame_up(S,i,[0,0,1]))
            [ for (p = lame(sk_w(u), sk_h(u), 3.4, 40))
                S[i] + F[1]*p[0] + F[2]*p[1] ] ];
    color(DORS) smesh([ for (g=G) part_of(g, 20, 0, 3) ]);
    V = [ for (g=G) part_of(g, 0, 20, 3) ];
    color(VENT) smesh(V);
    // The banded throat. An adult king cobra keeps these long after the
    // juvenile's body chevrons have faded, so they belong on the head.
    color(DARK) sband(V, 3, 5);
    color(DARK) sband(V, 9, 11);
    color(DARK) sband(V, 15, 17);
    color(CHIN) sband([ for (g=G) part_of(g, 8, 12, 2) ], 0, NH);
    color(CHIN) sband([ for (g=G) part_of(g, 28, 32, 2) ], 0, NH);
}

module canopy() {
    color(GLASS) translate([8.05, 0, 1.34]) scale([2.10, 1.05, 0.58])
        sphere(r=1.24, $fn=26);
    color(DARK)  translate([6.20, 0, 1.22]) scale([0.60, 1.16, 0.50])
        sphere(r=1.22, $fn=18);
    color(PANL)  translate([10.0, 0, 1.02]) scale([0.90, 0.66, 0.30])
        sphere(r=1.08, $fn=16);
}

// Chin intake: the mouth. A lip ring proud of the jaw with a dark
// throat behind it -- additive, so no boolean cuts the hull.
module intake() {
    color(GOLD) translate([10.7, 0, -1.55]) rotate([0,90,0])
        cylinder(h=0.35, r1=1.50, r2=1.50, $fn=28);
    color(CHIN) translate([11.0, 0, -1.55]) rotate([0,90,0])
        cylinder(h=1.5, r1=1.45, r2=1.62, $fn=28);
    color([0.07,0.07,0.07]) translate([11.2, 0, -1.55]) rotate([0,90,0])
        cylinder(h=0.9, r=1.32, $fn=24);
}

// Maxillary chine: the hard edge where the skull's widest line runs
// from braincase to snout. A cobra head is angular BECAUSE of this
// ridge; without it the fuselage is just a lozenge.
module jawline() {
    for (sgn=[1,-1]) color(CHIN) for (i=[0:NH-1])
        let(u=i/NH, u2=(i+1)/NH)
          rod([XB + (XN-XB)*u,  sgn*sk_w(u)*0.99,  -1.05*pow(u,2.3)],
              [XB + (XN-XB)*u2, sgn*sk_w(u2)*0.99, -1.05*pow(u2,2.3)], 0.155, 6);
}

// Dorsal keel down the neck: snakes carry a raised vertebral ridge, and
// it gives the spine a highlight the flat lighting can actually catch.
module keel() {
    color(PANL) for (i=[0:NB2-1]) let(u=i/NB2, u2=(i+1)/NB2)
        rod([XB - (XB-XA)*u,  0, HRZ*(1-u)*0.35  + 2.0*(1-0.22*u)],
            [XB - (XB-XA)*u2, 0, HRZ*(1-u2)*0.35 + 2.0*(1-0.22*u2)], 0.22, 7);
}

module brow() {
    both_y() color(DORS) translate([8.2, 1.95, 1.05]) rotate([0,0,-14])
        scale([1.25,0.5,0.34]) sphere(r=1.5, $fn=16);
}
module eyes() {
    both_y() color(EYE)  translate([8.6, 2.18, 0.58]) scale([1.15,0.7,0.95]) sphere(r=0.62, $fn=16);
    both_y() color(DARK) translate([8.95, 2.34, 0.58]) scale([0.45,0.5,0.95]) sphere(r=0.42, $fn=12);
}
module fangs() {
    both_y() {
        color(DARK) translate([11.9, 1.05, -1.20]) rotate([0,84,0])
            cylinder(h=1.5, r=0.62, $fn=14);
        color(BONE) translate([12.6, 1.05, -1.32]) rotate([0,80,0])
            cylinder(h=2.4, r1=0.36, r2=0.20, $fn=14);
    }
}

// ---- neck and drive --------------------------------------------------
NB2 = 20;
function bd_w(u) = 3.2*(1 - 0.38*u);
function bd_h(u) = 2.0*(1 - 0.22*u);
function bd_S()  = [ for (i=[0:NB2]) let(u=i/NB2) [ XB - (XB-XA)*u, 0, HRZ*(1-u)*0.35 ] ];

// d inflates the section, so a band built from bd_G(d) stands PROUD of
// the hull instead of lying on it.  v6 drew its body bands straight off
// the body's own grid: two surfaces at identical depth, and the
// rasteriser picked per pixel, so the bands came out as faint seams
// rather than frames.  A tenth of a unit is enough to settle it.
function bd_G(d=0) = let(S = bd_S())
  [ for (i=[0:NB2]) let(u=i/NB2, F=frame_up(S,i,[0,0,1]))
      [ for (p = lame(bd_w(u)+d, bd_h(u)+d, 2.8, 40))
          S[i] + F[1]*p[0] + F[2]*p[1] ] ];

module body() {
    G = bd_G();
    color(DORS) smesh([ for (g=G) part_of(g, 20,  0, 3) ]);
    color(VENT) smesh([ for (g=G) part_of(g,  0, 20, 3) ]);
}

// Fuselage frames.  A snake's body is visibly segmented and a neck this
// long needs cross-section cues; the two jobs turn out to be one job.
module frames() {
    F = bd_G(0.11);
    color(GOLD) sband(F,  2,  3);    // squadron band, just aft of the skull
    color(PANL) sband(F,  6,  7);
    color(PANL) sband(F, 10, 11);
    color(PANL) sband(F, 14, 15);
}

// Shoulder chine: the hard edge the jawline gives the head, carried back
// along the neck so head and body read as one line rather than two parts.
// lame() puts its first point at [+a,0], so the widest station of each
// section is exactly where this rod goes.
module body_chine() {
    S = bd_S();
    for (sgn=[1,-1]) color(CHIN) for (i=[0:NB2-1])
        let(u = i/NB2, u2 = (i+1)/NB2)
          rod([S[i][0],   sgn*bd_w(u)*1.01,  S[i][2]],
              [S[i+1][0], sgn*bd_w(u2)*1.01, S[i+1][2]], 0.17, 6);
}

// Dorsal kit: an avionics blister forward and a sensor window aft, sunk
// into the spine so only the crowns show.
module spine_kit() {
    color(PANL)  translate([ 3.4, 0, 2.24]) scale([1.55, 0.62, 0.44])
        sphere(r=1.0, $fn=16);
    color(GLASS) translate([-2.2, 0, 1.92]) scale([1.30, 0.55, 0.38])
        sphere(r=1.0, $fn=14);
}
module drive() {
    both_y() {
        color(DARK) translate([XA+0.5, 1.20, 0.25]) rotate([0,-90,0])
            bell(1.05, 1.60, 3.0, 0.24, 24, 10);
        color(GLOW) translate([XA-2.1, 1.20, 0.25]) plume(1.30, 6.0, GLOW, 7);
    }
    // verniers on the hood root
    both_y() color(DARK) translate([XA+1.6, 3.6, 0.6]) rotate([0,-90,0])
        bell(0.34, 0.52, 1.1, 0.10, 14, 6);
}

hood();
chine();
ribs();
spectacle();
tip_pods();
hardpoints();
translate([0,0,HRZ]) rotate([0,-HTIL,0]) { skull(); jawline(); canopy(); intake(); brow(); eyes(); fangs(); }
body();
frames();
body_chine();
spine_kit();
keel();
drive();
