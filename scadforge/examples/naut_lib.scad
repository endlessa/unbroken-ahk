// ===================================================================
//  naut_lib  --  organic surface machinery for scadforge
//  Everything here is additive: parametric surfaces emitted as
//  polyhedron() quad meshes. No difference(), no intersection().
//
//  WINDING (verified empirically against the kernel):
//    a face whose right-hand-rule normal points OUTWARD renders
//    INWARD.  So faces must be listed so the RHR normal points INTO
//    the solid.  For a grid with u = sweep direction and v ordered
//    CLOCKWISE in the section plane, the correct quad is
//        [ix(u,v), ix(u,v+1), ix(u+1,v+1), ix(u+1,v)]
// ===================================================================

// ---- 2D helpers ---------------------------------------------------
function unit2(v) = norm(v) < 1e-12 ? [0,0] : v/norm(v);
function ang2(v)  = atan2(v[1], v[0]);
function lerp(a,b,t) = a + (b-a)*t;

// Circular fillet of radius r between the segments A->B and B->C.
// Returns n+1 points from the tangent point on B->A to the tangent
// point on B->C.  Degenerates gracefully to [B] when there is no room.
function fillet(A, B, C, r, n) =
  let( u1 = unit2(A-B), u2 = unit2(C-B),
       cs = max(-1, min(1, u1*u2)),
       th = acos(cs)/2,                       // half the included angle, degrees
       ok = th > 0.35 && th < 89.65 && r > 1e-6,
       tl = !ok ? 0 : min(r/tan(th), norm(A-B)*0.48, norm(C-B)*0.48),
       rr = tl*tan(th),
       ctr = B + unit2(u1+u2)*(rr/sin(th)),
       T1  = B + u1*tl,
       T2  = B + u2*tl,
       a1  = ang2(T1-ctr), a2 = ang2(T2-ctr),
       da  = ((a2 - a1 + 540) % 360) - 180 )
  rr <= 1e-6 ? [ for (i=[0:n]) B ]
             : [ for (i=[0:n]) ctr + rr*[cos(a1 + da*i/n), sin(a1 + da*i/n)] ];

// Straight run between two points, EXCLUDING both ends (the ends are
// already supplied by the adjoining fillets).
function span(P, Q, n) = [ for (i=[1:n]) lerp(P, Q, i/(n+1)) ];

// ---- section builder ----------------------------------------------
// A monohull station as a closed loop, ordered CLOCKWISE in (y,z):
// deck crown -> starboard sheer -> chine -> keel -> port -> back.
// ctl = [D, S, C, K] control points; rad = [rD, rS, rC, rK] fillet radii.
FD = 8; FS = 8; FC = 10; FK = 8;      // fillet samples
SD = 4; ST = 6; SV = 8;               // straight-run samples

function half_section(ctl, rad) =
  let( D = ctl[0], S = ctl[1], C = ctl[2], K = ctl[3],
       Sm = [-S[0], S[1]], Cm = [-C[0], C[1]],
       fD = fillet(Sm, D, S,  rad[0], FD),
       fS = fillet(D,  S, C,  rad[1], FS),
       fC = fillet(S,  C, K,  rad[2], FC),
       fK = fillet(C,  K, Cm, rad[3], FK),
       // keep only the starboard half of the two on-centreline fillets
       hD = [ for (i=[FD/2 : FD]) fD[i] ],
       hK = [ for (i=[0 : FK/2]) fK[i] ] )
  concat( hD, span(hD[len(hD)-1], fS[0], SD),
          fS, span(fS[FS],        fC[0], ST),
          fC, span(fC[FC],        hK[0], SV),
          hK );

// Mirror the starboard half into a full closed loop (no duplicate
// points on the centreline).
function full_section(h) =
  concat( h, [ for (i=[len(h)-2 : -1 : 1]) [-h[i][0], h[i][1]] ] );

// ---- mesh emitter --------------------------------------------------
// grid: [NU+1][NV] points in 3-space, v closed, u open.
// Caps both ends with a fan through the section centroid.
function centroid(sec) =
  let(n = len(sec)) [ for (k=[0:2]) (
      [ for (p = sec) p[k] ] * [ for (p = sec) 1/n ] ) ];

