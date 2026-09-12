include <gears.scad>
// ===================================================================
//  Globoid (throated) worm and wheel
//
//  A cylindrical worm touches its wheel at a point.  A GLOBOID worm is
//  turned to an hourglass so that its body wraps the wheel's pitch
//  circle, and then several threads are in contact at once along a
//  line instead.  Same centre distance, several times the load.
//
//  The body is not a taper or a fillet, it is a definite surface.  Put
//  the wheel's axis on z and the worm's axis parallel to y at x = C.
//  A point on the worm's axis at axial station t is sqrt(C^2 + t^2)
//  from the wheel's axis, so for the worm's pitch surface to stay
//  tangent to the wheel's pitch cylinder everywhere,
//
//      r(t) = sqrt(C^2 + t^2) - rw
//
//  which is C - rw at the throat and grows either side of it.  That
//  one line is the whole difference between this and a plain worm.
//
//  The thread profile is specified in the AXIAL plane -- that is how a
//  lathe cuts it, the tool being fed in the plane of the axis -- so
//  the section swept here is the real one and not an approximation.
//
//  The wheel is left cylindrical.  Working the numbers below: the
//  throat a fully enveloping wheel would need is only about half a
//  millimetre deep over this face width, and a doubly-enveloping wheel
//  cannot be given a closed-form profile anyway -- it has to be hobbed
//  by a cutter that is a copy of the worm it will run with.
// ===================================================================

M    = 2.0;
ZW   = 40;                  // wheel teeth
NS   = 2;                   // worm starts
RW   = M*ZW/2;              // 40
RV   = 12;                  // worm pitch radius at the throat
C    = RW + RV;             // 52 -- centre distance
LEAD = NS*PI*M;             // axial advance per worm revolution
LAM  = atan(LEAD/(2*PI*RV));// lead angle at the throat = wheel helix angle
T    = 21;                  // half the worm's length
FW   = 15;                  // wheel face width

AD   = M;  DE = 1.25*M;
HT   = PI*M/4;              // half the axial thread thickness at the pitch line
PA   = 20;                  // flank angle in the axial plane

function wr(t) = sqrt(C*C + t*t) - RW;       // worm pitch radius at station t

echo("ratio =", ZW/NS, ": 1   (", NS, "start worm )");
echo("lead", LEAD, " lead angle at throat", LAM, " = wheel helix angle");
echo("worm pitch radius: throat", wr(0), " end", wr(T));
// self-locking when the lead angle is under the friction angle, about
// 5.7 degrees for steel on bronze
echo("lead angle", LAM, LAM < 5.7 ? "-- self locking" : "-- will back-drive");
// how deep a throat the wheel would need: the worm's root surface, at
// its nearest approach to the wheel's axis, at the centre and the end
function wtip(t) = sqrt(pow(C - (wr(t) - DE), 2) + t*t);
echo("wheel throat depth would be", wtip(FW/2) - wtip(0));

WORM  = [0.78, 0.62, 0.30];
WHEEL = [0.46, 0.62, 0.72];
BODY  = [0.46, 0.49, 0.54];
SHAFT = [0.32, 0.35, 0.39];

// ---- the worm ------------------------------------------------------
// built about its own +z, then laid on its side by the caller
module worm_body(n=40) {
    p = concat( [[0, -T]],
                [ for (i=[0:n]) let(t = -T + 2*T*i/n) [wr(t) - DE, t] ],
                [[0, T]] );
    rotate_extrude($fn=72) polygon(p);
}

// One thread.  Sweep parameter is the axial station; the section lies
// in the axial plane and the helix carries it round at 360*t/LEAD.
// Section order is (axial, radial) and comes out with the right-hand-
// rule normal pointing at the axis, which is what polyhedron wants.
function thread_sec() =
  let( tn = tan(PA), w = HT + DE*tn, n = HT - AD*tn )
    [ [-w, -DE], [-n, AD], [n, AD], [w, -DE] ];

module worm_thread(start, NU=160) {
    sec = thread_sec();
    gsweep([ for (i=[0:NU])
        let( t = -T + 2*T*i/NU,
             psi = 360*t/LEAD + 360*start/NS,
             r = wr(t), cp = cos(psi), sp = sin(psi) )
        [ for (q = sec) [ (r + q[1])*cp, (r + q[1])*sp, t + q[0] ] ] ]);
}

// ---- assembly ------------------------------------------------------
// The worm's start 1 sits at psi = 180 at t = 0, which in world terms
// is the point (C - RV, 0, 0) = (RW, 0, 0): dead on the wheel's pitch
// circle at azimuth 0. So the wheel wants a tooth SPACE there, and
// gear_poly already puts one on the 90 degree line -- 90 is a whole
// number of 9 degree pitches for 40 teeth, so a space lands on 0 too.
TW = deg(FW*tan(LAM)/RW);          // total twist over the face
PHW = TW/2;                        // ... so the mid-plane is the datum

translate([C,0,0]) rotate([90,0,0]) {
    color(BODY)  worm_body();
    for (s=[0:NS-1]) color(WORM) worm_thread(s);
    color(SHAFT) translate([0,0,-T-13]) cylinder(h=2*T+26, r=5.0, $fn=48);
}

translate([0,0,-FW/2]) {
    color(WHEEL) helical(M, ZW, FW, LAM, 20, 0, PHW, 24);
    color(BODY)  translate([0,0,1.5]) cylinder(h=FW-3, r=RW-DE-3.5, $fn=96);
}
color(SHAFT) translate([0,0,-FW/2-12]) cylinder(h=FW+24, r=6.5, $fn=56);
