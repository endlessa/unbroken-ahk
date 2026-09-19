include <char_kit.scad>
// ===================================================================
//  char_lib.scad -- parameterised humanoid, shared by every species
//
//  Faces +x, up +z, the character's LEFT is +y.  Metres, feet on z = 0.
//
//  INCLUDE PATHS resolve against the SERVER's working directory here,
//  not the file's, so build a flat copy to render:
//      cat ../ships/ship_lib.scad  >  /tmp/c.scad
//      grep -v '^include' char_lib.scad >> /tmp/c.scad
//      grep -v '^include' orc.scad     >> /tmp/c.scad
//
//  WHY A SHARED BASE.  Five species drawn as five separate files become
//  five versions of whoever was drawn first, wearing different hats.
//  The thing that actually separates a gnome from an orc is not the
//  ears, it is PROPORTION -- and proportion is only legible when the
//  five are expressed in the same terms and can be read off against
//  each other.  So every body here is one profile vector, and the
//  species files add only what proportion cannot express.
//
//  THE PROFILE, and what each number does to the silhouette:
//    H      standing height, metres.
//    heads  how many head-heights tall.  The classic canon device and
//           the strongest single lever: 4.2 reads childlike-and-clever,
//           8.5 reads inhumanly elegant, and nothing else you do to a
//           figure will override it.
//    shld   shoulder HALF-breadth as a fraction of H.  Torso only --
//           the arms hung off it carry the true breadth further.
//    hipf   hip height as a fraction of H.  Leg length, effectively:
//           the difference between 0.42 and 0.54 is the difference
//           between a dwarf and an elf even at identical height.
//    girth  limb and torso thickness, 1.0 = human.  Mass, not scale.
//    armf   fingertip height as a fraction of H.  LOWER means LONGER
//           arms -- 0.26 puts an orc's knuckles near its knees.
//    lean   forward spine lean in degrees, applied about the hip.
//    dep    chest depth as a multiple of shoulder half-breadth.
//
//  All the section ratios below were taken off the dwarf, which was
//  built by hand first, and divided by its own shld*H*girth.  So the
//  dwarf is the calibration figure: feed it its own profile and it
//  comes back out.
//
//  TWO RULES that make the joins work, learned the hard way on the
//  dwarf and worth restating because every species depends on them:
//    1. Every limb STARTS INSIDE its parent.  The arm's first station
//       sits in the ribcage, the head's in the chest, the boot's in the
//       shin.  A buried cap is interior geometry and cannot show a
//       seam, so no join ever needs hiding.
//    2. Limbs sweep on pt_N (parallel transport), never a fixed up
//       vector.  frame_up(S,i,[0,0,1]) is DEGENERATE on a leg: the
//       tangent is vertical, cross(up,T) is zero, and the section
//       collapses to a line.
//
//  MIRRORING IS SAFE -- measured, not assumed.  mirror() reverses face
//  order in this kernel, so a mirrored sweep keeps a positive signed
//  volume and stays correctly lit.  Every paired limb is built once on
//  the +y side and mirrored.
// ===================================================================

function prof(H, heads, shld, hipf, girth, armf, lean, dep) =
    [H, heads, shld, hipf, girth, armf, lean, dep];

function pH(P)     = P[0];
function pHEADS(P) = P[1];
function pSH(P)    = P[2]*P[0];          // shoulder half-breadth, absolute
function pHIP(P)   = P[3]*P[0];          // hip height, absolute
function pGIRTH(P) = P[4];
function pARM(P)   = P[5]*P[0];          // fingertip height, absolute
function pLEAN(P)  = P[6];
function pDEP(P)   = P[7];

function pHH(P)    = P[0]/P[1];          // head height, chin to crown
function pCHIN(P)  = P[0] - pHH(P);
function pSHZ(P)   = pCHIN(P) - 0.055*pHH(P) - 0.030*P[0];   // acromion
function pT(P)     = pSH(P)*pGIRTH(P);   // the thickness unit for limbs

// Lean pivots about the hip, so the legs stay plumb and only the trunk,
// arms and head tip forward.  An orc that leans by translating instead
// just ends up standing in a hole.
function tip(P, p) = let( a = pLEAN(P), h = pHIP(P), dz = p[2]-h )
    [ p[0]*cos(a) + dz*sin(a), p[1], h - p[0]*sin(a) + dz*cos(a) ];

