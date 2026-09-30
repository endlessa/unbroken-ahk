// letterform.scad -- a type specimen plate, built to work the 2D pipeline.
//
// WHAT THIS IS
//
// A portrait plate, 124 by 168 by 6, carrying a raised panel and on that panel
// the five things a type specimen shows. The word TYPE at display size stands
// on a ledge grown from its own outline. Under it a ruler of four bars, each
// one exactly as long as the advance of the letter above it. Under that a
// line of small capitals set on a circular arc. Under that the letter A three
// times over, with the three offset joins applied to it at the same distance.
// A constructed capital O stands at the head of the plate as a turning,
// tapering column. Two footnotes run along the bottom margin.
//
// Everything is additive. There is no difference() and no intersection() in
// this file. The counter of the O is a hole because polygon() is handed two
// paths and the even-odd rule makes the inner one a hole, not because anything
// was cut out of anything.
//
// WHY EVERY PART STANDS CLEAR OF THE ONE BELOW IT
//
// Each layer begins 0.15 above the top of the layer under it, so no two solids
// in this model share any volume. The cost is a hairline of air beneath every
// raised element. What it buys is that the export runs no boolean at all and
// finishes in half a second, and that the exported mesh is exactly the mesh
// this file describes, solid for solid. The volume then stops being an
// estimate: the figure quoted below was computed from the font file and this
// geometry by a separate program before the model was run, and the export has
// to reproduce it. The alternative, resting the letters exactly on the panel,
// is the tangential contact a polygon kernel handles worst of anything.
//
// THE PLATE OUTLINE
//
// polygon() is handed a superellipse, |x/a|^n + |y/b|^n = 1, with a = 62,
// b = 84 and n = 4, sampled by polar angle. For each direction t the radius
// follows from the implicit equation:
//
//     r(t) = ( (|cos t|/a)^n + (|sin t|/b)^n ) ^ (-1/n)
//
// The textbook parameterisation, x = a sgn(cos t) |cos t|^(2/n) with y to
// match, was tried first and is useless at this exponent. For n = 4 the y
// coordinate goes as the square root of sin t near t = 0, so at 256 points the
// very first edge of the outline came out 13.2 long while the edges near the
// diagonals were under a tenth of that. The polar form has no such
// singularity. Its area by the shoelace sum is 19309.491 against the closed
// form 4ab*gamma(1+1/n)^2/gamma(1+2/n) = 19312.042, and the 0.013 percent
// shortfall is the inscribed 256-gon and nothing else.
//
// The panel is that same outline under offset(delta = -9), a straight-join
// inward offset. On a convex outline that is exactly the intersection of the
// inward-moved edge half-planes, so the script works the panel's area out for
// itself and echoes it, and the kernel's offset() has to agree.
//
// THE ADVANCE WIDTHS, AND THE RULER THAT PROVES THEM
//
// The bundled face is Instrument Sans Regular at 1000 units to the em, with
// hhea ascent 970, descent -250 and cap height 720. text(size = s) scales the
// em to s, so a capital stands 0.72*s tall and the ascent line sits at 0.97*s.
// The advance widths below were read out of the font's hmtx table in units of
// 1/1000 em. At size 30 the four letters of TYPE advance 19.44, 20.28, 19.68
// and 19.14, and the whole run is 78.54 wide.
//
// The ruler is four bars laid end to end, each exactly one letter's advance
// long, beginning at -78.54/2 because the word is set with halign = "center"
// and that is where its pen starts. Every joint between bars therefore falls
// on a glyph advance. Nothing in software asserts that; if the modeller's
// advance widths or its halign arithmetic were wrong by a tenth of a
// millimetre the bars would visibly walk out from under the letters they
// measure, and they do not.
//
// THE LEDGE, AND WHAT A MITER ACTUALLY REACHES
//
// The word stands on a ledge which is the same run under offset(delta = 1.2).
// A straight join was chosen so the acute apexes of the Y keep their spurs
// instead of being rounded away.
//
// The distance was picked against the measured side bearings. T leaves 36
// units to its right and Y takes 24 to its left, so at size 30 their ink is
// 1.80 apart; Y to P is 3.30 and P to E is 3.84. Half of any one of those
// closes a gap to exactly nothing, which is the case worth avoiding above all
// others, so 1.2 carries the first pair 0.60 past closing and leaves the other
// two, on those figures, 0.90 and 1.44 apart.
//
// Two of those three predictions hold and the middle one does not, and the
// reason is worth knowing. A mitered corner of turn angle q carries its apex
// d/cos(q/2) away from the vertex, not d. The tip of the Y's arm turns 124.7
// degrees, so at d = 1.2 its spur reaches 2.59 and it arrives at the P long
// before the two flanks would have met. The ledge comes out as two islands,
// T-Y-P joined into one and E standing 1.44 clear of them, and the
// perpendicular arithmetic never saw it coming. That arithmetic tells you when
// two FLANKS meet. It says nothing about when a spike arrives.
//
// THE THREE JOINS
//
// Along the foot of the panel the same A, at size 22, appears three times: on
// the left under offset(r = 1.8), in the middle under offset(delta = 1.8), on
// the right under offset(delta = 1.8, chamfer = true). One letter, one
// distance, three joins. The apexes are where they differ, and the measured
// areas come out 200.431, 204.716 and 195.416, the arc sitting between the
// spike and the flat cut exactly as it should.
//
// This A is also the shape that settles what offset() does with overlapping
// children. It is drawn as two slanted stems and a crossbar, three contours
// that overlap. A round offset is a Minkowski sum with a disc, and Minkowski
// sums distribute over union, so for r it makes no difference whether the
// pieces are merged first. A miter is not a Minkowski sum of anything and
// there it matters: offsetting each stem separately carries the slab of the
// buried inner edge outward with it, and that slab escapes past the true
// offset near the apex. At delta = 1.8 that is worth 0.42 percent of the
// area, 205.570 against 204.716. The kernel gives 204.716, the merged answer,
// which is what the language reference calls for when it says offset() unions
// its children first and then offsets the boundary of the union.
//
// THE CONSTRUCTED O
//
// The column is the Renaissance constructed capital O: an outer ellipse, and
// inside it a second ellipse turned off the vertical by the stress angle. The
// rotation is the whole trick. With the inner ellipse upright the ring would
// be symmetric and dead; turning it by 20 degrees makes the stroke thick on
// one pair of flanks and thin on the other, which is what gives a drawn O its
// life. Measuring the stroke radially, as the difference of the two polar
// radii in a common direction, it runs from 1.620 at 112 degrees to 5.282 at
// 25 degrees, a contrast of 3.3 to 1. The script computes both and echoes them
// with the directions they occur in.
//
// The ring is extruded 20 with twist = 25 and scale = 0.70, the only place in
// the file where both are used. linear_extrude turns the section about the Z
// axis THROUGH THE ORIGIN, not about the profile's own centre, so the O is
// built centred on the origin and the finished extrusion is translated into
// place. Built the other way round, at y = 46, a twist of 25 degrees would
// have swung the column 20 sideways along a helix instead of turning it where
// it stands. The section at height fraction u is the profile scaled by
// 1 + (0.70 - 1)u and then turned -25u degrees, so a smooth solid would hold
// ring_area * 20 * (1 + 0.7 + 0.49)/3 = 5116.829. The mesh holds 5119.798,
// which is 0.058 percent more, and the difference is the flat quads of the
// side walls standing slightly proud of the ruled surface they approximate.
// The audit uses the mesh figure, because that is what gets exported.
//
// The plinth under the column is the same ring under offset(r = 1.6). A round
// join on a 180-gon ellipse barely shows as a join, but the offset still has
// work to do: it grows the outer boundary and shrinks the counter, and the
// counter here is wide enough to take it with room over.
//
// ALIGNMENT IS BY METRICS, NOT BY INK
//
// The two footnotes make that visible. The left one is set valign = "bottom",
// the right one valign = "top", and they are placed so their baselines
// coincide. The shift needed between them is (ascent - descent)/1000 * size,
// which is 1.22 * size and has nothing to do with which letters either string
// contains. Both are capitals without descenders, so an ink-based alignment
// would have put them in quite different places.
//
// WHAT THE MODEL PRODUCES
//
// 33 disjoint solids falling into 54 connected pieces, since the word, the
// ledge, the ruler, the specimen line, the three A's and the two footnotes
// each break into several islands. 58002 triangles, and a volume of
// 148233.445 which was predicted in full before the model was ever run and
// which the export reproduces to eight significant figures. It takes about
// half a second, because nothing overlaps and so nothing has to be merged.
//
// One thing the export does NOT come out clean on, and it is the modeller's
// and not this file's. linear_extrude triangulates a cap by ear clipping when
// the profile is one simple contour, and by a sweep when it is anything else.
// The sweep splits boundary edges at its scan events, and the side walls are
// built from the unsplit contours, so cap and wall disagree about where the
// vertices along a boundary edge are. The result is a mesh full of
// T-junctions. This mesh has 38618 directed edges with no matching reverse.
// It is not leaking: a vertical ray fired through 157610 grid points of it
// enters and leaves the solid cleanly every time, and the volume is right to
// eight figures. But it will not pass any edge-pairing check, and every
// extruded letter in this file carries the problem, because every text()
// profile takes the sweep path. The smallest case is the language reference's
// own polygon example, a 20-square with a 10-square hole, which extrudes to
// 32 triangles carrying 32 unmatched edges.
//
// The tracking on the specimen line has a second job because of a second piece
// of exporter behaviour. Above 25000 triangles the export-time union is
// skipped, and whether it is skipped is decided by testing every pair of
// shells' bounding boxes. Set solid, the arc's letters are rotated enough that
// neighbouring boxes overlap even though the letters do not, and the export
// then prints a warning saying overlapping shells were left separate, which is
// not true of anything in this file. Opening the fit by 14 percent separates
// the boxes and the warning goes away. The merge that then runs is a no-op and
// produces the identical mesh, so either way the file is right; only the
// console differs.

