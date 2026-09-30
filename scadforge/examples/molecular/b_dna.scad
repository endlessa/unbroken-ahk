// ===================================================================
//  A segment of B-form DNA, and why it has two different grooves
//
//  B-DNA is usually handed over as four numbers: 10.5 base pairs per
//  turn, a rise of 3.4 angstroms per base pair, a pitch of 34 angstroms,
//  a diameter of 20.  Three of those cannot all hold at once.  Pitch is
//  rise times base pairs per turn by definition, and 10.5 x 3.4 is 35.7.
//  The 34 belongs to the older fibre-diffraction model with exactly 10
//  base pairs per turn; 10.5 is what DNA in solution does.  This model
//  takes the rise and the helical repeat as given and lets the pitch fall
//  out of them, and echoes how far that puts it from the number usually
//  printed beside it.  Nothing below is allowed to use both.
//
//  ---- where the grooves come from ---------------------------------
//
//  Two identical helices of equal radius and pitch, lying on one
//  cylinder, are fixed by one further number: the angle DELTA by which
//  the second leads the first at any given height.  Draw a line up the
//  cylinder at fixed azimuth and walk it.  Strand A crosses once per
//  pitch.  Strand B reaches that azimuth at a height lower by DELTA/360
//  of a pitch, so relative to an A crossing the B crossings sit at
//
//      z_B = z_A + (1 - DELTA/360) P    (mod P)
//
//  and the two gaps along the line are
//
//      w_minor = (1 - DELTA/360) P,     w_major = (DELTA/360) P.
//
//  They sum to P.  That is an identity and not a fit, and it is the
//  whole of the groove geometry.  At DELTA = 180 the two gaps are equal,
//  both P/2, and the molecule is a ladder with two indistinguishable
//  furrows.  DNA is not built that way.  The two backbones subtend about
//  120 degrees of the axis on one side and 240 on the other, so
//  DELTA = 240, w_minor = P/3 and w_major = 2P/3.  Get that one angle
//  wrong and every other number here can still be right and the model is
//  still a ladder.
//
//  The measured widths are 12 and 22 angstroms, and they sum to 34.  So
//  the published pair is self-consistent only with the ten-base-pair
//  pitch; at 10.5 per turn the two grooves are obliged to add to 35.7
//  instead, and the measured ratio 12:22 would ask for DELTA = 232.9
//  rather than 240.  All three comparisons are echoed below rather than
//  reconciled, because the discrepancy is in the source numbers and not
//  in the construction.
//
//  Each gap is centred on a base-pair plane.  On the minor side the
//  bisector sits at azimuth theta - 60 from strand A, and the two nearest
//  backbone crossings on that line are at z_n -/+ P/6; on the major side
//  the bisector is at theta + 120 and the crossings are at z_n -/+ P/3.
//  So the grooves are not features added to the model.  They are what is
//  left of the cylinder once the two sweeps have been placed.
//
//  ---- where the base pair's size comes from ------------------------
//
//  A Watson-Crick pair is a rigid chemical object, and the distance
//  between the two glycosidic carbons that tie it to the two sugars is
//  about 10.5 angstroms whichever pair it is.  That is a fact about the
//  bases, independent of any helix.  Those two carbons are what subtend
//  the minor-groove angle at the axis, so they lie on a circle of radius
//
//      R_C = C1C1 / (2 sin(MINOR/2)) = 10.5 / (2 sin 60) = 6.062
//
//  and the measured C1' radius in B-DNA is 5.8 to 5.9.  The base pair's
//  long axis is therefore a chord of that circle, and because the chord
//  subtends only 120 degrees its midpoint lies R_C cos(60) = 3.031
//  angstroms off the axis, displaced towards the minor groove.  That
//  displacement is not a defect of the drawing.  It is why the minor
//  groove is shallow and narrow while the major groove is deep and wide:
//  the pair leans away from one and towards the other.
//
//  The slab drawn for each base runs from that chord outwards across the
//  axis towards the major groove.  Its far edge is the major groove's
//  floor, so the one free choice is the major groove depth, taken as the
//  measured 8.5 angstroms below the 10-angstrom envelope, putting the
//  edge at radius 1.5.  The minor groove's depth is then not a choice at
//  all: its floor is the chord, at radius 3.031, so the depth is
//  10 - 3.031 = 6.97 against a measured 7.5.  One input, one check.
//
//  Each base's slab stops 0.3 angstroms short of the pair's midpoint, so
//  the two of them leave 0.6 of air at the hydrogen-bonded edge, and
//  outwards each one ends on the C1' circle rather than running on to the
//  backbone.  The nearly 2 angstroms of air between a slab end and the
//  backbone tube is where the deoxyribose would be, and this model does
//  not draw sugars.
//
//  ---- propeller twist pays for itself in rise ----------------------
//
//  The two bases of a real pair are not coplanar.  They counter-rotate
//  about the pair's long axis by about 12 degrees between them, the
//  propeller twist, which is what lets each base stack better on its own
//  strand's neighbours than a flat pair could.  Modelling it costs
//  something measurable.  Rotating a slab of half-width HW and half-
//  thickness T/2 by PROP/2 about its long axis raises its highest corner
//  to HW sin(PROP/2) + (T/2) cos(PROP/2) above the pair's plane, and
//  consecutive pairs are only RISE apart.  So the thickness is not
//  chosen; it is what is left:
//
//      T = 2 (RISE/2 - AIR - HW sin(PROP/2)) / cos(PROP/2)
//
//  which comes out at 2.54 angstroms where a flat pair could have used
//  3.00.  The propeller eats 0.46 angstroms of stacking room, at the
//  ends of the long axis, which is exactly where real propeller twist
//  causes cross-strand clashes.
//
//  The sign convention for propeller twist is a labelling question this
//  model does not try to settle.  What is drawn is stated exactly: the
//  strand-A base is turned by +PROP/2 about the unit vector running from
//  the strand-A attachment to the strand-B attachment, and the strand-B
//  base by -PROP/2 about the same vector.  The dihedral between the two
//  base planes is PROP either way, and that is the geometry.
//
//  ---- nothing here shares volume with anything else ----------------
//
//  Every point of a backbone tube is within r_B of a helix of radius R_P,
//  so it lies at radius at least R_P - r_B = 8 from the axis.  Every
//  point of every slab lies at radius at most
//
//      sqrt( (|v_mid| + HW cos(PROP/2) + (T/2) sin(PROP/2))^2
//            + (C1C1/2)^2 ) = 6.12
//
//  taking the corner furthest out in both directions at once, which is a
//  bound and not a sample.  So slabs and tubes miss each other by 1.88
//  angstroms with no search required.  The two backbone centre lines
//  never come within 10.02 angstroms of each other, which is echoed and
//  happens across the minor groove, so the tubes clear by 8.02 of that.
//  Consecutive base pairs miss by 2 AIR = 0.4 by the construction of T,
//  and the two bases of one pair by the 0.6 left at the Watson-Crick
//  edge.  Rotation about the long axis cannot close that last gap because
//  it does not move points along the axis it turns about.
//
//  The point of all that is arithmetic rather than tidiness.  With no two
//  solids sharing volume the export is the mesh as written, face for
//  face, so the volume below is a PREDICTION: 42 boxes whose volumes are
//  exact, plus two tubes whose ideal volume is pi r^2 L because the first
//  moment of a centred section kills the curvature term, reduced by the
//  known factor (NC/2pi) sin(360/NC) for an inscribed NC-gon section.
//  What is left after that is one more inscribing loss, the chorded and
//  lofted centre line, and it is second order: measured on this geometry
//  at 6.110, 1.531, 0.383, 0.0957 and 0.0239 cubic angstroms for 60, 120,
//  240, 480 and 960 stations, a clean factor of four for every doubling
//  across a factor of sixteen.  So the prediction has two terms, both of
//  them inscribing losses, both of them signed the same way, and anything
//  outside them is a finding.
//
//  ---- winding ------------------------------------------------------
//
//  polyhedron() wants each face wound so the right-hand normal points
//  INTO the solid.  Wound the other way a mesh renders identically and
//  every boolean on it quietly loses geometry, and the only cheap witness
//  is the sign of the exported volume.  The test applied by hand to both
//  face lists here is the local one: two faces sharing an edge must
//  traverse it in opposite directions.  The tube's end caps were checked
//  against the wall quads beside them that way, edge by edge, and the
//  box's six faces against each other, all twelve edges.
//
//  ---- what is not claimed ------------------------------------------
//
//  No atoms, no sugars, no phosphate groups, no distinction between
//  purine and pyrimidine, no sequence, and no A-tract bending.  The two
//  strands are drawn as two right-handed helices of the same pitch; real
//  strands are antiparallel, but that is a chemical direction along the
//  backbone and not a geometric handedness, and nothing in a swept tube
//  can express it.  Base-pair tilt and roll are taken as zero, which for
//  B-DNA is close, and slide and x-displacement are folded into the one
//  quantity this model does use, the chord offset R_C cos(MINOR/2).
// ===================================================================