// The head follows the lean in POSITION but only a quarter of it in
// ORIENTATION: a stooped creature still looks where it is going.  The
// orc at lean 9 showed the camera the top of its skull until this
// existed, because a rotation about the hip tips the face down by the
// full lean angle.  Rotating back about the neck base keeps the head
// where the spine put it and brings the face up.
function tipH(P, p) =
  let( b = -pLEAN(P)*0.75,
       n = tip(P, [0, 0, pCHIN(P) - 0.60*pHH(P)]),
       q = tip(P, p), d = q - n )
    n + [ d[0]*cos(b) + d[2]*sin(b), d[1], -d[0]*sin(b) + d[2]*cos(b) ];

function ortho(N0, T) = unit3(N0 - T*(T*N0));

function limb_G(S, W, D, E, N0, n=20, d=0) =
  let( M0 = ortho(N0, tang(S,0)) )
    [ for (i=[0:len(S)-1])
        let( T = tang(S,i), N = pt_N(S,i,M0), B = cross(T,N) )
          [ for (p = lame(W[i]+d, D[i]+d, E[i], n)) S[i] + N*p[0] + B*p[1] ] ];

// ---- torso -----------------------------------------------------------
// Stations are fractions of the hip-to-shoulder run; -0.12 is below the
// hip, inside the thighs.  The widths are multiples of the shoulder
// half-breadth and the depths of that again times dep, so a species
// changes its whole trunk with two numbers.
TOR_F = [ -0.12, 0.18, 0.34, 0.56, 0.80, 0.95, 1.06 ];
TOR_RW= [ 0.64, 0.59, 0.61, 0.76, 0.92, 1.00, 0.81 ];
TOR_RD= [ 0.50, 0.47, 0.48, 0.59, 0.66, 0.61, 0.50 ];
TOR_E = [ 2.60, 2.80, 3.00, 3.00, 2.90, 2.80, 2.60 ];

function tor_S(P) = let( h = pHIP(P), s = pSHZ(P), r = s-h )
  [ for (i=[0:6]) tip(P, [ 0.015*pH(P)*sin(180*TOR_F[i]) , 0, h + r*TOR_F[i] ]) ];
function tor_G(P, d=0) = let( SH = pSH(P) )
  limb_G(tor_S(P),
         [ for (r=TOR_RW) r*SH ],
         [ for (r=TOR_RD) r*SH*pDEP(P)/0.66 ],
         TOR_E, [0,1,0], 28, d);

// ---- legs ------------------------------------------------------------
LEG_FZ= [ 1.13, 0.88, 0.695, 0.53, 0.355, 0.238 ];   // as fractions of hip height
LEG_RW= [ 0.364, 0.342, 0.294, 0.294, 0.219, 0.190 ];
LEG_RD= [ 0.372, 0.357, 0.320, 0.331, 0.234, 0.208 ];
LEG_E = [ 2.60, 2.60, 2.70, 2.70, 2.80, 2.90 ];

function leg_S(P) = let( h = pHIP(P), y = 0.48*pSH(P) )
  [ for (i=[0:5]) [ 0.006*pH(P)*(i==2?1.4:1), y*(1 - 0.10*i/5), h*LEG_FZ[i] ] ];
function leg_G(P, d=0) = let( T = pT(P) )
  limb_G(leg_S(P), [ for (r=LEG_RW) r*T ], [ for (r=LEG_RD) r*T ], LEG_E, [0,1,0], 18, d);

// The boot sweeps with N0 = +x so its local x runs fore-aft, which is
// what lame4 needs: the toe grows forward while the heel stays put.
BOOT_FZ= [ 0.151, 0.103, 0.059, 0.024, 0.006 ];      // fractions of H
BOOT_RF= [ 0.245, 0.320, 0.416, 0.513, 0.491 ];
BOOT_RA= [ 0.245, 0.283, 0.320, 0.342, 0.327 ];
BOOT_RH= [ 0.238, 0.290, 0.320, 0.312, 0.286 ];
BOOT_E = [ 3.00, 3.20, 3.40, 3.60, 3.60 ];

function boot_S(P) = let( y = 0.48*pSH(P)*0.90 )
  [ for (i=[0:4]) [ 0.006*pH(P)*i/4, y, BOOT_FZ[i]*pH(P) ] ];