$fn = 96;

// -- the bundled face, in units of 1/1000 em ---------------------------------
UPEM      = 1000;
ASCENT    =  970;
DESCENT   = -250;
CAP       =  720;
//                 A   B   C   D   E   F   G   H   I   J   K   L   M
ADV_UNITS = [    728,636,741,752,638,602,765,736,254,455,692,588,906,
//                 N   O   P   Q   R   S   T   U   V    W   X   Y   Z
                 736,786,656,787,656,608,648,712,728,1089,688,676,623];
SPACE_ADV = 200;
function adv(c) = c == " " ? SPACE_ADV : ADV_UNITS[ord(c) - 65];

function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0 : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

// -- the plate ---------------------------------------------------------------
PA = 62; PB = 84; PN = 4; PM = 256;
T_PLATE  = 6;
PANEL_IN = 9; T_PANEL = 1.4;
Z_PANEL  = T_PLATE + 0.15;
Z_TOP    = Z_PANEL + T_PANEL + 0.15;
AIR      = 0.15;

function se_r(t) = pow(pow(abs(cos(t))/PA, PN) + pow(abs(sin(t))/PB, PN), -1/PN);
PLATE = [ for (i = [0:PM-1]) let(t = 360*i/PM) se_r(t) * [cos(t), sin(t)] ];

