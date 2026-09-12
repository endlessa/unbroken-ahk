// ===================================================================
//  gears.scad -- exact involute gearing for scadforge
//
//  Tooth flanks are true involutes of the base circle.  Roots are the
//  real TROCHOID swept by the generating rack's tip radius, computed as
//  the envelope of the rack-tip circle -- not a cosmetic fillet.  That
//  is the difference between a gear that looks like a gear and one that
//  would actually run: the trochoid is what a hobbing machine cuts, and
//  it is what sets the true root stress and the start of active profile.
//
//  Everything is additive.  A whole gear is ONE polygon, extruded once.
//  No difference(), no intersection(), anywhere in this file.
// ===================================================================

function rad(d) = d*PI/180;
function deg(r) = r*180/PI;
function invd(a) = deg(tan(a) - rad(a));      // involute function, degrees in/out

// ---- principal radii -----------------------------------------------
function g_rp(m,z)        = m*z/2;
function g_rb(m,z,al)     = g_rp(m,z)*cos(al);
function g_ra(m,z,x)      = g_rp(m,z) + m*(1 + x);
function g_rf(m,z,x)      = g_rp(m,z) - m*(1.25 - x);

// half tooth angle at the pitch circle, allowing for profile shift
function g_ht(z,al,x) = (90 + deg(2*x*tan(al)))/z;

// involute flank: polar angle offset from the tooth centreline at radius r
function g_phi(r,m,z,al,x) =
    let(rb = g_rb(m,z,al))
      g_ht(z,al,x) + invd(al) - invd(acos(min(0.999999, rb/max(r,rb))));

// ---- the generating rack's tip-arc centre ---------------------------
// Tangent to the rack tip line and to the rack flank; this is the point
// whose sweep envelope is the root trochoid.
function rk_y0(m,z,al,x,rho) = g_rp(m,z) - m*(1.25 - x) + rho;
function rk_u0(m,al,x,rho)   = PI*m/4 - (m*(1.25 - x) - rho)*tan(al) - rho/cos(al);

// Centre of the rack tip arc in the gear frame after the gear turns t.
// Rolling without slip ties the two together: when the gear turns by t the
// rack feeds by rp*t, so the SAME t must drive both the rotation and the
// slide. Driving them with opposite signs (easy to do, and wrong) wraps
// the root curve most of the way round the gear instead of tucking it
// into one tooth space.
function tro_u(t,m,z,al,x,rho) = rk_u0(m,al,x,rho) - g_rp(m,z)*rad(t);

function tro_c(t,m,z,al,x,rho) =
    let(u = tro_u(t,m,z,al,x,rho), y = rk_y0(m,z,al,x,rho))
      [ u*cos(t) + y*sin(t), -u*sin(t) + y*cos(t) ];

function tro_d(t,m,z,al,x,rho) =
    let(rp = g_rp(m,z), u = tro_u(t,m,z,al,x,rho), y = rk_y0(m,z,al,x,rho))
      [ (y - rp)*cos(t) - u*sin(t), (rp - y)*sin(t) - u*cos(t) ];

// envelope of the tip circle: the branch nearer the gear centre
function tro_p(t,m,z,al,x,rho) =
    let( c = tro_c(t,m,z,al,x,rho), d = tro_d(t,m,z,al,x,rho),
         n = [-d[1], d[0]]/norm(d), a = c + n*rho, b = c - n*rho )
      norm(a) < norm(b) ? a : b;

