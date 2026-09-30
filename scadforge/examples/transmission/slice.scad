// ===================================================================
//  slice.scad -- ONE transmission slice, assembled.
//
//  Row 1 of the design table in spherical_gear.scad, built as a
//  working planetary set on its own bevel sphere: a 46 tooth sun, a
//  59 tooth internal ring, SEVEN 13 tooth planets, and a carrier that
//  carries them -- hub, web, pin boss band, seven pins, retainer.
//
//  Every number below is read from the contract with sg_row(0),
//  sg_ns(), sg_m(), sg_phi(), sg_jt().  No number from that table
//  appears as a literal in the code.  What this file adds is the four
//  things the contract leaves open: WHERE the planets go, WHAT PHASE
//  each one is turned to, HOW the ring is clocked, and WHAT the
//  carrier is.
//
//  ---------------------------------------------------------------
//  1.  THE ROW, AS ANGLES
//
//  All members of a row share one apex and one cone-distance scale
//  D = 2L/m, so by the contract's (7) every cone angle is an arcsine
//  of an exact rational:
//
//      sin(gs) = Ns/D,  sin(gp) = Np/D,  sin(gr) = Nr/D        (1)
//
//  and the tangency condition (8), gr = gs + 2 gp, holds as an
//  identity of integers, not of decimals.  This file prints gs+2gp
//  and gr side by side.  The planet axis makes
//
//      beta = gs + gp                                          (2)
//
//  with the polar axis: the sun cone and the planet cone are tangent
//  along the ray at colatitude gs, and the planet cone and the ring
//  cone are tangent along the ray at colatitude gs + 2 gp = gr.  The
//  first of those rays sits at planet-frame azimuth 180 and the
//  second at planet-frame azimuth 0; the file prints, for each ray,
//  its angle to the planet axis against gp, and its planet-frame
//  azimuth.
//
//  This row hands over one small gift.  Expanding sin(gs+gp) under
//  (1) and clearing D,
//
//      sin(beta) D^2 = Ns sqrt(D^2-Np^2) + Np sqrt(D^2-Ns^2)      (2a)
//
//  and on this row D^2-Ns^2 = 1728 = 24^2*3 and D^2-Np^2 = 3675 =
//  35^2*3, both three times a square, so the right side is
//  (46*35 + 13*24) sqrt(3) = 1922 sqrt(3), and 2*1922 = 3844 = 62^2.
//  So sin(beta) = sqrt(3)/2 and beta is 60 degrees EXACTLY, not 60 to
//  five figures.  The file prints twice the right side of (2a)
//  against D^2 sqrt(3), and prints beta - 60.
//
//  The k planets are spaced Psi_i = 360 i/k about the polar axis.
//
//  ---------------------------------------------------------------
//  2.  THE PHASE EACH PLANET IS TURNED TO, AND WHERE IT COMES FROM
//
//  Take the sun-planet mesh of planet i.  The two pitch cones touch
//  along one ray; put a common arc coordinate s on the sphere of
//  radius L at that ray, running along the direction of increasing
//  global azimuth.  A point of the SUN at global azimuth Psi sits at
//
//      s = L sin(gs) rad(Psi - Psi_i) = p (Ns Psi/360 - Ns i/k)  (3)
//
//  because L sin(gs) = Ns m/2 and the circular pitch is p = pi m.
//  The planet's tangential direction at that ray points the OTHER
//  way -- that is what external meshing means -- so a point of the
//  planet at planet-frame azimuth psi sits at
//
//      s = -L sin(gp) rad(psi - 180) = -p (Np psi/360 - Np/2)   (4)
//
//  The sun's teeth are centred on 360 j/Ns.  The planet's teeth are
//  centred on 360 j'/Np + ph_i, where ph_i is the rotation of the
//  planet about its OWN axis that we are solving for.  Both lattices
//  have spacing p in s.  They mesh when one member's tooth centres
//  land on the other's space centres, that is, half a pitch apart:
//
//      -(Np ph_i/360 - Np/2)  ==  -Ns i/k + 1/2   (mod 1)
//
//   => Np ph_i/360 == Ns i/k + (Np - 1)/2         (mod 1)       (5)
//
//   => ph_i = (360/Np) frac( Ns i/k + (Np-1)/2 )                (6)
//
//  For Np odd the (Np-1)/2 is a whole number of pitches and drops
//  out, and (6) collapses to an exact rational:
//
//      ph_i = 360 ((Ns i) mod k) / (k Np)  degrees              (7)
//
//  with k Np = 91 here.  The file prints (6) and (7) together and
//  their difference, and prints the seven phases as the integers
//  ((Ns i) mod k).
//
//  Now the ring.  Its mesh with planet i is at planet azimuth 0 and
//  the tangential directions there agree in sign, because an
//  internal mesh rolls the same way.  Repeating (3) and (4) for the
//  ring, whose teeth are centred on 360 j''/Nr + alpha_r, and asking
//  again for the half pitch interleave,
//
//      Np ph_i/360 == Nr alpha_r/360 - Nr i/k + 1/2   (mod 1)   (8)
//
//  Subtracting (5) from (8) kills ph_i and leaves
//
//      (Ns + Nr) i/k == Nr alpha_r/360 + (2 - Np)/2   (mod 1)   (9)
//
//  ONE clocking alpha_r has to serve all k planets, so the left side
//  must not depend on i: (Ns+Nr)/k must be an integer.  That is the
//  contract's assembly gate (11), and it is why 105/7 = 15 matters.
//  With it, the left side vanishes and
//
//      alpha_r = (360/Nr) frac( (Np-2)/2 )                     (10)
//
//  which for Np odd is exactly half a ring pitch, 180/Nr.  The file
//  prints Nr alpha_r/360 and 1/2 side by side.
//
//  So the phasing is not fitted: it is (6) and (10), and the seven
//  planets are at seven DIFFERENT phases because Ns i/k is a
//  different fraction for each i.  The file prints all seven.
//
//  ---------------------------------------------------------------
//  3.  WHAT IS CHECKED, AND HOW
//
//  Every solid here is a cone from the common apex, so interference
//  between two members is a question on the unit sphere and nothing
//  else.  The file takes the boundary points the MESH is built from
//  -- the same sg_tooth_e / sg_tooth_i chains -- turns each into a
//  unit direction, rotates it into the other member's frame, and
//  evaluates the other member's own half width function sg_hw_e or
//  sg_hw_i at that colatitude.  The reported clearance is
//
//      |psi - nearest tooth centre| - halfwidth(colatitude)    (11)
//
//  in degrees of azimuth, minimised over every sampled point that
//  falls inside the other member's colatitude band.  It is done in
//  BOTH directions for every meshing pair, so a crossing shows up
//  from whichever side it happens on.  Sun and ring never share a
//  colatitude at all, and the file prints their two bands and the
//  count of sun points that reach the ring's, which is zero.  A point that lands in the other
//  member's root annulus is reported as interference outright.
//
//  Each member is thinned by jt/2 at each flank, so a centred mesh
//  should show jt m/2 of clearance on each flank.  The file prints
//  the measured minimum as an arc at the outer pitch circle next to
//  jt m/2.  Radially, the tooth proportions give
//
//      L tan(tf) - L tan(ta) = 1.25 m - m = 0.25 m             (12)
//
//  of tip-to-root clearance at every mesh, printed both sides.
//
//  The neighbour-planet margin is the tip cone separation
//
//      sep = acos( cos^2(beta) + sin^2(beta) cos(360/k) )      (13)
//      margin = sep - 2 (gp + ta)
//
//  and, because each planet solid is contained in its tip cone and
//  reaches it, (13) IS the exact minimum angle between two
//  neighbours.  The file prints it next to the smallest angle found
//  between two placed point sets sampled every fifth point; a coarse
//  sample can only overstate a minimum, so that number is expected to
//  land above (13), and it does.
//
//  ---------------------------------------------------------------
//  4.  THE CARRIER
//
//  Gate 4 of the contract, "intermediate sun bearing", was deferred
//  to this part.  Here it is, and it is a CONE, because everything
//  that shares this apex is.  The sun's back cone -- the bore the
//  library gives an external member -- is at colatitude
//
//      Ghub_s = max( gs/4, gs - tf - 2.5 (ta + tf) )           (14)
//
//  The carrier hub is a conical sleeve whose outer surface sits at
//  Ghub_s minus the angle that a running clearance CJ subtends at
//  the mean cone distance.  The sun turns on it over its whole face
//  width.  Its journal diameter is quoted at the mean cone distance
//  and its projected area is that diameter times the axial length of
//  the sun's face, which is the ordinary d*l of a plain journal.
//
//  Each planet turns on a conical pin the same way, inside its own
//  back cone Ghub_p.  The pins are seated on the web's pin boss band
//  and tied at their outer ends by a retainer band; the web's outer
//  spherical face and the retainer's inner spherical face are the
//  two thrust faces, and the gears float between them on CA.
//
//  EVERY RUNNING clearance in this machine is therefore an ANGLE, and
//  the linear gap it gives grows with cone distance.  The file prints
//  both of them at both ends of the face, Li and Lo, so that is
//  visible rather than asserted.  The clearances along the cone
//  distance -- gear face to thrust face, pin end to plate -- are
//  lengths, and are printed as lengths.
//
//  The ring's back cone is its housing seat.  The housing, and the
//  band that drives the slice at its equator, are not in this file.
//
//  (14) is written out here because sg_sector does not publish it.
//  It is not taken on trust: the file computes the solid angle of
//  each member from the contract's own sg_boundary using THIS Ghub
//  and prints it against sg_omega, which uses the library's.  If the
//  two ever drifted apart the identity would stop printing equal.
//
//  ---------------------------------------------------------------
//  5.  ONE POLYHEDRON PER MEMBER
//
//  sg_sector builds a SECTOR: an open strip with two end walls.  A
//  whole member closes on itself, so it has no end walls and its
//  surface is a torus, not a sphere: the toothed face, the back cone
//  face, and the two spherical end faces are four strips that wrap.
//  sl_tube below is that topology.  Its winding is the library's own
//  -- the cross section is traversed (j,k) = (0,1) (0,0) (1,0) (1,1)
//  so that each face reproduces sg_shell's face word for word -- and
//  that is why the right-hand normals point INTO the solid.  Two
//  faces sharing an edge traverse it in opposite directions.  That is
//  an orientation-blind property of the face word, and it is checked
//  from OUTSIDE this file -- by the exporter, which does not stay
//  quiet about an inside-out shell, and by the signed volume of the
//  exported mesh.  Nothing here relies on the picture: an inside-out
//  solid shades identically.
//
//  The carrier parts are bodies of revolution, and a body of
//  revolution is the same tube with a profile for a cross section.
//  The map (r, colatitude) -> (rho, z) has negative Jacobian, so a
//  profile listed counter-clockwise in the (r, colatitude) half
//  plane comes out clockwise in (rho, z), which is exactly the
//  handedness sl_tube wants.  That is why the profiles below read
//  naturally and still have their normals pointing inwards.
//
//  NOTHING in this file is subtracted.  Every bore is a back cone
//  that was built, not a plug that was removed.
//
//  The eighteen solids counted at the end of the report share no
//  volume.  That they leave the exporter as eighteen separate closed
//  shells says they share no VERTEX; that they do not interpenetrate
//  is what the clearance table says, and the clearance table is the
//  proof, not the count.
// ===================================================================

