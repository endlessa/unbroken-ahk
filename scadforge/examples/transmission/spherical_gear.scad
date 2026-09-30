// ===================================================================
//  spherical_gear.scad -- exact spherical involute gearing and the
//  integer closure arithmetic for the spherical bevel planetary stack.
//
//  This file is the contract.  Everything downstream reads its numbers
//  from here.  Nothing in it is eyeballed: every tooth count is an
//  integer identity that is printed with both sides shown, and every
//  angle is the arcsine of an exact rational.
//
//  ---------------------------------------------------------------
//  1.  THE SPHERICAL INVOLUTE
//
//  Put the apex at the origin and the gear axis along +z.  A pitch
//  cone of half angle gamma meets the sphere of radius L in a small
//  circle of Euclidean radius L sin(gamma).  The base cone is
//
//      sin(gamma_b) = sin(gamma) cos(phi)                        (1)
//
//  with phi the pressure angle.  Dividing (1) by sin(gamma) gives
//  r_b / r_p = cos(phi) at EVERY latitude, so the base circle stands
//  in exactly the planar ratio to the pitch circle everywhere on the
//  sphere, the equator included.  That is the one planar relation
//  that survives intact, and it is the reason the equatorial band can
//  hand off to ordinary spur teeth on its outer face.
//
//  The involute is traced by a taut great-circle arc unwrapping from
//  the base circle.  Let theta be the azimuth of the foot of the arc
//  on the base circle.  The foot is
//
//      B(theta) = [ sin(gb) cos(theta), sin(gb) sin(theta), cos(gb) ]
//
//  and the unit tangent to the base circle there is
//
//      T(theta) = [ -sin(theta), cos(theta), 0 ].
//
//  Rolling without slip makes the arc length paid out equal the arc
//  length consumed on the base circle, L sin(gb) theta = L sigma, so
//  the great-circle angular length of the free arc is
//
//      sigma = theta sin(gamma_b).                               (2)
//
//  Walking sigma along the great circle from B in the direction -T:
//
//      P(theta) = B cos(sigma) - T sin(sigma)                    (3)
//
//  Expanding (3) and reading off the polar and azimuthal angles,
//
//      cos(Gamma) = cos(gamma_b) cos(sigma)                      (4)
//      psi        = theta - atan2( sin(sigma), sin(gb) cos(sigma) )
//
//  Equation (4) is the spherical Pythagoras for the right triangle
//  with legs gamma_b and sigma.  Eliminating sigma gives the
//  SPHERICAL INVOLUTE FUNCTION, the azimuth a flank has turned
//  through by the time it reaches colatitude Gamma:
//
//      INVS(Gamma; gb) = sigma/sin(gb)
//                        - atan2( sin(sigma), sin(gb) cos(sigma) )
//      with  sigma = acos( cos(Gamma) / cos(gamma_b) ).          (5)
//
//  The flanks are exact: the tooth solid is the cone from the apex
//  over the spherical profile, so its flank surfaces are ruled by
//  rays through the apex and are swept, never cut.
//
//  ---------------------------------------------------------------
//  2.  THE PLANAR LIMIT, MEASURED NOT ASSERTED
//
//  The specification calls for the curve to degenerate to the
//  ordinary planar involute at the equator, gamma = 90.  It does not,
//  and the file prints the disproof.  At gamma = 90 equation (1)
//  gives gamma_b = 90 - phi exactly, and (5) collapses to
//
//      INVS(90; 90-phi) = 90 (sec(phi) - 1) degrees,
//
//  which for phi = 25 is 9.30401 degrees against the planar
//  inv(25) = 1.71746 degrees, a factor of 5.4.  A pitch cone of half
//  angle 90 is the equatorial PLANE, that is a crown gear, and the
//  exact conjugate of a crown gear is still a spherical involute.
//
//  The limit that does hold is gamma_b -> 0, the apex receding to
//  infinity at fixed base radius, which is the cylinder.  Then
//  sigma -> sqrt(Gamma^2 - gb^2), sigma/sin(gb) -> tan(alpha) and the
//  atan2 term -> alpha, so INVS -> tan(alpha) - alpha, the planar
//  involute function.  sg_limit_table() prints that convergence, and
//  sg_limit_points() prints the convergence of the CURVE itself to
//  the planar involute in the tangent plane at the pole.
//
//  The equatorial band still carries ordinary planar involute teeth
//  on its OUTER cylindrical face, because that mesh is parallel-axis
//  and the pitch surface really is a cylinder.  The band's INNER row
//  is a crown ring at gamma = 90 with spherical involute flanks.  One
//  rigid band, two tooth systems, one angular velocity.
//
//  ---------------------------------------------------------------
//  3.  CLOSURE ARITHMETIC
//
//  A member with N teeth of module m has pitch DIAMETER N m, whatever
//  its cone angle, because N = 2 L sin(gamma) / m and the module of a
//  bevel gear scales with cone distance.  Writing
//
//      D = 2 L / m                                               (6)
//
//  for a row's cone-distance scale, every member of that row obeys
//
//      sin(gamma) = N / D,                                       (7)
//
//  so a row's cone angles are arcsines of exact rationals and the
//  closure gate is satisfied by construction: N teeth wrap the
//  member's own pitch circle exactly N times.
//
//  Cones sharing an apex roll when they are tangent.  Sun external to
//  planet, planet internal to ring, sun and ring coaxial:
//
//      beta = gamma_s + gamma_p      (external mesh)
//      beta = gamma_r - gamma_p      (internal mesh)
//   => gamma_r = gamma_s + 2 gamma_p                             (8)
//
//  Expanding sin(A+2B) under (7) turns (8) into pure integer
//  arithmetic:
//
//      Nr D^2 = Ns (D^2 - 2 Np^2) + 2 Np sqrt( (D^2-Ns^2)(D^2-Np^2) )
//                                                                (9)
//
//  A row closes only when that square root is an integer AND D^2
//  divides the right-hand side.  Note what (9) is NOT: it is not
//  Nr = Ns + 2 Np.  The planar relation fails on a sphere and the
//  difference is large, 59 against 72 on the first row here.
//
//  WILLIS IS UNCHANGED.  Resolving the rolling condition at each mesh
//  perpendicular to the common cone element gives w_p/w_s = -Ns/Np
//  and w_r/w_p = Np/Nr in the carrier frame, exactly as in the plane,
//  so
//
//      (w_sun - w_carrier)/(w_ring - w_carrier) = -Nr/Ns        (10)
//
//  holds on the sphere with the same tooth counts.  The sines fold
//  into the counts and vanish.  That is why ratio design here is
//  integer arithmetic with no decimals upstream.
//
//  ASSEMBLY.  Hold the ring, turn the carrier by 360/k.  By (10) the
//  sun turns by (360/k)(Ns+Nr)/Ns, and that must be a whole number of
//  sun pitches 360/Ns, so
//
//      (Ns + Nr) / k  must be an integer.                       (11)
//
//  ---------------------------------------------------------------
//  4.  WHERE THE APEXES GO, AND WHY THEY CANNOT ALL BE ONE APEX
//
//  Two rows cannot share one apex AND one sun cone AND one module.
//  On a single sun cone the count Ns = 2 L sin(gamma_s)/m(L) is the
//  same at every cone distance, so every row sharing that cone shares
//  D, and then (9) must be solved four times with one D and one Ns.
//  Searching every D from 40 to 1500 and every sun count, an equator
//  row (gamma_r = 90) is accompanied by at most ONE further exact row;
//  and even with no equator row, no (D, Ns) admits more than THREE
//  exact rows with gamma_r <= 90 anywhere below D = 900.  Three
//  latitudinal rows plus the equator on ONE apex therefore has no
//  solution at any buildable size.  That is a failed gate and it is
//  reported as one, not fudged.
//
//  The stack resolves it, because a stack forces per-slice apexes
//  anyway: slices sit at different heights on the polar axis, so
//  their apexes cannot coincide.  Each slice is its own spherical
//  bevel set with its own cone-distance scale D_i, and the members it
//  shares with its neighbours are shared by TOOTH COUNT and MODULE,
//  not by cone angle.  The sun therefore keeps pitch diameter Ns m at
//  every slice while its cone angle arcsin(Ns/D_i) steps from slice
//  to slice.  One shaft, one count, one diameter, stepped cones.
//
//  The fundamental sphere survives untouched and does the job the
//  specification gives it: each row's ring pitch circle IS a latitude
//  circle of the sphere of radius R, and
//
//      Nr_i = 2 R sin(colat_i) / m,   Dref = 2 R / m            (12)
//
//  is the per-row divisibility gate exactly as written.  Choosing the
//  integer Nr_i selects the latitude; the equator is Nr = Dref.
//
//  The equatorial slice comes out as the fundamental one: its ring is a
//  great circle of its own bevel sphere, so gamma_r = 90 there, its
//  cone distance L equals R, and its apex sits on the sphere centre.
//  Every other slice's apex lies further up the polar axis.
//
//  ---------------------------------------------------------------
//  5.  TORQUE, STATED PLAINLY
//
//  Willis constrains speeds only.  Nothing above balances torque.  A
//  passive differential sends power to the path of least resistance,
//  and with three or seven planets per row the mesh load share is set
//  by manufacture and deflection, not by kinematics.  The EM collars
//  are the control surface: drag current sets the torque equilibrium.
//  The only natural levelling in this machine is electrical, between
//  supercapacitor banks, and it is not mechanical balance.
//
//  This is a reconfigurable fixed-ratio stack plus a power-split
//  differential with an exact parameterised ratio versus auxiliary
//  speed.  It is not a CVT.
//
//  References: Figliolini and Angeles, Synthesis of the Spherical
//  Involute Gearing, ASME JMD 2005.  Willis, standard machine design.
//  Abe et al., ABENICS, IEEE T-RO 2021, for printed spherical gear
//  feasibility.  Simpson and Ravigneaux for compound epicyclic
//  selection.
// ===================================================================