function cross2(p, q) = p.x*q.y - p.y*q.x;
function shoelace(P) = sum([ for (i = [0:len(P)-1]) cross2(P[i], P[(i+1)%len(P)]) ]) / 2;

// The inward straight-join offset of a convex outline: move each edge in along
// its inward normal and intersect consecutive moved edges.
function inward(P, d, i) =
  let( n = len(P),
       a = P[(i+n-1)%n], b = P[i], c = P[(i+1)%n],
       u = (b - a)/norm(b - a), v = (c - b)/norm(c - b),
       nu = [-u.y, u.x] * d, nv = [-v.y, v.x] * d,
       den = cross2(u, v) )
    den == 0 ? b + nu : b + nu + u * (cross2(nv - nu, v) / den);
PANEL_PRED = [ for (i = [0:PM-1]) inward(PLATE, PANEL_IN, i) ];

// -- the word, its ledge and its ruler ---------------------------------------
WORD = "TYPE"; WSIZE = 30;
LEDGE_D = 1.2; T_LEDGE = 1.4; T_WORD = 4.4;
Y_WORD = 0;
WADV = [ for (i = [0:len(WORD)-1]) adv(WORD[i]) * WSIZE / UPEM ];
WRUN = sum(WADV);
// Left and right side bearings of the four letters, from the same outlines,
// in units of 1/1000 em.
SB   = [ [36,36], [24,24], [86,42], [86,70] ];
GAPS = [ for (i = [0:len(WORD)-2]) (SB[i][1] + SB[i+1][0]) * WSIZE / UPEM ];
// How far each gap is from closing under the ledge, measured perpendicular.
// This is a lower bound on the offset's reach and never an upper one: a
// mitered corner of turn q reaches d/cos(q/2), and the tip of the Y's arm
// turns 124.7 degrees and so reaches 2.16 times the distance asked for.
SLACK = [ for (g = GAPS) g - 2*LEDGE_D ];
assert(min([ for (s = SLACK) abs(s) ]) > 0.25,
       "a ledge gap closes to nothing; choose another LEDGE_D");