use <spherical_gear.scad>

// ---- which row this file BUILDS -------------------------------------
// Every derived number below is a FUNCTION of the row index, because the
// stack needs all four rows in one model and a stack cannot override a
// used file's variables.  SL_I only picks which row this file draws and
// reports on; it is a Customizer parameter, so `-D SL_I=3` builds the
// equator row instead.
SL_I = 0;

// ---- design choices made HERE, not read from the table --------------
SL_F     = 0.25;   // face width as a fraction of cone distance, so the
                   // face is L/4 against the contract's bound L/3
SL_CJ    = 0.05;   // sun journal running clearance, mm at mean cone dist
SL_CP    = 0.04;   // planet pin running clearance, mm at mean cone dist
SL_CA    = 0.25;   // cone-distance clearance, gear face to carrier face
SL_CS    = 0.05;   // seating clearance where a pin meets a plate
SL_THUB  = 3.0;    // carrier hub wall, mm at mean cone distance
SL_TWEB  = 3.0;    // carrier web thickness, mm of cone distance
SL_TBOSS = 2.0;    // extra depth of the pin boss band, mm
SL_TRET  = 3.0;    // retainer band thickness, mm of cone distance
SL_NRV   = 120;    // facets per revolution, revolved carrier bodies
SL_NRP   = 48;     // facets per revolution, pins
SL_ARC   = 1.2;    // degrees per sample on a constant-radius profile edge