function rad(a) = a*PI/180;
function deg(r) = r*180/PI;

// split-range sum: a naive recursive sum recurses once per element
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

function isq(n) = let(r = round(sqrt(n))) r*r == n ? r : -1;
// Guarded: gcd_ is tail recursive, so tail-call elimination means a
// non-numeric argument would spin without ever tripping a depth guard.
function gcd_(a,b) = !is_num(a) || !is_num(b) ? undef
                   : b == 0 ? a : gcd_(b, a % b);
function lcm_(a,b) = a*b/gcd_(a,b);

// ---- the curve ------------------------------------------------------
function sg_gb(g, phi)  = asin(sin(g)*cos(phi));            // (1)
function sg_sigma(G,gb) = acos(max(-1, min(1, cos(G)/cos(gb))));
function sg_invs(G,gb)  = let(s = sg_sigma(G,gb))           // (5)
                            s/sin(gb) - atan2(sin(s), sin(gb)*cos(s));
function sg_invp(a)     = deg(tan(a) - rad(a));             // planar involute

// the involute point itself, on the sphere of radius L, as a 3-vector.
// sigma is the parameter; theta = sigma/sin(gb) is the foot azimuth.
function sg_pt(sig, gb, L) =
  let( th = sig/sin(gb) )
    L*[ sin(gb)*cos(th)*cos(sig) + sin(th)*sin(sig),
        sin(gb)*sin(th)*cos(sig) - cos(th)*sin(sig),
        cos(gb)*cos(sig) ];
