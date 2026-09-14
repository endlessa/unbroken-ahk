// ===================================================================
//  ship_lib.scad -- shared hull toolkit for the faction fleets
//
//  CONVENTION: every ship points +x (bow), +z is up, +y is port.
//  Scene units are metres.  Fleets are laid out along -y.
//
//  Everything here is ADDITIVE.  No difference(), no intersection().
//  A preview union is a concatenation, so a hull costs its triangles
//  and nothing else; one boolean would cost more than all the ships
//  put together.
//
//  WINDING, measured not assumed.  A swept quad's right-hand-rule
//  normal is (section tangent) x (sweep direction), and polyhedron
//  here wants that pointing INTO the solid, so for a sweep running up
//  +z the section must be listed CLOCKWISE in xy.  Get it backwards
//  and the solid still renders -- flat ambient grey, no gradient
//  across a curved face -- which reads as a dull colour rather than
//  as a bug.  linear_extrude wants the opposite hand; rotate_extrude
//  normalises its profile and takes either.
//
//  LIGHTING.  One directional light from normalize(0.5,0.35,0.85):
//  starboard and above.  No specular, no shadows, no texture.  Form
//  reads only through silhouette and the rate of change of the
//  surface normal, so a hull whose interest lives in fine detail on
//  the port flank reads as a dark blob.  Shape the outline first.
// ===================================================================

function unit3(v) = norm(v) < 1e-12 ? [0,0,1] : v/norm(v);
function lerp(a,b,t) = a + (b-a)*t;
function sstep(t) = let(x = max(0,min(1,t))) x*x*(3-2*x);
function rev(s) = [ for (i=[len(s)-1:-1:0]) s[i] ];
function clamp(x,a,b) = max(a, min(b, x));

// ---- swept mesh -----------------------------------------------------
function cent3(L) = let(n=len(L)) [ for (k=[0:2]) ([for(p=L) p[k]] * [for(p=L) 1/n]) ];

module smesh(grid, cap0=true, cap1=true, conv=10) {
    NU = len(grid)-1; NV = len(grid[0]);
    pts = concat([ for (u=[0:NU]) each grid[u] ], [cent3(grid[0])], [cent3(grid[NU])]);
    B0 = (NU+1)*NV; B1 = B0+1;
    polyhedron(points = pts,
      faces = concat(
        [ for (u=[0:NU-1]) for (v=[0:NV-1])
            [ u*NV+v, u*NV+(v+1)%NV, (u+1)*NV+(v+1)%NV, (u+1)*NV+v ] ],
        // The cap fans are wound to match the SIDES, which is not the
        // same as matching each other.  Measured, on a tube of known
        // volume: with the fans the other way round the sides came out
        // correct and both caps inside-out, so every capped sweep in
        // this project was rendering its two ends at the flat ambient
        // floor.  It stayed hidden for a long time because a hull's
        // caps are nearly always buried inside the next solid or seen
        // edge-on.  A limb is what exposed it -- a leg carries a large
        // cap high up, and a reversed cap's contribution to the signed
        // volume grows with its distance from the origin, so that one
        // was big enough to flip the whole mesh's sign.
        cap0 ? [ for (v=[0:NV-1]) [B0, (v+1)%NV, v] ] : [],
        cap1 ? [ for (v=[0:NV-1]) [B1, NU*NV+v, NU*NV+(v+1)%NV] ] : [] ),
      convexity = conv);
}

// A contiguous u-slice as its own closed mesh.  Adjacent slices share
// an exact boundary ring, so flush colour bands and inset canopies
// come free: no boolean, and no coplanar pair to fight in the depth
// buffer either.
module sband(grid, u0, u1, conv=6) { smesh([ for (u=[u0:u1]) grid[u] ], true, true, conv); }

// ---- frames along a spine -------------------------------------------
// secfn must be a FUNCTION LITERAL, not a bare function name:
//     sweep_up(S, function(i) my_section(i), [0,0,1])
// A named function passed bare resolves to nothing here -- the tell is
// a storm of "unknown function 'secfn'" and a mesh of undefs that
// still renders, because polyhedron shrugs at a points list full of
// undef and just draws fewer triangles.
function tang(S,i) = let(n=len(S)-1)
    unit3( i==0 ? S[1]-S[0] : (i==n ? S[n]-S[n-1] : S[i+1]-S[i-1]) );

function frame_up(S,i,up) = let(T=tang(S,i), N=unit3(cross(up,T))) [T,N,cross(T,N)];