// ---- the measurements this model is allowed to use ------------------
BPT   = 10.5;     // base pairs per helical turn
RISE  = 3.4;      // axial rise per base pair, angstroms
DIAM  = 20;       // duplex diameter, angstroms
R_P   = 9;        // backbone (phosphate) helix radius, angstroms
MINOR = 120;      // angle the two backbones subtend at the axis on the
                  // minor-groove side, degrees
C1C1  = 10.5;     // glycosidic carbon separation across a pair, angstroms
DMAJ  = 8.5;      // measured major groove depth, angstroms
PROP  = 12;       // propeller twist between the two bases of a pair, deg
NBP   = 21;       // base pairs drawn.  21 is the smallest whole number of
                  // base pairs that is also a whole number of turns.

// ---- drawing tolerances ---------------------------------------------
AIR   = 0.20;     // air left between consecutive base pairs, each side
GAPWC = 0.60;     // air left at the hydrogen-bonded edge of a pair
NC    = 48;       // sides of the backbone tube's section
NU    = 120;      // stations along each backbone, over the whole sweep
                  // Those two set the mesh's volume deficit, which is
                  // 5529/NC^2 + 22000/NU^2 cubic angstroms on this
                  // geometry, and they also set the triangle count, which
                  // this kernel will only put through its export-time
                  // union below 25000.  Under that ceiling the deficit is
                  // smallest near NC = 54; 48 by 120 is near enough, at
                  // 23736 triangles and a deficit of 0.12 percent.

