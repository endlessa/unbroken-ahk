include <gears.scad>
include <cyclo.scad>
// ---- cycloidal reducer: 12 lobes rolling inside 13 pins -----------
//  One lobe fewer than there are pins, so a full turn of the eccentric
//  walks the disc back by exactly one pin: 12:1 from a single stage,
//  with roughly half the pins carrying load at any instant.
N = 13; R = 44; E = 3.2; rp = 4.6; H = 9;
echo("E < R/N ?", E, "<", R/N, "->", cyc_valid(N,R,E));
echo("pins", N, "lobes", N-1, "reduction", N-1, ": 1");
DISC=[0.88,0.72,0.36]; DISC2=[0.60,0.65,0.72]; PIN=[0.46,0.50,0.56]; HOUS=[0.26,0.29,0.34];
color(HOUS) translate([0,0,-5]) linear_extrude(height=5)
    polygon(points = concat([for(i=[0:119]) (R+11)*[cos(3*i),sin(3*i)]],
                            [for(i=[0:119]) (R-16)*[cos(-3*i),sin(-3*i)]]),
            paths  = [[for(i=[0:119]) i],[for(i=[0:119]) 120+i]]);
for (k=[0:N-1]) color(PIN) translate([R*cos(360*k/N), R*sin(360*k/N), -5])
    cylinder(h=H+7, r=rp, $fn=36);
color(DISC2) translate([-E,0,0])    linear_extrude(height=H*0.45)
    polygon(cyc_disc(N,R,E,rp));
color(DISC)  translate([ E,0,H*0.5]) rotate([0,0,180/(N-1)])
    linear_extrude(height=H*0.45) polygon(cyc_disc(N,R,E,rp));