module sweep_mesh(grid, cap0 = true, cap1 = true, conv = 8) {
    NU = len(grid) - 1;
    NV = len(grid[0]);
    body = [ for (u=[0:NU]) each grid[u] ];
    c0 = centroid(grid[0]);
    c1 = centroid(grid[NU]);
    pts = concat(body, [c0], [c1]);
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

// ---- 3D sweep along a spine ----------------------------------------
function unit3(v) = norm(v) < 1e-12 ? [0,0,1] : v/norm(v);

function tang(S, i) =
  let(n = len(S)-1)
    unit3( i==0 ? S[1]-S[0] : (i==n ? S[n]-S[n-1] : S[i+1]-S[i-1]) );

// Place a 2D section into the plane spanned by N,B at origin o.
function place(sec2, o, N, B) = [ for (p = sec2) o + N*p[0] + B*p[1] ];

// Sweep: S is the spine (list of 3D points); secfn(i) returns the 2D
// section at station i already at its final size.  `up` must never be
// parallel to the spine tangent - for a transverse arch living in the
// y-z plane, use up = [1,0,0].
function sweep_grid(S, secfn, up) =
  [ for (i = [0 : len(S)-1])
      let( T = tang(S, i), N = unit3(cross(up, T)), B = cross(T, N) )
        place(secfn(i), S[i], N, B) ];

// ---- reusable closed 2D sections ------------------------------------
// Symmetric aerofoil-ish lozenge: chord c along x, thickness t along y,
// nose radius controlled by the exponent p.  Convex, closed, CW.
function foil(c, t, p, n) =
  concat(
    [ for (i=[0:n])   let(u = 0.5 - 0.5*cos(180*i/n))
        [ c*(u - 0.30),  (t/2)*pow(4*u*(1-u), p) ] ],
    [ for (i=[n-1:-1:1]) let(u = 0.5 - 0.5*cos(180*i/n))
        [ c*(u - 0.30), -(t/2)*pow(4*u*(1-u), p) ] ] );

// Superellipse (Lame) closed curve, exponent e: 2 = ellipse,
// >2 squarer, <2 pinched.  CW ordering.
function lame(a, b, e, n) =
  [ for (i=[0:n-1]) let(th = -360*i/n, c = cos(th), s = sin(th))
      [ a*sign(c)*pow(abs(c), 2/e), b*sign(s)*pow(abs(s), 2/e) ] ];

// Superellipse with independent fore/aft/side semi-axes and exponent.
// Returns a closed CW loop in (x,y).  ax_f = forward semi-axis (+x),
// ax_a = aft semi-axis (-x), by = half-width, e = Lame exponent.
function lame4(ax_f, ax_a, by, e, n) =
  [ for (i=[0:n-1]) let(th = -360*i/n, c = cos(th), s = sin(th),
                        ax = c >= 0 ? ax_f : ax_a)
      [ ax*sign(c)*pow(abs(c), 2/e), by*sign(s)*pow(abs(s), 2/e) ] ];

// Emit a contiguous u-slice of a grid as its own closed mesh.  Adjacent
// slices share an exact boundary ring, so the composite surface is
// continuous - this is how flush colour bands and inset glazing are done
// without a single boolean.
module band(grid, u0, u1, conv = 6) {
    sweep_mesh([ for (u=[u0:u1]) grid[u] ], true, true, conv);
}

// ---- splitting a closed section along v ------------------------------
// A closed section can be cut into two closed sub-sections that share a
// chord.  Each half is then its own solid with its own colour, and the
// shared face is interior and invisible.  Still zero booleans.
function arc_of(sec, i0, i1) =
  let(n = len(sec))
    [ for (k = [0 : ((i1 - i0 + n) % n)]) sec[(i0 + k) % n] ];

function part(sec, i0, i1, m) =
  concat( arc_of(sec, i0, i1), span(sec[i1], sec[i0], m) );

// ---- rounded open polyline ------------------------------------------
// Smooths P[0..m] with a circular fillet of radius R[i] at each interior
// vertex, joined by straight runs.  Returns a fixed-length point list,
// so it can be used as a station in a sweep grid.
function chain(P, R, nf, ns) =
  let( m = len(P) - 1,
       node = [ for (i = [0:m])
                  (i == 0) ? [P[0]] :
                  (i == m) ? [P[m]] : fillet(P[i-1], P[i], P[i+1], R[i], nf) ] )
  concat( [ for (i = [0:m-1]) each
              concat( node[i],
                      span(node[i][len(node[i])-1], node[i+1][0], ns) ) ],
          node[m] );

// ---- applied shells --------------------------------------------------
// A band of a closed section, offset inward by r and given thickness d,
// emitted as its own closed loop.  Used for glazing that sits in a
// groove and for a roof cap that laps over the body: neither shares a
// face with the body, so nothing z-fights.
function cent2(sec) = let(n=len(sec))
    [ ([for (p=sec) p[0]]) * [for (p=sec) 1/n],
      ([for (p=sec) p[1]]) * [for (p=sec) 1/n] ];

// Inward offset along the LOCAL SURFACE NORMAL.  Offsetting toward the
// section centroid instead (the obvious shortcut) collapses points near
// a narrow crown, where the centroid direction is almost tangential --
// the panel then sinks inside the body it is supposed to sit proud of.
function nrm_at(a, i) =
  let( n = len(a),
       t = (i == 0)     ? a[1] - a[0] :
           (i == n-1)   ? a[n-1] - a[n-2] : a[i+1] - a[i-1] )
    unit2([ -t[1], t[0] ]);

function offset_in(a, d) = [ for (i = [0:len(a)-1]) a[i] - nrm_at(a, i)*d ];

function part_shell(sec, i0, i1, r, d) =
  let( a = offset_in(arc_of(sec, i0, i1), r), b = offset_in(a, d) )
    concat( a, [ for (k = [len(b)-1 : -1 : 0]) b[k] ] );
