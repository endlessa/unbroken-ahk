//! Degree trigonometry, shared by the evaluator and the geometry kernel.
//!
//! Every angle in the language is in degrees, and the reference pins the
//! forward functions to EXACT results wherever the angle reduces to a
//! multiple of 30 or 45: `sin(180) == 0` rather than 1.2e-16, `cos(60) ==
//! 0.5`, `tan(45) == 1`. That exactness is observable — golden echo tests
//! read it, and `rotate(90)` is specified to yield a matrix of exact 0 and
//! ±1 entries "with no 6.1e-17 dust", which is only true if the matrices
//! come through here too.

/// Total-precision-loss guard from the reference: |angle| >= 2^52 * 360
/// returns NaN for sin/cos/tan.
const TRIG_MAX: f64 = 4_503_599_627_370_496.0 * 360.0; // 2^52 * 360

/// The exact sine of an angle that reduces to a multiple of 30 or 45
/// degrees, or None when it does not.
fn exact_sin_deg(x: f64) -> Option<f64> {
    let mut r = x % 360.0;
    if r < 0.0 {
        r += 360.0;
    }
    let table: &[(f64, f64)] = &[
        (0.0, 0.0),
        (30.0, 0.5),
        (45.0, std::f64::consts::FRAC_1_SQRT_2),
        (60.0, 3.0_f64.sqrt() / 2.0),
        (90.0, 1.0),
        (120.0, 3.0_f64.sqrt() / 2.0),
        (135.0, std::f64::consts::FRAC_1_SQRT_2),
        (150.0, 0.5),
        (180.0, 0.0),
        (210.0, -0.5),
        (225.0, -std::f64::consts::FRAC_1_SQRT_2),
        (240.0, -(3.0_f64.sqrt()) / 2.0),
        (270.0, -1.0),
        (300.0, -(3.0_f64.sqrt()) / 2.0),
        (315.0, -std::f64::consts::FRAC_1_SQRT_2),
        (330.0, -0.5),
    ];
    table.iter().find(|(deg, _)| *deg == r).map(|(_, v)| *v)
}

pub fn sin_deg(x: f64) -> f64 {
    if !x.is_finite() || x.abs() >= TRIG_MAX {
        return f64::NAN;
    }
    exact_sin_deg(x).unwrap_or_else(|| x.to_radians().sin())
}

pub fn cos_deg(x: f64) -> f64 {
    if !x.is_finite() || x.abs() >= TRIG_MAX {
        return f64::NAN;
    }
    exact_sin_deg(x + 90.0).unwrap_or_else(|| x.to_radians().cos())
}

pub fn tan_deg(x: f64) -> f64 {
    sin_deg(x) / cos_deg(x)
}

/// sin and cos together, for building a rotation matrix.
pub fn sin_cos_deg(x: f64) -> (f64, f64) {
    (sin_deg(x), cos_deg(x))
}

/// Snap an inverse-trig result to a whole number of degrees when that whole
/// degree ROUND-TRIPS through the forward function — the reference's rule,
/// and the only way `asin(0.5) == 30` can hold.
///
/// It has to be the round trip and not merely "close to a whole number":
/// the forward functions are exact here, so asin(0.5) is the angle whose
/// sine is EXACTLY 0.5, and 30 is the right answer; whereas asin of a value
/// a hair off 0.5 is genuinely not 30 and must not be snapped to it. Without
/// this, asin(0.5) printed as 30 — the echo formatter rounds to six
/// significant digits — while `asin(0.5) == 30` compared FALSE.
fn snap(v: f64, whole_is_exact: impl Fn(f64) -> bool) -> f64 {
    if !v.is_finite() {
        return v;
    }
    let r = v.round();
    if (v - r).abs() < 1e-9 && whole_is_exact(r) {
        r
    } else {
        v
    }
}

pub fn asin_deg(x: f64) -> f64 {
    snap(x.asin().to_degrees(), |r| sin_deg(r) == x)
}

pub fn acos_deg(x: f64) -> f64 {
    snap(x.acos().to_degrees(), |r| cos_deg(r) == x)
}

pub fn atan_deg(x: f64) -> f64 {
    snap(x.atan().to_degrees(), |r| tan_deg(r) == x)
}

/// atan2 snaps on the direction rather than a ratio, so it works on the
/// axes too: r is right when (cos r, sin r) is parallel to (x, y). The
/// quadrant is already correct from atan2 itself, and the rounding only
/// moves the angle by under a degree, so parallel implies equal here.
pub fn atan2_deg(y: f64, x: f64) -> f64 {
    snap(y.atan2(x).to_degrees(), |r| {
        sin_deg(r) * x == cos_deg(r) * y
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference states these as equalities, not approximations:
    /// "asin(1)==90, asin(0.5)==30, asin(-1)==-90", "acos(1)==0,
    /// acos(0)==90, acos(-1)==180", and "inverse trig snaps to whole
    /// degrees when the whole-degree value round-trips".
    #[test]
    fn inverse_trig_snaps_to_whole_degrees() {
        assert_eq!(asin_deg(0.5), 30.0);
        assert_eq!(asin_deg(1.0), 90.0);
        assert_eq!(asin_deg(-1.0), -90.0);
        assert_eq!(asin_deg(0.0), 0.0);
        assert_eq!(acos_deg(0.5), 60.0);
        assert_eq!(acos_deg(1.0), 0.0);
        assert_eq!(acos_deg(0.0), 90.0);
        assert_eq!(acos_deg(-1.0), 180.0);
        assert_eq!(atan_deg(1.0), 45.0);
        assert_eq!(atan_deg(0.0), 0.0);
        assert_eq!(atan2_deg(1.0, 1.0), 45.0);
        assert_eq!(atan2_deg(1.0, 0.0), 90.0);
        assert_eq!(atan2_deg(0.0, -1.0), 180.0);
        // Round trips both ways.
        for d in [0.0, 30.0, 45.0, 60.0, 90.0] {
            assert_eq!(asin_deg(sin_deg(d)), d, "asin(sin({d}))");
        }
        for d in [0.0, 30.0, 45.0, 60.0, 90.0, 120.0, 180.0] {
            assert_eq!(acos_deg(cos_deg(d)), d, "acos(cos({d}))");
        }
    }

    /// The snap must not fire on an angle that is merely NEAR a whole
    /// degree — that would invent exactness the input does not have.
    #[test]
    fn the_snap_needs_a_real_round_trip() {
        let off = sin_deg(30.0) + 1e-12;
        assert_ne!(asin_deg(off), 30.0, "a value near sin(30) is not 30 degrees");
        assert!((asin_deg(off) - 30.0).abs() < 1e-6);
        assert_ne!(asin_deg(0.3), asin_deg(0.3).round());
    }
}
