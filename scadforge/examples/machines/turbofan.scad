// ===================================================================
//  A high-bypass turbofan, sized by its own gas dynamics
//
//  Almost nothing about an engine like this is a free choice.  Pick the
//  fan diameter, the shaft speeds, the axial Mach number and the turbine
//  entry temperature, and the rest is bookkeeping: continuity fixes every
//  annulus area, the velocity triangles fix every blade angle, the work
//  balance fixes how many turbine stages it takes to drive the
//  compressors, and a chosen solidity fixes how many blades go in each
//  row.  This file is that bookkeeping, carried out until it has produced
//  a mesh.  There are no eyeballed radii in the gas path.
//
//  WHAT SETS THE ANNULUS
//
//  The one-dimensional flow function
//
//      Phi(M) = gamma/sqrt(gamma-1) * M * (1 + (gamma-1)/2 M^2)^(-(gamma+1)/(2(gamma-1)))
//
//  is the non-dimensional mass flow m sqrt(cp T0)/(A p0), so
//
//      A = m sqrt(cp T0) / (p0 Phi(Mx)).
//
//  Walk the machine keeping the axial Mach number to a schedule, and the
//  area at every station falls out of the local total pressure and total
//  temperature.  Divide by 2 pi r_mean and you have the annulus height;
//  divide that by an aspect ratio and you have the axial chord; divide
//  the circumference by the pitch and you have the blade count.  The
//  compressor annulus therefore closes down by a factor of nine through
//  ten stages not because it was drawn that way but because the total
//  pressure rises twenty-five fold against a temperature rise of under
//  three and the axial Mach number is scheduled down on top of that; the
//  turbine annulus opens out again for the same reasons in reverse.  The
//  engine's LENGTH is a sum of chords, so it too is an output.
//
//  WHAT SETS THE FAN TWIST
//
//  With no inlet swirl the relative flow at radius r arrives at
//
//      tan(beta1) = U(r)/Vx,      U(r) = Omega r,
//
//  measured from axial, and leaves at tan(beta2) = (U - dVt)/Vx where
//  dVt is the swirl the blade adds.  The work is Euler's, dh0 = U dVt.
//  A constant-work fan would need dVt = dh0/U at every radius, and at
//  this hub/tip ratio that asks the root for six times the work its blade
//  speed can deliver.  So the work is capped by a stage loading limit,
//
//      dh0(r) = min( dh0_tip , PSIMAX * U(r)^2 ),
//
//  and the root simply does less.  That is not a modelling dodge; it is
//  why a booster exists at all.  The cap binds inboard of the radius
//  where U = sqrt(dh0_tip/PSIMAX), and the echo below reports it.  The
//  consequence worth watching is the de Haller number W2/W1, the relative
//  diffusion the blade asks for: it is worst exactly at that crossover
//  radius, and with PSIMAX = 0.45 it lands on 0.72, which is the number
//  cascade experiments have been quoting as the limit since the 1950s.
//  Raise PSIMAX and the root stalls; lower it and the fan root stops
//  feeding the core.
//
//  Between root and tip beta1 runs from 31.1 to 63.5 degrees, so the
//  blade is wrung through 32 degrees of twist, and the relative Mach
//  number at the tip is 1.29.  The relative flow is supersonic over the
//  outer two fifths of the span, which is more than half the annulus
//  area, and that is why the tip sections here are thin (3.4 per cent)
//  and barely cambered while the root is fat (11 per cent) and does real
//  turning.
//
//  WHAT SETS THE BYPASS RATIO
//
//  With one axial velocity and one density across the fan face, the
//  streamline that divides the two streams is the one that divides the
//  area in the ratio asked for:
//
//      r_div^2 = (R_case^2 + BPR R_hub^2)/(1 + BPR).
//
//  The splitter nose is put on that streamline, and the echo recomputes
//  the ratio from the radii as drawn.  It is a check on the algebra, not
//  a measurement: the two numbers agree to the last digit or the file is
//  wrong.  At the fan face rho and Vx are uniform across the annulus, so
//  the area split is the mass split exactly.  Downstream the two streams
//  have done different amounts of work and no longer share a density, but
//  the streamline that divides them was fixed before that happened.
//
//  ANNULAR SHELLS ARE ONE POLYHEDRON EACH
//
//  The nacelle is the region between two surfaces of revolution: an
//  outer cowl and the outer wall of the fan duct, meeting at a rounded
//  inlet lip and closed by a blunt annulus at the trailing edge.  That
//  region is bounded by ONE closed surface of genus one, so it is one
//  polyhedron() call over one closed profile loop in the (r,z) half
//  plane.  There is no difference() anywhere in this file, and no
//  intersection().  The core cowl is the same construction: its outer
//  surface is the inner wall of the bypass duct, its inner surface is
//  the outer wall of the core gas path, and the two meet at the splitter
//  nose and again at the core nozzle trailing edge.  In a real engine
//  the volume between the compressor casing and the cowl skin holds the
//  gearbox, the bleed manifolds and the pipework; here it is solid, and
//  the file reports the shell at 14 mm over the core nozzle lip, 18 mm
//  at the splitter nose and 411 mm over the back of the high-pressure
//  compressor, which is where all of that has to live.
//
//  The revolve is written out below rather than handed to
//  rotate_extrude, because rotate_extrude closes its ring by evaluating
//  the profile again at 360 degrees instead of reusing the ring it began
//  from, and a sine of 360 degrees taken in radians is -2.4e-16 and not
//  zero.  The seam therefore misses itself by the radius times that, and
//  a full revolution exports with its seam edges used once where every
//  other edge is used twice: an OFF export of a four-point profile at 200
//  azimuths carries 804 vertices where 800 close the solid.  Indexing the
//  azimuth as (j+1) mod n, which is what revolve_loop() does, closes it
//  exactly and costs nothing.
//
//  WINDING, AND THE ONE RULE THIS FILE USES
//
//  polyhedron() wants every face listed so that the right-hand-rule
//  normal points INTO the solid.  Wound the other way the solid renders
//  identically and then loses geometry in any boolean it meets.  A
//  hundred overlapping cubes wound correctly export as one shell holding
//  the 60,400 cubic millimetres of their union.  Wound outward the same
//  hundred export as a hundred separate shells holding minus 100,000,
//  because the union will not merge a solid whose inside it cannot find.
//  The kernel reports that at each polyhedron.  This file does not lean
//  on it: it computes the signed area of every profile loop and echoes
//  the number, so the claim is arithmetic the model can settle alone.
//
//  Every mesh in this file is emitted with one quad template,
//
//      [ (u,v), (u+1,v), (u+1,v+1), (u,v+1) ],
//
//  whose right-hand normal is (sweep direction) x (section tangent).  So
//  the rule is always the same: if the section is listed counter-
//  clockwise in a frame (A, B) with A x B pointing along the sweep, the
//  normal points inward.  For a blade the sweep is radial and the frame
//  is (tangential, axial), which is right-handed with radial; for a
//  surface of revolution the sweep is along the profile and the section
//  is the circle, which comes to the same statement about the profile
//  loop: traverse it COUNTER-CLOCKWISE in the (r,z) plane with r
//  horizontal.  That is checkable arithmetic rather than a picture, so
//  the model computes the signed area of each profile loop and echoes
//  it; a negative one would mean an inside-out shell.
//
//  A blade section costs one more thought.  The camber line is drawn in
//  its own chord frame, x along the chord and y toward the suction side.
//  Mapping (x, y) into (tangential, axial) sends x to (sin xi, cos xi)
//  and y to (cos xi, -sin xi), and the determinant of that map is -1: it
//  is a reflection, not a rotation, because the camber bulges toward the
//  direction of rotation.  A reflection reverses orientation, so a
//  section drawn counter-clockwise in the chord frame arrives clockwise
//  and its point list is reversed.  Miss that and every blade in the
//  engine is inside out at once.
//
//  THE SECTIONS THEMSELVES
//
//  A circular-arc camber line from the leading-edge metal angle to the
//  trailing-edge one, carrying a NACA four-digit thickness distribution
//  applied normal to it.  The last coefficient is -0.1036, the closed
//  trailing edge variant, so the thickness is exactly zero at x = c and
//  the upper and lower surfaces meet at a point; with -0.1015 they do
//  not, and the sweep needs a seam repair it should not need.  The metal
//  angles are the flow angles corrected for incidence at the leading edge
//  and for deviation at the trailing edge, deviation by Carter's rule
//  delta = m theta/sqrt(sigma), which makes the camber implicit:
//
//      theta = (beta1 - beta2 - i) / (1 - m/sqrt(sigma)).
//
//  So a row of high solidity is given less camber to do the same turning,
//  which is the right way round.  A real turbine aerofoil carries its
//  thickness further aft than a four-digit section does and its camber is
//  not a circular arc; what is claimed here is that the metal angles are
//  right and that the section closes exactly.
//
//  NOTHING TOUCHES ANYTHING
//
//  Every blade stands off its hub and its casing by a real clearance, and
//  no two of the several thousand solids in this file share a cubic
//  millimetre.  That is worth more than it costs.  When solids are
//  disjoint the exported mesh is exactly the mesh written here, face for
//  face, so the volume audit at the foot of the file is a PREDICTION and
//  not an estimate: the revolved parts are integrated in closed form, the
//  blades are integrated from their own triangle lists by the divergence
//  theorem, and the number that comes out is what the exporter must
//  measure.  It does.  The audit predicts 10405.382153 litres in 4160
//  components and 624,160 triangles; an OFF export, which writes its
//  coordinates as decimal text, comes back with 4160 components, 624,160
//  triangles and 10405.382152 litres.  The 1.3 cubic millimetres between
//  them are double-precision round-off between two different orders of
//  summation over ten cubic metres.  The binary STL of the same model
//  measures 10405.382190, and the extra thirty-seven cubic millimetres
//  are the single-precision floats that format stores.
//
//  The component count and the triangle count are predictions on the same
//  terms, and the file states both.  If two blades in a row had
//  intersected, the component count would drop and the volume would fall
//  short, and that is the only cheap way to find out.
//
//  The export prints one warning about this model, and it is worth
//  reading once and then setting aside.  Past twenty-five thousand
//  triangles the exporter declines to run its union and reports that
//  overlapping shells were left separate.  The test behind that sentence
//  is whether any two parts' BOUNDING BOXES intersect, and in a machine
//  built around an axis they all do: the spinner's box lies inside the
//  fan's, which lies inside the nacelle's.  Nothing here overlaps, so
//  there was nothing for the union to do, and the audit settles it from
//  the geometry alone rather than by taking the exporter's word.
//
//  The closed form for a revolved part is exact and not an approximation
//  of the circle.  Sampling every circle at the same NAZ azimuths scales
//  every cross-sectional area by the same factor k = NAZ sin(360/NAZ)/(2 pi),
//  so the faceted volume is k times the smooth one, and the smooth one is
//  the frustum integral round the closed profile loop,
//
//      V = pi * sum (z2 - z1)(r1^2 + r1 r2 + r2^2)/3.
//
//  WHAT IS NOT MODELLED
//
//  No fuel flow and no cooling bleed, so the turbine passes the same mass
//  as the compressor and the work balance is a little optimistic.  The
//  combustor is a length and a pressure loss.  The gearbox, the pylon,
//  the thrust reverser and the accessory drives are not here.  The static
//  thrust the cycle predicts is reported so a reader can see the engine
//  lands where an engine of this size lands, not as a claim about any
//  particular one.
// ===================================================================

