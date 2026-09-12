include <gears.scad>
// ===================================================================
//  Two-axis gear ball -- six face gears and eight corner gears
//
//  The obvious thing to try is an octahedral ball: six bevel gears on
//  the +-x, +-y, +-z axes, each meshing its four neighbours.  It is
//  kinematically LOCKED and no amount of tooth cutting will free it.
//  Meshing gears must counter-rotate, so the mesh graph has to be
//  two-colourable, and on the octahedron +x, +y and +z are mutually
//  adjacent -- an odd cycle.  Three gears round a triangle cannot all
//  counter-rotate.
//
//  Put the second set on the CUBE DIAGONALS instead and the graph
//  becomes bipartite: faces mesh only corners and corners only faces,
//  so every cycle is even and the ball turns.  Six face gears, eight
//  corner gears, twenty-four meshes, one degree of freedom.
//
//  Shaft angle between a face axis and an adjacent diagonal is
//  acos(1/sqrt 3) = 54.7356 degrees, and for a bevel pair
//
//      tan(gam1) = sin(SIG) / (z2/z1 + cos(SIG))
//
//  with gam1 + gam2 = SIG.  Two packing limits: gam_face < 45, because
//  neighbouring face axes are 90 apart, and gam_corner < 35.264,
//  because neighbouring diagonals are acos(1/3) = 70.53 apart.
//
//  ASSEMBLY.  Twenty-four meshes and fourteen phases is over-
//  determined, and it does not close for arbitrary tooth counts.
//  Writing x_i = z_i * Phi_i, every mesh is x_i + x_j = const (mod
//  360); propagating round the graph and testing the surplus cycles
//  gives a residual that is always either exactly zero or exactly 120
//  degrees -- a three-fold mismatch, which is the three-fold symmetry
//  about each cube diagonal complaining.  Sweeping the counts, the
//  condition is exactly
//
//      z_corner divisible by 3,   z_face even
//
//  and nothing else.  24 and 20 -- the counts that first suggest
//  themselves -- do NOT assemble: 20 is not a multiple of 3, and the
//  eighth corner gear ends up a third of a pitch out.  24 and 18 do.
//
//  And the solution is clean.  Take each gear's datum to be the +x
//  axis carried round by rotate([0,0,phi]) rotate([0,theta,0]), the
//  same transform that puts the gear on its axis.  Then EVERY face
//  gear wants a tooth centreline on its datum and EVERY corner gear
//  wants a space -- one number each, for all fourteen.
// ===================================================================

ZF  = 24;                 // face gears   (must be even)
ZC  = 18;                 // corner gears (must be a multiple of 3)
M   = 1.8;
AL  = 20;
SIG = acos(1/sqrt(3));                        // 54.7356

GF  = atan2(sin(SIG), ZC/ZF + cos(SIG));      // face cone angle
GC  = SIG - GF;                               // corner cone angle
LO  = M*ZF/(2*sin(GF));                       // shared outer cone distance
LI  = 0.60*LO;
LH  = 0.26*LO;

EXPLODE = 0;              // 0 assembled; try 9 to lift them off the hub

echo("shaft angle", SIG, " face cone", GF, " corner cone", GC, " sum", GF+GC);
echo("packing: face cone <45 ?", GF < 45, "  corner cone <35.264 ?", GC < 35.264);
echo("outer cone distance from face", LO, " from corner", M*ZC/(2*sin(GC)));
echo("assembles ? corner teeth %3 =", ZC%3, " face teeth %2 =", ZF%2,
     "  (both must be 0)");
echo("ratio face:corner =", ZC/ZF, "  ball diameter =",
     2*(LO*sin(GF) + M*cos(GF)));

FACE = [0.86, 0.70, 0.34];
CORN = [0.40, 0.62, 0.72];
CORE = [0.30, 0.33, 0.38];

AXF = [ [1,0,0], [-1,0,0], [0,1,0], [0,-1,0], [0,0,1], [0,0,-1] ];
AXC = [ for (i=[-1,1]) for (j=[-1,1]) for (k=[-1,1]) [i,j,k] ];

function th_of(a) = acos(a[2]/norm(a));
function ph_of(a) = atan2(a[1], a[0]);

module gear_on(a, z, gam, ph, col) {
    rotate([0,0,ph_of(a)]) rotate([0,th_of(a),0]) translate([0,0,EXPLODE]) {
        color(col) bevel_true(M, z, gam, LO, LI, AL, ph);
        color(col) bev_dome(M, z, gam, LO, LH);
    }
}

// a tooth on the datum for the faces, a space for the corners
for (a = AXF) gear_on(a, ZF, GF, 0,       FACE);
for (a = AXC) gear_on(a, ZC, GC, 180/ZC,  CORN);

color(CORE) sphere(r = 8.0, $fn = 64);
