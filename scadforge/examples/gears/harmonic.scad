include <gears.scad>
// ===================================================================
//  Strain-wave (harmonic) drive
//
//  The only common gearset that works by BENDING one of its members.
//  A thin flexspline with zf teeth is squeezed into an ellipse by the
//  wave generator and pressed into a rigid circular spline with zc
//  teeth.  Teeth engage at the two ends of the major axis and are
//  completely clear at the minor axis.  Turn the wave generator one
//  revolution and the flexspline has walked backwards by the tooth
//  difference, so
//
//      ratio = (zf - zc)/zf = -2/zf,   i.e. -zf/2 : 1
//
//  which is -30:1 here from a set 62 mm across.  The minus sign is
//  real: the output turns against the input.
//
//  zc - zf must be EVEN -- the wave generator engages at two places at
//  once, so the walk per revolution is shared between them -- and two
//  is what everyone uses, because the radial deflection is (zc-zf)m/2
//  and the flexspline has to survive it twice per turn, for ever.
//
//  The teeth are short and straight-sided, not involute, and that is
//  not a simplification: the motion here is radial in and out rather
//  than rolling, so an involute profile buys nothing.  Real strain
//  wave gearing uses exactly this kind of tooth, with a high flank
//  angle, and gets 20 or 30 tooth pairs in mesh at once -- which is
//  where the torque density and the zero backlash come from.
//
//  The neutral line of the flexspline must not stretch.  An ellipse is
//  only approximately right -- see the perimeter echoed below, which
//  is why real cams are not quite elliptical.
// ===================================================================

// Drawn at a deliberately LOW ratio. Real units run 30:1 to 320:1, and
// at zf = 60 the ellipse is 7 per cent out of round -- perfectly real
// and completely invisible. At zf = 30 the deflection is 7 per cent of
// the radius and you can see the wave, which is the point of a picture.
M   = 2.2;
ZF  = 30;                    // flexspline
ZC  = 32;                    // circular spline
RF  = M*ZF/2;                // 30
RC  = M*ZC/2;                // 31
W   = (ZC-ZF)*M/2;           // 1.0 -- radial deflection at the major axis
A   = RF + W;                // ellipse semi-major = RC: teeth fully in
B   = RF - W;                // semi-minor: teeth fully clear

AD  = 0.60*M;                // addendum  (short teeth)
DE  = 0.80*M;                // dedendum
FL  = 30;                    // flank angle from the tooth normal
HA  = PI*M/4;                // half the tooth thickness at the pitch line

H   = 15;                    // face width
WALL= 1.9;                   // flexspline rim
ZT  = 8;                     // underside of the toothed band

echo("ratio =", (ZF-ZC)/ZF, " i.e. 1 :", -ZF/2);
echo("deflection W =", W, " a =", A, " b =", B);
// Ramanujan's approximation -- good to about one part in 10^9 here
ELP = PI*(3*(A+B) - sqrt((3*A+B)*(A+3*B)));
echo("ellipse perimeter =", ELP, " vs circle", 2*PI*RF,
     " stretch =", ELP/(2*PI*RF) - 1);
echo("engaged: flex tip", A+AD, "< circ root", RC+DE, " clearance", RC+DE-(A+AD));
echo("clear:   flex tip", B+AD, "< circ tip ", RC-AD, " gap      ", RC-AD-(B+AD));

FLEX = [0.80, 0.60, 0.32];
SPL  = [0.56, 0.60, 0.66];
WG   = [0.34, 0.46, 0.58];
HUB  = [0.44, 0.47, 0.52];
SHFT = [0.30, 0.33, 0.37];

// ---- trapezoidal tooth, in a (tangent, outward normal) frame --------
// sgn = +1 external (tip outward), -1 internal (tip inward).  Wide at
// the root, narrow at the tip, either way round.
function trap(ha, ad, de, fl, sgn) =
  let( t = tan(fl), w = ha + de*t, n = ha - ad*t )
    [ [-w, -sgn*de], [-n, sgn*ad], [n, sgn*ad], [w, -sgn*de] ];

function place2(p, P, T, N) = P + T*p[0] + N*p[1];

// ---- the deformed neutral curve ------------------------------------
function hp(th)  = [A*cos(th), B*sin(th)];
function hd(th)  = [-A*sin(th), B*cos(th)];
function hT(th)  = hd(th)/norm(hd(th));
function hN(th)  = let(t = hT(th)) [t[1], -t[0]];      // outward