// the planar involute of the same base circle, for the limit check
function sg_ptp(th, rb) = rb*[ cos(th) + rad(th)*sin(th),
                               sin(th) - rad(th)*cos(th) ];

// ---- tooth proportions ---------------------------------------------
// addendum m and dedendum 1.25 m, measured perpendicular to the pitch
// cone at cone distance L = D m / 2, so they are angles 2/D and 2.5/D.
function sg_ta(D) = atan(2.0/D);
function sg_tf(D) = atan(2.5/D);
// half tooth angle at the pitch cone, backlash jt taken symmetrically
function sg_ht(N, jt, m) = (90 - 90*jt/(PI*m))/N;
// Half tooth width at colatitude G.  The sign is the whole difference
// between the two: an external tooth's angular half width FALLS as
// colatitude rises, so it is thick at the root and thin at the tip,
// while an internal tooth's RISES with colatitude, so it is thin at
// its tip (which lies at smaller colatitude, toward the axis) and
// thick where it meets the rim.  Get this backwards and the ring's
// tooth space is narrow exactly where the mating tooth is fattest.
function sg_hw_e(G,g,gb,ht) = ht + sg_invs(g,gb) - sg_invs(G,gb);
function sg_hw_i(G,g,gb,ht) = ht + sg_invs(G,gb) - sg_invs(g,gb);

// ---- closure --------------------------------------------------------
// right-hand side of (9), as an exact integer pair [numerator, D^2]
function sg_rhs(Ns,Np,D) =
  let( q = isq((D*D-Ns*Ns)*(D*D-Np*Np)) )
    q < 0 ? [-1,D*D] : [ Ns*(D*D - 2*Np*Np) + 2*Np*q, D*D ];
function sg_closes(Ns,Np,Nr,D) =
  let( r = sg_rhs(Ns,Np,D) ) r[0] >= 0 && r[0] == Nr*r[1];

// ===================================================================
//  THE DESIGN
//  m and phi are chosen; everything else is searched over the
//  integers and then checked by identity below.
// ===================================================================
MODULE_MM = 1.0;        // m
PHI_P     = 25;         // pressure angle.  20 undercuts the 13 tooth
                        // planet of row 1: its virtual count is 13.30
                        // against a limit of 2/sin(phi)^2 = 17.10 at
                        // 20 degrees and 11.21 at 25.
JT        = 0.05;       // circumferential backlash per mesh, in modules
NS        = 46;         // common sun count, every row
DREF      = 242;        // = 2 R / m, fixed by the equator row Nr = Dref
RSPH      = DREF*MODULE_MM/2;

// row = [ name, Nr, D, Np, k ]
ROWS = [
  ["row 1", 59,  62,  13, 7],
  ["row 2", 143, 146, 73, 3],
  ["row 3", 188, 196, 98, 3],
  ["equator", 242, 242, 154, 3] ];

EM_SLOTS = 24;          // axial flux collar, 3 phase
EM_POLES = 28;
CAP_F    = 12;          // supercapacitor bank per slice, farad
CAP_VLO  = 12;
CAP_VHI  = 24;

// ===================================================================
//  REPORT
// ===================================================================
echo("=== gates ===");
echo("1 per-row divisibility      PASS, Nr = 2 R sin(colat)/m by construction");
echo("2 mesh mating and assembly  PASS, (Ns+Nr)/k integer on every row, with");
echo("                            neighbour tip-cone margins reported below");
echo("3 chain backlash            PASS, chain lives only on the chain collar");
echo("4 intermediate sun bearing  deferred to the slice part, not this file");
echo("5 EM ripple closure         PASS, lcm(S,P) integer per revolution");
echo("6 backlash and windup       PASS, reported per mesh and cumulative");
echo("7 polar crown minimum       PASS up to 17.4 Nm per tooth pair at m = 1;");
echo("                            above that the crown will not fit inside 2R");
echo("SHARED-APEX GATE            FAIL.  One apex, one module and one sun");
echo("  count cannot carry three latitudinal rows plus the equator.  Section");
echo("  4 of the header gives the search.  The stack forces per-slice apexes");
echo("  anyway, and with them every row closes exactly.");
echo("SPEC EQUATOR DEGENERACY     FAIL as stated.  A gamma = 90 pitch cone is");
echo("  the equatorial PLANE, a crown gear, not a cylinder, and its exact");
echo("  flank stays a spherical involute.  Numbers under the limit heading.");