function boot_G(P, d=0) =
  let( S = boot_S(P), T = pT(P), M0 = ortho([1,0,0], tang(S,0)) )
    [ for (i=[0:4])
        let( Tg = tang(S,i), N = pt_N(S,i,M0), B = cross(Tg,N) )
          [ for (p = lame4(BOOT_RF[i]*T+d, BOOT_RA[i]*T+d, BOOT_RH[i]*T+d, BOOT_E[i], 20))
              S[i] + N*p[0] + B*p[1] ] ];

// ---- arms ------------------------------------------------------------
// Station 0 is inside the ribcage.  The rhythm matters more than the
// length: upper arm slim, elbow pinched, forearm swollen PAST the upper
// arm.  That reversal is what stops an arm reading as a tube.
ARM_RW= [ 0.320, 0.312, 0.264, 0.245, 0.275, 0.223 ];
ARM_RD= [ 0.320, 0.312, 0.271, 0.260, 0.290, 0.238 ];
ARM_E = [ 2.60, 2.60, 2.60, 2.60, 2.70, 2.80 ];
ARM_T = [ 0.00, 0.16, 0.44, 0.72, 1.00, 1.24 ];      // along shoulder->wrist

function arm_S(P) =
  let( sz = pSHZ(P), wz = pARM(P) + 0.055*pH(P), sy = pSH(P),
       run = sz - wz )
    [ for (i=[0:5]) tip(P, [ 0.012*pH(P)*ARM_T[i]*ARM_T[i],
                             sy*(0.88 + 0.28*sin(min(90, 96*ARM_T[i]))),
                             sz - run*ARM_T[i]/1.24 ]) ];
function arm_G(P, d=0) = let( T = pT(P) )
  limb_G(arm_S(P), [ for (r=ARM_RW) r*T ], [ for (r=ARM_RD) r*T ], ARM_E, [1,0,0], 18, d);

HAND_RW= [ 0.212, 0.219, 0.197, 0.156, 0.108 ];
HAND_RD= [ 0.230, 0.268, 0.260, 0.212, 0.149 ];
HAND_E = [ 2.80, 3.00, 3.00, 2.90, 2.80 ];

function hand_S(P) =
  let( A = arm_S(P), w = A[5], v = unit3(w - A[4]) )
    [ for (i=[0:4]) w + v*(0.055*pH(P)*i/4) - [0,0,0.010*pH(P)*0] ];
function hand_G(P, d=0) = let( T = pT(P) )
  limb_G(hand_S(P), [ for (r=HAND_RW) r*T ], [ for (r=HAND_RD) r*T ], HAND_E, [1,0,0], 16, d);

// ---- head ------------------------------------------------------------
// Stations are offsets from the chin in head-heights; -0.594 is down
// inside the chest, so the neck emerges rather than being stuck on.
// jaw, brow and muzzle let a species reshape the skull without
// redrawing it: jaw widens stations 2-3, brow widens 4, muzzle pushes
// the front of 2-3 forward.
HEAD_FZ= [ -0.594, -0.230, 0.000, 0.249, 0.536, 0.785, 0.939, 0.985 ];
HEAD_RW= [ 0.276, 0.291, 0.387, 0.433, 0.437, 0.387, 0.245, 0.092 ];
HEAD_RD= [ 0.284, 0.307, 0.421, 0.456, 0.444, 0.391, 0.253, 0.096 ];
HEAD_E = [ 2.60, 2.60, 2.80, 2.90, 2.90, 2.80, 2.60, 2.40 ];
HEAD_JW= [ 0, 0.15, 1.00, 0.70, 0.15, 0, 0, 0 ];     // where "jaw" applies
HEAD_BW= [ 0, 0, 0.10, 0.45, 1.00, 0.35, 0, 0 ];     // where "brow" applies

function head_S(P, muzzle=0) =
  let( c = pCHIN(P), hh = pHH(P) )
    [ for (i=[0:7]) tipH(P, [ -0.004*pH(P) + muzzle*hh*HEAD_JW[i]*0.5, 0,
                              c + hh*HEAD_FZ[i] ]) ];