// ---- the contract, per row ------------------------------------------
function sl_rowv(i) = sg_row(i);        // ["row n", Nr, D, Np, k]
function sl_nr(i)   = sg_row(i)[1];
function sl_dd(i)   = sg_row(i)[2];     // D, the cone-distance scale
function sl_np(i)   = sg_row(i)[3];
function sl_k(i)    = sg_row(i)[4];

function sl_gs(i)   = asin(sg_ns()/sl_dd(i));                      // (1)
function sl_gp(i)   = asin(sl_np(i)/sl_dd(i));
function sl_gr(i)   = asin(sl_nr(i)/sl_dd(i));
function sl_beta(i) = sl_gs(i) + sl_gp(i);                         // (2)
function sl_ta(i)   = sg_ta(sl_dd(i));
function sl_tf(i)   = sg_tf(sl_dd(i));
function sl_lo(i)   = sl_dd(i)*sg_m()/2;    // cone distance
function sl_li(i)   = sl_lo(i)*(1 - SL_F);  // inner cone distance
function sl_lm(i)   = (sl_li(i) + sl_lo(i))/2;

// ---- the library's back cone rule, reproduced and then checked ------
function sl_ghub(g, ext, i) =
  ext ? max(0.25*g, g - sl_tf(i) - 2.5*(sl_ta(i)+sl_tf(i)))         // (14)
      : g + sl_tf(i) + 2.5*(sl_ta(i)+sl_tf(i));
function sl_ghs(i) = sl_ghub(sl_gs(i), true,  i);   // sun bore
function sl_ghp(i) = sl_ghub(sl_gp(i), true,  i);   // planet bore
function sl_ghr(i) = sl_ghub(sl_gr(i), false, i);   // ring back cone

// ---- carrier, in (cone distance, colatitude) ------------------------
function sl_gj(i)    = sl_ghs(i) - deg(SL_CJ/sl_lm(i));   // sun journal cone
function sl_ghi(i)   = sl_gj(i)  - deg(SL_THUB/sl_lm(i)); // hub bore cone
function sl_gpin(i)  = sl_ghp(i) - deg(SL_CP/sl_lm(i));   // planet pin cone
function sl_gbore(i) = sl_gpin(i)/3;                      // oil bore
function sl_grim0(i) = sl_beta(i) - 2*sl_gpin(i);         // boss band, inner
function sl_grim1(i) = sl_beta(i) + 2*sl_gpin(i);         // boss band, outer

function sl_rw1(i)   = sl_li(i) - SL_CA;       // web outer face (a thrust face)
function sl_rw0(i)   = sl_rw1(i) - SL_TWEB;    // web inner face
function sl_rr0(i)   = sl_rw0(i) - SL_TBOSS;   // boss band inner face
function sl_r0(i)    = sl_rr0(i);              // carrier inboard face
function sl_rhub1(i) = sl_lo(i) + SL_CA;       // hub outboard end
function sl_rpin0(i) = sl_rw1(i) + SL_CS;      // pin seated end
function sl_rpin1(i) = sl_lo(i) + SL_CA - SL_CS;   // pin outer end
function sl_rret0(i) = sl_lo(i) + SL_CA;       // retainer inner face (thrust)
function sl_rret1(i) = sl_rret0(i) + SL_TRET;  // retainer outer face

// ---- where the row sits on the polar axis ---------------------------
// The row's ring pitch circle IS a latitude circle of the fundamental
// sphere: Nr = Dref sin(colat), and that circle's radius Nr m/2 equals
// R sin(colat) either way you compute it.  The row's own apex frame puts
// the circle at z = L cos(gamma_r) above the apex, so
//
//     z_apex = R cos(colat) - L cos(gamma_r),
//     colat  = asin(Nr/Dref)                                     (15)
//
// and the equator row, where gamma_r = colat = 90, has its apex ON the
// sphere centre.  Every other apex lies further up the axis.  That is
// the per-slice apex the contract's section 4 says a stack forces
// anyway, and (15) is where each one goes: one shaft, one sun count,
// one sun pitch diameter, stepped cones.
function sl_colat(i) = asin(sl_nr(i)/sg_dref());
function sl_apex(i)  = sg_r()*cos(sl_colat(i)) - sl_lo(i)*cos(sl_gr(i));