STRA = [0.86, 0.34, 0.30];
STRB = [0.28, 0.50, 0.78];
BASA = [0.95, 0.82, 0.42];
BASB = [0.58, 0.78, 0.50];

// ---- everything else follows ----------------------------------------
PITCH = BPT*RISE;             // 35.7, not 34
TWIST = 360/BPT;              // degrees of helix per base pair
DELTA = 360 - MINOR;          // azimuthal lead of strand B over strand A
ROUT  = DIAM/2;
R_B   = ROUT - R_P;           // tube radius that makes the envelope DIAM

WMINOR = PITCH*(1 - DELTA/360);
WMAJOR = PITCH*DELTA/360;

R_C  = C1C1/(2*sin(MINOR/2)); // radius of the C1' circle
WMIN = R_C*cos(MINOR/2);      // chord offset from the axis, minor side
WMAJ = ROUT - DMAJ;           // major-groove floor radius
HW   = (WMIN + WMAJ)/2;       // half-width of a base slab
VMID = (WMAJ - WMIN)/2;       // where the pair's long axis sits in v

// thickness is what the rise has left after the propeller
THK  = 2*(RISE/2 - AIR - HW*sin(PROP/2))/cos(PROP/2);

Z0   = -(NBP-1)*RISE/2;       // centre the segment on the origin
PHI0 = -180*(NBP-1)*RISE/PITCH - TWIST/2;   // sweep starts half a rise low
PHI1 = -PHI0;

function unit(v) = v/norm(v);

// ---- one backbone ----------------------------------------------------
// The helix, and a frame on it.  n is the outward radial direction, which
// is perpendicular to the tangent for any helix; b = t x n completes a
// right-handed triad (n, b, t).  With that handedness, listing the
// section by increasing angle and the wall quads as [u,v],[u+1,v],
// [u+1,v+1],[u,v+1] puts the right-hand normal inside, which is what
// polyhedron() asks for.  No transport is needed and there is no
// holonomy: the frame is a pointwise function of the azimuth.
function frame(ph, ph0) =
  let( a = ph + ph0,
       c = [ R_P*cos(a), R_P*sin(a), PITCH*ph/360 ],
       t = unit([ -2*PI*R_P*sin(a), 2*PI*R_P*cos(a), PITCH ]),
       n = [ cos(a), sin(a), 0 ] )
  [ c, n, cross(t, n) ];

