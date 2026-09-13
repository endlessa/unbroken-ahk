include <ship_lib.scad>
E2=len(geo_edges(2)); V2=len(geo_pts(2)); F2=len(geo_tris(2));
echo("f=2  V",V2," E",E2," F",F2,"   V-E+F =",V2-E2+F2,"(want 2)");
echo("myring d=4: r(0)",myring_r(0,4,3,6,5)," r(a)",myring_r(3,4,3,6,5),
     " r(a+b)",myring_r(9,4,3,6,5)," r(end)",myring_r(14,4,3,6,5));
echo("gielis starfish m=5 n=.3: r(0)",gielis_r(0,5,0.3,0.3,0.3),
     " r(36)",gielis_r(36,5,0.3,0.3,0.3));
echo("gielis diatom m=6 n1=40: r(0)",gielis_r(0,6,40,10,10),
     " r(30)",gielis_r(30,6,40,10,10));
geo_ball(3,2);
