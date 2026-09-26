// ===================================================================
//  Left ventricle -- the idealised geometry, with its fibre helices
//
//  There is no equation for a heart.  A heart is what is left over
//  after a tube loops, septates and twists, and nobody designed it to
//  be parameterisable.  But ONE part of it is rule-governed enough to
//  write down, and cardiac mechanics has been writing it down for
//  decades: the left ventricle as a truncated PROLATE SPHEROID, and
//  the muscle fibres in its wall as helices whose pitch rotates
//  steadily from the inner surface to the outer.
//
//  Prolate spheroidal coordinates, focal length d:
//
//      x = d sinh(L) sin(M) cos(T)
//      y = d sinh(L) sin(M) sin(T)
//      z = -d cosh(L) cos(M)
//
//  Surfaces of constant L are confocal prolate spheroids.  The wall is
//  the shell between two of them, L_endo and L_epi, cut off by a plane
//  at the base.  M = 0 is the apex; T goes round.
//
//  Confocal is not a convenience -- it is the reason the model is worth
//  anything.  Two confocal spheroids are FURTHER APART at the equator
//  than at the pole, so the wall comes out thickest around the middle
//  and thinnest at the apex, which is what a left ventricle does.  It
//  is not fitted here.  It falls out of sharing a focus.
//
//  The fibres are the other half.  Myocardium is not a bag of muscle:
//  the fibres run helically, and the helix angle rotates through the
//  wall -- roughly +60 degrees at the inner surface to -60 at the
//  outer, the classic transmural measurements being Streeter's, around
//  1969.  That crossing is why a ventricle WRINGS as it contracts
//  instead of merely squeezing, and it is why ejection fraction is
//  what it is: shortening a fibre by 15 percent empties the cavity by
//  60, because the two families work against each other.
//
//  A fibre at angle A on a surface of constant L obeys
//
//      dT/dM = (h_M / h_T) cot(A),   h_M/h_T = sqrt(s^2 + sin^2 M)
//                                             / (s sin M),  s = sinh L
//
//  which has no elementary antiderivative, so it is integrated here.
//  Near the apex sin(M) -> 0 and the fibre spirals without bound; real
//  fibres do the same thing, converging into the apical vortex, and the
//  traces simply start below it.
//
//  What is NOT claimed: this is an idealised ventricle, not an
//  anatomical heart.  No right ventricle, no valves, no papillary
//  muscles, no trabeculation.  The dimensions are fitted to ordinary
//  adult end-diastolic figures, and the volumes that come out are
//  echoed below as a check on the fit rather than as an input to it.
// ===================================================================

LCAV = 80;         // cavity long axis, apex to base plane, mm
DCAV = 48;         // cavity diameter at its widest, mm
WALL = 10;         // wall thickness at the equator, mm
MUB  = 120;        // base truncation, degrees of M on the endocardium

NM   = 64;         // stations along a meridian
NT   = 96;         // panels around
MU0  = 21;         // where the fibre traces start, degrees of M

LAYERS = 6;        // transmural layers of fibre drawn (even: no 0-degree layer,
                   // where the fibre is a closed circle and cot blows up)
PERLAY = 2;        // fibres per layer
A_ENDO = 60;       // fibre helix angle at the inner surface, degrees
A_EPI  = -60;      // and at the outer
RFIB = 0.60;       // drawn fibre radius, mm
FINSET = 0.10;     // draw the outer layers this far inside the two surfaces,
                   // so a tube of finite radius stays within the wall
NS   = 132;        // stations along a fibre
NC   = 7;          // sides of the drawn fibre

MYO  = [0.72, 0.36, 0.34];
FIB  = [0.97, 0.88, 0.55];
FIB2 = [0.45, 0.72, 0.86];

// ---- the functions the kernel does not have -------------------------
function sinh_(x)  = (exp(x) - exp(-x))/2;
function cosh_(x)  = (exp(x) + exp(-x))/2;
function atanh_(x) = 0.5*ln((1+x)/(1-x));
function asinh_(x) = ln(x + sqrt(x*x + 1));