function head_G(P, d=0, jaw=1, brow=1, muzzle=0, slim=1) = let( hh = pHH(P) )
  limb_G(head_S(P, muzzle),
         [ for (i=[0:7]) hh*HEAD_RW[i]*slim*(1 + (jaw-1)*HEAD_JW[i] + (brow-1)*HEAD_BW[i]) ],
         [ for (i=[0:7]) hh*HEAD_RD[i]*(1 + (jaw-1)*HEAD_JW[i]*0.5 + (brow-1)*HEAD_BW[i]*0.4) ],
         HEAD_E, [0,1,0], 24, d);

// WHERE THE FACE ACTUALLY IS.  A species reshapes its skull with the
// jaw, brow and muzzle knobs, and those move the front surface by a lot:
// the orc's brow line ends up at x = 0.17 where the elf's is 0.09.  So a
// nose or an eye placed at a fixed fraction of head height is buried
// inside the skull on one species and floating in front of it on
// another -- which is exactly what happened on the first pass, where the
// orc had no visible face at all and the human's nose barely cleared.
// Features are placed against these two instead.
function head_front(P, i, jaw=1, brow=1, muzzle=0) =
  let( hh = pHH(P) )
    -0.004*pH(P) + muzzle*hh*HEAD_JW[i]*0.5
    + hh*HEAD_RD[i]*(1 + (jaw-1)*HEAD_JW[i]*0.5 + (brow-1)*HEAD_BW[i]*0.4);
function head_half(P, i, jaw=1, brow=1, slim=1) =
  let( hh = pHH(P) )
    hh*HEAD_RW[i]*slim*(1 + (jaw-1)*HEAD_JW[i] + (brow-1)*HEAD_BW[i]);
function head_z(P, i) = pCHIN(P) + pHH(P)*HEAD_FZ[i];

// Every facial feature lives BETWEEN head stations 3 (the jaw) and 4
// (the brow) -- none of them sits on a station.  head_front() reports
// one station only, and on a muzzled species the two are four
// centimetres apart, so placing an eye or a nose against either one
// alone buries it in the skull or floats it in front.  face_t() converts
// a height given as an offset from the brow station, in head-heights,
// into the fraction between the two; face_x() and face_w() then give the
// interpolated surface and half-width there.
function face_t(zoff) = (HEAD_FZ[4] + zoff - HEAD_FZ[3]) / (HEAD_FZ[4] - HEAD_FZ[3]);
function face_x(P, t, jaw=1, brow=1, muzzle=0) =
    lerp(head_front(P,3,jaw,brow,muzzle), head_front(P,4,jaw,brow,muzzle), t);
function face_w(P, t, jaw=1, brow=1, slim=1) =
    lerp(head_half(P,3,jaw,brow,slim), head_half(P,4,jaw,brow,slim), t);

// ---- shared features -------------------------------------------------
// Hair, as the REAR arc of each head section rather than a full ring.
// A band that stops at one station leaves a clean horizontal edge across
// the forehead, and a clean horizontal edge across a forehead is the
// brim of a knitted cap however the colour is tuned.  part_of() takes an
// arc instead, so the face stays open and the front edges fall
// vertically beside the temples the way a hairline does.  The chord that
// closes each arc runs through the skull's interior and is never seen.
// head_G sections index 0 = left, 6 = front, 12 = right, 18 = back, so
// 9..3 is the wrap that excludes the face.
module hair_shell(P, u0, u1, d=0.010, i0=9, i1=3, jaw=1, brow=1, muzzle=0, slim=1) {
    sband([ for (g = head_G(P,d,jaw,brow,muzzle,slim)) part_of(g, i0, i1, 3) ], u0, u1);
}

// A pauldron: a plate swept over the deltoid on an arc in the y-z plane.
// The fore-aft length falls away at BOTH ends of that arc, via a half
// sine, or the plate is a constant-width band -- which reads as a hook
// from the front and a bucket from the side.
function pld_S(P, r, n=7, a0=34, a1=124) =
  [ for (i=[0:n]) tip(P, [ 0, pSH(P)*1.02 + r*sin(a0 + (a1-a0)*i/n),
                           pSHZ(P) + 0.010*pH(P) + r*cos(a0 + (a1-a0)*i/n) ]) ];