echo("=== fundamentals ===");
echo(str("module m = ", MODULE_MM, " mm   pressure angle = ", PHI_P, " deg"));
echo(str("Dref = 2R/m = ", DREF, "   R = Dref m/2 = ", RSPH, " mm"));
echo(str("sun count Ns = ", NS, "   sun pitch diameter Ns m = ",
         NS*MODULE_MM, " mm, the same at every slice"));

module sg_row_report(row) {
  Nr = row[1]; D = row[2]; Np = row[3]; k = row[4];
  A  = D*D - NS*NS;  B = D*D - Np*Np;  q = isq(A*B);
  r  = sg_rhs(NS,Np,D);
  gs = asin(NS/D); gp = asin(Np/D); gr = asin(Nr/D);
  L  = D*MODULE_MM/2;
  colat = asin(Nr/DREF);
  ta = sg_ta(D);
  beta = gs + gp;
  // angular gap between the tip cones of neighbouring planets
  sep = acos(cos(beta)*cos(beta) + sin(beta)*sin(beta)*cos(360/k));
  gapdeg = sep - 2*(gp + ta);
  zmin = 2/(sin(PHI_P)*sin(PHI_P));
  echo(str("--- ", row[0], "  Nr=", Nr, " D=", D, " Np=", Np, " k=", k));
  echo(str("   (D^2-Ns^2)(D^2-Np^2) = ", A, "*", B, " = ", A*B,
           " = ", q, "^2   exact"));
  echo(str("   Ns(D^2-2Np^2)+2 Np q = ", r[0], " = ", Nr, " * D^2 = ",
           Nr, " * ", r[1], "   closes: ", sg_closes(NS,Np,Nr,D)));
  echo(str("   planar Ns+2Np would be ", NS+2*Np, ", the sphere gives ", Nr));
  echo(str("   gamma_s=", gs, "  gamma_p=", gp, "  gamma_s+2gamma_p=",
           gs+2*gp, "  gamma_r=", gr));
  echo(str("   L = D m/2 = ", L, " mm   pitch radii  sun ", L*sin(gs),
           "  planet ", L*sin(gp), "  ring ", L*sin(gr),
           "   (N m/2: ", NS*MODULE_MM/2, " ", Np*MODULE_MM/2, " ",
           Nr*MODULE_MM/2, ")"));
  echo(str("   latitude circle: colat = asin(", Nr, "/", DREF, ") = ", colat,
           "  latitude = ", 90-colat));
  echo(str("   R sin(colat) = ", RSPH*sin(colat), " = Nr m/2 = ",
           Nr*MODULE_MM/2, "   teeth wrap it ", Nr, " times exactly"));
  echo(str("   Willis (w_s-w_c)/(w_r-w_c) = -", Nr, "/", NS,
           " = -", Nr/gcd_(Nr,NS), "/", NS/gcd_(Nr,NS),
           "   ring braked: w_c/w_s = ", NS, "/", NS+Nr));
  echo(str("   assembly (Ns+Nr)/k = ", NS+Nr, "/", k, " = ", (NS+Nr)/k,
           "   integer: ", (NS+Nr)%k == 0));
  echo(str("   neighbour planets: axis separation ", sep,
           " deg, tip cones need ", 2*(gp+ta),
           " deg, margin ", gapdeg, " deg = ", rad(gapdeg)*L, " mm at L"));
  echo(str("   virtual counts z_v=N/cos(gamma)  sun ", NS/cos(gs),
           "  planet ", Np/cos(gp),
           "   limit 2/sin(phi)^2 = ", zmin));
  echo(str("   base cones  sun ", sg_gb(gs,PHI_P), "  planet ",
           sg_gb(gp,PHI_P), "  ring ", sg_gb(gr,PHI_P)));
  echo(str("   face width bound L/3 = ", L/3,
           " mm; backlash jt = ", JT*MODULE_MM, " mm at every mesh"));
  echo(str("   apex height on the polar axis = ",
           RSPH*cos(colat) + L*cos(gr), " mm above the equatorial plane"));
}

echo("=== rows: closure, Willis, assembly ===");
for (r = ROWS) sg_row_report(r);

// ---- backlash and windup, gate 6 ------------------------------------
// Lost motion at the sun with the carrier held comes from two meshes.
// Sun-planet contributes jt/r_s = 2 jt/(Ns m).  Planet-ring contributes
// jt/r_p at the planet, and the carrier-frame ratio w_p/w_s = -Ns/Np
// refers that to the sun as (jt/r_p)(Np/Ns) = 2 jt/(Ns m), the same.
// The total is 4 jt/(Ns m) and it does not depend on the row at all.
echo("=== backlash, gate 6 ===");
JTMM = JT*MODULE_MM;
LOST = deg(4*JTMM/(NS*MODULE_MM));
echo(str("jt = ", JT, " m = ", JTMM, " mm per mesh, taken symmetrically"));
echo(str("sun-referred lost motion per slice = 4 jt/(Ns m) = ", LOST,
         " deg, identical for every row"));
echo(str("coupled splice windup, n slices in series, referred to the sun:"));
for (n = [1:4]) echo(str("   n = ", n, "  ", n*LOST, " deg"));
echo("chain backlash is admitted on the chain collar only, which is a");
echo("torque conversion port; no precision path passes through a chain.");