// ---- solving the geometry from the three measurements ---------------
// d cosh(L) (1 - cos MUB) = LCAV   and   d sinh(L) = DCAV/2
CZ    = LCAV/(1 - cos(MUB));            // = d cosh(L_endo)
LAM_E = atanh_((DCAV/2) / CZ);          // tanh L = (d sinh) / (d cosh)
D     = CZ / cosh_(LAM_E);              // focal length
LAM_P = asinh_(sinh_(LAM_E) + WALL/D);  // epicardium, WALL further out
ZBASE = -D*cosh_(LAM_E)*cos(MUB);       // the cutting plane

// Both surfaces are cut by the SAME plane, so each meets it at its own M.
function mu_max(lam) = acos(-ZBASE / (D*cosh_(lam)));

function P(lam, mu, th) =
  [  D*sinh_(lam)*sin(mu)*cos(th),
     D*sinh_(lam)*sin(mu)*sin(th),
    -D*cosh_(lam)*cos(mu) ];

// Outward normal of a constant-L surface, up to a positive factor.
function Nout(lam, mu, th) =
  let( v = [ cosh_(lam)*sin(mu)*cos(th),
             cosh_(lam)*sin(mu)*sin(th),
            -sinh_(lam)*cos(mu) ] ) v/norm(v);

// Faces are wound so the RIGHT-HAND normal points INTO the solid, which
// is the convention polyhedron() takes: a face is listed clockwise as
// seen from outside. Wound the other way the solid is inside-out --
// which renders identically, because shading uses |n|, and then quietly
// ruins every boolean it touches. It cost three models before it showed.

// ---- a solid of revolution whose profile ends on the axis -----------
module revolve(prof, n, conv = 6) {
    M = len(prof);
    pts = concat(
      [ [0, 0, prof[0][1]] ],
      [ for (i = [1 : M-2]) each
          [ for (j = [0:n-1]) let(a = 360*j/n)
              [ prof[i][0]*cos(a), prof[i][0]*sin(a), prof[i][1] ] ] ],
      [ [0, 0, prof[M-1][1]] ]);
    TOP = 1 + (M-2)*n;
    polyhedron(
      points = pts,
      faces = concat(
        [ for (j = [0:n-1]) [ 0, 1 + j, 1 + (j+1)%n ] ],
        [ for (i = [1 : M-3]) for (j = [0:n-1])
            [ 1+(i-1)*n+j, 1+i*n+j, 1+i*n+(j+1)%n, 1+(i-1)*n+(j+1)%n ] ],
        [ for (j = [0:n-1]) [ TOP, 1+(M-3)*n+(j+1)%n, 1+(M-3)*n+j ] ]),
      convexity = conv);
}

// The wall's cross-section: up the outside, across the base, down the
// inside.  Both ends sit on the axis, so revolving it closes the solid.
MP = mu_max(LAM_P);
ME = mu_max(LAM_E);
function rz(lam, mu) = [ D*sinh_(lam)*sin(mu), -D*cosh_(lam)*cos(mu) ];

PROFILE = concat(
    [ for (i = [0:NM])  rz(LAM_P, MP*i/NM) ],
    [ for (i = [NM:-1:0]) rz(LAM_E, ME*i/NM) ]);

color(MYO) revolve(PROFILE, NT);

// ---- fibres ---------------------------------------------------------
// dT/dM for unit cot(A); the angular units cancel, so M and T may both
// stay in degrees.
function dTdM(lam, mu) =
  let( s = sinh_(lam) ) sqrt(s*s + sin(mu)*sin(mu)) / (s*sin(mu));

function sum(v, a = 0, b = -1) =
  let( hi = b < 0 ? len(v) : b )
    hi - a <= 0 ? 0
  : hi - a == 1 ? v[a]
  : let( m = floor((a + hi)/2) ) sum(v, a, m) + sum(v, m, hi);

// e is the true transmural fraction: 0 at the endocardium, 1 at the
// epicardium.  Both the layer's surface AND its fibre angle come from
// the same e, so insetting the drawing does not falsify the angle -- it
// just means the outermost drawn fibre is at 96 percent of the wall and
// carries the angle that belongs there, rather than the +-60 that
// belongs to the surfaces themselves.
function e_of(l)     = FINSET + (1 - 2*FINSET)*l/(LAYERS-1);
function lam_of(l)   = LAM_E + (LAM_P - LAM_E)*e_of(l);
function angle_of(l) = A_ENDO + (A_EPI - A_ENDO)*e_of(l);
function mu_hi(l)   = mu_max(lam_of(l));
function mu_at(l, i) = MU0 + (mu_hi(l) - MU0)*i/NS;

