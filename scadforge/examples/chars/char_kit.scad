// ===================================================================
//  CHAR_KIT -- the sweep and profile primitives the character models
//  need, carried here so chars/ is self-contained.
//
//  WHY A COPY.  include/use resolve a relative path against the
//  directory of the file holding the directive, and this kernel then
//  requires the result to stay under that model's own directory -- a
//  deliberate sandbox, since the web app renders whatever source it is
//  handed.  There is no library search path.  So chars/ cannot reach
//  ships/ship_lib.scad, and the include that tried to went unresolved:
//  every character rendered as a scatter of fragments with several
//  hundred "unknown function" warnings behind it.
//
//  These 17 definitions are the transitive closure of what the
//  characters actually call, lifted verbatim from ships/ship_lib.scad.
//  Fix them there and here together.
// ===================================================================

function unit3(v) = norm(v) < 1e-12 ? [0,0,1] : v/norm(v);

function lerp(a,b,t) = a + (b-a)*t;

function sstep(t) = let(x = max(0,min(1,t))) x*x*(3-2*x);

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

module both_y() { children(); mirror([0,1,0]) children(); }

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