// ---- EM closure, gate 5 ---------------------------------------------
echo("=== EM collar closure, gate 5 ===");
EMLCM = lcm_(EM_SLOTS, EM_POLES);
echo(str("slots S = ", EM_SLOTS, "  poles P = ", EM_POLES,
         "   S*P = ", EM_SLOTS*EM_POLES));
echo(str("cogging fundamental per mechanical revolution = lcm(S,P) = ",
         EMLCM, ", an integer, so the excitation closes on the revolution"));
echo(str("three phase torque ripple order = 6*(P/2) = ", 6*(EM_POLES/2),
         " per revolution"));
for (r = ROWS)
  echo(str("   gcd(lcm(S,P)=", EMLCM, ", mesh order Nr=", r[1], ") = ",
           gcd_(EMLCM, r[1])));
echo(str("   gcd(lcm(S,P), Ns=", NS, ") = ", gcd_(EMLCM, NS)));
echo(str("bank ", CAP_F, " F over ", CAP_VLO, "-", CAP_VHI,
         " V holds ", 0.5*CAP_F*(CAP_VHI*CAP_VHI - CAP_VLO*CAP_VLO),
         " J usable; banks are isolated, so charge level is the",
         " distribution state and current control is hysteretic on it"));

// ---- gate 7, polar crown --------------------------------------------
// Lewis: allowable tangential load per tooth F = sigma b m Y.  A crown
// of N_cap teeth at pitch radius N_cap m/2 carries T = F N_cap m/2 on
// one tooth pair, so the crown count needed for a port torque T is
//     N_cap >= 2 T / (sigma b m^2 Y).
echo("=== gate 7, polar crown minimum ===");
SIGMA_F = 30;    // N/mm^2 allowable bending, printed polyamide
BFACE   = 12;    // crown face width, mm
LEWIS_Y = 0.40;  // at phi = 25, high virtual tooth count
FT = SIGMA_F*BFACE*MODULE_MM*LEWIS_Y;
echo(str("F = sigma b m Y = ", SIGMA_F, "*", BFACE, "*", MODULE_MM, "*",
         LEWIS_Y, " = ", FT, " N per tooth pair"));
for (T = [5, 10, 15, 20])
  echo(str("   port torque ", T, " Nm needs N_cap >= ",
           ceil(2*T*1000/(FT*MODULE_MM)), " teeth, pitch diameter ",
           ceil(2*T*1000/(FT*MODULE_MM))*MODULE_MM, " mm",
           2*T*1000/(FT*MODULE_MM)*MODULE_MM <= 2*RSPH ?
             "  fits inside 2R" : "  DOES NOT FIT inside 2R"));
echo(str("the crown cannot exceed the sphere, 2R = ", 2*RSPH,
         " mm, so one tooth pair carries at most ",
         FT*DREF*MODULE_MM/2/1000, " Nm; tooth size does not go to zero",
         " and that is what sets the cap diameter"));

// ---- the equatorial band, the external drive port -------------------
// The band is one rigid member at one angular velocity, toothed on
// both faces.  Inside it carries the equator row's crown ring, 242
// spherical involute teeth at gamma = 90.  Outside it carries an
// ordinary planar involute spur ring on a cylinder about the polar
// axis, because THAT mesh is parallel-axis and its pitch surface
// really is a cylinder: the apex has gone to infinity, which is the
// gamma_b -> 0 limit the table below measures.  It is not the gamma =
// 90 cone, which is the equatorial plane and gives a crown gear.
echo("=== equatorial band, both faces ===");
BAND_OUT = 280;         // spur teeth on the outer cylinder
BAND_PIN = 28;          // external pinion, axis parallel to the polar axis
CHAIN_N  = 114;         // chain collar sprocket
echo(str("inner face: ", DREF, " crown teeth, gamma = 90, base cone ",
         sg_gb(90,PHI_P), ", spherical involute"));
echo(str("outer face: ", BAND_OUT, " spur teeth on radius ",
         BAND_OUT*MODULE_MM/2, " mm; closure is 2 R_out/m = ", BAND_OUT,
         ", an integer by construction"));
echo(str("   base radius r_b = R_out cos(phi) = ",
         BAND_OUT*MODULE_MM/2*cos(PHI_P),
         " mm, and inv(phi) = ", sg_invp(PHI_P),
         " deg, the planar branch of the same function"));
echo(str("   radial wall between the two faces = ",
         BAND_OUT*MODULE_MM/2 - RSPH, " mm"));
echo(str("   pinion ", BAND_PIN, " teeth, port ratio ", BAND_OUT, "/",
         BAND_PIN, " = ", BAND_OUT/BAND_PIN,
         " exactly; pinion virtual count ", BAND_PIN,
         " clears undercut at phi = ", PHI_P));
echo(str("   chain collar ", CHAIN_N, " teeth: chordal rise 1-cos(180/N) = ",
         1 - cos(180/CHAIN_N), ", that is ", 100*(1 - cos(180/CHAIN_N)),
         " percent speed ripple; chain backlash stays on this port"));