// ===================================================================
//  one half tooth pitch: space centreline (at 90 deg) out to the tooth
//  centreline.  Trochoid root, then involute flank, then half the tip.
// ===================================================================
NT = 24; NI = 20; NC = 6;
function half_pitch(m,z,al,x,rho) =
  let( rp = g_rp(m,z), rb = g_rb(m,z,al), ra = g_ra(m,z,x), rf = g_rf(m,z,x),
       tht = 90 - 180/z,
       rjoin = max(rb, rf) + 0.0015*m,
       tmax = 3.0*180/z,
       tro  = [ for (i=[0:NT]) tro_p(-tmax*i/NT, m,z,al,x,rho) ],
       traw = [ for (p = tro) if (norm(p) <= rjoin) p ],
       // Past about 42 teeth the base circle drops BELOW the root circle,
       // rjoin becomes rf, and the trochoid's own first point -- offset
       // tangentially by u0, so not quite at the bottom of the sweep --
       // lands a few thousandths outside it. The filter then comes back
       // EMPTY, trok[0] is undef, and the whole gear silently evaluates
       // to nothing. It is not an error: it means the fillet has shrunk
       // to nothing on a nearly-straight flank. Fall back to the root
       // circle on the space centreline so the cap still has a target.
       // Bevel teeth hit this every time, because they are cut on the
       // virtual gear of z/cos(gam) teeth, which is always the larger.
       trok = len(traw) > 0 ? traw : [ rf*[cos(90), sin(90)] ],
       rs   = max(rjoin, norm(trok[len(trok)-1])),
       inv  = [ for (i=[0:NI]) let(r = rs + (ra-rs)*pow(i/NI, 0.72))
                  r*[cos(tht + g_phi(r,m,z,al,x)), sin(tht + g_phi(r,m,z,al,x))] ],
       pa   = g_phi(ra,m,z,al,x),
       tip  = [ for (i=[1:NC]) let(a = tht + pa*(1 - i/NC)) ra*[cos(a), sin(a)] ],
       // The rack tip arc centre is offset tangentially, so the trochoid
       // does not start exactly on the space centreline. Close that last
       // fraction of a degree along the root circle, or the profile has a
       // gap at every tooth and the polygon never closes.
       a0   = atan2(trok[0][1], trok[0][0]),
       rr   = norm(trok[0]),
       cap  = [ for (i=[0:2]) let(a = 90 + (a0-90)*i/3) rr*[cos(a), sin(a)] ] )
    concat(cap, trok, inv, tip);

// mirror a half pitch about the tooth centreline and repeat z times
function gear_poly(m,z,al=20,x=0,rho=0.38) =
  let( h = half_pitch(m,z,al,x,rho*m), tht = 90 - 180/z,
       // reflect about the line at angle tht
       hm = [ for (i=[len(h)-2:-1:1]) let(p = h[i], a = 2*tht)
                [ p[0]*cos(a) + p[1]*sin(a), p[0]*sin(a) - p[1]*cos(a) ] ],
       one = concat(h, hm),
       // the profile is built running clockwise from the space centreline;
       // linear_extrude wants a counter-clockwise outer contour, so reverse
       all = [ for (k=[0:z-1]) for (p = one)
                 let(a = -360*k/z) [ p[0]*cos(a) - p[1]*sin(a),
                                     p[0]*sin(a) + p[1]*cos(a) ] ] )
    [ for (i=[len(all)-1 : -1 : 0]) all[i] ];

// ---- internal (ring) gear: the same flanks, addendum and dedendum
//      swapped, wrapped in a plain outer rim.
function ring_poly(m,z,al=20,x=0,rho=0.38) =
  let( rp = g_rp(m,z), rb = rp*cos(al), rai = rp - m*(1 - x), rfi = rp + m*(1.25 + x),
       tht = 90 - 180/z,
       inv = [ for (i=[0:NI]) let(r = max(rb,rai) + (rfi - max(rb,rai))*i/NI)
                 r*[cos(tht + g_phi(r,m,z,al,-x)), sin(tht + g_phi(r,m,z,al,-x))] ],
       tip = [ for (i=[1:NC]) let(a = tht + g_phi(max(rb,rai),m,z,al,-x)*(1 - i/NC))
                 max(rb,rai)*[cos(a), sin(a)] ],
       h   = concat([for (i=[len(inv)-1:-1:0]) inv[i]], tip),
       hm  = [ for (i=[len(h)-2:-1:1]) let(p = h[i], a = 2*tht)
                 [ p[0]*cos(a) + p[1]*sin(a), p[0]*sin(a) - p[1]*cos(a) ] ],
       one = concat(h, hm) )
    [ for (k=[0:z-1]) for (p = one)
        let(a = -360*k/z) [ p[0]*cos(a) - p[1]*sin(a), p[0]*sin(a) + p[1]*cos(a) ] ];

// ---- bodies ---------------------------------------------------------
module spur(m,z,h,al=20,x=0,ph=0)
    { rotate([0,0,ph]) linear_extrude(height=h) polygon(gear_poly(m,z,al,x)); }