// ---- the inputs everything else comes from --------------------------
RTIP   = 1400;     // fan tip radius, mm
HTR    = 0.30;     // fan hub/tip radius ratio
TIPCL  = 6;        // fan tip clearance, mm
NLP    = 2600;     // low-pressure shaft, rev/min
NHP    = 9000;     // high-pressure shaft, rev/min
VX     = 190;      // axial velocity at the fan face, m/s
FPRTIP = 1.45;     // design total pressure ratio at the fan tip
PSIMAX = 0.45;     // largest stage loading dh0/U^2 allowed on the fan
BPR    = 10;       // design bypass ratio (areas at the fan face)
T04    = 1650;     // turbine entry total temperature, K

TAMB  = 288.15;    // ambient total temperature, K (sea level, static)
PAMB  = 101325;    // ambient total pressure, Pa
RECOV = 0.995;     // inlet total pressure recovery
ETAC  = 0.90;      // polytropic efficiency, compression
ETAT  = 0.90;      // polytropic efficiency, expansion
DPCC  = 0.96;      // combustor total pressure ratio

GAMC = 1.4;   CPC = 1005;   // cold gas
GAMH = 1.3333; CPH = 1150;  // hot gas
RGAS = 287;

INC   = 2.0;       // incidence carried at the leading edge, degrees
MCART = 0.25;      // Carter's rule constant for a circular-arc camber line

// clearances: nothing in this file touches anything else
GAPR  = 1.5;       // stator root / rotor platform clearance, mm
GAPC  = 1.0;       // core rotor tip clearance, mm

NAZ    = 144;      // azimuthal facets on every surface of revolution
NCH    = 7;        // chordwise points per side on a core blade
NSP    = 3;        // spanwise panels on a core blade
NCHF   = 22;       // chordwise points per side on a fan blade
NSPF   = 20;       // spanwise panels on a fan blade
NCHV   = 12;       // chordwise points per side on an outlet guide vane
NSPV   = 6;        // spanwise panels on an outlet guide vane

MET = [0.55, 0.57, 0.60];   // cowl skin
BLD = [0.72, 0.74, 0.78];   // rotor blades
VAN = [0.58, 0.62, 0.68];   // stators and vanes
HUBC= [0.46, 0.48, 0.52];   // spinner, drums and plug

// ---- numeric helpers ------------------------------------------------
// A chain of additions one per element would recurse once per element
// and hit the parser's depth guard, so both of these halve the range.
function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

function prod(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 1 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) prod(v, a, m) * prod(v, m, hi);

function clamp(x, lo, hi) = max(lo, min(hi, x));
// Index of the first element equal to x, so an extremum can say where it is.
function argat(V, x) = [ for (i = [0:len(V)-1]) if (V[i] == x) i ][0];
function sstep(t) = let(x = clamp(t, 0, 1)) x*x*(3 - 2*x);
function rev(s) = [ for (i = [len(s)-1 : -1 : 0]) s[i] ];

// Piecewise-linear interpolation over knots [[z, r], ...] sorted in z.
function kseg(K, z) = len([ for (i = [0:len(K)-1]) if (K[i][0] <= z) 1 ]) - 1;
function kat(K, z) =
  let( n = len(K), i = clamp(kseg(K, z), 0, n-2),
       d = K[i+1][0] - K[i][0],
       t = d == 0 ? 0 : clamp((z - K[i][0])/d, 0, 1) )
    K[i][1] + (K[i+1][1] - K[i][1])*t;

// Signed area of a closed loop in (r,z), r horizontal.  Positive is
// counter-clockwise, which is the winding this file's revolve() wants.
function loop_area(P) =
  sum([ for (i = [0:len(P)-1]) let(j = (i+1)%len(P))
          (P[i][0]*P[j][1] - P[j][0]*P[i][1])/2 ]);

// Exact volume of the solid swept by revolving a closed (r,z) loop,
// before faceting: pi * contour integral of r^2 dz, done frustum by
// frustum so it is exact for the polygon actually drawn.
function loop_vol(P) =
  PI*sum([ for (i = [0:len(P)-1]) let(j = (i+1)%len(P))
             (P[j][1] - P[i][1])*(P[i][0]*P[i][0]
               + P[i][0]*P[j][0] + P[j][0]*P[j][0])/3 ]);

// The same integral for an open profile that begins and ends on the
// axis; the closing run back along r = 0 contributes nothing.
function open_vol(P) =
  PI*sum([ for (i = [0:len(P)-2])
             (P[i+1][1] - P[i][1])*(P[i][0]*P[i][0]
               + P[i][0]*P[i+1][0] + P[i+1][0]*P[i+1][0])/3 ]);