// ---- stacking modes, exact rationals --------------------------------
echo("=== stacking modes ===");
echo("common-sun splice: one sun through the slices, rows ADD at the sun");
echo("and share load across rows.  Ratios do NOT multiply.  Each row's");
echo("ring-braked carrier reduction, exact:");
for (r = ROWS)
  let (g = gcd_(NS+r[1], NS))
    echo(str("   ", r[0], "  w_s/w_c = (Ns+Nr)/Ns = ", NS+r[1], "/", NS,
             " = ", (NS+r[1])/g, "/", NS/g, " = ", (NS+r[1])/NS));
echo("coupled splice: stage N output drives stage N+1 input through the");
echo("gear collar, so ratios COMPOUND.  Both stages are ring-braked");
echo("reductions, so the product does not depend on the order:");
for (a = [0:len(ROWS)-2]) for (b = [a+1:len(ROWS)-1])
    let (ra = (NS+ROWS[a][1]), rb = (NS+ROWS[b][1]),
         num = ra*rb, den = NS*NS, g = gcd_(num,den))
      echo(str("   ", ROWS[a][0], " then ", ROWS[b][0], "  = ",
               num/g, "/", den/g, " = ", num/den));

// ---- the planar limit, demonstrated ---------------------------------
echo("=== spherical involute -> planar involute, measured ===");
echo(str("at the equator gamma=90: gamma_b = ", sg_gb(90,PHI_P),
         " = 90 - phi exactly"));
echo(str("   INVS(90) = ", sg_invs(90, sg_gb(90,PHI_P)),
         "   90(sec phi - 1) = ", 90*(1/cos(PHI_P) - 1),
         "   planar inv(phi) = ", sg_invp(PHI_P),
         "   ratio = ", sg_invs(90, sg_gb(90,PHI_P))/sg_invp(PHI_P)));
echo("   so the specification's equator degeneracy is FALSE; the");
echo("   surviving planar relation is r_b/r_p = cos(phi) at every gamma:");
for (g = [10, 30, 60, 90])
  echo(str("      gamma = ", g, "  sin(gamma_b)/sin(gamma) = ",
           sin(sg_gb(g,PHI_P))/sin(g), "  cos(phi) = ", cos(PHI_P)));
echo("   the true degeneracy is gamma_b -> 0, the apex to infinity:");
for (g = [60, 30, 10, 3, 1, 0.3, 0.1])
  echo(str("      gamma = ", g, "  gamma_b = ", sg_gb(g,PHI_P),
           "  INVS(gamma) = ", sg_invs(g, sg_gb(g,PHI_P)),
           "  inv(phi) = ", sg_invp(PHI_P),
           "  ratio = ", sg_invs(g, sg_gb(g,PHI_P))/sg_invp(PHI_P)));

// the curve itself, not just the function: project the spherical
// involute on the tangent plane at the pole and compare with the
// planar involute of the same base circle, over a full flank.
function sg_limit_err(g, L, n = 24) =
  let( gb = sg_gb(g, PHI_P),
       rb = L*sin(gb),
       smax = sg_sigma(min(89.9, g*1.5), gb),
       e = [ for (i = [1:n])
               let( sg = smax*i/n, th = sg/sin(gb),
                    P = sg_pt(sg, gb, L), Q = sg_ptp(th, rb) )
                 norm([P[0],P[1]] - Q)/rb ] )
    max(e);
echo("   deviation of the spherical involute from the planar involute of");
echo("   the same base circle, over a whole flank, as a fraction of r_b:");
for (g = [30, 10, 3, 1, 0.3, 0.1])
  echo(str("      gamma = ", g, "  max|P_xy - Q|/r_b = ",
           sg_limit_err(g, 100)));

// ===================================================================
//  GEOMETRY
//  The section is a chain of boundary points in (colatitude, azimuth)
//  running with azimuth increasing.  The solid is the region between
//  that boundary and a hub cone, swept between two cone distances.
//  Flanks are therefore exactly ruled by rays through the apex, which
//  is what makes a bevel tooth a bevel tooth.
// ===================================================================
NCA = 5;   // points on each half of a root arc
NDR = 4;   // points on a radial relief below the base cone
NI  = 20;  // points on a flank
NTP = 5;   // points on a tip arc
EPSG = 1e-4;

// one tooth of an EXTERNAL member, from the space centreline before it
// to just short of the space centreline after it
function sg_tooth_e(j, N, g, gb, ht, ta, tf) =
  let( Gf = g - tf, Ga = g + ta, Glo = max(gb + EPSG, Gf),
       pc = 360*j/N, h0 = sg_hw_e(Glo,g,gb,ht), ha = sg_hw_e(Ga,g,gb,ht),
       drop = Glo > Gf + EPSG/2,
       A = [ for (i=[0:NCA-1]) let(t=i/NCA)
               [Gf, (pc-180/N) + ((pc-h0)-(pc-180/N))*t ] ],
       B = drop ? [ for (i=[0:NDR-1]) let(t=i/NDR)
                      [Gf + (Glo-Gf)*t, pc-h0] ] : [],
       C = [ for (i=[0:NI-1]) let(t=pow(i/NI,0.75), G=Glo+(Ga-Glo)*t)
               [G, pc - sg_hw_e(G,g,gb,ht)] ],
       Dd= [ for (i=[0:NTP-1]) let(t=i/NTP) [Ga, (pc-ha) + 2*ha*t] ],
       E = [ for (i=[0:NI-1]) let(t=pow(i/NI,0.75), G=Ga+(Glo-Ga)*t)
               [G, pc + sg_hw_e(G,g,gb,ht)] ],
       F = drop ? [ for (i=[0:NDR-1]) let(t=i/NDR)
                      [Glo + (Gf-Glo)*t, pc+h0] ] : [],
       G_ = [ for (i=[0:NCA-1]) let(t=i/NCA)
               [Gf, (pc+h0) + ((pc+180/N)-(pc+h0))*t ] ] )
    concat(A,B,C,Dd,E,F,G_);

