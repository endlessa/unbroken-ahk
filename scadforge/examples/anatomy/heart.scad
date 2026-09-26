// ===================================================================
//  The human heart -- four chambers and the great vessels
//
//  A WARNING ABOUT WHAT THIS IS.  The left ventricle in lv_fibres.scad
//  is DERIVED: three measurements go into prolate spheroidal
//  coordinates and the wall thickness, the cavity volume and the
//  myocardial mass come out, unasked.  This model is not that.  A whole
//  heart has no generative description -- it is the residue of a tube
//  that loops, septates and twists -- so what follows is SCULPTED from
//  anatomical knowledge to ordinary adult dimensions.  It is accurate
//  the way a good textbook plate is accurate, not the way a scan is.
//
//  What IS carried over from the derived model is the ventricular core.
//  The left ventricle is still the shell between two confocal prolate
//  spheroids, so its wall is still thick at the equator and thin at the
//  apex for the reason it is in a real heart, and its cavity still
//  comes out at an ordinary end-diastolic volume.
//
//  The right ventricle is built the way the anatomy is built: NOT as a
//  chamber of its own, but as a free wall draped over the left one.
//  The septum is not a separate object here because it is not a
//  separate object in a heart -- the interventricular septum is part of
//  the left ventricular mass, which is why it bulges INTO the right
//  ventricle and why the right ventricle is a crescent in section
//  rather than a circle.  Its free wall is a third the thickness of the
//  left's, because it pumps against a sixth the pressure.
//
//  Everything is additive.  A chamber wall is the region between two
//  surfaces, and that is one closed polyhedron -- no subtraction is
//  needed to hollow anything.  The atria are cups sitting mouth-down on
//  the ventricular base, so each atrium opens into the ventricle below
//  it: those openings are the mitral and tricuspid orifices, and they
//  are holes in the model for the same reason they are holes in a
//  heart.
//
//  Orientation is anatomical: +x to the patient's LEFT, +y POSTERIOR,
//  +z SUPERIOR.  The ventricular mass is built about its own long axis
//  and then laid into the chest at the angle a heart sits at, apex
//  forward-left-down.
// ===================================================================

// ---- the ventricular core, as in lv_fibres.scad ---------------------
LCAV = 80;         // LV cavity long axis, mm
DCAV = 48;         // LV cavity diameter at its widest, mm
WALL = 10;         // LV wall at the equator, mm
MUB  = 120;        // base truncation, degrees

NM   = 40;         // stations along a meridian
NT   = 68;         // panels around

// ---- the right ventricle --------------------------------------------
RV_A   = -88;      // free wall spans this much of the way round the LV,
RV_B   =  88;      // in degrees either side of the anterior midline
RV_T   = 4.2;      // free wall thickness at its deepest, mm
                   // (the LV's is 10 -- a sixth of the pressure)
RV_BUL = 23;       // how far it stands off the LV at its deepest, mm
RV_MU0 = 46;       // it stops short of the apex, at this M
RV_SINK = 3.0;     // how far its edges are buried in the LV wall, mm
NRM    = 26;       // stations up the RV
NRT    = 40;       // panels across it

// ---- atria -----------------------------------------------------------
LA_C = [-17, 19,  0];   LA_R = [26, 22, 15];   // centre in x,y; radii
RA_C = [ 25,  1,  0];   RA_R = [26, 23, 16];
AT_W = 2.6;             // atrial wall, mm -- thinner again than the RV
NA   = 34;

// ---- great vessels ----------------------------------------------------
AO_R  = 15;        // aortic root radius, mm
PT_R  = 14;        // pulmonary trunk
SVC_R = 10;
IVC_R = 12;
PV_R  = 6;         // pulmonary veins
NV    = 18;        // panels round a vessel

TILT  = 38;        // how far the long axis leans from vertical, degrees
YAW   = 26;        // and how far it turns toward the front

MYO  = [0.70, 0.30, 0.28];   // left ventricle
RVC  = [0.86, 0.50, 0.42];   // right ventricle, a shade lighter
ATR  = [0.62, 0.42, 0.46];   // atria
ART  = [0.84, 0.40, 0.36];   // arteries
VEN  = [0.38, 0.45, 0.62];   // veins

// Faces are wound so the RIGHT-HAND normal points INTO the solid, which
// is the convention polyhedron() takes: a face is listed clockwise as
// seen from outside. Wound the other way the solid is inside-out --
// which renders identically, because shading uses |n|, and then quietly
// ruins every boolean it touches. It cost three models before it showed.

