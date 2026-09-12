// ===================================================================
//  cycloidal drive -- an epitrochoid disc rolling inside a ring of
//  pins.  One tooth fewer than there are pins, so one full turn of the
//  eccentric advances the disc by exactly one pin: the reduction is
//  N:1 from a single stage, with every pin in contact at once.
// ===================================================================
function cyc_psi(t,N,R,E,rp) =
    -atan2( sin((1-N)*t), (R/(E*N)) - cos((1-N)*t) );
function cyc_pt(t,N,R,E,rp) =
    let(ps = cyc_psi(t,N,R,E,rp))
      [  R*cos(t) - rp*cos(t+ps) - E*cos(N*t),
        -R*sin(t) + rp*sin(t+ps) + E*sin(N*t) ];
// The profile is traced clockwise, and linear_extrude wants CCW.
// Also: the eccentricity has a hard ceiling, E < R/N. Past it the
// epitrochoid loops over itself and the disc stops being a simple
// closed curve at all -- it is not a cosmetic limit, it is the
// condition for the curve to exist.
function cyc_valid(N,R,E) = E < R/N;
function cyc_disc(N,R,E,rp,NP=900) =
    [ for (i=[NP-1:-1:0]) cyc_pt(360*i/NP,N,R,E,rp) ];