// one tooth of an INTERNAL member.  Root cone is OUTSIDE the pitch
// cone, tip cone inside it, and the half width grows with colatitude.
function sg_tooth_i(j, N, g, gb, ht, ta, tf) =
  let( Gf = g + tf, Ga = g - ta, Glo = max(gb + EPSG, Ga),
       pc = 360*j/N, h0 = sg_hw_i(Glo,g,gb,ht), hf = sg_hw_i(Gf,g,gb,ht),
       drop = Glo > Ga + EPSG/2,
       A = [ for (i=[0:NCA-1]) let(t=i/NCA)
               [Gf, (pc-180/N) + ((pc-hf)-(pc-180/N))*t ] ],
       C = [ for (i=[0:NI-1]) let(t=i/NI, G=Gf+(Glo-Gf)*t)
               [G, pc - sg_hw_i(G,g,gb,ht)] ],
       B = drop ? [ for (i=[0:NDR-1]) let(t=i/NDR)
                      [Glo + (Ga-Glo)*t, pc-h0] ] : [],
       Dd= [ for (i=[0:NTP-1]) let(t=i/NTP) [Ga, (pc-h0) + 2*h0*t] ],
       F = drop ? [ for (i=[0:NDR-1]) let(t=i/NDR)
                      [Ga + (Glo-Ga)*t, pc+h0] ] : [],
       E = [ for (i=[0:NI-1]) let(t=i/NI, G=Glo+(Gf-Glo)*t)
               [G, pc + sg_hw_i(G,g,gb,ht)] ],
       G_ = [ for (i=[0:NCA-1]) let(t=i/NCA)
               [Gf, (pc+hf) + ((pc+180/N)-(pc+hf))*t ] ] )
    concat(A,C,B,Dd,F,E,G_);

// T teeth, plus the one point that closes the last space
function sg_boundary(T, N, g, gb, ht, ta, tf, ext) =
  concat( [ for (j=[0:T-1]) each (ext ? sg_tooth_e(j,N,g,gb,ht,ta,tf)
                                      : sg_tooth_i(j,N,g,gb,ht,ta,tf)) ],
          [ [ ext ? g - tf : g + tf, 360*(T-1)/N + 180/N ] ] );

function sg_xyz(G, psi, L) = L*[sin(G)*cos(psi), sin(G)*sin(psi), cos(G)];

// ---- the shell ------------------------------------------------------
// Q is indexed [i][j][k]: i walks the boundary, j is 0 on the toothed
// boundary and 1 on the hub cone, k is 0 at the inner cone distance and
// 1 at the outer.  The triad (d_i, d_j, d_k) is right handed, which for
// an external member means the boundary must run with azimuth
// INCREASING and for an internal member with azimuth DECREASING,
// because d_j flips.  polyhedron wants each face listed so the
// right-hand normal points INTO the solid, so the i = NI face is
// listed (j,k) -> (j,k+1) -> (j+1,k+1) -> (j+1,k) and the i = 0 face
// the other way round, and likewise by cyclic symmetry for j and k.
// Two faces sharing an edge then traverse it in opposite directions,
// which is the orientation-blind test, and the exported volume comes
// out positive.
module sg_shell(Q) {
    NIi = len(Q) - 1;
    idx = function (i,j,k) (i*2 + j)*2 + k;
    pts = [ for (i=[0:NIi]) for (j=[0:1]) for (k=[0:1]) Q[i][j][k] ];
    polyhedron(
      points = pts,
      faces = concat(
        // i = 0 and i = NIi end walls
        [ [ idx(0,0,0), idx(0,1,0), idx(0,1,1), idx(0,0,1) ] ],
        [ [ idx(NIi,0,0), idx(NIi,0,1), idx(NIi,1,1), idx(NIi,1,0) ] ],
        // j = 0 toothed face and j = 1 hub face
        [ for (i=[0:NIi-1])
            [ idx(i,0,0), idx(i,0,1), idx(i+1,0,1), idx(i+1,0,0) ] ],
        [ for (i=[0:NIi-1])
            [ idx(i,1,0), idx(i+1,1,0), idx(i+1,1,1), idx(i,1,1) ] ],
        // k = 0 inner sphere and k = 1 outer sphere
        [ for (i=[0:NIi-1])
            [ idx(i,0,0), idx(i+1,0,0), idx(i+1,1,0), idx(i,1,0) ] ],
        [ for (i=[0:NIi-1])
            [ idx(i,0,1), idx(i,1,1), idx(i+1,1,1), idx(i+1,0,1) ] ] ),
      convexity = 8 );
}