function pld_G(P, r, len, thk, d=0, n=7, a0=34, a1=124) =
  let( S = pld_S(P,r,n,a0,a1), M0 = ortho([1,0,0], tang(S,0)) )
    [ for (i=[0:n])
        let( T = tang(S,i), N = pt_N(S,i,M0), B = cross(T,N), t = i/n )
          [ for (p = lame(len*(0.44 + 0.56*sin(180*pow(t,0.8))) + d,
                          thk*(1 + 0.42*sstep(t)) + d, 3.0, 18))
              S[i] + N*p[0] + B*p[1] ] ];

// A pointed ear, swept from the skull outward and back.  curl bends it
// up and aft, which is the whole difference between an elf's and a
// gnome's: same part, different sign and length.
function ear_S(P, z, out, len, rise, sweep) =
  let( hh = pHH(P) )
    [ for (i=[0:5]) let( t = i/5 )
        tipH(P, [ -0.02*hh - sweep*hh*t*t, out*hh + len*hh*t, z + rise*hh*t*t ]) ];
function ear_G(P, z, out, len, rise, sweep, w0, w1, d=0) = let( hh = pHH(P) )
  limb_G(ear_S(P,z,out,len,rise,sweep),
         [ for (i=[0:5]) hh*(w0 + (w1-w0)*i/5) ],
         [ for (i=[0:5]) hh*(w0 + (w1-w0)*i/5)*2.1*(1 - 0.45*i/5) ],
         [ 2.4, 2.4, 2.5, 2.5, 2.6, 2.6 ], [0,1,0], 12, d);

// A beard lobe, built on the +y side and mirrored so the two halves meet
// on the median plane.  That gives the central parting and the forked
// tip for free.  The spine must travel FORWARD as it descends because
// the chest it lies on does too -- held at constant x, the bottom half
// ends up inside the ribcage and the beard looks sheared off at the
// collarbone.
function brd_G(P, len, wide, fwd, d=0) =
  let( hh = pHH(P), c = pCHIN(P), drop = len*hh,
       S = [ for (i=[0:5]) let( t = i/5 )
               tipH(P, [ 0.11*hh + fwd*hh*sin(115*t), hh*(0.20 + 0.07*sin(180*t)),
                         c + 0.22*hh - drop*t ]) ] )
    limb_G(S,
      [ for (i=[0:5]) let(t=i/5) hh*wide*(0.86 + 0.14*sin(180*min(1,t*1.3)) - 0.55*t*t) ],
      [ for (i=[0:5]) let(t=i/5) hh*wide*0.78*(0.90 + 0.10*sin(180*min(1,t*1.3)) - 0.52*t*t) ],
      [ 2.6, 2.7, 2.8, 2.8, 2.7, 2.6 ], [0,1,0], 20, d);

// ---- assembled body --------------------------------------------------
// Colours are passed in so a species never has to restate the geometry
// just to change a garment.
module body(P, skin, cloth, cloth2, mail, lthr, lthr2, plate, steel, brass) {
    color(cloth) smesh(tor_G(P));
    color(mail)  sband(tor_G(P, 0.010*pH(P)/1.358), 2, 6);
    color(lthr)  sband(tor_G(P, 0.018*pH(P)/1.358), 1, 2);
    color(brass) translate(tip(P, [pSH(P)*0.50, 0, pHIP(P) + 0.118*(pSHZ(P)-pHIP(P))]))
        scale([0.5,1,1]) sphere(r = 0.034*pH(P)/1.358*1.35, $fn = 20);
    both_y() {
        color(cloth2) smesh(leg_G(P));
        color(lthr2)  sband(leg_G(P, 0.008*pH(P)/1.358), 2, 3);
        color(lthr)   smesh(boot_G(P));
        color(plate)  sband(boot_G(P, 0.009*pH(P)/1.358), 1, 2);
        color(lthr2)  sband(boot_G(P, 0.004*pH(P)/1.358), 3, 4);
        color(cloth)  smesh(arm_G(P));
        color(mail)   sband(arm_G(P, 0.008*pH(P)/1.358), 1, 2);
        color(lthr)   sband(arm_G(P, 0.010*pH(P)/1.358), 4, 5);
        color(skin)   smesh(hand_G(P));
    }
}

module pauldrons(P, plate, steel, brass, r=0.062, len=0.068, thk=0.011) {
    S = pH(P)/1.358;
    both_y() {
        color(plate) smesh(pld_G(P, r*S, len*S, thk*S));
        color(steel) sband(pld_G(P, r*S, len*S, thk*S, 0.005*S), 5, 7);
    }
}