// ---- helpers ---------------------------------------------------------
function sinh_(x)  = (exp(x) - exp(-x))/2;
function cosh_(x)  = (exp(x) + exp(-x))/2;
function atanh_(x) = 0.5*ln((1+x)/(1-x));
function asinh_(x) = ln(x + sqrt(x*x + 1));
function unit(v)   = v/norm(v);

CZ    = LCAV/(1 - cos(MUB));
LAM_E = atanh_((DCAV/2) / CZ);
D     = CZ / cosh_(LAM_E);
LAM_P = asinh_(sinh_(LAM_E) + WALL/D);
ZBASE = -D*cosh_(LAM_E)*cos(MUB);
function mu_max(lam) = acos(-ZBASE / (D*cosh_(lam)));
function rr(lam, mu) = D*sinh_(lam)*sin(mu);
function zz(lam, mu) = -D*cosh_(lam)*cos(mu);

MP = mu_max(LAM_P);
ME = mu_max(LAM_E);

// A ring grid closed with a fan at each end.
module shell(grid, conv = 8) {
    MU_ = len(grid) - 1;
    MV  = len(grid[0]);
    ctr = function (p) [ (p[0][0]+p[floor(MV/3)][0]+p[floor(2*MV/3)][0])/3,
                         (p[0][1]+p[floor(MV/3)][1]+p[floor(2*MV/3)][1])/3,
                         (p[0][2]+p[floor(MV/3)][2]+p[floor(2*MV/3)][2])/3 ];
    pts = concat([ for (u = [0:MU_]) each grid[u] ],
                 [ ctr(grid[0]) ], [ ctr(grid[MU_]) ]);
    B0 = (MU_+1)*MV; B1 = B0 + 1;
    polyhedron(points = pts,
      faces = concat(
        [ for (u = [0:MU_-1]) for (v = [0:MV-1])
            [ u*MV+v, (u+1)*MV+v, (u+1)*MV+(v+1)%MV, u*MV+(v+1)%MV ] ],
        [ for (v = [0:MV-1]) [ B0, (v+1)%MV, v ] ],
        [ for (v = [0:MV-1]) [ B1, MU_*MV + v, MU_*MV + (v+1)%MV ] ]),
      convexity = conv);
}

// A stack of CRESCENT rings, closed with a flat strip at each end.
//
// The generic `shell` above closes a ring with a fan to the ring's
// average point, which is right for a circle and meaningless for a
// crescent: the average of a crescent's boundary lies OUTSIDE the
// crescent, so the fan folds over itself, and the solid comes out
// inside-out -- signed volume -13 mL rather than +13, after which its
// union with the left ventricle collapsed to nothing at all, silently.
// A crescent's end is an annular strip between its two arcs, and that is
// what is built here.
//
// Each ring is 2M points: M along the outer arc with theta increasing,
// then M along the inner arc coming back, so the point facing `j` across
// the wall is `2M-1-j`.
module crescent(rings, conv = 8) {
    NU_ = len(rings) - 1;
    MV  = len(rings[0]);
    M   = MV/2;
    pts = [ for (u = [0:NU_]) each rings[u] ];
    LAST = NU_*MV;
    polyhedron(points = pts,
      faces = concat(
        [ for (u = [0:NU_-1]) for (v = [0:MV-1])
            [ u*MV+v, (u+1)*MV+v, (u+1)*MV+(v+1)%MV, u*MV+(v+1)%MV ] ],
        [ for (j = [0:M-2]) [ j, j+1, MV-2-j, MV-1-j ] ],
        [ for (j = [0:M-2]) [ LAST+j, LAST+MV-1-j, LAST+MV-2-j, LAST+j+1 ] ]),
      convexity = conv);
}

// A solid of revolution whose profile starts and ends on the axis.
module revolve(prof, n, conv = 8) {
    M = len(prof);
    pts = concat(
      [ [0, 0, prof[0][1]] ],
      [ for (i = [1 : M-2]) each
          [ for (j = [0:n-1]) let(a = 360*j/n)
              [ prof[i][0]*cos(a), prof[i][0]*sin(a), prof[i][1] ] ] ],
      [ [0, 0, prof[M-1][1]] ]);
    TOP = 1 + (M-2)*n;
    polyhedron(points = pts,
      faces = concat(
        [ for (j = [0:n-1]) [ 0, 1 + j, 1 + (j+1)%n ] ],
        [ for (i = [1 : M-3]) for (j = [0:n-1])
            [ 1+(i-1)*n+j, 1+i*n+j, 1+i*n+(j+1)%n, 1+(i-1)*n+(j+1)%n ] ],
        [ for (j = [0:n-1]) [ TOP, 1+(M-3)*n+(j+1)%n, 1+(M-3)*n+j ] ]),
      convexity = conv);
}