function sweep_up(S, secfn, up) =
  [ for (i=[0:len(S)-1]) let(F=frame_up(S,i,up))
      [ for (p=secfn(i)) S[i] + F[1]*p[0] + F[2]*p[1] ] ];

// Parallel transport: carry the normal by the minimum rotation taking
// T[i-1] onto T[i], so the section never spins about its own spine.
// A spine that turns past a right angle needs this -- a fixed up-vector
// flips as the tangent sweeps through it.  Recurses once per station,
// so keep spines under about 64 stations.
function pt_N(S,i,N0) =
  i==0 ? unit3(N0)
  : let( Np=pt_N(S,i-1,N0), T0=tang(S,i-1), T1=tang(S,i), v=cross(T0,T1), s=norm(v) )
      s < 1e-9 ? Np
      : let( ax=v/s, ang=acos(clamp(T0*T1,-1,1)) )
          unit3( Np*cos(ang) + cross(ax,Np)*sin(ang) + ax*(ax*Np)*(1-cos(ang)) );

function sweep_pt(S, secfn, N0) =
  [ for (i=[0:len(S)-1]) let(T=tang(S,i), N=pt_N(S,i,N0), B=cross(T,N))
      [ for (p=secfn(i)) S[i] + N*p[0] + B*p[1] ] ];

// ---- closed 2D sections, all CLOCKWISE ------------------------------
function lame(a,b,e,n=48) =
  [ for (i=[0:n-1]) let(t=-360*i/n, c=cos(t), s=sin(t))
      [ a*sign(c)*pow(abs(c),2/e), b*sign(s)*pow(abs(s),2/e) ] ];

function lame4(af,aa,b,e,n=48) =
  [ for (i=[0:n-1]) let(t=-360*i/n, c=cos(t), s=sin(t), a = c>=0 ? af : aa)
      [ a*sign(c)*pow(abs(c),2/e), b*sign(s)*pow(abs(s),2/e) ] ];

// GIELIS SUPERFORMULA (Gielis 2003), published as a generalisation of
// natural form -- diatom valves, starfish, flowers, all one equation:
//   r(t) = [ |cos(m t/4)/a|^n2 + |sin(m t/4)/b|^n3 ] ^ (-1/n1)
// m is the rotational symmetry, n1 the inflation, n2/n3 the lobe pinch.
// Note n1=n2=n3=2 degenerates to r=1 for ANY m -- a plain circle, by
// Pythagoras -- so lobes need n2 and n3 pulled away from 2.  m=5,
// n1=0.3, n2=0.3, n3=0.3 is a fat starfish; m=6, n1=40, n2=10, n3=10
// is a diatom-like hexagon with softened corners.
function gielis_r(t,m,n1,n2,n3,a=1,b=1) =
  pow(max(1e-9, pow(abs(cos(m*t/4)/a),n2) + pow(abs(sin(m*t/4)/b),n3)), -1/n1);
function gielis(m,n1,n2,n3,sx=1,sy=1,a=1,b=1,n=120) =
  [ for (i=[0:n-1]) let(t=-360*i/n, r=gielis_r(t,m,n1,n2,n3,a,b))
      [ sx*r*cos(t), sy*r*sin(t) ] ];

function naca4(u,tc) = 5*tc*(0.2969*sqrt(u) - 0.1260*u - 0.3516*u*u
                            + 0.2843*u*u*u - 0.1036*u*u*u*u);
function foil(c,tc,n=24) =
  concat( [ for (i=[0:n])      let(u=0.5-0.5*cos(180*i/n)) [c*u,  c*naca4(u,tc)] ],
          [ for (i=[n-1:-1:1]) let(u=0.5-0.5*cos(180*i/n)) [c*u, -c*naca4(u,tc)] ] );

function spin2(sec,ang,about=[0,0]) =
  [ for (p=sec) let(q=p-about)
      about + [ q[0]*cos(ang)-q[1]*sin(ang), q[0]*sin(ang)+q[1]*cos(ang) ] ];
function scale2(sec,sx,sy) = [ for (p=sec) [p[0]*sx, p[1]*sy] ];
function move2(sec,d)      = [ for (p=sec) p+d ];
function mix2(A,B,t)       = [ for (i=[0:len(A)-1]) A[i]*(1-t) + B[i]*t ];