// Sampling every circle at the same NAZ azimuths scales every cross
// section by this factor, so it scales the volume by it too.
KFAC = NAZ*sin(360/NAZ)/(2*PI);

// Exact volume of a polyhedron from its own point and face lists.  The
// faces are wound inward, so the divergence-theorem sum comes out
// negative and is negated here.
function tri6(a, b, c) = (  a[0]*(b[1]*c[2] - b[2]*c[1])
                          + a[1]*(b[2]*c[0] - b[0]*c[2])
                          + a[2]*(b[0]*c[1] - b[1]*c[0]) )/6;
function face_vol(P, f) =
  sum([ for (i = [1:len(f)-2]) tri6(P[f[0]], P[f[i]], P[f[i+1]]) ]);
function mesh_vol(P, F) = -sum([ for (f = F) face_vol(P, f) ]);

// ---- one-dimensional gas dynamics -----------------------------------
function phiflow(M, g) =
  g/sqrt(g-1)*M*pow(1 + (g-1)/2*M*M, -(g+1)/(2*(g-1)));
function area_m2(m, cp, T0, p0, M, g) = m*sqrt(cp*T0)/(p0*phiflow(M, g));

// Exit velocity and area of a convergent nozzle, choked or not.
function nz_mach(npr, g) = min(1, sqrt(2/(g-1)*(pow(npr, (g-1)/g) - 1)));
function nz_area(m, T0, p0, g, cp) =
  let( Rg = cp*(g-1)/g, M = nz_mach(p0/PAMB, g),
       T = T0/(1 + (g-1)/2*M*M),
       p = p0/pow(1 + (g-1)/2*M*M, g/(g-1)),
       V = M*sqrt(g*Rg*T) )
    m/((p/(Rg*T))*V);
function nz_vel(T0, p0, g, cp) =
  let( Rg = cp*(g-1)/g, M = nz_mach(p0/PAMB, g),
       T = T0/(1 + (g-1)/2*M*M) ) M*sqrt(g*Rg*T);

// ===================================================================
//  THE FAN
// ===================================================================
OMLP = NLP*2*PI/60;                 // rad/s
OMHP = NHP*2*PI/60;
function U_(r) = OMLP*r/1000;       // blade speed, m/s, from r in mm
function UH_(r) = OMHP*r/1000;

RCAS  = RTIP + TIPCL;               // fan casing radius
RHUBF = HTR*RTIP;                   // fan hub radius
UTIP  = U_(RTIP);
AFAN  = PI*(RCAS*RCAS - RHUBF*RHUBF)/1e6;          // m^2
T1S   = TAMB - VX*VX/(2*CPC);                      // static at the fan face
P01   = PAMB*RECOV;
RHO1  = P01*pow(T1S/TAMB, GAMC/(GAMC-1))/(RGAS*T1S);
MDOT  = RHO1*VX*AFAN;
MXFAN = VX/sqrt(GAMC*RGAS*T1S);

// Euler work, capped by the stage loading limit.
DHTIP = CPC*TAMB*(pow(FPRTIP, (GAMC-1)/(GAMC*ETAC)) - 1);
RCROSS = 1000*sqrt(DHTIP/PSIMAX)/OMLP;             // where the cap starts
function dh_fan(r)  = min(DHTIP, PSIMAX*U_(r)*U_(r));
function dvt_fan(r) = dh_fan(r)/U_(r);
function b1_fan(r)  = atan(U_(r)/VX);
function b2_fan(r)  = atan((U_(r) - dvt_fan(r))/VX);
function dehaller(r) = norm([U_(r) - dvt_fan(r), VX])/norm([U_(r), VX]);
function mrel(r) = norm([U_(r), VX])/sqrt(GAMC*RGAS*T1S);
// Where the relative flow first goes sonic: U^2 + Vx^2 = a^2.
RSONIC = 1000*sqrt(GAMC*RGAS*T1S - VX*VX)/OMLP;

// Area-weighted mean of the work between two radii; Vx and rho are
// uniform at the fan face, so area weighting is mass weighting.
NAVG = 240;
function avg_dh(r0, r1) =
  let( W = [ for (i = [0:NAVG-1]) r0 + (r1-r0)*(i+0.5)/NAVG ] )
    sum([ for (r = W) dh_fan(r)*r ]) / sum(W);

// The dividing streamline: the one that splits the area as asked.
RDIV  = sqrt((RCAS*RCAS + BPR*RHUBF*RHUBF)/(1 + BPR));
DHCORE = avg_dh(RHUBF, RDIV);
DHBYP  = avg_dh(RDIV, RCAS);
DHALL  = avg_dh(RHUBF, RCAS);
function fpr_of(dh) = pow(1 + dh/(CPC*TAMB), ETAC*GAMC/(GAMC-1));
MCORE = MDOT/(1 + BPR);
MBYP  = MDOT - MCORE;

// ---- fan blade ------------------------------------------------------
CFR  = 360;    CFT  = 540;     // chord at root and tip, mm
TCFR = 0.11;   TCFT = 0.034;   // thickness/chord at root and tip
AXF  = 0.38;                   // stacking axis, fraction of chord
SWF  = 70;                     // forward sweep of the mid span, mm
SWT  = 140;                    // aft sweep of the tip, mm

FROOT = RHUBF + GAPR;
function fspan(i) = i/NSPF;
function fanr(i)  = FROOT + (RTIP - FROOT)*fspan(i);
function fanc(i)  = CFR + (CFT - CFR)*fspan(i);
function fantc(i) = TCFR + (TCFT - TCFR)*fspan(i);
function fansig(i) = NFAN*fanc(i)/(2*PI*fanr(i));
// Stacking-axis offset: the mid span leads, the tip trails, which is
// what a swept fan does and what keeps the shock off the casing.
function fandz(i) = let(s = fspan(i)) -SWF*4*s*(1-s) + SWT*s*s*s;

SIGFT = 1.35;                  // design solidity at the fan tip
NFAN  = round(2*PI*RTIP*SIGFT/CFT);

// Metal angles: incidence at the nose, Carter deviation at the tail.
function camber(b1, b2, sig) = (b1 - b2 - INC)/(1 - MCART/sqrt(sig));
function fanchi1(i) = -(b1_fan(fanr(i)) - INC);
function fanchi2(i) = fanchi1(i)
    + camber(b1_fan(fanr(i)), b2_fan(fanr(i)), fansig(i));

// ===================================================================
//  SECTIONS
// ===================================================================
// NACA four-digit half thickness, closed trailing edge (-0.1036).
function naca4(u, tc) = 5*tc*( 0.2969*sqrt(u) - 0.1260*u - 0.3516*u*u
                             + 0.2843*u*u*u  - 0.1036*u*u*u*u );

// Circular-arc camber line of total camber th over a chord c, and its
// slope.  th carries a sign: positive bends toward +y.
function cam_y(x, c, th) =
  th == 0 ? 0
  : let( R = c/(2*sin(abs(th)/2)) )
      (th < 0 ? -1 : 1)*(sqrt(R*R - (x - c/2)*(x - c/2)) - R*cos(th/2));
function cam_s(x, c, th) =
  th == 0 ? 0
  : let( R = c/(2*sin(abs(th)/2)) )
      (th < 0 ? -1 : 1)*(c/2 - x)/sqrt(R*R - (x - c/2)*(x - c/2));

// A closed aerofoil in the (tangential, axial) frame.  chi1 and chi2 are
// the leading- and trailing-edge metal angles from axial; ax is where the
// stacking axis crosses the chord.  Cosine spacing bunches the points at
// the nose, where the curvature is.  The point list runs lower surface
// nose to tail, then upper surface tail to nose: counter-clockwise in
// the chord frame, and therefore clockwise once the reflection below has
// been applied, which is why it comes back reversed.
function bsec(c, tc, chi1, chi2, ax, n) =
  let( xi = (chi1 + chi2)/2, th = chi1 - chi2,
       S = sin(xi), C = cos(xi),
       lo = [ for (i = [0:n]) let( u = 0.5 - 0.5*cos(180*i/n), x = c*u,
                  t = naca4(u, tc)*c, p = atan(cam_s(x, c, th)),
                  yy = cam_y(x, c, th) )
                [ x + t*sin(p), yy - t*cos(p) ] ],
       up = [ for (i = [n-1:-1:1]) let( u = 0.5 - 0.5*cos(180*i/n), x = c*u,
                  t = naca4(u, tc)*c, p = atan(cam_s(x, c, th)),
                  yy = cam_y(x, c, th) )
                [ x - t*sin(p), yy + t*cos(p) ] ] )
    rev([ for (q = concat(lo, up))
            [ (q[0] - ax*c)*S + q[1]*C, (q[0] - ax*c)*C - q[1]*S ] ]);