// ---- published so a stack can place a row without restating anything
function sl_if_apex(i)  = sl_apex(i);            // 1 z of the apex, mm
function sl_if_colat(i) = sl_colat(i);           // 2 the ring's latitude
function sl_if_beta(i)  = sl_beta(i);            // 3 the planet axis cone
function sl_if_cone(i)  = [sl_li(i), sl_lo(i)];  // 4 inner, outer cone dist
function sl_if_ring(i)  = [sl_gr(i), sl_ghr(i), sl_clock(i)];
                                     // 5 ring pitch cone, back cone, clocking
function sl_if_carrier(i) = [sl_r0(i), sl_rret1(i), sl_ghi(i), sl_grim1(i)];
                                     // 6 the carrier's own extent
function sl_if_sun(i)   = [sl_gs(i), sl_ghs(i), sl_gj(i)];
                                     // 7 sun pitch cone, bore, journal

// ---- the row SL_I, which is what this file draws and reports on -----
ROW  = sl_rowv(SL_I);
NR_  = sl_nr(SL_I);  D = sl_dd(SL_I);  NP_ = sl_np(SL_I);  K = sl_k(SL_I);
NS_  = sg_ns();  M = sg_m();  PHI = sg_phi();  JT = sg_jt();

GS = sl_gs(SL_I);  GP = sl_gp(SL_I);  GR = sl_gr(SL_I);
BETA = sl_beta(SL_I);
TA = sl_ta(SL_I);  TF = sl_tf(SL_I);
LO = sl_lo(SL_I);  LI = sl_li(SL_I);  LM = sl_lm(SL_I);
GHS = sl_ghs(SL_I);  GHP = sl_ghp(SL_I);  GHR = sl_ghr(SL_I);
GJ = sl_gj(SL_I);  GHI = sl_ghi(SL_I);
GPIN = sl_gpin(SL_I);  GBORE = sl_gbore(SL_I);
GRIM0 = sl_grim0(SL_I);  GRIM1 = sl_grim1(SL_I);
RW1 = sl_rw1(SL_I);  RW0 = sl_rw0(SL_I);  RR0 = sl_rr0(SL_I);  R0 = sl_r0(SL_I);
RHUB1 = sl_rhub1(SL_I);  RPIN0 = sl_rpin0(SL_I);  RPIN1 = sl_rpin1(SL_I);
RRET0 = sl_rret0(SL_I);  RRET1 = sl_rret1(SL_I);

// ---- phasing --------------------------------------------------------
function sl_frac(x)        = x - floor(x);
function sl_phase(j, i)    = (360/sl_np(i))
                             *sl_frac(sg_ns()*j/sl_k(i) + (sl_np(i)-1)/2); // (6)
function sl_phase_rat(j,i) = 360*((sg_ns()*j) % sl_k(i))/(sl_k(i)*sl_np(i));
                                                                          // (7)
function sl_clock(i)       = (360/sl_nr(i))*sl_frac((sl_np(i)-2)/2);      // (10)

// ---- small vector helpers -------------------------------------------
function rz(v,a) = [v[0]*cos(a) - v[1]*sin(a), v[0]*sin(a) + v[1]*cos(a), v[2]];
function ry(v,a) = [v[0]*cos(a) + v[2]*sin(a), v[1], -v[0]*sin(a) + v[2]*cos(a)];
// planet i's frame <-> world
function to_world(u,j,i)  = rz(ry(rz(u, sl_phase(j,i)), sl_beta(i)),
                               360*j/sl_k(i));
function to_planet(u,j,i) = rz(ry(rz(u, -360*j/sl_k(i)), -sl_beta(i)),
                               -sl_phase(j,i));
function colat(u) = acos(max(-1, min(1, u[2])));
function azim(u)  = atan2(u[1], u[0]);
function wrapc(x,P) = ((x + P/2) % P + P) % P - P/2;   // to [-P/2, P/2)

// ===================================================================
//  MESH PRIMITIVES
// ===================================================================

// A closed tube.  rings[i] is a closed cross-section loop of the same
// length for every i, and i itself is cyclic, so the surface is a
// torus with no end walls.  The face word is sg_shell's, so the
// right-hand normal of every face points INTO the solid.
module sl_tube(rings) {
    NI = len(rings);  NP = len(rings[0]);
    pts = [ for (i=[0:NI-1]) for (p=[0:NP-1]) rings[i][p] ];
    id  = function (i,p) ((i % NI)*NP + (p % NP));
    polyhedron(points = pts,
      faces = [ for (i=[0:NI-1]) for (p=[0:NP-1])
                  [ id(i,p), id(i+1,p), id(i+1,p+1), id(i,p+1) ] ],
      convexity = 8);
}

// Corners given as [cone distance, colatitude], counter-clockwise in
// that half plane.  Edges at constant cone distance are spherical, so
// they are sampled as arcs; edges at constant colatitude are rays
// through the apex and stay straight.
function sl_prof(C) =
  [ for (i=[0:len(C)-1])
      let (a = C[i], b = C[(i+1) % len(C)],
           n = abs(a[0]-b[0]) < 1e-9 ? max(1, ceil(abs(b[1]-a[1])/SL_ARC)) : 1)
        each [ for (s=[0:n-1]) [ a[0] + (b[0]-a[0])*s/n,
                                 a[1] + (b[1]-a[1])*s/n ] ] ];