BAR_H = 2.6; BAR_IN = 0.3; T_BAR = 0.9; Y_BAR = -7.1;
BAR_X = [ for (i = [0:len(WORD)-1]) -WRUN/2 + sum(WADV, 0, i) ];

// -- the specimen line, set on an arc ----------------------------------------
ARC_S = "HAMBURGEFONSTIV"; ARC_SIZE = 7.5; ARC_R = 130; T_ARC = 2.2; Y_ARC = -18;
// Small sizes want looser fitting than display sizes, and setting on a curve
// wants it more: the tracking below opens each advance by 14 percent. It also
// keeps adjacent letters' bounding boxes apart, which the exporter cares about
// for a reason set out at the foot of this header.
TRACK = 1.14;
AADV  = [ for (i = [0:len(ARC_S)-1]) adv(ARC_S[i]) * ARC_SIZE / UPEM * TRACK ];
ARUN  = sum(AADV);
// arc length from the crown to the centre of letter i, and the angle that
// subtends at the centre of the circle
function arc_phi(i) = (-ARUN/2 + sum(AADV, 0, i) + AADV[i]/2) / ARC_R * 180 / PI;

// -- the three joins ---------------------------------------------------------
TRIO = "A"; TSIZE = 22; TD = 1.8; T_TRIO = 3; Y_TRIO = -45; X_TRIO = 34;

// -- the footnotes -----------------------------------------------------------
LSIZE = 5; T_LAB = 1.2; Y_LAB = -64; X_LAB = 41;
VSHIFT = (ASCENT - DESCENT) * LSIZE / UPEM;

// -- the constructed O -------------------------------------------------------
OX = 17; OY = 17.6; IX = 11.8; IY = 15.9; STRESS = 20; OM = 180;
Y_COL = 46; PLINTH_R = 1.6; T_PLINTH = 1.4;
COL_H = 20; COL_TWIST = 25; COL_SCALE = 0.70; COL_SLICES = 16;

function ell(t, a, b, rot) =
  let( p = [a*cos(t), b*sin(t)] )
    [ p.x*cos(rot) - p.y*sin(rot), p.x*sin(rot) + p.y*cos(rot) ];
O_OUT   = [ for (i = [0:OM-1])    ell(360*i/OM, OX, OY, 0) ];
O_IN    = [ for (i = [OM-1:-1:0]) ell(360*i/OM, IX, IY, STRESS) ];
O_PTS   = concat(O_OUT, O_IN);
O_PATHS = [ [ for (i = [0:OM-1]) i ], [ for (i = [0:OM-1]) OM + i ] ];

function ell_r(th, a, b) = a*b / sqrt(pow(b*cos(th), 2) + pow(a*sin(th), 2));
STROKE = [ for (i = [0:359]) ell_r(i, OX, OY) - ell_r(i - STRESS, IX, IY) ];
RING_A = shoelace(O_OUT) + shoelace(O_IN);

// -- what the script can work out for itself ---------------------------------
echo(face = "Instrument Sans Regular", upem = UPEM, ascent = ASCENT,
     descent = DESCENT, cap_height = CAP);
echo(word = WORD, size = WSIZE, advances_mm = WADV, run_mm = WRUN);
echo(ink_gaps_mm = GAPS, slack_under_ledge_mm = SLACK, miter_reach = LEDGE_D/cos(124.7/2));
echo(specimen = ARC_S, size = ARC_SIZE, run_mm = ARUN,
     arc_radius = ARC_R, arc_sweep_deg = ARUN / ARC_R * 180 / PI);