// Teeth sit at equal ARC LENGTH round the ellipse, not equal angle:
// the flexspline's rim does not stretch, so it is arc that is shared
// out evenly.  Spacing them by angle instead bunches them at the ends
// of the major axis, which is exactly where they have to mesh.
NSI = 1080;
function hseg(i) = norm(hp(360*(i+1)/NSI) - hp(360*i/NSI));
function hsum(v,i=0,acc=[0]) = i >= len(v) ? acc : hsum(v,i+1,concat(acc,[acc[i]+v[i]]));
HCUM = hsum([ for (i=[0:NSI-1]) hseg(i) ]);
HTOT = HCUM[NSI];
function h_idx(s) = [ for (i=[0:NSI-1]) if (HCUM[i] <= s && HCUM[i+1] > s) i ][0];
function h_th(s)  = let(i = h_idx(s), f = (s - HCUM[i])/(HCUM[i+1] - HCUM[i]))
                      360*(i + f)/NSI;

// outer contour: tooth, then a little of the root curve, then the next
function flex_outer() =
  [ for (k=[0:ZF-1])
      let( th = h_th(HTOT*k/ZF), P = hp(th), T = hT(th), N = hN(th) )
        each concat(
          [ for (p = trap(HA, AD, DE, FL, 1)) place2(p, P, T, N) ],
          [ for (j=[1:3])
              let( t2 = h_th(HTOT*(k + j/4)/ZF) )
                hp(t2) - hN(t2)*DE ] ) ];

function flex_inner(w) =
  [ for (i=[0:143]) let(th = 360*i/144) hp(th) - hN(th)*(DE + w) ];

module flexspline(h, w) {
    o = flex_outer(); n1 = len(o);
    i = flex_inner(w); n2 = len(i);
    linear_extrude(height = h)
        polygon(points = concat(o, i),
                paths  = [ [for (j=[0:n1-1]) j], [for (j=[0:n2-1]) n1+j] ]);
}

// ---- circular spline: rigid internal ring, same tooth form ----------
function circ_bore() =
  [ for (k=[0:ZC-1])
      let( th = 360*k/ZC, P = RC*[cos(th), sin(th)],
           T = [-sin(th), cos(th)], N = [cos(th), sin(th)] )
        each concat(
          [ for (p = trap(HA, AD, DE, FL, -1)) place2(p, P, T, N) ],
          [ for (j=[1:3]) let(t2 = th + (360/ZC)*j/4)
              (RC + DE)*[cos(t2), sin(t2)] ] ) ];

module circular_spline(h, ro) {
    b = circ_bore(); n = len(b);
    o = [ for (i=[0:127]) ro*[cos(360*i/128), sin(360*i/128)] ];
    linear_extrude(height = h)
        polygon(points = concat(o, b),
                paths  = [ [for (j=[0:127]) j], [for (j=[0:n-1]) 128+j] ]);
}

// ---- wave generator: the cam plus its bearing race ------------------
// The real cam is solid, and drawn solid it hides everything inside the
// flexspline. This one is the cam's bearing race plus a hub and two
// arms on the major axis -- same outline, and you can see through it.
module wave_gen(h, off, k) {
    e = [ for (i=[0:143]) let(th =  360*i/144) hp(th) - hN(th)*off ];
    i = [ for (j=[0:143]) let(th = -360*j/144) k*(hp(th) - hN(th)*off) ];
    n = len(e);
    linear_extrude(height = h)
        polygon(points = concat(e, i),
                paths  = [ [for (j=[0:n-1]) j], [for (j=[0:143]) n+j] ]);
}

// ===================================================================
WGO = DE + WALL + 0.3;            // cam rides the flexspline's bore
WGK = 0.80;                       // race thickness, as a fraction

color(SPL)  translate([0,0,ZT+0.6]) circular_spline(H-1.2, RC+5.5);
color(FLEX) translate([0,0,ZT])   flexspline(H, WALL);
color(WG)   translate([0,0,ZT+1.5]) {
    wave_gen(H-3, WGO, WGK);
    cylinder(h=H-3, r=9.5, $fn=64);
    for (sg=[0,180]) rotate([0,0,sg]) translate([0,-3.2,0])
        cube([WGK*(A-WGO)+0.5, 6.4, H-3]);
}
color(SHFT) translate([0,0,-8]) cylinder(h=H+ZT+12, r=5.4, $fn=64);

// output: the flexspline's flange, and the bolt circle it drives through
color(HUB)  translate([0,0,ZT-5]) cylinder(h=5, r1=22, r2=B-DE-WALL, $fn=96);
color(HUB)  translate([0,0,ZT-8]) cylinder(h=3, r=24.5, $fn=96);
for (k=[0:7]) color(SHFT) rotate([0,0,22.5+45*k])
    translate([20,0,ZT-8.7]) cylinder(h=1.4, r=1.7, $fn=20);