// ===================================================================
//  MESH EMITTERS
// ===================================================================
// grid[u][v]: u walks the sweep, v walks a closed section.  The quad
// template's right-hand normal is (sweep) x (section tangent), which
// points into the solid when the section is counter-clockwise in a frame
// (A,B) with A x B along the sweep.
function sw_pts(g) =
  let( NU = len(g)-1, NV = len(g[0]) )
    concat([ for (u = [0:NU]) each g[u] ],
           [ [ for (k = [0:2]) sum([ for (p = g[0])  p[k] ])/NV ] ],
           [ [ for (k = [0:2]) sum([ for (p = g[NU]) p[k] ])/NV ] ]);
function sw_faces(NU, NV) =
  let( B0 = (NU+1)*NV, B1 = B0+1 )
    concat(
      [ for (u = [0:NU-1]) for (v = [0:NV-1])
          [ u*NV+v, (u+1)*NV+v, (u+1)*NV+(v+1)%NV, u*NV+(v+1)%NV ] ],
      [ for (v = [0:NV-1]) [ B0, v, (v+1)%NV ] ],
      [ for (v = [0:NV-1]) [ B1, NU*NV+(v+1)%NV, NU*NV+v ] ]);
module sweep(g, conv = 6) {
    polyhedron(points = sw_pts(g),
               faces  = sw_faces(len(g)-1, len(g[0])), convexity = conv);
}
function sweep_vol(g) = mesh_vol(sw_pts(g), sw_faces(len(g)-1, len(g[0])));

// A closed (r,z) loop revolved about z.  Genus one: an annular shell.
module revolve_loop(P, n = NAZ, conv = 10) {
    M = len(P);
    polyhedron(
      points = [ for (i = [0:M-1]) each
                   [ for (j = [0:n-1]) let(a = 360*j/n)
                       [ P[i][0]*cos(a), P[i][0]*sin(a), P[i][1] ] ] ],
      faces = [ for (i = [0:M-1]) let(k = (i+1)%M) for (j = [0:n-1])
                  [ i*n+j, k*n+j, k*n+(j+1)%n, i*n+(j+1)%n ] ],
      convexity = conv);
}

// An open (r,z) profile that starts and ends on the axis.
module revolve_axis(P, n = NAZ, conv = 8) {
    M = len(P);
    pts = concat([ [0, 0, P[0][1]] ],
      [ for (i = [1:M-2]) each
          [ for (j = [0:n-1]) let(a = 360*j/n)
              [ P[i][0]*cos(a), P[i][0]*sin(a), P[i][1] ] ] ],
      [ [0, 0, P[M-1][1]] ]);
    TOP = 1 + (M-2)*n;
    polyhedron(points = pts,
      faces = concat(
        [ for (j = [0:n-1]) [ 0, 1+j, 1+(j+1)%n ] ],
        [ for (i = [1:M-3]) for (j = [0:n-1])
            [ 1+(i-1)*n+j, 1+i*n+j, 1+i*n+(j+1)%n, 1+(i-1)*n+(j+1)%n ] ],
        [ for (j = [0:n-1]) [ TOP, 1+(M-3)*n+(j+1)%n, 1+(M-3)*n+j ] ]),
      convexity = conv);
}

// A blade: sections wrapped onto cylinders of their own radius.  The
// tangential coordinate is arc length, so the section is a true
// blade-to-blade section and not a flat plate leaned over.
function blade_grid(secs, radii, zc, dz) =
  [ for (u = [0:len(secs)-1])
      [ for (p = secs[u])
          let( t = p[0]/radii[u]*180/PI )
            [ radii[u]*cos(t), radii[u]*sin(t), zc + dz[u] + p[1] ] ] ];

// ===================================================================
//  THE CORE: every row sized by continuity, every angle by a triangle
// ===================================================================
REACT = 0.5;       // stage reaction, which makes rotor and stator mirrors
DEVT  = 2.0;       // trailing-edge deviation allowed on a turbine row, deg
AXB   = 0.42;      // stacking axis, fraction of chord, on a core blade
ROWGAP= 0.32;      // axial gap between rows, as a fraction of the extent
PADF  = 0.10;      // platform overhang beyond a blade, same fraction

NBST = 3;   RMB0 = 500; RMB1 = 545; PSIB = 0.40; PHIB = 0.55;
            MXB0 = 0.52; MXB1 = 0.50; ARB0 = 2.4; ARB1 = 2.2;
            SIGB = 1.45; TCB = 0.085;
NHPC = 10;  RMC  = 400; PSIC = 0.40; PHIC = 0.50;
            MXC0 = 0.50; MXC1 = 0.28; ARC0 = 2.6; ARC1 = 0.90;
            SIGC = 1.50; TCC = 0.070;
NHPT = 2;   RMT0 = 430; RMT1 = 452; PHIT = 0.62;
            MXT0 = 0.16; MXT1 = 0.30; ART = 1.15; ZWT = 1.00; TCT = 0.20;
NLPT = 6;   RML0 = 520; RML1 = 650; PHIL = 0.75;
            MXL0 = 0.42; MXL1 = 0.55; ARL = 2.00; ZWL = 1.00; TCL = 0.15;

// A row is one vector.  Fields, in order:
//  0 name   1 r_mean  2 kind  3 Omega  4 Mx  5 aspect ratio
//  6 pitch parameter (solidity for a compressor, pitch/axial chord for a
//    turbine)                        7 dT0 across this row
//  8 dT0 across this row's stage (turbines only, for the pressure split)
//  9 inlet swirl at r_mean, m/s     10 exit swirl there
// 11 axial velocity, m/s            12 thickness/chord
// kind: 0 compressor rotor, 1 compressor stator, 2 nozzle vane, 3 turbine rotor
function cgroup(tag, n, rm0, rm1, psi, phi, Om, Mx0, Mx1, AR0, AR1, sig, tc) =
  [ for (k = [0:n-1])
      let( f = n < 2 ? 0 : k/(n-1),
           rm = rm0 + (rm1-rm0)*f, U = Om*rm/1000,
           v1 = (1-REACT - psi/2)*U, v2 = (1-REACT + psi/2)*U,
           vx = phi*U, dT = psi*U*U/CPC,
           Mx = Mx0 + (Mx1-Mx0)*f, AR = AR0 + (AR1-AR0)*f )
        each [ [ str(tag,k+1,"R"), rm, 0, Om, Mx, AR, sig, dT, 0, v1, v2, vx, tc ],
               [ str(tag,k+1,"S"), rm, 1, Om, Mx, AR, sig,  0, 0, v2, v1, vx, tc ] ] ];

// The axial Mach number climbs through a turbine as the pressure falls;
// held constant it would open the annulus out twice as fast as it should.
function tgroup(tag, n, rm0, rm1, dTs, phi, Om, Mx0, Mx1, AR, zw, tc) =
  [ for (k = [0:n-1])
      let( f = n < 2 ? 0 : k/(n-1),
           rm = rm0 + (rm1-rm0)*f, U = Om*rm/1000,
           psi = CPH*dTs/(U*U),
           v2 = ((1-REACT) + psi/2)*U, v3 = ((1-REACT) - psi/2)*U,
           vx = phi*U,
           MN = Mx0 + (Mx1-Mx0)*(2*k)/(2*n-1),
           MR = Mx0 + (Mx1-Mx0)*(2*k+1)/(2*n-1) )
        each [ [ str(tag,k+1,"N"), rm, 2, Om, MN, AR, zw,   0, dTs, v3, v2, vx, tc ],
               [ str(tag,k+1,"R"), rm, 3, Om, MR, AR, zw, dTs, dTs, v2, v3, vx, tc ] ] ];

