// ===================================================================
//  turbine.scad -- twisted rotor blades
//
//  A twisted blade is a stack of aerofoil sections, each one rotated a
//  little further than the one below it.  Which way it is wrung, and
//  how hard, is the whole design:
//
//    * Vertical axis (Gorlov / helical Darrieus).  A straight-bladed
//      vertical rotor makes its torque in pulses -- every blade stalls
//      twice per revolution -- and shakes its bearings out.  Wrap each
//      of N blades through 360/N degrees and at any instant some part
//      of some blade sits at every angle of attack.  The ripple
//      flattens, and the rotor will start on its own.
//
//    * Vertical axis (helical Savonius).  Two scoops, drag-driven,
//      wrung through a half turn so that no rotor angle is ever a dead
//      spot.  Slow and torquey where the Gorlov is fast and fussy.
//
//    * Horizontal axis.  The twist is not for smoothness but for
//      incidence: a section at radius r meets the wind at atan(V/wr),
//      and wr grows down the span, so the root must be set coarse and
//      the tip nearly flat to hold one angle of attack all the way
//      out.  It is the same reason a propeller looks wrung out.
//
//  Everything here is additive.  No difference(), no intersection():
//  a preview union is a concatenation, so the cost of this file is
//  the cost of its triangles and nothing more.
// ===================================================================

// ---- mesh emitter ---------------------------------------------------
// grid[u][v] -- u walks the sweep, v walks a closed section.
//
// WINDING.  The quad [u,v  u,v+1  u+1,v+1  u+1,v] has right-hand-rule
// normal t x s, where t is the section tangent (+v) and s the sweep
// (+u).  This kernel wants that normal pointing INTO the solid, so for
// a sweep running up +z the section must be listed CLOCKWISE in xy:
// at the rightmost point of a CW loop t = -y, and -y x +z = -x, which
// points back at the axis.  A counter-clockwise section here gives a
// solid that is inside out -- it renders flat ambient grey, with no
// gradient across a curved face, which is the tell.
//
// af() below is CCW (that is what linear_extrude wants), so every
// blade builder that hands its sections straight to tsweep passes
// them through rev() first.
function centroid(sec) =
  let(n = len(sec)) [ for (k=[0:2]) (
      [ for (p = sec) p[k] ] * [ for (p = sec) 1/n ] ) ];

function rev(s) = [ for (i = [len(s)-1 : -1 : 0]) s[i] ];

