//! The fdlibm functions behind Java's `StrictMath`, bit for bit. Port of `jdk.internal.math`'s
//! `FdLibm.Log` (fdlibm 5.3 `e_log.c`); the `libm` crate's `log` is musl's newer one, which
//! differs in the last bit for about 2% of the inputs.

// the constants as fdlibm writes them
#![allow(clippy::excessive_precision)]

const LN2_HI: f64 = 6.93147180369123816490e-01; // 0x3fe62e42 fee00000
const LN2_LO: f64 = 1.90821492927058770002e-10; // 0x3dea39ef 35793c76
const TWO54: f64 = 1.80143985094819840000e+16; // 0x43500000 00000000
const LG1: f64 = 6.666666666666735130e-01;
const LG2: f64 = 3.999999999940941908e-01;
const LG3: f64 = 2.857142874366239149e-01;
const LG4: f64 = 2.222219843214978396e-01;
const LG5: f64 = 1.818357216161805012e-01;
const LG6: f64 = 1.531383769920937332e-01;
const LG7: f64 = 1.479819860511658591e-01;

fn high(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

fn low(x: f64) -> u32 {
    x.to_bits() as u32
}

fn with_high(x: f64, hi: i32) -> f64 {
    f64::from_bits(((hi as u32 as u64) << 32) | low(x) as u64)
}

/// `StrictMath.log(double)`
pub fn log(x: f64) -> f64 {
    let mut x = x;
    let mut hx = high(x);
    let lx = low(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        // x < 2**-1022
        if ((hx & 0x7fff_ffff) as u32 | lx) == 0 {
            return f64::NEG_INFINITY; // log(+-0) = -inf
        }
        if hx < 0 {
            return f64::NAN; // log(-#) = NaN
        }
        k -= 54;
        x *= TWO54; // subnormal number, scale up x
        hx = high(x);
    }
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000f_ffff;
    let i = (hx + 0x9_5f64) & 0x10_0000;
    x = with_high(x, hx | (i ^ 0x3ff0_0000)); // normalize x or x/2
    k += i >> 20;
    let f = x - 1.0;
    if (0x000f_ffff & (2 + hx)) < 3 {
        // |f| < 2**-20
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            let dk = k as f64;
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r = f * f * (0.5 - 0.33333333333333333 * f);
        if k == 0 {
            return f - r;
        }
        let dk = k as f64;
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let dk = k as f64;
    let z = s * s;
    let mut i = hx - 0x6_147a;
    let w = z * z;
    let j = 0x6b851 - hx;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

#[cfg(test)]
mod tests {
    /// `StrictMath.log` on Java 21, as raw bits; the last four are inputs where musl's `log`
    /// is one ulp off.
    #[test]
    fn log_matches_strict_math() {
        let cases: [(u64, u64); 7] = [
            (0x3fe74833a06ff457, 0xbfd45afe1ecf772f),
            (0x3fe5dcf778622e01, 0xbfd86135fc79dabd),
            (0x3fd3c20f3f12bbb4, 0xbff2ce1ec888f460),
            (0x3fea2b647816a9f1, 0xbfc9befee1808a87),
            (0x3fe736e1fb765d46, 0xbfd48aab8221bfe7),
            (0x3fe1915b4e06c255, 0xbfe3307b79f7308e),
            (0x3fe88c42b0db5ef5, 0xbfd0f78f7b8a1103),
        ];
        for (x, expected) in cases {
            assert_eq!(super::log(f64::from_bits(x)).to_bits(), expected, "log({})", f64::from_bits(x));
        }
    }
}
