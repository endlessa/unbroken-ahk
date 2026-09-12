include <gears.scad>
// ---- compound epicyclic set, herringbone cut -----------------------
m = 2.2; zs = 20; zp = 26; zr = 72; N = 4; H = 16; BETA = 24;
a  = m*(zs + zp)/2;
echo("assembly (zs+zr)/N =", plan_assembles(zs,zr,N), " must be whole");
echo("ratio  1 + zr/zs =", 1 + zr/zs);

SUN  = [0.86,0.70,0.38];
PLAN = [0.62,0.66,0.72];
RING = [0.40,0.44,0.50];
CARR = [0.30,0.33,0.38];

color(SUN)  herringbone(m, zs, H, BETA, 20, 0, 0);
for (k=[0:N-1]) {
    psi = 90 + 360*k/N;
    color(PLAN) translate([a*cos(psi), a*sin(psi), 0])
        herringbone(m, zp, H, -BETA, 20, 0, plan_phase(psi, 90, zs, zp));
}
color(RING) ring_herring(m, zr, H, m*zr/2 + 6.0, -BETA, 20, 0, 0);
// carrier: a spider, not a plate, so the train stays readable
color(CARR) translate([0,0,-8]) linear_extrude(height=5)
    polygon([ for (i=[0:143]) (9 + 2)*[cos(2.5*i), sin(2.5*i)] ]);
for (k=[0:N-1]) { psi = 90 + 360*k/N;
    color(CARR) translate([0,0,-8]) linear_extrude(height=5)
        polygon([ [0, -5], [a*cos(psi)+5*sin(psi), a*sin(psi)-5*cos(psi)],
                  [a*cos(psi)-5*sin(psi), a*sin(psi)+5*cos(psi)], [0, 5] ]);
    color(CARR) translate([a*cos(psi), a*sin(psi), -8]) cylinder(h=H+12, r=4.2, $fn=40); }