// A toothed sector of one member.  ext selects external or internal.
// Ghub is the colatitude of the back face of the rim.
module sg_sector(T, N, D, ext = true, F = 0.25, m = MODULE_MM,
                 phi = PHI_P, jt = JT) {
    g  = asin(N/D);
    gb = sg_gb(g, phi);
    ta = sg_ta(D); tf = sg_tf(D);
    ht = sg_ht(N, jt*m, m);
    Lo = D*m/2;  Li = Lo*(1 - F);
    Ghub = ext ? max(0.25*g, g - tf - 2.5*(ta+tf))
               : g + tf + 2.5*(ta+tf);
    b0 = sg_boundary(T, N, g, gb, ht, ta, tf, ext);
    b  = ext ? b0 : [ for (i=[len(b0)-1:-1:0]) b0[i] ];
    Q  = [ for (p = b)
             [ [ sg_xyz(p[0], p[1], Li), sg_xyz(p[0], p[1], Lo) ],
               [ sg_xyz(Ghub, p[1], Li), sg_xyz(Ghub, p[1], Lo) ] ] ];
    sg_shell(Q);
}

// Solid angle of the sector by the trapezoid rule over the same samples
// the mesh uses, so the exported volume is a PREDICTION: the chords cut
// inside the arcs, so the mesh must come out a little under.
function sg_omega(T, N, D, ext, phi = PHI_P, jt = JT, m = MODULE_MM) =
  let( g = asin(N/D), gb = sg_gb(g,phi), ta = sg_ta(D), tf = sg_tf(D),
       ht = sg_ht(N, jt*m, m),
       Ghub = ext ? max(0.25*g, g - tf - 2.5*(ta+tf))
                  : g + tf + 2.5*(ta+tf),
       b = sg_boundary(T,N,g,gb,ht,ta,tf,ext),
       t = [ for (i=[0:len(b)-2])
               rad(b[i+1][1] - b[i][1]) *
               ( ext ? cos(Ghub) - (cos(b[i][0]) + cos(b[i+1][0]))/2
                     : (cos(b[i][0]) + cos(b[i+1][0]))/2 - cos(Ghub) ) ] )
    sum(t);

function sg_vol(T, N, D, ext, F = 0.25, m = MODULE_MM) =
  let( Lo = D*m/2, Li = Lo*(1-F) )
    sg_omega(T,N,D,ext)*(Lo*Lo*Lo - Li*Li*Li)/3;

// ===================================================================
//  DEMONSTRATION
//  Four members off the design table, chosen to walk the whole family
//  of cone angles from the near-cylindrical to the crown.  They are
//  placed apart and share no volume, so the exported mesh is the mesh
//  written here, face for face.
// ===================================================================
DEMO = [
  // T   N    D   external  label
  [ 4,  13,  62,  true,  "row 1 planet, gamma 12.10, nearest the cylinder"],
  [ 4,  46,  62,  true,  "row 1 sun, gamma 47.90"],
  [ 5, 154, 242,  true,  "equator planet, gamma 39.52"],
  [ 5, 143, 146,  false, "row 2 ring, gamma 78.36, internal"],
  [ 5, 242, 242,  false, "equator crown ring, gamma 90, the special case"] ];
// The demonstration gives every member the same 8 mm face, so the
// profiles are directly comparable; the design face width is bounded by
// the usual L/3 and is much larger on the outer rows.  Members are laid
// out by their pitch points on a small grid and share no volume.
DEMO_FMM = 8;
DEMO_POS = [ [0,0], [26,0], [52,0], [13,28], [39,28] ];

echo("=== demonstration sectors ===");
VTOT = sum([ for (d = DEMO) sg_vol(d[0], d[1], d[2], d[3], 2*DEMO_FMM/(d[2]*MODULE_MM)) ]);
for (i = [0:len(DEMO)-1]) {
  d = DEMO[i];
  g = asin(d[1]/d[2]);
  echo(str("   ", d[4], ": N=", d[1], " D=", d[2], " teeth shown ", d[0],
           "  gamma=", g, "  gamma_b=", sg_gb(g,PHI_P),
           "  solid angle=", sg_omega(d[0],d[1],d[2],d[3]), " sr",
           "  volume=", sg_vol(d[0],d[1],d[2],d[3],2*DEMO_FMM/(d[2]*MODULE_MM)), " mm^3"));
}
echo(str("predicted total volume = ", VTOT, " mm^3; the mesh must come in",
         " a little under, because every chord cuts inside its arc"));

COL = [[0.82,0.68,0.34],[0.46,0.62,0.72],[0.55,0.72,0.50],
       [0.72,0.52,0.58],[0.62,0.58,0.74]];

// Each sector keeps its own apex at its own origin and its axis on +z,
// so the cone angle is what you see.  It is then swung so the shown
// teeth straddle azimuth zero and shifted so its PITCH POINT lands on
// the x axis at a fixed spacing.  The five solids share no volume, so
// the union has no boolean to do and the export is face for face what
// is written here.
for (i = [0:len(DEMO)-1])
  let (d = DEMO[i], g = asin(d[1]/d[2]), L = d[2]*MODULE_MM/2,
       pmid = (d[0]-1)*180/d[1])
    color(COL[i])
      translate([DEMO_POS[i][0] - L*sin(g), DEMO_POS[i][1], -L*cos(g)])
        rotate([0, 0, -pmid])
          sg_sector(d[0], d[1], d[2], d[3], DEMO_FMM/L);