module sl_revolve(C, n) {
    P = [ for (q = sl_prof(C)) [ q[0]*sin(q[1]), q[0]*cos(q[1]) ] ];
    sl_tube([ for (a=[0:n-1]) let (t = 360*a/n)
                [ for (q = P) [ q[0]*cos(t), q[0]*sin(t), q[1] ] ] ]);
}

// Exact volume of a [r0,r1] x [G0,G1] rectangle of that half plane,
// swept round: the spherical shell wedge.
function sl_wedge(r0,r1,G0,G1) = 2*PI*(cos(G0)-cos(G1))*(r1*r1*r1 - r0*r0*r0)/3;

// ===================================================================
//  MEMBERS
//  A whole member: N teeth all the way round, one polyhedron, apex at
//  the origin, axis on +z, clocked by alpha.
// ===================================================================
module sl_member(ri, N, ext, alpha = 0) {
    g  = asin(N/sl_dd(ri));  gb = sg_gb(g, sg_phi());
    ht = sg_ht(N, sg_jt()*sg_m(), sg_m());
    Gh = sl_ghub(g, ext, ri);
    ta = sl_ta(ri);  tf = sl_tf(ri);  lo = sl_lo(ri);  li = sl_li(ri);
    b0 = [ for (j=[0:N-1]) each (ext ? sg_tooth_e(j,N,g,gb,ht,ta,tf)
                                     : sg_tooth_i(j,N,g,gb,ht,ta,tf)) ];
    // the triad (d_i, d_j, d_k) has to stay right handed, and d_j flips
    // between an external and an internal member, so the internal
    // boundary is walked the other way round.
    b  = ext ? b0 : [ for (q=[len(b0)-1:-1:0]) b0[q] ];
    rotate([0,0,alpha])
      sl_tube([ for (p = b)
                  [ sg_xyz(p[0], p[1], lo), sg_xyz(p[0], p[1], li),
                    sg_xyz(Gh,   p[1], li), sg_xyz(Gh,   p[1], lo) ] ]);
}

// the member's boundary as unit directions, for the interference test
function sl_dirs(ri, N, ext, alpha) =
  let( g = asin(N/sl_dd(ri)), gb = sg_gb(g, sg_phi()),
       ht = sg_ht(N, sg_jt()*sg_m(), sg_m()),
       ta = sl_ta(ri), tf = sl_tf(ri) )
    [ for (j=[0:N-1])
        each [ for (p = (ext ? sg_tooth_e(j,N,g,gb,ht,ta,tf)
                             : sg_tooth_i(j,N,g,gb,ht,ta,tf)))
                 sg_xyz(p[0], p[1] + alpha, 1) ] ];

// ===================================================================
//  CARRIER
// ===================================================================
// hub, web and pin boss band are one body of revolution: a conical
// sleeve carrying the sun, a spherical web, and a deeper band under
// the seven pins.
function sl_carrier(ri) =
  [ [sl_r0(ri),    sl_ghi(ri)  ], [sl_rhub1(ri), sl_ghi(ri)  ],
    [sl_rhub1(ri), sl_gj(ri)   ], [sl_rw1(ri),   sl_gj(ri)   ],
    [sl_rw1(ri),   sl_grim1(ri)], [sl_rr0(ri),   sl_grim1(ri)],
    [sl_rr0(ri),   sl_grim0(ri)], [sl_rw0(ri),   sl_grim0(ri)],
    [sl_rw0(ri),   sl_gj(ri)   ], [sl_r0(ri),    sl_gj(ri)   ] ];
module sl_carrier_body(ri) { sl_revolve(sl_carrier(ri), SL_NRV); }

// one planet pin: a conical journal with an oil bore up the middle
module sl_pin(ri) {
    sl_revolve([ [sl_rpin0(ri), sl_gbore(ri)], [sl_rpin1(ri), sl_gbore(ri)],
                 [sl_rpin1(ri), sl_gpin(ri) ], [sl_rpin0(ri), sl_gpin(ri) ] ],
               SL_NRP);
}

// the retainer band that ties the pin ends together
module sl_retainer(ri) {
    sl_revolve([ [sl_rret0(ri), sl_grim0(ri)], [sl_rret1(ri), sl_grim0(ri)],
                 [sl_rret1(ri), sl_grim1(ri)], [sl_rret0(ri), sl_grim1(ri)] ],
               SL_NRV);
}

// ===================================================================
//  INTERFERENCE:  (11), evaluated with the other member's own half
//  width function at the colatitude the sampled point actually lands
//  on.  Positive is clear, negative is interference, and a point that
//  lands in the other member's root annulus returns -BIG.
// ===================================================================
BIG = 1e6;
function sl_cl_e(ri, u, N, alpha) =
  let( g = asin(N/sl_dd(ri)), gb = sg_gb(g,sg_phi()),
       ht = sg_ht(N, sg_jt()*sg_m(), sg_m()),
       Gf = g - sl_tf(ri), Ga = g + sl_ta(ri), Gh = sl_ghub(g, true, ri),
       G = colat(u), d = wrapc(azim(u) - alpha, 360/N) )
    (G < Gh || G > Ga) ? BIG : (G <= Gf) ? -BIG
                             : abs(d) - max(0, sg_hw_e(G,g,gb,ht));