// A tube swept along a polyline.  `ref` is any direction not parallel
// to the path, used to raise a frame; each vessel is planar, so its
// plane normal serves.
function tube_ring(pts, i, r, ref, n) =
  let( M = len(pts),
       a = pts[max(0, i-1)], b = pts[min(M-1, i+1)],
       t = unit(b - a), u = unit(cross(ref, t)), v = cross(t, u) )
  [ for (k = [0:n-1]) let(g = 360*k/n) pts[i] + r*(cos(g)*u + sin(g)*v) ];

module tube_path(pts, r, ref = [1,0,0], n = 18) {
    shell([ for (i = [0:len(pts)-1]) tube_ring(pts, i, r, ref, n) ]);
}

// ---- left ventricle ---------------------------------------------------
// Up the epicardium, across the base, down the endocardium.  Both ends
// land on the axis, so revolving the profile closes the solid.
LV_PROFILE = concat(
    [ for (i = [0:NM])    [ rr(LAM_P, MP*i/NM), zz(LAM_P, MP*i/NM) ] ],
    [ for (i = [NM:-1:0]) [ rr(LAM_E, ME*i/NM), zz(LAM_E, ME*i/NM) ] ]);

// ---- right ventricle --------------------------------------------------
// A crescent in section: the inner face rides on the LV epicardium,
// standing off it by a bulge that dies to nothing at the two
// interventricular grooves.  That is why the chamber is a crescent and
// not a circle -- the septum it shares is convex toward it.
// The bulge dies to nothing at both interventricular grooves and fades
// out toward the apex, and the WALL THICKNESS dies with it.  That is the
// whole trick: where the bulge is zero the outer face has come back down
// onto the LV epicardium, so the two solids meet flush and the union
// reads as one heart with a chamber pushed into its flank -- rather than
// as a separate shell draped over a cone with a hard edge all round it.
function smoothstep(x) = let( t = x <= 0 ? 0 : x >= 1 ? 1 : x ) t*t*(3 - 2*t);

function rv_span(mu) = smoothstep((mu - RV_MU0)/(0.45*(MP - RV_MU0)));
function rv_shape(mu, th) =
  let( f = (th - RV_A)/(RV_B - RV_A) )
    rv_span(mu) * sin(180*f)*sin(180*f);

// The free wall is SUNK into the left ventricle at its edges rather than
// laid exactly on it.  A shell whose inner face lies exactly on another
// surface is a tangency, not an overlap, and the two surfaces here are
// tessellated differently -- 68 panels round the left ventricle against
// 40 across the right -- so they interleave in a fine sawtooth that no
// boolean can resolve.  Left tangent, the union of the two came out at
// 0.1 mL instead of 180, silently.  Buried three millimetres, they
// genuinely overlap and the union is ordinary.  It is also the truer
// statement: the right ventricle's free wall is attached to the left's,
// not resting on it.
function rv_ring(mu) =
  let( rb = rr(LAM_P, mu) - RV_SINK, z = zz(LAM_P, mu) )
  concat(
    [ for (j = [0:NRT]) let( th = RV_A + (RV_B - RV_A)*j/NRT,
                             sh = rv_shape(mu, th),
                             q  = rb + (RV_BUL + RV_SINK)*sh + RV_T*(0.25 + 0.75*sh) )
        [ q*cos(th), q*sin(th), z ] ],
    [ for (j = [NRT:-1:0]) let( th = RV_A + (RV_B - RV_A)*j/NRT,
                                q  = rb + (RV_BUL + RV_SINK)*rv_shape(mu, th) )
        [ q*cos(th), q*sin(th), z ] ]);

// ---- an atrium: a cup, mouth down ------------------------------------
// The mouth is the atrioventricular orifice.  It is a hole in the model
// because it is a hole in a heart: each atrium opens straight into the
// ventricle beneath it.
//
// The sweep stops at AMAX, short of the equator-and-round -- taken all
// the way to 180 the profile closes into a hollow sphere with no mouth
// at all, and pinches on the axis at both poles.
AMAX = 132;
// Traced INSIDE first, from the dome down to the rim, then back up the
// outside -- the same sense in which the ventricle's profile is traced.
// Written the other way round the cup revolves inside-out, and takes the
// union down with it.
function cup_profile(R, wall, n) = concat(
    [ for (i = [0:n])    let( a = AMAX*i/n ) [ (R-wall)*sin(a), (R-wall)*cos(a) ] ],
    [ for (i = [n:-1:0]) let( a = AMAX*i/n ) [ R*sin(a), R*cos(a) ] ]);