// Walls emitted as triangles with the diagonal alternating by parity, so
// that neighbouring saddle quads bias their triangulation in opposite
// directions and the two biases cancel instead of accumulating.
module tube(rr, nc, fr, conv = 6) {
    nu = len(fr);
    pts = concat(
      [ for (u = [0:nu-1]) each
          [ for (k = [0:nc-1]) let( g = 360*k/nc )
              fr[u][0] + rr*(cos(g)*fr[u][1] + sin(g)*fr[u][2]) ] ],
      [ fr[0][0] ], [ fr[nu-1][0] ]);
    B0 = nu*nc; B1 = B0 + 1;
    polyhedron(
      points = pts,
      faces = concat(
        [ for (u = [0:nu-2]) for (k = [0:nc-1])
            let( a = u*nc + k,       b = (u+1)*nc + k,
                 c = (u+1)*nc + (k+1)%nc, d = u*nc + (k+1)%nc )
            each ((u + k)%2 == 0 ? [ [a,b,c], [a,c,d] ]
                                 : [ [a,b,d], [b,c,d] ]) ],
        [ for (k = [0:nc-1]) [ B0, k, (k+1)%nc ] ],
        [ for (k = [0:nc-1]) [ B1, (nu-1)*nc + (k+1)%nc, (nu-1)*nc + k ] ]),
      convexity = conv);
}

FRAMES = [ for (i = [0:NU]) PHI0 + i*(PHI1 - PHI0)/NU ];
color(STRA) tube(R_B, NC, [ for (p = FRAMES) frame(p, 0)     ], 4);
color(STRB) tube(R_B, NC, [ for (p = FRAMES) frame(p, DELTA) ], 4);

// ---- one base --------------------------------------------------------
// A box on a right-handed triad (ea, eb, ec) spanning [a0,a1] x [b0,b1] x
// [c0,c1] about p.  Vertex index is i*4 + j*2 + k over the three ranges.
// The six faces below were checked edge by edge: every one of the twelve
// edges is traversed in opposite directions by the two faces that share
// it, and each face's right-hand normal points inwards.
module slab(p, ea, eb, ec, a0, a1, b0, b1, c0, c1) {
    polyhedron(
      points = [ for (i = [0,1]) for (j = [0,1]) for (k = [0,1])
                   p + (i == 0 ? a0 : a1)*ea
                     + (j == 0 ? b0 : b1)*eb
                     + (k == 0 ? c0 : c1)*ec ],
      faces  = [ [0,2,3,1], [4,5,7,6], [0,1,5,4],
                 [2,6,7,3], [0,4,6,2], [1,3,7,5] ],
      convexity = 2);
}

// u runs from the strand-A attachment to the strand-B attachment; v is
// u x z, which points into the major groove; the pair's long axis is the
// line through v = VMID at the pair's own height.  Propeller turns the
// (v, z) pair about u and leaves u alone, which is why the Watson-Crick
// gap survives it.
function u_of(th)  = unit([ cos(th + DELTA) - cos(th),
                            sin(th + DELTA) - sin(th), 0 ]);
function v_of(th)  = cross(u_of(th), [0,0,1]);
function vrot(th, psi) = v_of(th)*cos(psi) - [0,0,1]*sin(psi);
function zrot(th, psi) = [0,0,1]*cos(psi) + v_of(th)*sin(psi);

module base_pair(n) {
    th = n*TWIST + PHI0 + TWIST/2;
    z  = Z0 + n*RISE;
    p  = [0, 0, z] + VMID*v_of(th);
    color(BASA)
      slab(p, vrot(th,  PROP/2), u_of(th), zrot(th,  PROP/2),
           -HW, HW, -C1C1/2, -GAPWC/2, -THK/2, THK/2);
    color(BASB)
      slab(p, vrot(th, -PROP/2), u_of(th), zrot(th, -PROP/2),
           -HW, HW, GAPWC/2, C1C1/2, -THK/2, THK/2);
}

for (n = [0:NBP-1]) base_pair(n);

// ---- what the geometry says ------------------------------------------
echo("B-DNA:", BPT, "bp per turn x", RISE, "angstrom rise =", PITCH,
     "angstrom pitch");
echo("the textbook 34 angstrom pitch belongs to the 10 bp model; this is",
     100*(PITCH/34 - 1), "percent above it");
echo("twist per base pair", TWIST, "degrees; drawn", NBP, "bp =",
     NBP*TWIST/360, "turns over", (NBP-1)*RISE, "angstroms of rise");
echo("backbone helix radius", R_P, "tube radius", R_B,
     "so the envelope diameter is", 2*(R_P + R_B));
echo("helix pitch angle from the horizontal",
     atan2(PITCH, 2*PI*R_P), "degrees");