function sl_cl_i(ri, u, N, alpha) =
  let( g = asin(N/sl_dd(ri)), gb = sg_gb(g,sg_phi()),
       ht = sg_ht(N, sg_jt()*sg_m(), sg_m()),
       Gf = g + sl_tf(ri), Ga = g - sl_ta(ri), Gh = sl_ghub(g, false, ri),
       G = colat(u), d = wrapc(azim(u) - alpha, 360/N) )
    (G > Gh || G < Ga) ? BIG : (G >= Gf) ? -BIG
                             : abs(d) - max(0, sg_hw_i(G,g,gb,ht));
function sl_engaged(v) = len([ for (x = v) if (x < BIG) 1 ]);

SUN_D  = sl_dirs(SL_I, NS_, true,  0);
PLAN_D = sl_dirs(SL_I, NP_, true,  0);
RING_D = sl_dirs(SL_I, NR_, false, sl_clock(SL_I));

// arc at the outer pitch circle subtended by an azimuthal clearance
function sl_arc(ri, cldeg, g) = rad(cldeg)*sl_lo(ri)*sin(g);

// ===================================================================
//  REPORT
// ===================================================================
echo("=== slice: row 1 of the design table, assembled ===");
echo(str(ROW[0], ":  Nr=", NR_, " D=", D, " Np=", NP_, " k=", K,
         "   Ns=", NS_, "   m=", M, " mm   phi=", PHI, " deg   jt=", JT,
         " modules = ", JT*M, " mm per mesh"));
echo(str("gs = asin(", NS_, "/", D, ") = ", GS,
         "   gp = asin(", NP_, "/", D, ") = ", GP,
         "   gr = asin(", NR_, "/", D, ") = ", GR));
echo(str("tangency (8):  gs + 2 gp = ", GS+2*GP, "   gr = ", GR,
         "   difference = ", GS+2*GP - GR));
echo(str("planet axis beta = gs + gp = ", BETA,
         "   planet azimuths Psi_i = 360 i/", K));
echo(str("   (2a) D^2-Ns^2 = ", D*D-NS_*NS_, " = ", isq((D*D-NS_*NS_)/3),
         "^2 * 3   D^2-Np^2 = ", D*D-NP_*NP_, " = ", isq((D*D-NP_*NP_)/3),
         "^2 * 3   so 2(Ns sqrt(D^2-Np^2) + Np sqrt(D^2-Ns^2)) = ",
         2*(NS_*sqrt(D*D-NP_*NP_) + NP_*sqrt(D*D-NS_*NS_)),
         "   D^2 sqrt(3) = ", D*D*sqrt(3),
         "   difference ", 2*(NS_*sqrt(D*D-NP_*NP_) + NP_*sqrt(D*D-NS_*NS_))
                           - D*D*sqrt(3),
         "   so beta - 60 = ", BETA - 60));
echo(str("L = D m/2 = ", LO, " mm   face = L*", SL_F, " = ", LO-LI,
         " mm   contract's bound L/3 = ", LO/3, " mm   within: ",
         LO - LI <= LO/3));
echo(str("pitch radii  L sin(gs) = ", LO*sin(GS), " = Ns m/2 = ", NS_*M/2,
         "   L sin(gp) = ", LO*sin(GP), " = Np m/2 = ", NP_*M/2,
         "   L sin(gr) = ", LO*sin(GR), " = Nr m/2 = ", NR_*M/2));

echo("--- Willis, exact rationals ---");
GW = gcd_(NR_, NS_);
echo(str("(w_sun - w_carrier)/(w_ring - w_carrier) = -Nr/Ns = -", NR_, "/", NS_,
         " = -", NR_/GW, "/", NS_/GW, " = ", -NR_/NS_,
         "   gcd(", NR_, ",", NS_, ") = ", GW));
GC = gcd_(NS_+NR_, NS_);
echo(str("ring braked: w_sun/w_carrier = (Ns+Nr)/Ns = ", NS_+NR_, "/", NS_,
         " = ", (NS_+NR_)/GC, "/", NS_/GC, " = ", (NS_+NR_)/NS_));
echo(str("carrier braked: w_ring/w_sun = -Ns/Nr = -", NS_, "/", NR_,
         " = -", NS_/GW, "/", NR_/GW, " = ", -NS_/NR_));
echo(str("assembly gate (11): (Ns+Nr)/k = ", NS_+NR_, "/", K, " = ",
         (NS_+NR_)/K, "   integer: ", (NS_+NR_) % K == 0));

echo("--- planet phase, equation (6) against equation (7) ---");
for (i = [0:K-1])
  echo(str("   planet ", i, "  Psi = ", 360*i/K,
           "   (Ns i) mod k = ", (NS_*i) % K,
           "   ph = 360*", (NS_*i) % K, "/(", K, "*", NP_, ") = ",
           sl_phase_rat(i,SL_I), " deg   (6) gives ", sl_phase(i,SL_I),
           "   difference ", sl_phase(i,SL_I) - sl_phase_rat(i,SL_I)));
echo(str("   (Np-1)/2 = ", (NP_-1)/2, ", a whole number of pitches, which is",
         " why (6) collapses to (7); frac((Np-1)/2) = ", sl_frac((NP_-1)/2),
         ";  the phases are the ", K, " multiples of 360/(k Np) = 360/", K*NP_,
         " = ", 360/(K*NP_), " deg, in the order ",
         [ for (i=[0:K-1]) (NS_*i) % K ]));
echo(str("ring clocking (10): alpha_r = ", sl_clock(SL_I), " deg = 180/", NR_,
         " = ", 180/NR_, "   Nr alpha_r/360 = ", NR_*sl_clock(SL_I)/360,
         "   against 1/2 = ", 0.5));