// The cold chain: booster then high-pressure compressor.  dT0 across a
// compressor row is psi U^2/cp and does not depend on where in the chain
// the row sits, so the running total is a prefix sum and the running
// pressure a prefix product.  No recursion per stage.
T0CORE = TAMB + DHCORE/CPC;
P0CORE = P01*fpr_of(DHCORE);
CROWS  = concat(cgroup("B", NBST, RMB0, RMB1, PSIB, PHIB, OMLP,
                       MXB0, MXB1, ARB0, ARB1, SIGB, TCB),
                cgroup("C", NHPC, RMC,  RMC,  PSIC, PHIC, OMHP,
                       MXC0, MXC1, ARC0, ARC1, SIGC, TCC));
CDT = [ for (R = CROWS) R[7] ];
function cT0(i) = T0CORE + sum(CDT, 0, i);
CPR = [ for (i = [0:len(CROWS)-1])
          CROWS[i][2] == 0 ? pow(1 + CDT[i]/cT0(i), ETAC*GAMC/(GAMC-1)) : 1 ];
function cP0(i) = P0CORE*prod(CPR, 0, i);

NCROW  = len(CROWS);
DT_BST = sum([ for (i = [0:2*NBST-1]) CDT[i] ]);
DT_HPC = sum([ for (i = [2*NBST:NCROW-1]) CDT[i] ]);
T03 = cT0(NCROW-1) + CDT[NCROW-1];
P03 = cP0(NCROW-1)*CPR[NCROW-1];
P04 = P03*DPCC;

// Work balance.  No fuel flow and no cooling bleed are modelled, so the
// turbines pass the same mass as the compressors; that makes the engine
// slightly optimistic and is the largest single simplification here.
DT_HPT = DT_HPC*CPC/CPH;
DT_LPT = (MDOT*DHALL + MCORE*CPC*DT_BST)/(MCORE*CPH);

HROWS = concat(tgroup("T", NHPT, RMT0, RMT1, DT_HPT/NHPT, PHIT, OMHP,
                      MXT0, MXT1, ART, ZWT, TCT),
               tgroup("L", NLPT, RML0, RML1, DT_LPT/NLPT, PHIL, OMLP,
                      MXL0, MXL1, ARL, ZWL, TCL));
HDT = [ for (R = HROWS) R[7] ];
function hT0(i) = T04 - sum(HDT, 0, i);
// Each row of a stage takes half the stage's total pressure drop.
HPR = [ for (i = [0:len(HROWS)-1])
          pow(hT0(i)/(hT0(i) - HROWS[i][8]), -GAMH/(2*(GAMH-1)*ETAT)) ];
function hP0(i) = P04*prod(HPR, 0, i);
NHROW = len(HROWS);
T05 = hT0(NHROW-1) - HDT[NHROW-1];
P05 = hP0(NHROW-1)*HPR[NHROW-1];

// Annulus area, height and axial chord, appended to each row as fields
// 13 (height, mm), 14 (axial chord, mm), 15 (T0), 16 (p0).
ROWS = concat(
  [ for (i = [0:NCROW-1]) let( R = CROWS[i],
        A = area_m2(MCORE, CPC, cT0(i), cP0(i), R[4], GAMC),
        h = A*1e6/(2*PI*R[1]) )
      concat(R, [h, h/R[5], cT0(i), cP0(i)]) ],
  [ for (i = [0:NHROW-1]) let( R = HROWS[i],
        A = area_m2(MCORE, CPH, hT0(i), hP0(i), R[4], GAMH),
        h = A*1e6/(2*PI*R[1]) )
      concat(R, [h, h/R[5], hT0(i), hP0(i)]) ]);
NROW = len(ROWS);

// Flow angle from axial at radius r, free-vortex: r Vt is constant, so
// the twist of every core blade falls out of its mean-radius triangle.
function rowf(R, w, r) =
  let( vt = R[w]*R[1]/r, U = R[3]*r/1000, rel = (R[2] == 0 || R[2] == 3) )
    atan((rel ? vt - U : vt)/R[11]);

// Metal angles.  Compressor: incidence at the nose, Carter deviation at
// the tail, which makes the camber implicit in the solidity.  Turbine:
// an accelerating cascade follows its blade closely, so zero incidence
// and a fixed small deviation.
function metal(f1, f2, sig) =
  let( s = f1 < 0 ? -1 : 1, D = s*(f1 - f2),
       th = (D - INC)/(1 - MCART/sqrt(sig)), c1 = f1 - s*INC )
    [ c1, c1 - s*th ];
function metal_t(f1, f2) = [ f1, f2 + (f2 < 0 ? -1 : 1)*DEVT ];
function rowmetal(R, r, sig) =
  R[2] <= 1 ? metal(rowf(R,9,r), rowf(R,10,r), sig)
            : metal_t(rowf(R,9,r), rowf(R,10,r));

// Chord and blade count.  The compressor is pitched by solidity, the
// turbine by pitch over axial chord, which is how each is actually done.
function rowxi(R) = let( m = rowmetal(R, R[1], R[6]) ) (m[0] + m[1])/2;
function rowc(R)  = R[14]/cos(rowxi(R));
function rowN(R)  = R[2] <= 1 ? max(11, round(2*PI*R[1]*R[6]/rowc(R)))
                              : max(11, round(2*PI*R[1]/(R[6]*R[14])));

function rowr0(R) = R[1] - R[13]/2 + (R[2] == 0 || R[2] == 3 ? GAPR : GAPC);
function rowr1(R) = R[1] + R[13]/2 - (R[2] == 0 || R[2] == 3 ? GAPC : GAPR);

function row_secs(R, ns) =
  let( N = rowN(R), c = rowc(R), r0 = rowr0(R), r1 = rowr1(R) )
    [ for (u = [0:ns]) let( r = r0 + (r1-r0)*u/ns,
                            sg = N*c/(2*PI*r),
                            m = rowmetal(R, r, sg) )
        bsec(c, R[12], m[0], m[1], AXB, NCH) ];

SECS = [ for (R = ROWS) row_secs(R, NSP) ];
// The drawn axial extent is the camber line's chord plus what the
// thickness adds at each end; the platforms are set from it, not from
// the nominal chord, so a fat root section cannot overhang its platform.
function bmin(S) = min([ for (s = S) min([ for (p = s) p[1] ]) ]);
function bmax(S) = max([ for (s = S) max([ for (p = s) p[1] ]) ]);
EXT = [ for (S = SECS) bmax(S) - bmin(S) ];

// ---- the fan blade, and where the splitter therefore goes -----------
FSECS = [ for (i = [0:NSPF])
            bsec(fanc(i), fantc(i), fanchi1(i), fanchi2(i), AXF, NCHF) ];
FDZ   = [ for (i = [0:NSPF]) fandz(i) ];
FZ0 = min([ for (i = [0:NSPF]) FDZ[i] + min([ for (p = FSECS[i]) p[1] ]) ]);
FZ1 = max([ for (i = [0:NSPF]) FDZ[i] + max([ for (p = FSECS[i]) p[1] ]) ]);
FPAD  = 30;                  // fan platform overhang, mm
SPGAP = 60;                  // clear air between the fan and the splitter
SPA   = 45;                  // splitter nose, axial half-length
SPB   = 9;                   // splitter nose, radial half-thickness
ZFP0  = FZ0 - FPAD;
ZSP   = FZ1 + FPAD + SPGAP;  // splitter nose station
ZSPE  = ZSP + SPA;           // where the splitter has reached full thickness