// The integrand sampled once per layer, so the running integral is a
// range-sum over a list that already exists rather than a fresh one per
// station: 132 stations over a list summed by halving, not a chain of
// 132 additions.
STEP = [ for (l = [0:LAYERS-1])
           let( dm = (mu_hi(l) - MU0)/NS )
           [ for (i = [0:NS-1]) dTdM(lam_of(l), (mu_at(l,i) + mu_at(l,i+1))/2)*dm ] ];

function th_at(l, i, th0) = th0 + sum(STEP[l], 0, i) / tan(angle_of(l));
function fib_pt(l, i, th0) = P(lam_of(l), mu_at(l,i), th_at(l,i,th0));

// A tube along the fibre, framed on the surface it lies in: the local
// normal and its cross with the tangent.  No transport needed -- the
// surface supplies the frame.
function fib_ring(l, i, th0) =
  let( a = fib_pt(l, max(0, i-1), th0),
       b = fib_pt(l, min(NS, i+1), th0),
       t = (b - a)/norm(b - a),
       nn = Nout(lam_of(l), mu_at(l,i), th_at(l,i,th0)),
       bb = cross(t, nn) )
  [ for (k = [0:NC-1]) let( g = 360*k/NC )
      fib_pt(l, i, th0) + RFIB*(cos(g)*nn + sin(g)*bb/norm(bb)) ];

module tube(l, th0) {
    grid = [ for (i = [0:NS]) fib_ring(l, i, th0) ];
    MU_ = len(grid) - 1;
    pts = concat([ for (u = [0:MU_]) each grid[u] ],
                 [ fib_pt(l, 0, th0) ], [ fib_pt(l, NS, th0) ]);
    B0 = (MU_+1)*NC; B1 = B0 + 1;
    polyhedron(points = pts,
      faces = concat(
        [ for (u = [0:MU_-1]) for (v = [0:NC-1])
            [ u*NC+v, (u+1)*NC+v, (u+1)*NC+(v+1)%NC, u*NC+(v+1)%NC ] ],
        [ for (v = [0:NC-1]) [ B0, (v+1)%NC, v ] ],
        [ for (v = [0:NC-1]) [ B1, MU_*NC + v, MU_*NC + (v+1)%NC ] ]),
      convexity = 4);
}

for (l = [0:LAYERS-1])
  for (f = [0:PERLAY-1])
    color(angle_of(l) >= 0 ? FIB : FIB2)
      tube(l, 360*f/PERLAY + 37*l);

// ---- what the geometry says -----------------------------------------
echo("focal length d", D, "L_endo", LAM_E, "L_epi", LAM_P);
echo("cavity: long axis", LCAV, "widest", DCAV, " base plane at z", ZBASE);
echo("base M: endo", ME, "epi", MP, "degrees");

// Wall thickness is not uniform, and was not asked to be.
echo("wall at equator", D*(sinh_(LAM_P) - sinh_(LAM_E)),
     " at apex", D*(cosh_(LAM_P) - cosh_(LAM_E)), "mm");

// Volume under a constant-L surface, truncated at M = mb:
//   V = pi d^3 sinh^2 cosh [ 2/3 - cos(mb) + cos^3(mb)/3 ]
function cap_vol(lam, mb) =
  PI*D*D*D*sinh_(lam)*sinh_(lam)*cosh_(lam)
    * (2/3 - cos(mb) + cos(mb)*cos(mb)*cos(mb)/3);

VCAV  = cap_vol(LAM_E, ME);
VWALL = cap_vol(LAM_P, MP) - VCAV;
echo("cavity volume", VCAV/1000, "mL   wall volume", VWALL/1000, "mL");
echo("myocardial mass at 1.05 g/mL", VWALL/1000*1.05, "g");

echo("drawn at wall fractions", [ for (l = [0:LAYERS-1]) e_of(l) ]);
echo("fibre angles there", [ for (l = [0:LAYERS-1]) angle_of(l) ],
     " at the surfaces themselves", [A_ENDO, A_EPI]);
echo("turns made by the innermost fibre",
     (th_at(0, NS, 0) - th_at(0, 0, 0))/360,
     " outermost", (th_at(LAYERS-1, NS, 0) - th_at(LAYERS-1, 0, 0))/360);
