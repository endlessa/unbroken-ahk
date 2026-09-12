include <gears.scad>
include <noncirc.scad>
// ---- elliptical gear pair: 1:1 average ratio, 4:1 cyclic swing -----
//  Both gears turn about a FOCUS. Gear 2 is NOT rotated: its far end
//  (r = 54.4) already faces gear 1's near end (r = 13.6), and 13.6 +
//  54.4 = 68 = the centre distance, which is the rolling condition.
//  With an ODD tooth count the half-perimeter lands mid-tooth, so a
//  tooth meets a space at the contact with no extra clocking.
a = 34; e = 0.60; N = 33; H = 10;
S = ell_cum(a,e)[NS];
m = S/(N*PI);
echo("perimeter", S, "module", m, "centre distance", 2*a, "teeth", N);
echo("pitch radius", ell_r(0,a,e), "to", ell_r(180,a,e),
     "-> instantaneous ratio swing", ell_r(180,a,e)/ell_r(0,a,e));
color([0.88,0.72,0.36]) linear_extrude(height=H) polygon(nc_gear(a,e,N,m,20,0));
color([0.58,0.63,0.70]) translate([2*a,0,0])
    linear_extrude(height=H) polygon(nc_gear(a,e,N,m,20,0));
color([0.28,0.31,0.36]) translate([0,0,-8])    cylinder(h=8, r=3.4, $fn=48);
color([0.28,0.31,0.36]) translate([2*a,0,-8])  cylinder(h=8, r=3.4, $fn=48);