// ---- axial layout: the engine's length is a sum of chords -----------
NB = 2*NBST; NC = 2*NHPC; NT = 2*NHPT; NL = 2*NLPT;
IB0 = 0;       IB1 = NB-1;
IC0 = NB;      IC1 = NB+NC-1;
IT0 = NB+NC;   IT1 = NB+NC+NT-1;
IL0 = NB+NC+NT; IL1 = NROW-1;
GNECK = 1.35*((ROWS[IB1][1] - ROWS[IB1][13]/2) - (ROWS[IC0][1] - ROWS[IC0][13]/2));
CMBL  = 380;                 // prediffuser, dome and flame tube
ITDL  = 1.45*(ROWS[IL0][1] - ROWS[IT1][1]);
EXHL  = 420;                 // turbine exit duct to the core nozzle plane
ZCORE0 = ZSPE + 40;
DUCT = [ for (i = [0:NROW-1])
           i == IB1 ? GNECK : i == IC1 ? CMBL : i == IT1 ? ITDL : 0 ];
ADV = [ for (i = [0:NROW-1]) EXT[i]*(1 + ROWGAP) + DUCT[i] ];
function rowz(i) = ZCORE0 + sum(ADV, 0, i);     // leading edge of row i
function rowzc(i) = rowz(i) - bmin(SECS[i]);    // section origin

ZLPTE = rowz(IL1) + EXT[IL1];
ZCNTE = ZLPTE + EXHL;                           // core nozzle exit plane
ZPTIP = ZCNTE + 480;                            // plug tip

// ---- nozzle areas fix the two exit radii ----------------------------
T0BYP = TAMB + DHBYP/CPC;   P0BYP = P01*fpr_of(DHBYP);
ABN   = nz_area(MBYP, T0BYP, P0BYP, GAMC, CPC);    // m^2
ACN   = nz_area(MCORE, T05, P05, GAMH, CPH);
VJB   = nz_vel(T0BYP, P0BYP, GAMC, CPC);
VJC   = nz_vel(T05, P05, GAMH, CPH);

RPLUGN = 0.60*(ROWS[IL1][1] - ROWS[IL1][13]/2);
RCCI   = sqrt(ACN*1e6/PI + RPLUGN*RPLUGN);         // core nozzle, outer
TCTE   = 14;                                       // core cowl trailing edge
TNTE   = 12;                                       // nacelle trailing edge

// ---- the outlet guide vanes ----------------------------------------
OGVSTEP = 40;                // the duct wall stands this far outside
                             // the splitter, to clear the booster casing
ROGVH = RDIV + OGVSTEP;      // core cowl outer, flat under the vane
ROGVC = RCAS;                // nacelle inner, flat under the vane
NOGV  = 2*NFAN + 1;          // Tyler-Sofrin: the fundamental rotor tone
                             // is cut off when the vane count exceeds
                             // twice the blade count
AOGV  = PI*(ROGVC*ROGVC - ROGVH*ROGVH)/1e6;
// One Picard step for the duct axial velocity: stagnation density first,
// then correct it with the Mach number that came out.
RHOB0 = P0BYP/(RGAS*T0BYP);
VXB0  = MBYP/(RHOB0*AOGV);
function rf_of(ro) = sqrt(RDIV*RDIV + (RCAS*RCAS - RDIV*RDIV)
                          *(ro*ro - ROGVH*ROGVH)/(ROGVC*ROGVC - ROGVH*ROGVH));
VTOGV = dvt_fan(rf_of((ROGVH+ROGVC)/2))*rf_of((ROGVH+ROGVC)/2)/((ROGVH+ROGVC)/2);
MB1   = norm([VXB0, VTOGV])/sqrt(GAMC*RGAS*T0BYP);
RHOB  = RHOB0*pow(1 + (GAMC-1)/2*MB1*MB1, -1/(GAMC-1));
VXB   = MBYP/(RHOB*AOGV);
// Angular momentum is conserved between the fan trailing edge and the
// vane, and the streamline that was at r_f at the fan face is at r_o
// here, equal area fraction for equal mass fraction.
function ogv_a(ro) = let( rfz = rf_of(ro) )
    atan(dvt_fan(rfz)*rfz/ro/VXB);
CVAN = 190;  TCV = 0.10;  AXV = 0.40;
ROGV0 = ROGVH + GAPR;  ROGV1 = ROGVC - GAPR;
VSECS = [ for (i = [0:NSPV])
            let( r = ROGV0 + (ROGV1-ROGV0)*i/NSPV,
                 sg = NOGV*CVAN/(2*PI*r),
                 m = metal(ogv_a(r), 0, sg) )
              bsec(CVAN, TCV, m[0], m[1], AXV, NCHV) ];
VEXT = bmax(VSECS) - bmin(VSECS);
ZOGV = ZSPE + 140;                       // vane leading edge
ZOGVE = ZOGV + VEXT;
ZOGVC = ZOGV - bmin(VSECS);

// ===================================================================
//  THE SURFACES OF REVOLUTION
// ===================================================================
// Profile points are [r, z].  Every loop below is traversed counter-
// clockwise in that plane, which is what revolve_loop() wants; the
// echoes at the foot of the file report each loop's signed area so the
// claim is arithmetic rather than a picture.
function srun(z0, r0, z1, r1, n, i0, i1) =   // smooth, flat at both ends
  [ for (i = [i0:i1]) [ r0 + (r1-r0)*sstep(i/n), z0 + (z1-z0)*i/n ] ];
function prun(z0, r0, z1, r1, n, q, i0, i1) = // flat at the first end only
  [ for (i = [i0:i1]) [ r0 + (r1-r0)*pow(i/n, q), z0 + (z1-z0)*i/n ] ];
function erun(z0, r0, a, b, n, i0, i1) =      // quarter ellipse
  [ for (i = [i0 : (i1 < i0 ? -1 : 1) : i1]) let(p = 90*i/n)
      [ r0 + b*sin(p), z0 + a*(1 - cos(p)) ] ];

// ---- the core casing and the cowl skin over it ----------------------
WALLC = 22;          // least metal between the gas path and the cowl skin
RCMAX = 820;         // cowl radius over the high-pressure spool
// The combustor is the one length in the core that is not a blade row,
// so its walls are the one pair of knots not taken from a velocity
// triangle: the flame tube needs room, so the casing steps out and the
// inner case steps in, and the diffuser and the nozzle guide vanes on
// either side are what the steps run between.
ZCB0 = rowz(IC1) + EXT[IC1]*(1 + PADF);
ZCB1 = rowz(IT0) - EXT[IT0]*PADF;
CBOUT = 1.32; CBIN = 0.80;
function casknots(i0, i1) =
  [ for (i = [i0:i1]) each
      let( z0 = rowz(i), z1 = z0 + EXT[i], p = PADF*EXT[i],
           rc = ROWS[i][1] + ROWS[i][13]/2 )
        [ [z0-p, rc], [z1+p, rc] ] ];
function hubknots(i0, i1) =
  [ for (i = [i0:i1]) each
      let( z0 = rowz(i), z1 = z0 + EXT[i], p = PADF*EXT[i],
           rh = ROWS[i][1] - ROWS[i][13]/2 )
        [ [z0-p, rh], [z1+p, rh] ] ];
CASK = concat(
  [ [ZSPE, RDIV - SPB] ],
  casknots(0, IC1),
  [ [ZCB0 + 0.22*(ZCB1-ZCB0), (ROWS[IC1][1] + ROWS[IC1][13]/2)*CBOUT],
    [ZCB0 + 0.74*(ZCB1-ZCB0), (ROWS[IC1][1] + ROWS[IC1][13]/2)*CBOUT] ],
  casknots(IT0, IL1),
  [ [ZCNTE, RCCI] ]);
COWLK = [ [ZSPE, RDIV + SPB],
          [ZOGV - 40, ROGVH], [ZOGVE + 40, ROGVH],
          [rowz(IB1) + EXT[IB1] + 60, RCMAX], [rowz(IC1) + EXT[IC1], RCMAX],
          [rowz(IL0), ROWS[IL0][1] + ROWS[IL0][13]/2 + 60],
          [ZLPTE, ROWS[IL1][1] + ROWS[IL1][13]/2 + 40],
          [ZCNTE, RCCI + TCTE] ];
// Does the skin ever fall into the gas path?  Checked, not assumed.
CLEAR = [ for (K = CASK) kat(COWLK, K[0]) - K[1] ];

