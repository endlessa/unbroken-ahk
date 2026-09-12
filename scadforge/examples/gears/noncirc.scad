// ===================================================================
//  noncirc.scad -- elliptical (non-circular) gear pair
//
//  This is the one that does not survive being drawn by hand.  On a
//  round gear every tooth is the same tooth, rotated.  On a non-circular
//  gear NO TWO TEETH ARE THE SAME: the pitch curve is an ellipse taken
//  about its FOCUS, so the radius, the local curvature and the angular
//  pitch all change tooth by tooth.
//
//  Two things have to be solved numerically or it will not mesh:
//    1. teeth must be spaced by equal ARC LENGTH along the pitch curve,
//       not by equal angle -- so the pitch curve is integrated and the
//       arc-length function inverted for every tooth;
//    2. each tooth is cut as the involute of ITS OWN local curvature
//       circle, so every tooth has a different base radius.
//
//  Mounted on its focus and paired with an identical gear at centre
//  distance 2a, the pair rolls with a 1:1 average ratio but a strongly
//  cyclic instantaneous ratio -- a mechanical function generator.
// ===================================================================

function ell_r(th,a,e)  = a*(1-e*e)/(1 + e*cos(th));
function ell_p(th,a,e)  = ell_r(th,a,e)*[cos(th), sin(th)];

NS = 1440;                                   // pitch-curve integration steps
function ell_seg(i,a,e) = norm(ell_p(360*(i+1)/NS,a,e) - ell_p(360*i/NS,a,e));
function cumsum(v,i=0,acc=[0]) = i >= len(v) ? acc : cumsum(v,i+1,concat(acc,[acc[i]+v[i]]));
function ell_cum(a,e) = cumsum([for (i=[0:NS-1]) ell_seg(i,a,e)]);

// invert the arc-length function: the grid index whose cumulative length
// brackets s, then linear interpolation inside that step
function s_index(cum,s) = [ for (i=[0:NS-1]) if (cum[i] <= s && cum[i+1] > s) i ][0];
function s_theta(cum,s) =
    let(i = s_index(cum,s), f = (s - cum[i])/(cum[i+1] - cum[i]))
      360*(i + f)/NS;

// local geometry at th, by central difference on the pitch curve
DTH = 0.35;
function ell_tan(th,a,e) = (ell_p(th+DTH,a,e) - ell_p(th-DTH,a,e))/2;
function ell_acc(th,a,e) = ell_p(th+DTH,a,e) - 2*ell_p(th,a,e) + ell_p(th-DTH,a,e);
function ell_kappa(th,a,e) =
    let(d = ell_tan(th,a,e), dd = ell_acc(th,a,e))
      (d[0]*dd[1] - d[1]*dd[0]) / pow(d*d, 1.5);
function ell_rho(th,a,e) = 1/max(1e-9, abs(ell_kappa(th,a,e)));

// one tooth, cut as the involute of the local curvature circle
NIV = 12;
function nc_tooth(th,a,e,m,al) =
  let( P   = ell_p(th,a,e),
       d   = ell_tan(th,a,e), t = d/norm(d),
       nout= [t[1], -t[0]],                        // outward normal
       kap = ell_kappa(th,a,e),
       rho = ell_rho(th,a,e),
       cvx = kap < 0,                              // convex toward outside
       C   = cvx ? P - nout*rho : P + nout*rho,    // centre of curvature
       zq  = 2*rho/m,                              // equivalent tooth count
       be  = atan2(P[1]-C[1], P[0]-C[0]),
       ra  = cvx ? rho + m    : rho - m,
       rf  = cvx ? rho - 1.3*m: rho + 1.3*m,
       sgn = cvx ? 1 : -1,
       fl  = [ for (i=[0:NIV]) let(r = rho + sgn*(-1.3*m + 2.3*m*i/NIV),
                                   ph = sgn*g_phi(cvx ? r : 2*rho - r, m, zq, al, 0))
                 C + r*[cos(be - ph), sin(be - ph)] ],
       fr = [ for (i=[NIV:-1:0]) let(r = rho + sgn*(-1.3*m + 2.3*m*i/NIV),
                                   ph = sgn*g_phi(cvx ? r : 2*rho - r, m, zq, al, 0))
                 C + r*[cos(be + ph), sin(be + ph)] ] )
    concat(fl, fr);

// the whole gear: teeth on equal arc length, joined by the root curve
// ph = 0 for the driver, 0.5 for its mate: the mating gear has to be
// offset by HALF A TOOTH measured in arc length, not in angle -- on a
// non-circular pitch curve those are not the same thing.
function nc_gear(a,e,N,m,al=20,ph=0) =
  let( cum = ell_cum(a,e), S = cum[NS],
       ths = [ for (k=[0:N-1]) s_theta(cum, S*(k+ph)/N) ] )
    [ for (k=[0:N-1]) each
        concat( nc_tooth(ths[k],a,e,m,al),
                [ for (j=[1:3])
                    let(t0 = ths[k], t1 = k==N-1 ? ths[0]+360 : ths[k+1],
                        th = t0 + (t1-t0)*j/4,
                        P = ell_p(th,a,e), d = ell_tan(th,a,e), t = d/norm(d))
                      P - [t[1],-t[0]]*1.3*m ] ) ];