echo("backbone azimuthal offset DELTA", DELTA,
     "degrees, minor sector", MINOR);
echo("groove widths along a cylinder generator, backbone centre to",
     "backbone centre: minor", WMINOR, "major", WMAJOR);
echo("measured 12 and 22, so the drawn minor is", 100*(WMINOR/12 - 1),
     "percent off and the major", 100*(WMAJOR/22 - 1), "percent off");
echo("but 12 and 22 sum to 34 and so belong to the 10 bp pitch, while",
     "any pair of grooves must sum to the pitch, here", PITCH);
echo("measured pair rescaled to this pitch", 12*PITCH/34, 22*PITCH/34,
     " and the DELTA that would give the measured 12:22 ratio here",
     360*22/34);
// the tube has width along that generator too, so the open gap between
// the two backbone surfaces is narrower than their centre-line spacing
TAX = R_B/cos(atan2(PITCH, 2*PI*R_P));
echo("subtracting the tube's own axial width", 2*TAX,
     "the grooves measured surface to surface are",
     WMINOR - 2*TAX, WMAJOR - 2*TAX);
echo("minor groove centred on the base-pair plane at azimuth theta -",
     180 - MINOR, "with crossings at +/-", PITCH/6,
     "; major at theta +", 180 - MINOR/2, "with crossings at +/-", PITCH/3);

echo("C1' circle radius from a", C1C1, "angstrom pair subtending", MINOR,
     "degrees:", R_C, " measured 5.8 to 5.9");
echo("pair long axis passes", WMIN, "angstroms off the helix axis,",
     "towards the minor groove");
echo("major groove depth", DMAJ, "was the input, so the floor sits at",
     WMAJ, "; the minor groove depth is then", ROUT - WMIN,
     "against a measured 7.5");
echo("base slab", C1C1/2 - GAPWC/2, "long,", 2*HW, "wide,", THK, "thick");
echo("a flat pair could have been", RISE - 2*AIR,
     "thick; the propeller spends", RISE - 2*AIR - THK, "of that");

// ---- clearances, so that the volume below is a prediction -------------
MAXR = sqrt(pow(abs(VMID) + HW*cos(PROP/2) + THK/2*sin(PROP/2), 2)
            + pow(C1C1/2, 2));
// closest approach of the two backbone centre lines, sampled
BBMIN = min([ for (i = [0:2000]) let( t = -360 + 360*i/1000 )
                norm([ R_P*cos(t + DELTA) - R_P,
                       R_P*sin(t + DELTA),
                       PITCH*t/360 ]) ]);
echo("no slab reaches past radius", MAXR,
     "and no tube comes inside radius", R_P - R_B,
     ": clearance", R_P - R_B - MAXR);
echo("the two backbone centre lines never come closer than", BBMIN,
     "so the tubes clear each other by", BBMIN - 2*R_B);
echo("consecutive base pairs clear by", 2*AIR,
     "and the two bases of a pair by", GAPWC);
echo("at the same height the two backbones are",
     2*R_P*sin(DELTA/2), "apart, centre to centre");

// ---- volume, predicted ------------------------------------------------
VSLAB = (C1C1/2 - GAPWC/2)*2*HW*THK;
LTUBE = (PHI1 - PHI0)/360*sqrt(pow(PITCH,2) + pow(2*PI*R_P,2));
VTUBE = PI*R_B*R_B*LTUBE;
SECF  = NC/(2*PI)*sin(360/NC);
echo("solids:", 2, "tubes and", 2*NBP, "base slabs =", 2 + 2*NBP);
echo("each tube sweeps", LTUBE, "angstroms of arc; ideal volume",
     VTUBE, "= pi r^2 L");
echo("each base slab", VSLAB, "exactly;", 2*NBP, "of them =", 2*NBP*VSLAB);
echo("ideal total", 2*NBP*VSLAB + 2*VTUBE, "cubic angstroms");
echo("an inscribed", NC, "-gon section keeps", SECF,
     "of a round tube, which brings that to",
     2*NBP*VSLAB + 2*SECF*VTUBE);
echo("the chorded centre line takes a further", 22000/pow(NU,2),
     "which is second order in the", NU, "stations, so the export",
     "should read", 2*NBP*VSLAB + 2*SECF*VTUBE - 22000/pow(NU,2));
echo("triangles expected", 2*(NU*NC*2 + NC*2) + 2*NBP*12,
     "which is under the 25000 this kernel will merge");