// Placed so the RIM lands on the given plane, whatever the dome's size.
module atrium(c, rad, wall, zrim, n) {
    translate([c[0], c[1], zrim - rad[2]*cos(AMAX)])
      scale([rad[0]/rad[2], rad[1]/rad[2], 1])
        revolve(cup_profile(rad[2], wall, n), NA);
}

// ---- assembly ---------------------------------------------------------
rotate([0, 0, YAW]) rotate([TILT, 0, 0]) {

    color(MYO) revolve(LV_PROFILE, NT);

    color(RVC) crescent([ for (i = [0:NRM])
        rv_ring(RV_MU0 + (MP - RV_MU0)*i/NRM) ]);

    // Atria, sitting on the base plane.
    // Sunk below the base plane for the same reason the right ventricle
    // is sunk into the left: a rim sitting exactly ON a plane is tangent
    // to it, and tangency is what the boolean cannot resolve.
    color(ATR) atrium(LA_C, LA_R, AT_W, ZBASE - 4, 26);
    color(ATR) atrium(RA_C, RA_R, AT_W, ZBASE - 4, 26);

    // Aorta: out of the left ventricle, straight up, then over to the
    // left and backwards as the arch, with its three branches.
    AO_RT = 25;                            // radius of the arch itself
    AO_TOP = ZBASE + 34;                   // where the ascending part ends
    AOP = concat(
      [ for (i=[0:5]) [ 5, 7, ZBASE - 6 + (AO_TOP - ZBASE + 6)*i/5 ] ],
      [ for (i=[1:10]) let( a = 165*i/10 )
          [ 5 - AO_RT*(1 - cos(a)), 7 + 15*(1 - cos(a))/2, AO_TOP + AO_RT*sin(a) ] ]);
    color(ART) tube_path(AOP, AO_R, [0,1,0], NV);
    // brachiocephalic, left common carotid, left subclavian
    for (k = [0:2])
      color(ART) tube_path(
        [ for (i=[0:3]) let( a = 42 + 30*k )
            [ 5 - AO_RT*(1-cos(a)), 7 + 15*(1-cos(a))/2 + 2*i,
              AO_TOP + AO_RT*sin(a) + 7*i ] ], 4.6, [1,0,0], 12);

    // Pulmonary trunk: out of the right ventricle, ANTERIOR to the aorta
    // and to its left, then dividing.  The two vessels cross because
    // their outflow tracts spiral around one another as they form.
    PT_TOP = ZBASE + 26;
    PTP = [ for (i=[0:6]) [ -3 - 2.0*i, -12 + 2.2*i, ZBASE - 4 + (PT_TOP - ZBASE + 4)*i/6 ] ];
    color(ART) tube_path(PTP, PT_R, [0,1,0], NV);
    for (sx = [-1, 1])                     // right and left pulmonary arteries
      color(ART) tube_path(
        [ for (i=[0:4]) [ -15 + sx*9*i, 1 + 3*i, PT_TOP + 3*i ] ], 8.5, [0,0,1], 14);

    // Caval veins into the right atrium, from above and below.
    color(VEN) tube_path([ for (i=[0:5]) [ 30, 6, ZBASE + 20 + 8*i ] ], SVC_R, [0,1,0], NV);
    color(VEN) tube_path([ for (i=[0:4]) [ 28, 10, ZBASE - 2 - 9*i ] ], IVC_R, [0,1,0], NV);

    // Four pulmonary veins into the left atrium, two a side, from behind.
    for (sx = [-1, 1]) for (sz = [-1, 1])
      color(VEN) tube_path(
        [ for (i=[0:4]) [ -22 + sx*12, 20 + 7*i, ZBASE + 12 + sz*12 ] ],
        PV_R, [0,0,1], 14);
}

// ---- what is claimed, and what is measured ---------------------------
function cap_vol(lam, mb) =
  PI*D*D*D*sinh_(lam)*sinh_(lam)*cosh_(lam)
    * (2/3 - cos(mb) + cos(mb)*cos(mb)*cos(mb)/3);

echo("LV wall at equator", D*(sinh_(LAM_P) - sinh_(LAM_E)),
     " at apex", D*(cosh_(LAM_P) - cosh_(LAM_E)),
     " RV free wall", RV_T, "mm");
echo("LV cavity", cap_vol(LAM_E, ME)/1000, "mL",
     " LV myocardium", (cap_vol(LAM_P, MP) - cap_vol(LAM_E, ME))/1000*1.05, "g");
echo("RV free wall wraps", RV_B - RV_A, "degrees of the LV, standing off",
     RV_BUL, "mm at its deepest");
echo("DERIVED: the ventricular core.  SCULPTED: everything else.");