// ---- MYRING (1976) body of revolution -------------------------------
// The hull family fitted to fast-swimming fish and cetaceans, and the
// standard AUV form ever since.
//
//   0     <= x <= a      r = (d/2)[ 1 - ((x-a)/a)^2 ]^(1/n)
//   a     <  x <= a+b    r = d/2
//   xi = x-a-b, 0..c     r = R - (3R/c^2 - tan(th)/c) xi^2
//                            + (2R/c^3 - tan(th)/c^2) xi^3
//
// The cubic is the one satisfying all four conditions at once: r(0)=R
// and r'(0)=0 join the cylinder smoothly, r(c)=0 closes the tail, and
// r'(c) = -tan(th) makes th the true included half-angle at the tip.
function myring_r(x, d, a, b, c, n=2, th=20) =
    x <= 0   ? 0
  : x <= a   ? (d/2)*pow(max(0, 1 - pow((x-a)/a, 2)), 1/n)
  : x <= a+b ? d/2
  : let( R=d/2, xi=x-a-b )
      max(0, R - (3*R/(c*c) - tan(th)/c)*xi*xi
               + (2*R/(c*c*c) - tan(th)/(c*c))*xi*xi*xi);

// ---- icosahedral geodesic -------------------------------------------
// Edge length of this vertex set is exactly 2, which is how the faces
// are found rather than transcribed: a face is any triple whose three
// pairwise distances are all 2.  Winding is then fixed against the
// outward radial, so no triangle is listed by hand and none can be
// listed backwards.
function ico_v() = let(p=(1+sqrt(5))/2)
  [ [0,1,p],[0,-1,p],[0,1,-p],[0,-1,-p],
    [1,p,0],[-1,p,0],[1,-p,0],[-1,-p,0],
    [p,0,1],[-p,0,1],[p,0,-1],[-p,0,-1] ];
function ico_f() = let(V=ico_v())
  [ for (i=[0:9]) for (j=[i+1:10]) for (k=[j+1:11])
      if (abs(norm(V[i]-V[j])-2)<1e-6 && abs(norm(V[j]-V[k])-2)<1e-6
                                      && abs(norm(V[i]-V[k])-2)<1e-6)
        let( n = cross(V[j]-V[i], V[k]-V[i]) )
          (n*(V[i]+V[j]+V[k])) > 0 ? [i,k,j] : [i,j,k] ];

// Barycentric lattice point on a spherical triangle, projected out.
// i counts toward A, j toward B, the remainder toward C.
function geo_pt(A,B,C,f,i,j) = unit3( (i*A + j*B + (f-i-j)*C)/f );

// Triangle soup on the unit sphere at frequency f.  Upward and
// downward sub-triangles both inherit the parent's winding.  Vertices
// repeat along shared edges, which is harmless -- the mesh is drawn,
// not indexed.
function geo_tris(f=2) = let(V=ico_v(), F=ico_f())
  concat(
    [ for (t=F) let(A=V[t[0]],B=V[t[1]],C=V[t[2]])
        for (i=[0:f-1]) for (j=[0:f-1-i])
          [ geo_pt(A,B,C,f,i,j), geo_pt(A,B,C,f,i+1,j), geo_pt(A,B,C,f,i,j+1) ] ],
    f < 2 ? [] :
    [ for (t=F) let(A=V[t[0]],B=V[t[1]],C=V[t[2]])
        for (i=[0:f-2]) for (j=[0:f-2-i])
          [ geo_pt(A,B,C,f,i+1,j), geo_pt(A,B,C,f,i+1,j+1), geo_pt(A,B,C,f,i,j+1) ] ] );

module geo_ball(R, f=2) {
    T = geo_tris(f);
    polyhedron(points = [ for (tr=T) each [for(p=tr) p*R] ],
               faces  = [ for (k=[0:len(T)-1]) [3*k, 3*k+1, 3*k+2] ],
               convexity = 4);
}

// Unique edges and vertices, deduplicated by position.  O(n^2), so
// frequency 2 (240 raw edges) is comfortable and 3 is the practical
// ceiling.  The i==0 guard is load-bearing: [0:-1] is not an empty
// range here, it is a REVERSED one, so without the guard element zero
// is compared against itself, counted as its own duplicate, and
// dropped.  The tell is Euler -- 41 vertices and 119 edges instead of
// 42 and 120, which with 80 faces misses V-E+F=2 by exactly nothing
// you would notice by looking.
function geo_edges(f=2) =
  let( T = geo_tris(f),
       E = [ for (tr=T) each [ [tr[0],tr[1]], [tr[1],tr[2]], [tr[2],tr[0]] ] ],
       M = [ for (e=E) (e[0]+e[1])/2 ] )
    [ for (i=[0:len(E)-1])
        if (i==0 || len([ for (j=[0:i-1]) if (norm(M[j]-M[i])<1e-6) 1 ])==0) E[i] ];