module tsweep(grid, cap0 = true, cap1 = true, conv = 8) {
    NU = len(grid) - 1;
    NV = len(grid[0]);
    body = [ for (u=[0:NU]) each grid[u] ];
    pts  = concat(body, [centroid(grid[0])], [centroid(grid[NU])]);
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

// ---- sections -------------------------------------------------------
// NACA 4-digit symmetric half-thickness.  The last coefficient is the
// closed-trailing-edge variant (-0.1036 rather than -0.1015), so the
// loop shuts exactly at x = c and the sweep needs no seam repair.
function naca4(u, tc) = 5*tc*( 0.2969*sqrt(u) - 0.1260*u - 0.3516*u*u
                             + 0.2843*u*u*u  - 0.1036*u*u*u*u );

// Closed aerofoil, chord along +x from 0 to c, cosine-spaced so the
// points bunch where the curvature is (the nose), CCW, 2n points.
function af(c, tc, n = 26) =
  concat( [ for (i=[0:n])      let(u = 0.5-0.5*cos(180*i/n)) [c*u, -c*naca4(u,tc)] ],
          [ for (i=[n-1:-1:1]) let(u = 0.5-0.5*cos(180*i/n)) [c*u,  c*naca4(u,tc)] ] );

// A circle of diameter d parameterised exactly like af(), so the two
// can be blended point for point.  This is how a real blade root is
// made: a round spar stub that morphs into an aerofoil over the first
// tenth of the span.
function circ_like(d, n = 26) =
  concat( [ for (i=[0:n])      let(a = 180*(0.5-0.5*cos(180*i/n)))
                                 [ (d/2)*(1-cos(a)), -(d/2)*sin(a) ] ],
          [ for (i=[n-1:-1:1]) let(a = 180*(0.5-0.5*cos(180*i/n)))
                                 [ (d/2)*(1-cos(a)),  (d/2)*sin(a) ] ] );

function mix(A, B, t) = [ for (i=[0:len(A)-1]) A[i]*(1-t) + B[i]*t ];
function sstep(t) = let(x = max(0, min(1, t))) x*x*(3-2*x);

// Rotate a 2D section about a point.
function spin(sec, ang, about=[0,0]) =
  [ for (p = sec) let(q = p - about)
      about + [ q[0]*cos(ang) - q[1]*sin(ang), q[0]*sin(ang) + q[1]*cos(ang) ] ];

// Half-cylinder scoop of wall thickness w: outer arc out, inner arc
// back.  off is how far the scoop's own centre sits off the rotor
// axis -- the classic Savonius overlap is 2*(Rs-off), the slot the
// downwind scoop breathes through.
function scoop_sec(Rs, off, w, n = 30) =
  concat( [ for (i=[0:n])    let(a=180*i/n) [off + Rs*cos(a),     Rs*sin(a)] ],
          [ for (i=[n:-1:0]) let(a=180*i/n) [off + (Rs-w)*cos(a), (Rs-w)*sin(a)] ] );

// ---- vertical-axis helical blade ------------------------------------
// The quarter-chord rides the cylinder r = R and climbs while it turns.
// Section coords: a is chordwise (tangential), b is thickness (radial).
// tip is the fraction of span at each end over which the chord is
// rolled off, so the blade ends in a faired tip and not a sawn slab.
function vawt_blade(R, H, NU, th0, wrap, c, tc, pitch = 0, tip = 0) =
  [ for (u=[0:NU])
      let( s = u/NU,
           e = tip <= 0 ? 1
             : 0.34 + 0.66*sqrt(sstep(min(s, 1-s)/tip)),
           cc = c*e,
           sec = spin(af(cc, tc), pitch, [0.25*cc, 0]),
           th = th0 + wrap*s, ct = cos(th), st = sin(th) )
        // no rev(): the (a,b) -> (R+b, a) swap is orientation-reversing
        // all by itself, so a CCW aerofoil lands CW in xy already.
        [ for (p = sec)
            let( a = p[0] - 0.25*cc, b = p[1] )
              [ (R+b)*ct - a*st, (R+b)*st + a*ct, H*s ] ] ];

// ---- vertical-axis helical scoop (Savonius) -------------------------
function sav_blade(Rs, off, wall, H, NU, th0, twist) =
  [ for (u=[0:NU])
      let( s = u/NU, th = th0 + twist*s,
           sec = spin(scoop_sec(Rs, off, wall), th) )
        [ for (p = rev(sec)) [p[0], p[1], H*s] ] ];

// ---- horizontal-axis twisted blade ----------------------------------
// Built along +z as span; the caller stands it up.  In the section's
// own frame x is axial and y tangential, so a chord set at beta from
// the plane of rotation is the aerofoil spun by 90-beta.  Chord falls
// as s^0.72 (fat inboard, where the torque arm is short and the
// section must work hard), twist as s^tw.
function hawt_blade(r0, R, NU, c0, cTip, b0, bTip, tc,
                    tw = 1.35, ax = 0.30, root = 0.16, dRoot = 0, tip = 0.05) =
  [ for (u=[0:NU])
      let( s = u/NU,
           r = r0 + (R-r0)*s,
           // roll the chord off over the last few per cent of span so
           // the blade ends in a faired tip and not a sawn-off slab
           q = tip <= 0 || s < 1-tip ? 1
             : 0.18 + 0.82*sqrt(max(0, 1 - pow((s-(1-tip))/tip, 2))),
           c = q*(c0 + (cTip-c0)*pow(s, 0.72)),
           b = b0 + (bTip-b0)*pow(s, tw),
           foil = spin(af(c, tc*(1-0.5*s)), 90-b, [ax*c, 0]),
           blend = root <= 0 ? 0 : 1 - sstep(s/root),
           dr = dRoot > 0 ? dRoot : 0.62*c0,
           rnd = [ for (p = circ_like(dr)) p - [dr/2 - ax*c, 0] ],
           sec = blend <= 0 ? foil : mix(foil, rnd, blend) )
        [ for (p = rev(sec)) [ p[0] - ax*c, p[1], r ] ] ];

// ---- a radial arm ---------------------------------------------------
// Chord tangential (the way the arm travels), thickness vertical,
// swept out along +x and then turned to az.  rotate([0,90,0]) after
// rotate([0,0,90]) maps local (x,y,z) to (z,x,y): chord to +y, thick
// to +z, extrusion to +x -- exactly that.  linear_extrude wants CCW,
// which is what af() already is, so no rev() here.
module arm(r0, r1, z, c, tc, az, taper = 0.55) {
    translate([0,0,z]) rotate([0,0,az]) rotate([0,90,0]) rotate([0,0,90])
        translate([0,0,r0])
            linear_extrude(height = r1-r0, scale = taper)
                polygon([ for (p = af(c,tc)) p - [0.3*c, 0] ]);
}