module helical(m,z,h,beta,al=20,x=0,ph=0,sl=28) {
    tw = deg(h*tan(beta)/g_rp(m,z));
    rotate([0,0,ph]) linear_extrude(height=h, twist=tw, slices=sl)
        polygon(gear_poly(m,z,al,x));
}

module herringbone(m,z,h,beta,al=20,x=0,ph=0,sl=22) {
    tw = deg((h/2)*tan(beta)/g_rp(m,z));
    rotate([0,0,ph]) {
        linear_extrude(height=h/2, twist=tw, slices=sl)  polygon(gear_poly(m,z,al,x));
        translate([0,0,h/2]) rotate([0,0,tw])
            linear_extrude(height=h/2, twist=-tw, slices=sl) polygon(gear_poly(m,z,al,x));
    }
}

// ring gear as a single polygon with a hole -- even-odd, still no boolean
module ring(m,z,h,ro,al=20,x=0,ph=0) {
    p = ring_poly(m,z,al,x);
    n = len(p);
    o = [ for (i=[0:95]) ro*[cos(360*i/96), sin(360*i/96)] ];
    rotate([0,0,ph]) linear_extrude(height=h)
        polygon(points=concat(o,p), paths=[[for(i=[0:95]) i],[for(i=[0:n-1]) 96+i]]);
}

// ===================================================================
//  PLANETARY PHASING
//  The part that is genuinely fiddly by hand.  A planet dropped at an
//  arbitrary angle will not mesh: its teeth have to be clocked to the
//  sun it rolls on.  Rolling a planet round a stationary sun turns it
//  at (1 + zs/zp) times the carrier rate, so the phase of a planet
//  carried to angle psi is fixed, not free.  Get it wrong by half a
//  tooth and the whole train interferes.
// ===================================================================
function plan_phase0(psi0,zp) = psi0 + 90 - 180/zp;
function plan_phase(psi,psi0,zs,zp) =
    plan_phase0(psi0,zp) + (1 + zs/zp)*(psi - psi0);

// assembly condition: equally spaced planets only drop in when this is
// a whole number
function plan_assembles(zs,zr,n) = (zs + zr)/n;

// herringbone ring -- an internal gear has to take the SAME helix hand
// as the planet it swallows, not the opposite one an external pair does
module ring_herring(m,z,h,ro,beta,al=20,x=0,ph=0,sl=20) {
    p = ring_poly(m,z,al,x); n = len(p);
    o = [ for (i=[0:95]) ro*[cos(360*i/96), sin(360*i/96)] ];
    tw = deg((h/2)*tan(beta)/g_rp(m,z));
    rotate([0,0,ph]) {
        linear_extrude(height=h/2, twist=tw, slices=sl)
            polygon(points=concat(o,p), paths=[[for(i=[0:95]) i],[for(i=[0:n-1]) 96+i]]);
        translate([0,0,h/2]) rotate([0,0,tw])
            linear_extrude(height=h/2, twist=-tw, slices=sl)
                polygon(points=concat(o,p), paths=[[for(i=[0:95]) i],[for(i=[0:n-1]) 96+i]]);
    }
}

// ===================================================================
//  STRAIGHT BEVEL GEARS
//  Every pitch cone in a bevel set shares ONE apex.  For a 90 degree
//  shaft angle the two cone angles are complementary and tan(g1)=z1/z2,
//  so the pair is fixed by the tooth counts alone.  Both members must
//  be laid out on the SAME outer cone distance Lo = rp/sin(g): a bevel
//  pair whose cones do not share an apex does not mesh anywhere along
//  its face, and that is the single easiest thing to get wrong.
//  The tooth is the outer profile scaled linearly toward the apex --
//  depth, thickness and pitch all shrink together, which is what makes
//  it a bevel tooth rather than a cylindrical one on a slant.
// ===================================================================
function bev_gamma(z1,z2) = atan2(z1,z2);        // this gear's cone angle
function bev_Lo(m,z,gam)  = (m*z/2)/sin(gam);    // outer cone distance