echo("--- the two mesh rays, in the planet's own frame ---");
RAY_S = [sin(GS), 0, cos(GS)];    RAY_R = [sin(GR), 0, cos(GR)];
PAX   = [sin(BETA), 0, cos(BETA)];
echo(str("   sun-planet ray at colat ", GS, ": angle to the planet axis = ",
         acos(RAY_S*PAX), " = gp = ", GP, ", planet azimuth ",
         azim(ry(RAY_S, -BETA))));
echo(str("   ring-planet ray at colat ", GR, ": angle to the planet axis = ",
         acos(RAY_R*PAX), " = gp = ", GP, ", planet azimuth ",
         azim(ry(RAY_R, -BETA))));

echo("--- back cone rule (14) checked against the library ---");
module sl_omega_check(ri, N, ext, label) {
    g = asin(N/sl_dd(ri)); gb = sg_gb(g,sg_phi());
    ht = sg_ht(N, sg_jt()*sg_m(), sg_m());
    Gh = sl_ghub(g, ext, ri);
    b  = sg_boundary(N, N, g, gb, ht, sl_ta(ri), sl_tf(ri), ext);
    t  = [ for (i=[0:len(b)-2])
             rad(b[i+1][1]-b[i][1]) *
             ( ext ? cos(Gh) - (cos(b[i][0]) + cos(b[i+1][0]))/2
                   : (cos(b[i][0]) + cos(b[i+1][0]))/2 - cos(Gh) ) ];
    echo(str("   ", label, "  Ghub = ", Gh, "  omega from sg_boundary with it = ",
             sum(t), "   sg_omega = ", sg_omega(N,N,sl_dd(ri),ext),
             "   difference = ", sum(t) - sg_omega(N,N,sl_dd(ri),ext)));
}
sl_omega_check(SL_I, NS_, true,  "sun   ");
sl_omega_check(SL_I, NP_, true,  "planet");
sl_omega_check(SL_I, NR_, false, "ring  ");

echo("--- radial clearance, equation (12) ---");
echo(str("   L tan(tf) - L tan(ta) = ", LO*tan(TF), " - ", LO*tan(TA), " = ",
         LO*tan(TF) - LO*tan(TA), "   against 0.25 m = ", 0.25*M));
echo(str("   sun root cone gs-tf = ", GS-TF, "   planet tip reaches colat ",
         BETA - (GP+TA), " = gs-ta = ", GS-TA, "   gap ", (GS-TA) - (GS-TF),
         " deg = ", rad((GS-TA)-(GS-TF))*LO, " mm at L"));
echo(str("   ring root cone gr+tf = ", GR+TF, "   planet tip reaches colat ",
         BETA + GP+TA, " = gr+ta = ", GR+TA, "   gap ", (GR+TF)-(GR+TA),
         " deg = ", rad((GR+TF)-(GR+TA))*LO, " mm at L"));

echo("--- flank clearance, equation (11), both directions, every pair ---");
echo(str("    design half backlash jt m/2 = ", JT*M/2, " mm at the outer pitch"));
for (i = [0:K-1]) {
  a = [ for (u = PLAN_D) sl_cl_e(SL_I, to_world(u,i,SL_I), NS_, 0) ];
  b = [ for (u = SUN_D)  sl_cl_e(SL_I, to_planet(u,i,SL_I), NP_, 0) ];
  echo(str("   sun /planet ", i, "  planet pts engaged ", sl_engaged(a),
           " min ", min(a), " deg = ", sl_arc(SL_I, min(a), GS),
           " mm  |  sun pts ", sl_engaged(b), " min ", min(b), " deg = ",
           sl_arc(SL_I, min(b), GP), " mm"));
}
for (i = [0:K-1]) {
  a = [ for (u = PLAN_D) sl_cl_i(SL_I, to_world(u,i,SL_I), NR_,
                                  sl_clock(SL_I)) ];
  b = [ for (u = RING_D) sl_cl_e(SL_I, to_planet(u,i,SL_I), NP_, 0) ];
  echo(str("   ring/planet ", i, "  planet pts engaged ", sl_engaged(a),
           " min ", min(a), " deg = ", sl_arc(SL_I, min(a), GR),
           " mm  |  ring pts ", sl_engaged(b), " min ", min(b), " deg = ",
           sl_arc(SL_I, min(b), GP), " mm"));
}
SUNRING = [ for (u = SUN_D) sl_cl_i(SL_I, u, NR_, sl_clock(SL_I)) ];
echo(str("   sun/ring   sun pts engaged in the ring's colatitude band ",
         sl_engaged(SUNRING), " -- the two never share a colatitude: sun spans ",
         sl_ghub(GS,true,SL_I), " to ", GS+TA, ", ring spans ", GR-TA, " to ",
         GHR));

echo("--- neighbouring planets, equation (13) ---");
SEP = acos(cos(BETA)*cos(BETA) + sin(BETA)*sin(BETA)*cos(360/K));
P0 = [ for (s=[0:5:len(PLAN_D)-1]) to_world(PLAN_D[s], 0, SL_I) ];
P1 = [ for (s=[0:5:len(PLAN_D)-1]) to_world(PLAN_D[s], 1, SL_I) ];
NBR = [ for (u = P0) min([ for (v = P1) acos(max(-1, min(1, u*v))) ]) ];
echo(str("   axis separation sep = ", SEP, " deg   two tip cones need ",
         2*(GP+TA), " deg   margin = ", SEP - 2*(GP+TA), " deg = ",
         rad(SEP-2*(GP+TA))*LO, " mm at L"));
