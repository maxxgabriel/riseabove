//! Platform-independent transcendental functions for outcome paths.
//!
//! IEEE-754 guarantees `+ - * / sqrt` are correctly rounded everywhere, but
//! `exp`/`ln`/`powf` delegate to the platform libm and may differ by an ulp
//! between machines — enough to fork a seeded world. These use only the
//! guaranteed operations.

const LN2: f32 = std::f32::consts::LN_2;
const LOG2_E: f32 = std::f32::consts::LOG2_E;

#[inline]
pub fn exp(x: f32) -> f32 {
    let x = x.clamp(-87.0, 88.0);
    let k = (x * LOG2_E).round();
    let r = x - k * 0.693_145_75 - k * 1.428_606_8e-6;
    let p = 1.0 + r * (1.0 + r * (0.5 + r * (1.0 / 6.0 + r * (1.0 / 24.0 + r * (1.0 / 120.0 + r * (1.0 / 720.0))))));
    p * f32::from_bits(((k as i32 + 127) as u32) << 23)
}

#[inline]
pub fn ln(x: f32) -> f32 {
    if x <= 0.0 {
        return -87.0;
    }
    let bits = x.to_bits();
    let e = ((bits >> 23) & 0xff) as i32 - 127;
    let m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let series = s * (2.0 + s2 * (2.0 / 3.0 + s2 * (2.0 / 5.0 + s2 * (2.0 / 7.0 + s2 * (2.0 / 9.0 + s2 * (2.0 / 11.0))))));
    e as f32 * LN2 + series
}

#[inline]
pub fn powf(base: f32, e: f32) -> f32 {
    if base <= 0.0 {
        return 0.0;
    }
    exp(e * ln(base))
}

#[inline]
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + exp(-x))
}

/// Smooth 0→1 ramp between `lo` and `hi`.
#[inline]
pub fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Piecewise-linear interpolation over `(x, y)` knots sorted by x.
pub fn interp(knots: &[(f32, f32)], x: f32) -> f32 {
    match knots {
        [] => 0.0,
        [only] => only.1,
        _ => {
            if x <= knots[0].0 {
                return knots[0].1;
            }
            for w in knots.windows(2) {
                let ((x0, y0), (x1, y1)) = (w[0], w[1]);
                if x <= x1 {
                    return lerp(y0, y1, (x - x0) / (x1 - x0));
                }
            }
            knots[knots.len() - 1].1
        }
    }
}

/// Exponentially weighted moving average step.
#[inline]
pub fn ewma(prev: f32, sample: f32, alpha: f32) -> f32 {
    prev + alpha * (sample - prev)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accuracy() {
        for i in -400..400 {
            let x = i as f32 * 0.05;
            let rel = (exp(x) - x.exp()).abs() / x.exp();
            assert!(rel < 2e-6, "exp({x}) rel err {rel}");
        }
        for i in 1..2000 {
            let x = i as f32 * 0.37;
            assert!((ln(x) - x.ln()).abs() < 2e-6 * x.ln().abs().max(1.0), "ln({x})");
        }
    }
}