// The quick construction: drop the scaled transverse profile into a
// horizontal plane and let linear_extrude(scale=) taper it. Two
// approximations live here -- the flanks are the real gear's, not the
// virtual back-cone gear's, and the addendum grows radially rather than
// perpendicular to the cone. Cheap and fine for a decorative tooth;
// bevel_true() below does it properly.
module bevel(m,z,gam,Lo,Li,al=20,ph=0) {
    k = Li/Lo;
    rotate([0,0,ph]) translate([0,0,Li*cos(gam)])
        linear_extrude(height=(Lo-Li)*cos(gam), scale=1/k)
            polygon([ for (p = gear_poly(m,z,al)) p*k ]);
}

// Body of revolution: spherical outside, conical back down to the teeth.
// Winding note, measured not assumed: rotate_extrude normalises the
// hand of its profile, so either direction round the (r,z) half-plane
// gives the same solid -- both export a hemisphere at +2/3 pi r^3, and
// both shade identically. That is NOT true of polyhedron, which takes
// the winding literally; see gsweep below.
module bev_body(R, thc, gam, Lo, Li, NA=26) {
    p = concat(
        [ for (i=[0:NA]) let(t = thc*i/NA) [R*sin(t), R*cos(t)] ],
        [ [Lo*sin(gam), Lo*cos(gam)], [Li*sin(gam), Li*cos(gam)], [0, Li*cos(gam)] ] );
    rotate_extrude($fn=96) polygon(p);
}

// ===================================================================
//  SWEPT SOLIDS
//  grid[u][v] -- u walks the sweep, v walks a closed section.  The
//  quad's right-hand-rule normal is t x s (section tangent crossed
//  with sweep direction) and polyhedron wants that pointing INTO the
//  solid, so for a sweep running up +z the section must be listed
//  CLOCKWISE in xy.  gear_poly() is counter-clockwise, because that is
//  what linear_extrude wants, so anything handed to gsweep gets
//  reversed on the way in.  Get this backwards and the solid still
//  renders -- flat ambient grey, no gradient across a curved face,
//  which reads as a dull colour rather than as a bug.
// ===================================================================
function g_cent(sec) =
  let(n = len(sec)) [ for (k=[0:2]) (
      [ for (p = sec) p[k] ] * [ for (p = sec) 1/n ] ) ];

module gsweep(grid, cap0 = true, cap1 = true, conv = 10) {
    NU = len(grid) - 1;
    NV = len(grid[0]);
    pts = concat([ for (u=[0:NU]) each grid[u] ],
                 [g_cent(grid[0])], [g_cent(grid[NU])]);
    B0 = (NU+1)*NV; B1 = B0 + 1;
    polyhedron(
      points = pts,
      faces = concat(
        [ for (u=[0:NU-1]) for (v=[0:NV-1])
            [ u*NV + v, u*NV + (v+1)%NV, (u+1)*NV + (v+1)%NV, (u+1)*NV + v ] ],
        cap0 ? [ for (v=[0:NV-1]) [ B0, v, (v+1)%NV ] ] : [],
        cap1 ? [ for (v=[0:NV-1]) [ B1, NU*NV + (v+1)%NV, NU*NV + v ] ] : [] ),
      convexity = conv );
}

// ===================================================================
//  SPIRAL BEVEL GEARS
//  The tooth trace is a CIRCULAR ARC, which is what a face-mill cutter
//  of radius rc actually leaves in the blank.  Develop the pitch cone
//  into a flat sector: one whole turn of the gear becomes 360*sin(gam)
//  degrees of sector, so a real azimuth phi appears in the development
//  as phi*sin(gam).  The cutter is a circle of radius rc whose centre
//  sits rho from the apex, and the tooth trace is where that circle
//  crosses the cone distance L:
//
//      cos(theta - theta0) = (L^2 + rho^2 - rc^2) / (2 L rho)
//
//  rho follows from the mean spiral angle psi at mid-face Lm.  psi is
//  the angle between the trace and the cone element, so in the triangle
//  apex-centre-point the angle at the point is 90-psi and the law of
//  cosines gives
//
//      rho^2 = Lm^2 + rc^2 - 2 Lm rc sin(psi)
//
//  A bevel whose trace is a straight slant is a SKEW bevel, not a
//  spiral one: it has no lengthwise curvature, so teeth come into
//  contact all at once instead of rolling in from one end, and it is
//  as noisy as the straight bevel it was meant to replace.  The pair
//  must be opposite hands -- same hand and they simply will not mesh.
//  A useful check that the arithmetic is right: both members must come
//  out with the SAME face contact ratio, because the developed arc
//  they share is the same arc.
// ===================================================================
function sb_rho(Lm, rc, psi) = sqrt(Lm*Lm + rc*rc - 2*Lm*rc*sin(psi));
function sb_theta(L, rho, rc) =
    acos(max(-1, min(1, (L*L + rho*rho - rc*rc)/(2*L*rho))));