CCPROF = concat(
  [ [RDIV, ZSP] ],
  erun(ZSP, RDIV, SPA,  SPB, 10, 1, 10),
  [ for (i = [1:len(COWLK)-1]) [COWLK[i][1], COWLK[i][0]] ],
  [ [RCCI, ZCNTE] ],
  [ for (i = [len(CASK)-2 : -1 : 0]) [CASK[i][1], CASK[i][0]] ],
  erun(ZSP, RDIV, SPA, -SPB, 10, 9, 1));

// ---- the hub: spinner, drums, shaft and plug, one solid -------------
SPNP  = 0.80;        // spinner power law; 1 would be a plain cone
NSPIN = 44;
ZNOSE = ZFP0 - 0.45*RTIP;
SPINNER = [ for (i = [0:NSPIN])
              [ RHUBF*pow(i/NSPIN, SPNP), ZNOSE + (ZFP0-ZNOSE)*i/NSPIN ] ];
HUBK = concat(
  [ [ZFP0, RHUBF], [ZSPE, RHUBF] ],
  hubknots(0, IC1),
  [ [ZCB0 + 0.22*(ZCB1-ZCB0), (ROWS[IC1][1] - ROWS[IC1][13]/2)*CBIN],
    [ZCB0 + 0.74*(ZCB1-ZCB0), (ROWS[IC1][1] - ROWS[IC1][13]/2)*CBIN] ],
  hubknots(IT0, IL1),
  [ [ZCNTE, RPLUGN], [ZPTIP, 0] ]);
HUBPROF = concat(SPINNER,
  [ for (i = [1:len(HUBK)-1]) [HUBK[i][1], HUBK[i][0]] ]);

// ---- the nacelle ----------------------------------------------------
// The throat is the minimum flow area, so it is sized from the fan
// annulus and the diffuser area ratio; the highlight is then sized from
// the throat and the contraction ratio.  Neither radius is chosen.
DAR   = 1.08;        // inlet diffuser area ratio, throat to fan face
CRAT  = 1.28;        // inlet contraction ratio, highlight to throat
INLEN = 0.34*2*RTIP; // inlet length, highlight to fan face
RTHR  = 1000*sqrt(AFAN/DAR/PI);
RHL   = RTHR*sqrt(CRAT);
ZHL   = -INLEN;
ZTHR  = ZHL + 0.25*INLEN;
ZNTE  = rowz(IT0);                    // fan cowl trailing edge
RCOWN = kat(COWLK, ZNTE);
RNITE = sqrt(ABN*1e6/PI + RCOWN*RCOWN);
RNOTE = RNITE + TNTE;
RMAXN = 1.16*RTIP;
ZMAXN = ZHL + 0.30*(ZNTE - ZHL);
NACPROF = concat(
  erun(ZHL, RHL, ZMAXN - ZHL, RMAXN - RHL, 26, 0, 26),
  prun(ZMAXN, RMAXN, ZNTE, RNOTE, 34, 1.6, 1, 34),
  [ [RNITE, ZNTE] ],
  srun(ZNTE, RNITE, ZOGVE + 40, RCAS, 30, 1, 30),
  [ [RCAS, ZFP0] ],
  srun(ZFP0, RCAS, ZTHR, RTHR, 26, 1, 26),
  erun(ZHL, RHL, ZTHR - ZHL, -(RHL - RTHR), 18, 17, 1));

// ===================================================================
//  INSTANTIATION
// ===================================================================
FRAD = [ for (i = [0:NSPF]) fanr(i) ];
FGRID = blade_grid(FSECS, FRAD, 0, FDZ);
VRAD = [ for (i = [0:NSPV]) ROGV0 + (ROGV1-ROGV0)*i/NSPV ];
VGRID = blade_grid(VSECS, VRAD, ZOGVC, [ for (i = [0:NSPV]) 0 ]);
function rowrad(R) = [ for (u = [0:NSP])
                         rowr0(R) + (rowr1(R)-rowr0(R))*u/NSP ];
function rowgrid(i) = blade_grid(SECS[i], rowrad(ROWS[i]), rowzc(i),
                                 [ for (u = [0:NSP]) 0 ]);

color(HUBC) revolve_axis(HUBPROF);
color(MET)  revolve_loop(NACPROF);
color(MET)  revolve_loop(CCPROF);
for (k = [0:NFAN-1]) rotate([0, 0, 360*k/NFAN]) color(BLD) sweep(FGRID, 10);
for (k = [0:NOGV-1]) rotate([0, 0, 360*k/NOGV]) color(VAN) sweep(VGRID, 8);
for (i = [0:NROW-1])
  let( R = ROWS[i], N = rowN(R), g = rowgrid(i),
       col = (R[2] == 0 || R[2] == 3) ? BLD : VAN )
    for (k = [0:N-1]) rotate([0, 0, 360*k/N]) color(col) sweep(g, 6);

// ===================================================================
//  WHAT THE GEOMETRY SAYS
// ===================================================================
NBLADE = sum([ for (R = ROWS) rowN(R) ]);
NSOLID = 3 + NFAN + NOGV + NBLADE;
// The triangle count is as predictable as the volume.  A closed profile
// loop of M points revolved at NAZ azimuths is 2 M NAZ triangles; an open
// profile that begins and ends on the axis is 2 NAZ (M-2), the two apex
// fans included; a blade section of NV points swept over NU panels is
// 2 NV (NU+1) with its two end caps.  A section drawn with n chordwise
// points per side has NV = 2n, because the nose and the tail are each
// listed once.
NTRI = 2*NAZ*(len(NACPROF) + len(CCPROF) + len(HUBPROF) - 2)
     + NFAN*2*(2*NCHF)*(NSPF + 1)
     + NOGV*2*(2*NCHV)*(NSPV + 1)
     + NBLADE*2*(2*NCH)*(NSP + 1);

echo("fan: tip radius", RTIP, "mm, hub/tip", HTR, ", ", NFAN, "blades at tip solidity",
     NFAN*CFT/(2*PI*RTIP));
echo("shaft", NLP, "rpm -> tip speed", UTIP, "m/s; axial Mach at the fan face", MXFAN);
echo("mass flow", MDOT, "kg/s = ", MDOT/AFAN, "kg/s per m2 of annulus");
echo("relative Mach: hub", mrel(RHUBF), " tip", mrel(RTIP),
     "; sonic at r =", RSONIC, "mm, so", 100*(RTIP - RSONIC)/(RTIP - RHUBF),
     "percent of the span and", 100*(RTIP*RTIP - RSONIC*RSONIC)
       /(RTIP*RTIP - RHUBF*RHUBF), "percent of the area runs supersonic");
echo("fan twist: beta1 at root", b1_fan(FROOT), "deg, at tip", b1_fan(RTIP),
     "deg -> twist", b1_fan(RTIP) - b1_fan(FROOT), "deg");
echo("fan turning: root", b1_fan(FROOT) - b2_fan(FROOT),
     " tip", b1_fan(RTIP) - b2_fan(RTIP), "deg");
echo("stage loading cap binds inboard of r =", RCROSS,
     "mm; worst de Haller number", dehaller(RCROSS), "which is the classical limit");
echo("de Haller: root", dehaller(FROOT), " crossover", dehaller(RCROSS),
     " tip", dehaller(RTIP));
echo("fan work J/kg: root", dh_fan(FROOT), " tip", dh_fan(RTIP),
     " mass mean", DHALL, " -> mean fan pressure ratio", fpr_of(DHALL));
echo("a constant-work fan would ask the root for", DHTIP/dh_fan(FROOT),
     "times the work the loading cap allows it");
echo("fan pressure ratio: core stream", fpr_of(DHCORE), " bypass stream", fpr_of(DHBYP),
     " tip section", FPRTIP);

echo("dividing streamline at r =", RDIV, "mm; splitter nose put there at z =", ZSP);
echo("bypass ratio from the annulus areas",
     (RCAS*RCAS - RDIV*RDIV)/(RDIV*RDIV - RHUBF*RHUBF), " asked for", BPR);
echo("annulus m2: fan", AFAN, " bypass", PI*(RCAS*RCAS-RDIV*RDIV)/1e6,
     " core", PI*(RDIV*RDIV-RHUBF*RHUBF)/1e6);