echo(str("   smallest angle found between the placed point sets of planet 0",
         " and planet 1, every 5th boundary point, ", len(P0), " x ", len(P1),
         " pairs = ", min(NBR), " deg; a coarse sample can only overstate a",
         " minimum, and ", min(NBR), " >= ", SEP - 2*(GP+TA), " is ",
         min(NBR) >= SEP - 2*(GP+TA)));

echo("--- carrier ---");
echo(str("   sun back cone (14) Ghub_s = max(gs/4 = ", 0.25*GS, ", gs-tf-2.5(ta+tf) = ",
         GS-TF-2.5*(TA+TF), ") = ", GHS));
echo(str("   journal cone GJ = Ghub_s - deg(CJ/Lm) = ", GHS, " - ",
         deg(SL_CJ/LM), " = ", GJ,
         "   gap to the sun bore: ", LI*sin(GHS-GJ), " mm at Li = ", LI,
         ", ", LO*sin(GHS-GJ), " mm at Lo = ", LO));
echo(str("   sun journal diameter at Lm = ", LM, ": d = 2 Lm sin(GJ) = ",
         2*LM*sin(GJ), " mm; axial length of the sun face l = (Lo-Li) cos(GJ) = ",
         (LO-LI)*cos(GJ), " mm; projected area d*l = ", 2*LM*sin(GJ)*(LO-LI)*cos(GJ),
         " mm^2"));
echo(str("   hub wall ", SL_THUB, " mm at Lm: bore cone GHI = ", GHI,
         ", wall ", R0*sin(GJ-GHI), " mm at r = ", R0, " and ",
         RHUB1*sin(GJ-GHI), " mm at r = ", RHUB1));
echo(str("   planet back cone Ghub_p = ", GHP, "   pin cone GPIN = ", GPIN,
         "   gap to the planet bore: ", LI*sin(GHP-GPIN), " mm at Li, ",
         LO*sin(GHP-GPIN), " mm at Lo"));
echo(str("   pin diameter at Lm = 2 Lm sin(GPIN) = ", 2*LM*sin(GPIN),
         " mm; bearing length (Lo-Li) cos(GPIN) = ", (LO-LI)*cos(GPIN),
         " mm; projected area = ", 2*LM*sin(GPIN)*(LO-LI)*cos(GPIN), " mm^2",
         "; oil bore ", 2*LM*sin(GBORE), " mm at Lm"));
echo(str("   thrust faces: web outer at r = ", RW1, ", retainer inner at r = ",
         RRET0, "; the gears span r = ", LI, " to ", LO,
         ", so each floats on ", LI-RW1, " and ", RRET0-LO, " mm"));
echo(str("   pin seated at r = ", RPIN0, " on the web face at r = ", RW1,
         " (", RPIN0-RW1, " mm), outer end r = ", RPIN1,
         " under the retainer at r = ", RRET0, " (", RRET0-RPIN1, " mm)"));
echo(str("   pin boss band spans colat ", GRIM0, " to ", GRIM1,
         ", which is beta +/- 2 GPIN = ", BETA, " +/- ", 2*GPIN));

echo(str("--- predicted volumes; the mesh must come in under, because",
         " every chord cuts inside its arc ---"));
VS = sg_vol(NS_, NS_, D, true,  SL_F, M);
VP = sg_vol(NP_, NP_, D, true,  SL_F, M);
VRG= sg_vol(NR_, NR_, D, false, SL_F, M);
VHUB = sl_wedge(R0, RHUB1, GHI, GJ);
VWEB = sl_wedge(RW0, RW1, GJ, GRIM0);
VBOS = sl_wedge(RR0, RW1, GRIM0, GRIM1);
VPIN = sl_wedge(RPIN0, RPIN1, GBORE, GPIN);
VRET = sl_wedge(RRET0, RRET1, GRIM0, GRIM1);
echo(str("   sun ", VS, "   planet ", VP, " x ", K, " = ", K*VP,
         "   ring ", VRG));
echo(str("   carrier body ", VHUB, " + ", VWEB, " + ", VBOS, " = ",
         VHUB+VWEB+VBOS, "   pin ", VPIN, " x ", K, " = ", K*VPIN,
         "   retainer ", VRET));
echo(str("   predicted total = ", VS + K*VP + VRG + VHUB+VWEB+VBOS
                                 + K*VPIN + VRET, " mm^3"));
echo(str("   solids in the assembly: 1 sun + ", K, " planets + 1 ring + 1",
         " carrier body + ", K, " pins + 1 retainer = ", 3 + 2*K + 1));

// ===================================================================
//  THE SLICE
//  Apex at the origin, polar axis on +z.  `ring = false` leaves the ring
//  out, which is what the equator row wants: its ring IS the equatorial
//  band, a part of its own.
// ===================================================================
C_SUN  = [0.85, 0.70, 0.36];
C_PLAN = [0.47, 0.63, 0.73];
C_RING = [0.55, 0.72, 0.51];
C_CARR = [0.72, 0.53, 0.58];
C_PIN  = [0.34, 0.37, 0.43];   // pins read as steel, not as gear

module sl_slice(ri, ring = true) {
    color(C_SUN) sl_member(ri, sg_ns(), true, 0);
    if (ring) color(C_RING) sl_member(ri, sl_nr(ri), false, sl_clock(ri));
    for (j = [0:sl_k(ri)-1])
      rotate([0, 0, 360*j/sl_k(ri)]) rotate([0, sl_beta(ri), 0]) {
        color(C_PLAN) sl_member(ri, sl_np(ri), true, sl_phase(j,ri));
        color(C_PIN)  sl_pin(ri);
      }
    color(C_CARR) sl_carrier_body(ri);
    color(C_CARR) sl_retainer(ri);
}
sl_slice(SL_I);
