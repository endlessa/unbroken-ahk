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
       trok = [ for (p = tro) if (norm(p) <= rjoin) p ],
       rs   = len(trok) > 0 ? max(rjoin, norm(trok[len(trok)-1])) : rjoin,
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