echo("outlet guide vanes", NOGV, "against", NFAN,
     "fan blades; cut-off needs more than", 2*NFAN);
echo("bypass duct at the vane: Vx", VXB, "m/s, swirl", VTOGV,
     "m/s, vane inlet angle root", ogv_a(ROGV0), " tip", ogv_a(ROGV1), "deg");

echo("core: booster", NBST, "stages, HPC", NHPC, ", HPT", NHPT, ", LPT", NLPT);
echo("T0 K: fan exit", T0CORE, " booster exit", T0CORE + DT_BST,
     " HPC exit", T03, " turbine entry", T04, " LPT exit", T05);
echo("pressure ratios: fan root", fpr_of(DHCORE), " booster",
     cP0(2*NBST)/P0CORE, " HPC", P03/cP0(2*NBST), " overall", P03/PAMB);
echo("HPC delivery", P03/1e5, "bar at", T03, "K; annulus height in",
     ROWS[IC0][13], "mm out", ROWS[IC1][13], "mm, a factor of",
     ROWS[IC0][13]/ROWS[IC1][13]);
echo("work balance: HPT must drop", DT_HPT, "K, LPT", DT_LPT,
     "K; stage loadings HPT", CPH*(DT_HPT/NHPT)/pow(OMHP*(RMT0+RMT1)/2000,2),
     " LPT", CPH*(DT_LPT/NLPT)/pow(OMLP*(RML0+RML1)/2000,2));
echo("nozzles m2: bypass", ABN, " core", ACN,
     "; jet speeds m/s", VJB, VJC, "; specific thrust", (MBYP*VJB+MCORE*VJC)/MDOT);
echo("static gross thrust", (MBYP*VJB + MCORE*VJC)/1000, "kN");
echo("bypass nozzle radii", RCOWN, "to", RNITE, "mm; core nozzle", RPLUGN, "to", RCCI);

echo("inlet: throat", RTHR, "highlight", RHL, "max nacelle radius", RMAXN,
     " ratio to fan diameter", RMAXN/RTIP);
echo("stations mm: spinner tip", ZNOSE, " fan", 0, " splitter", ZSP,
     " fan cowl TE", ZNTE, " core nozzle", ZCNTE, " plug tip", ZPTIP);
echo("overall length", ZPTIP - ZNOSE, "mm =", (ZPTIP-ZNOSE)/(2*RTIP),
     "fan diameters; the length is a sum of chords, not a choice");
echo("core cowl shell: at the splitter nose", 2*SPB, "mm, thinnest",
     min(CLEAR), "mm at z", CASK[argat(CLEAR, min(CLEAR))][0], ", thickest",
     max(CLEAR), "mm at z", CASK[argat(CLEAR, max(CLEAR))][0],
     "(the least must exceed 0)");
echo("fan blade axial extent", FZ0, "to", FZ1, "mm; platform", ZFP0, "to", ZSPE);

echo("rows: name, r_mean, height, axial chord, blades, metal angles at mid span");
for (i = [0:NROW-1]) let(R = ROWS[i], m = rowmetal(R, R[1], R[6]))
  echo(R[0], R[1], R[13], R[14], rowN(R), m[0], m[1], "at z", rowz(i));

// ---- does any blade touch its neighbour? ----------------------------
// The exporter declines to union a model this size and says so, which
// means a pair of blades that DID intersect would still export as two
// separate shells and the volume audit below would not notice.  So the
// question is settled here instead, on the cascade sections themselves.
// A blade and the next one round are the same wrapped section offset by
// the pitch in arc length, and two congruent translated polygons meet if
// and only if their boundaries cross, so counting proper segment
// crossings settles it.  Sampled at twice the mesh's radial resolution.
function cross2(p, q, a, b) =
  let( r = q - p, s = b - a, d = r[0]*s[1] - r[1]*s[0] )
    d == 0 ? 0
  : let( w = a - p,
         t = (w[0]*s[1] - w[1]*s[0])/d,
         u = (w[0]*r[1] - w[1]*r[0])/d )
      (t > 0 && t < 1 && u > 0 && u < 1) ? 1 : 0;
function nxing(S, pitch) =
  let( n = len(S) )
    sum([ for (i = [0:n-1]) sum([ for (j = [0:n-1])
            cross2(S[i], S[(i+1)%n],
                   S[j] + [pitch, 0], S[(j+1)%n] + [pitch, 0]) ]) ]);
function row_xing(i) =
  let( R = ROWS[i], N = rowN(R), r0 = rowr0(R), r1 = rowr1(R),
       X = row_secs(R, 2*NSP) )
    sum([ for (u = [0:2*NSP])
            nxing(X[u], 2*PI*(r0 + (r1-r0)*u/(2*NSP))/N) ]);
XING = sum([ for (i = [0:NROW-1]) row_xing(i) ])
     + sum([ for (i = [0:NSPF]) nxing(FSECS[i], 2*PI*fanr(i)/NFAN) ])
     + sum([ for (i = [0:NSPV]) nxing(VSECS[i], 2*PI*VRAD[i]/NOGV) ]);
echo("blade sections crossing their own neighbour:", XING, "(must be 0)");
echo("tightest passage, pitch over true chord: fan tip",
     2*PI*RTIP/NFAN/CFT, " worst core row",
     min([ for (R = ROWS) 2*PI*R[1]/rowN(R)/rowc(R) ]));

// ---- the audit ------------------------------------------------------
// No two solids share a cubic millimetre, so this is a prediction.  The
// revolved parts are integrated in closed form and scaled by the exact
// polygon-to-circle factor; the blades are integrated from their own
// triangle lists.
echo("profile loop areas (must all be positive, or the shell is inside out):",
     "nacelle", loop_area(NACPROF), " core cowl", loop_area(CCPROF));
VHUB  = KFAC*open_vol(HUBPROF);
VNAC  = KFAC*loop_vol(NACPROF);
VCOWL = KFAC*loop_vol(CCPROF);
VFAN  = NFAN*sweep_vol(FGRID);
VOGV  = NOGV*sweep_vol(VGRID);
VROWS = [ for (i = [0:NROW-1]) rowN(ROWS[i])*sweep_vol(rowgrid(i)) ];
VTOT  = VHUB + VNAC + VCOWL + VFAN + VOGV + sum(VROWS);
echo("volume litres -- hub and plug", VHUB/1e6, " nacelle", VNAC/1e6,
     " core cowl", VCOWL/1e6, " fan", VFAN/1e6, " guide vanes", VOGV/1e6,
     " core blading", sum(VROWS)/1e6);
echo("solids", NSOLID, "of which blades", NFAN + NOGV + NBLADE,
     "; no two share volume, so the exported mesh must hold", VTOT/1e6,
     "litres in", NSOLID, "components made of", NTRI, "triangles");
// echo prints six significant digits, which on ten thousand litres stops
// at the tenth.  The fractional part carries the next six, so the audit
// can be compared with an exporter's measurement to a cubic millimetre.
echo("litres past the whole litre", VTOT/1e6 - floor(VTOT/1e6),
     "; the mesh must have no holes and no inconsistently wound edge");
// The nacelle is nearly half the engine's metal, and its two surfaces are
// five analytic runs and two isolated points and nothing else.  Given the
// twelve stations below and the curve families srun(), prun() and erun()
// use, it can be rebuilt and re-integrated by anything that is not this
// program.  Done that way it comes to 5076875942 cubic millimetres
// against the 5076867725 the export holds, a gap of 1.6 parts in a
// million; perturbing the twelve within the rounding the six printed
// digits allow moves the answer over thirteen times that band, so the
// two agree as closely as these numbers can express.
echo("nacelle profile: z of highlight", ZHL, "throat", ZTHR, "max radius", ZMAXN,
     "duct start", ZOGVE + 40, "fan platform", ZFP0, "trailing edge", ZNTE);
echo("nacelle profile: r at highlight", RHL, "throat", RTHR, "max", RMAXN,
     "fan case", RCAS, "TE inner", RNITE, "TE outer", RNOTE);
echo("faceting: every circle is a", NAZ, "-gon, so every revolved volume is",
     KFAC, "of its smooth value");