function geo_pts(f=2) =
  let( P = [ for (tr=geo_tris(f)) each tr ] )
    [ for (i=[0:len(P)-1])
        if (i==0 || len([ for (j=[0:i-1]) if (norm(P[j]-P[i])<1e-6) 1 ])==0) P[i] ];

// ---- detail vocabulary ----------------------------------------------
module rod(P, Q, r, n=12) {
    d = Q-P; L = norm(d);
    if (L > 1e-9)
      translate(P) rotate([0, acos(clamp(d[2]/L,-1,1)), atan2(d[1],d[0])])
        cylinder(h=L, r=r, $fn=n);
}

// Engine bell: a flared nozzle of wall thickness w, throat rt, exit re.
module bell(rt, re, L, w=0.35, n=40, m=14) {
    P = concat( [ for (i=[0:m]) let(s=i/m) [lerp(rt,re,pow(s,1.7)), L*s] ],
                [ for (i=[m:-1:0]) let(s=i/m) [max(0.04, lerp(rt,re,pow(s,1.7))-w), L*s] ] );
    rotate_extrude($fn=n) polygon(P);
}

// Drive plume: stacked discs fading outward.  Emissive-looking without
// a shader -- the trick is that each disc is its own flat colour, so
// the stack reads as a gradient the lighting model cannot flatten.
module plume(r, L, col, k=7) {
    for (i=[0:k-1]) let(s=i/(k-1))
      color(col*(1-0.62*s)) translate([-L*s*0.999,0,0]) rotate([0,90,0])
        cylinder(h=0.001+L*0.06, r=r*(1-0.72*s), $fn=28);
}

// A swept fin from a root chord to a tip chord, with sweep and dihedral.
module fin(root_c, tip_c, span, sweep, tc=0.12, dih=0, n=18) {
    G = [ for (i=[0:n]) let(s=i/n,
             c  = lerp(root_c, tip_c, pow(s,0.75)),
             xo = sweep*s, yo = span*s, zo = span*s*tan(dih))
           [ for (p = foil(c, tc)) [ xo+p[0], yo, zo+p[1] ] ] ];
    smesh(G);
}

// ---- symmetry -------------------------------------------------------
// mirror() flips face winding correctly here, so a mirrored hull shades
// like the original rather than going inside out.  Note mirror([0,0,0])
// is an error, so the both-sides idiom needs two explicit calls, not a
// loop over [0,1].
module both_y() { children(); mirror([0,1,0]) children(); }
module both_z() { children(); mirror([0,0,1]) children(); }
module ring_of(n) { for (i=[0:n-1]) rotate([i*360/n,0,0]) children(); }

// ---- splitting a section for two-tone hulls --------------------------
// A closed section cut into two closed sub-sections sharing a chord.
// Each half sweeps as its own solid with its own colour and the shared
// face is interior, so countershading costs no boolean and leaves no
// coplanar pair to fight.  Both halves inherit the parent's winding.
function arc_of(sec, i0, i1) = let(n=len(sec))
  [ for (k=[0 : (i1-i0+n)%n]) sec[(i0+k)%n] ];
function span2(P, Q, m) = [ for (i=[1:m]) lerp(P, Q, i/(m+1)) ];
function part_of(sec, i0, i1, m=3) =
  concat(arc_of(sec,i0,i1), span2(sec[i1], sec[i0], m));

// ---- tubercled leading edge -----------------------------------------
// Humpback flippers carry a scalloped leading edge (Fish & Battle
// 1995); the bumps keep flow attached past the angle where a smooth
// edge stalls. amp is a fraction of local chord, k the count across
// the span.
module tfin(root_c, tip_c, span, sweep, tc=0.12, dih=0, amp=0.045, k=7, n=40) {
    G = [ for (i=[0:n]) let(s=i/n,
             c  = lerp(root_c, tip_c, pow(s,0.75)),
             bump = amp*c*cos(360*k*s),
             xo = sweep*s - bump, yo = span*s, zo = span*s*tan(dih))
           [ for (p = foil(c + bump, tc)) [ xo+p[0], yo, zo+p[1] ] ] ];
    smesh(G);
}