echo(plate_area = shoelace(PLATE), plate_volume = shoelace(PLATE) * T_PLATE,
     superellipse_closed_form = 19312.0418);
echo(panel_area_predicted = shoelace(PANEL_PRED),
     panel_volume_predicted = shoelace(PANEL_PRED) * T_PANEL);
echo(O_outer_area = shoelace(O_OUT), O_ring_area = RING_A,
     radial_stroke_min = min(STROKE), radial_stroke_max = max(STROKE),
     stroke_min_at_deg = search(min(STROKE), STROKE)[0],
     stroke_max_at_deg = search(max(STROKE), STROKE)[0]);
echo(column_volume_smooth = RING_A * COL_H * (1 + COL_SCALE + COL_SCALE*COL_SCALE) / 3,
     column_volume_mesh = 5119.798);
echo(total_volume_predicted = 148233.445);

// -- geometry ----------------------------------------------------------------

module plate() {
    linear_extrude(height = T_PLATE) polygon(PLATE);
}

module panel() {
    translate([0, 0, Z_PANEL])
        linear_extrude(height = T_PANEL)
            offset(delta = -PANEL_IN) polygon(PLATE);
}

module word_ledge() {
    translate([0, 0, Z_TOP])
        linear_extrude(height = T_LEDGE)
            offset(delta = LEDGE_D)
                translate([0, Y_WORD])
                    text(WORD, size = WSIZE, halign = "center", valign = "baseline");
}

module word() {
    translate([0, 0, Z_TOP + T_LEDGE + AIR])
        linear_extrude(height = T_WORD)
            translate([0, Y_WORD])
                text(WORD, size = WSIZE, halign = "center", valign = "baseline");
}

module ruler() {
    for (i = [0:len(WORD)-1])
        translate([BAR_X[i], Y_BAR - BAR_H/2, Z_TOP])
            linear_extrude(height = T_BAR)
                offset(delta = -BAR_IN, chamfer = true)
                    square([WADV[i], BAR_H]);
}

module specimen_line() {
    for (i = [0:len(ARC_S)-1])
        let( phi = arc_phi(i) )
        translate([ARC_R*sin(phi), Y_ARC - ARC_R + ARC_R*cos(phi), Z_TOP])
            rotate([0, 0, -phi])
                linear_extrude(height = T_ARC)
                    text(ARC_S[i], size = ARC_SIZE,
                         halign = "center", valign = "baseline");
}

module join_trio() {
    translate([-X_TRIO, Y_TRIO, Z_TOP])
        linear_extrude(height = T_TRIO)
            offset(r = TD)
                text(TRIO, size = TSIZE, halign = "center", valign = "center");
    translate([0, Y_TRIO, Z_TOP])
        linear_extrude(height = T_TRIO)
            offset(delta = TD)
                text(TRIO, size = TSIZE, halign = "center", valign = "center");
    translate([X_TRIO, Y_TRIO, Z_TOP])
        linear_extrude(height = T_TRIO)
            offset(delta = TD, chamfer = true)
                text(TRIO, size = TSIZE, halign = "center", valign = "center");
}

module footnotes() {
    translate([-X_LAB, Y_LAB, Z_TOP])
        linear_extrude(height = T_LAB)
            text("SPECIMEN", size = LSIZE, halign = "left", valign = "bottom");
    translate([X_LAB, Y_LAB + VSHIFT, Z_TOP])
        linear_extrude(height = T_LAB)
            text("INSTRUMENT SANS", size = LSIZE, halign = "right", valign = "top");
}

module plinth() {
    translate([0, Y_COL, Z_TOP])
        linear_extrude(height = T_PLINTH)
            offset(r = PLINTH_R)
                polygon(points = O_PTS, paths = O_PATHS);
}

// Built about the origin, because linear_extrude turns the section about the
// Z axis through the origin, and only then carried to its place on the panel.
module column() {
    translate([0, Y_COL, Z_TOP + T_PLINTH + AIR])
        linear_extrude(height = COL_H, twist = COL_TWIST,
                       scale = COL_SCALE, slices = COL_SLICES)
            polygon(points = O_PTS, paths = O_PATHS);
}

plate();
panel();
word_ledge();
word();
ruler();
specimen_line();
join_trio();
footnotes();
plinth();
column();