// azimuth the tooth section is swung to at cone distance L, zero at Lm
function sb_phi(L, Lm, rho, rc, gam, hand=1) =
    hand*(sb_theta(L,rho,rc) - sb_theta(Lm,rho,rc))/sin(gam);
// lengthwise arc the trace covers, in tooth pitches -- the face
// contact ratio, and the number that must match across the pair
function sb_face_ratio(Lo, Li, Lm, rho, rc, gam, z) =
    z*abs(sb_phi(Lo,Lm,rho,rc,gam) - sb_phi(Li,Lm,rho,rc,gam))/360;

// ---- one tooth, closed along the root ------------------------------
// gsweep caps a section by fanning through its centroid.  Hand it a
// whole gear and the centroid is the axis, so the "cap" is a disc the
// full diameter of the gear -- the tooth ring becomes a solid plate
// and you see that plate, not the teeth.  Sweep ONE tooth at a time
// instead: a single pitch of profile, closed by a short arc back along
// the root circle.  Each cap is then a small patch sitting on the root
// cone, where the blank hides it.
//
// half_pitch runs from the space centreline at 90 degrees down to the
// tooth centreline, so one mirrored pitch ends at 90-360/z.  Angles
// decrease the whole way round the outside, which makes this CLOCKWISE
// -- the hand gsweep wants, so it is not reversed on the way in.
// sink pushes the closing arc below the root circle, so that when a
// tooth is laid on a blank whose surface IS the root cone the tooth's
// base is buried in it rather than coplanar with it -- coplanar is what
// puts a dashed line of depth-buffer fighting round every root.
function one_tooth(m,z,al=20,x=0,rho=0.38,sink=0) =
  let( h = half_pitch(m,z,al,x,rho*m), tht = 90 - 180/z,
       hm = [ for (i=[len(h)-2:-1:1]) let(p = h[i], a = 2*tht)
                [ p[0]*cos(a) + p[1]*sin(a), p[0]*sin(a) - p[1]*cos(a) ] ],
       one = concat(h, hm),
       // Close from where the profile ACTUALLY ends, not from the nominal
       // 90-360/z: the mirror lands a fraction of a degree short of it,
       // and closing to the nominal angle overlaps the next tooth's root
       // by that fraction -- a sliver of coplanar face per tooth, which
       // shows up as a dashed line round the root circle.
       last = one[len(one)-1],
       r0 = norm(one[0]) - sink, a1 = atan2(last[1], last[0]),
       arc = [ for (i=[1:5]) let(a = a1 + (90-a1)*i/6) r0*[cos(a), sin(a)] ] )
    concat(one, arc);

// ---- TREDGOLD: putting a tooth on the cone --------------------------
// Two things have to be right, and the easy construction gets both
// wrong.
//
// FLANK SHAPE.  A bevel tooth's profile is not the profile of a spur
// gear with z teeth.  Develop the BACK cone -- the cone perpendicular
// to the pitch cone at the outer end -- into a plane and the tooth
// appears as one tooth of a spur gear of radius rp/cos(gam), that is
// of zv = z/cos(gam) teeth at the same module.  Use z instead of zv
// and the flanks come out far too curved: here the wheel's zv is 76
// against a real z of 34, and teeth cut to the z=34 shape foul their
// mates well before the pitch line.  zv is not an integer and does not
// need to be -- only one tooth is ever taken from that virtual gear.
//
// DEPTH DIRECTION.  The offset from the pitch circle has to be taken
// PERPENDICULAR to the cone element, not radially. The element runs
// along (sin g, cos g) in the (radius, z) half-plane, so perpendicular
// is (cos g, -sin g).  Grow the addendum radially instead -- which is
// what linear_extrude(scale=) does -- and on a steep cone the tip ends
// up outside the root cone, so the blank rises through its own teeth.
//
// The developed angle maps back to a real azimuth by dividing by
// cos(gam), which is what keeps the arc thickness at the pitch line
// equal on the two members: half a tooth is 90/zv developed, and
// (90/zv)/cos(gam) = 90/z real, for either member of the pair.
function bv_place(p, L, Lo, rpv, thv0, gam) =
  let( del = (norm(p) - rpv)*(L/Lo),
       a   = (atan2(p[1], p[0]) - thv0)/cos(gam),
       rr  = L*sin(gam) + del*cos(gam),
       zz  = L*cos(gam) - del*sin(gam) )
    [ rr*cos(a), rr*sin(a), zz ];

// ph places a TOOTH CENTRELINE at that azimuth.
module bevel_spiral(m, z, gam, Lo, Li, rc, psi, al=20, ph=0, hand=1, NS=10) {
    Lm   = (Lo+Li)/2;
    rho  = sb_rho(Lm, rc, psi);
    zv   = z/cos(gam);
    rpv  = g_rp(m,zv);
    thv0 = 90 - 180/zv;
    sec  = one_tooth(m, zv, al, 0, 0.38, 0.10*m);
    for (k=[0:z-1]) gsweep([ for (i=[0:NS])
        let( L = Li + (Lo-Li)*i/NS,
             a = ph - 360*k/z + sb_phi(L, Lm, rho, rc, gam, hand),
             ca = cos(a), sa = sin(a) )
        [ for (q = sec) let(p = bv_place(q, L, Lo, rpv, thv0, gam))
            [ p[0]*ca - p[1]*sa, p[0]*sa + p[1]*ca, p[2] ] ] ]);
}

// straight bevel, same construction with no lengthwise curvature
module bevel_true(m, z, gam, Lo, Li, al=20, ph=0) {
    zv   = z/cos(gam);
    rpv  = g_rp(m,zv);
    thv0 = 90 - 180/zv;
    sec  = one_tooth(m, zv, al, 0, 0.38, 0.10*m);
    for (k=[0:z-1]) gsweep([ for (L=[Li,Lo])
        let( a = ph - 360*k/z, ca = cos(a), sa = sin(a) )
        [ for (q = sec) let(p = bv_place(q, L, Lo, rpv, thv0, gam))
            [ p[0]*ca - p[1]*sa, p[0]*sa + p[1]*ca, p[2] ] ] ]);
}

// ---- the blank a bevel gear is cut from ------------------------------
// delta is measured outward from the pitch cone along (cos g, -sin g),
// so the tip (delta = +m) lies further out and LOWER, and the root
// (delta = -1.25m) lies further in and HIGHER. Blank material is
// therefore on the negative-delta side: a bevel gear is a conical dish
// whose front surface is the root cone and whose back is a second cone
// parallel to it, T further in. Cutting the back off with a flat plane
// instead gives a drum with the teeth lost round the edge of it.
//
// The teeth's outer end caps sit at L = Lo; the blank stops at Lo too,
// but its outer face is the BACK CONE (perpendicular to the pitch cone,
// which is where a real bevel gear's rim is), so the two meet edge to
// edge rather than overlapping in a plane -- no coplanar pair to fight
// in the depth buffer.
module bev_blank(m, z, gam, Lo, Lh, T, rh, zb, bore=0, NA=18) {
    d  = 1.25*m;
    fr = [ for (i=[0:NA]) let(L = Lh + (Lo-Lh)*i/NA)
             [ L*sin(gam) -  d   *cos(gam), L*cos(gam) +  d   *sin(gam) ] ];
    // On a shallow cone the back surface would run past the axis, so it
    // is clamped at the hub radius -- where it clamps, the cone simply
    // becomes the hub cylinder, which is what a real blank does too.
    bk = [ for (i=[NA:-1:0]) let(L = Lh + (Lo-Lh)*i/NA)
             [ max(rh, L*sin(gam) - (d+T)*cos(gam)),
                        L*cos(gam) + (d+T)*sin(gam) ] ];
    fz = Lh*cos(gam) + d*sin(gam);              // front cone at the hub
    rotate_extrude($fn=120)
        polygon(concat(fr, bk, [[rh, zb], [bore, zb], [bore, fz]]));
}
