//! Replica of the randomness the game uses: `java.util.Random` bit for bit, `Math.random()`,
//! `Collections.shuffle`, and Java's rounding.
//!
//! The game creates `new Random()` (seeded from the clock) all over the place. [`RandomSource`]
//! hands those out: from the OS clock when playing, or from a counter in deterministic mode,
//! which matches the Java harness whose `java.util.Random()` constructor is patched to take the
//! same counter. Seeded runs of both sides can then be compared result by result.

use serde::{Deserialize, Serialize};

const MULTIPLIER: i64 = 0x5_DEEC_E66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;

/// Every draw of every [`JavaRandom`], for the traces of the differential tests (the Java
/// harness counts `Random.bfDraws` the same way).
pub static DRAWS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// `java.util.Random` (the legacy LCG every JDK keeps for compatibility).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JavaRandom {
    seed: i64,
    next_next_gaussian: Option<f64>,
}

impl JavaRandom {
    /// `new Random(seed)`.
    pub fn new(seed: i64) -> Self {
        JavaRandom { seed: (seed ^ MULTIPLIER) & MASK, next_next_gaussian: None }
    }

    /// `setSeed`.
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
        self.next_next_gaussian = None;
    }

    fn next(&mut self, bits: u32) -> i32 {
        DRAWS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// `nextInt(bound)`; panics on a non-positive bound like Java throws.
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound - 1;
        if bound & m == 0 {
            return ((bound as i64 * r as i64) >> 31) as i32;
        }
        let mut u = r;
        loop {
            r = u % bound;
            if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                return r;
            }
            u = self.next(31);
        }
    }

    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1 << 24) as f32
    }

    pub fn next_double(&mut self) -> f64 {
        (((self.next(26) as i64) << 27) + self.next(27) as i64) as f64 * (1.0 / (1i64 << 53) as f64)
    }

    /// `nextGaussian`, polar method with `StrictMath.log` (fdlibm) and `sqrt` (IEEE, exact).
    pub fn next_gaussian(&mut self) -> f64 {
        if let Some(g) = self.next_next_gaussian.take() {
            return g;
        }
        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = (-2.0 * crate::fdlibm::log(s) / s).sqrt();
                self.next_next_gaussian = Some(v2 * multiplier);
                return v1 * multiplier;
            }
        }
    }

    /// `Collections.shuffle(list, random)`.
    pub fn shuffle<T>(&mut self, list: &mut [T]) {
        for i in (1..list.len()).rev() {
            let j = self.next_int_bounded(i as i32 + 1) as usize;
            list.swap(i, j);
        }
    }
}

/// Where the game's unseeded randomness comes from.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RandomSource {
    /// `Some(next seed)` in deterministic mode; `None` seeds from the clock.
    deterministic_next: Option<i64>,
    /// Java's mixing of clock seeds (`Random.seedUniquifier`).
    uniquifier: i64,
    /// The `Random` behind `Math.random()`, created on first use.
    math: Option<JavaRandom>,
    /// The `Random` behind `Collections.shuffle(list)`, created on first use.
    collections: Option<JavaRandom>,
}

impl Default for RandomSource {
    fn default() -> Self {
        Self::from_clock()
    }
}

impl RandomSource {
    pub fn from_clock() -> Self {
        RandomSource { deterministic_next: None, uniquifier: 8_682_522_807_148_012, math: None, collections: None }
    }

    /// Every `new Random()` gets the next number from `first_seed` on.
    pub fn deterministic(first_seed: i64) -> Self {
        RandomSource { deterministic_next: Some(first_seed), ..Self::from_clock() }
    }

    /// `new Random()`.
    pub fn new_random(&mut self) -> JavaRandom {
        match &mut self.deterministic_next {
            Some(next) => {
                let seed = *next;
                *next += 1;
                JavaRandom::new(seed)
            }
            None => {
                self.uniquifier = self.uniquifier.wrapping_mul(1_181_783_497_276_652_981);
                #[cfg(not(target_arch = "wasm32"))]
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0);
                #[cfg(target_arch = "wasm32")]
                let nanos = web_time::SystemTime::now()
                    .duration_since(web_time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i64)
                    .unwrap_or(0);
                JavaRandom::new(self.uniquifier ^ nanos)
            }
        }
    }

    /// A `static final Random` of a mod system: in deterministic mode the Java harness gives
    /// each one a fixed seed (`new Random(seed)`) instead of drawing from the counter.
    pub fn static_random(&mut self, seed: i64) -> JavaRandom {
        match self.deterministic_next {
            Some(_) => JavaRandom::new(seed),
            None => self.new_random(),
        }
    }

    /// The seed the next `new Random()` gets in deterministic mode.
    pub fn next_seed(&self) -> Option<i64> {
        self.deterministic_next
    }

    /// `new Random().nextInt(bound)`, the game's most common idiom.
    pub fn fresh_int(&mut self, bound: i32) -> i32 {
        self.new_random().next_int_bounded(bound)
    }

    /// `Math.random()`.
    pub fn math_random(&mut self) -> f64 {
        if self.math.is_none() {
            self.math = Some(self.new_random());
        }
        self.math.as_mut().unwrap().next_double()
    }

    /// `Collections.shuffle(list)`.
    pub fn shuffle<T>(&mut self, list: &mut [T]) {
        if self.collections.is_none() {
            self.collections = Some(self.new_random());
        }
        self.collections.as_mut().unwrap().shuffle(list);
    }
}

/// `Math.round(double)`: floor(x + 0.5), saturating, NaN to 0.
pub fn java_round(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    // Java computes this exactly; floor(x + 0.5) is wrong only for 0.49999999999999994
    if x.fract() == 0.0 || x.abs() >= (1u64 << 52) as f64 {
        return x as i64;
    }
    let floor = x.floor();
    if x - floor >= 0.5 { floor as i64 + 1 } else { floor as i64 }
}

/// `Math.round(float)`.
pub fn java_round_f32(x: f32) -> i32 {
    if x.is_nan() {
        return 0;
    }
    if x.fract() == 0.0 || x.abs() >= (1u32 << 23) as f32 {
        return x as i32;
    }
    let floor = x.floor();
    if x - floor >= 0.5 { floor as i32 + 1 } else { floor as i32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values printed by java.util.Random on JDK 21 (see crates/model/tests notes).
    struct Reference {
        seed: i64,
        int: i32,
        int10: i32,
        int100: i32,
        int1000: i32,
        int16: i32,
        int7e8: i32,
        long: i64,
        boolean: bool,
        float_bits: u32,
        double_bits: u64,
        gaussian_bits: [i64; 2],
        shuffled: [i32; 10],
    }

    const REFERENCES: [Reference; 4] = [
        Reference {
            seed: 0,
            int: -1155484576,
            int10: 8,
            int100: 29,
            int1000: 447,
            int16: 10,
            int7e8: 663681053,
            long: -8292973307042192125,
            boolean: true,
            float_bits: 1061687975,
            double_bits: 4599674348956770052,
            gaussian_bits: [-4639848932703132413, 4592968748734612262],
            shuffled: [5, 9, 2, 8, 3, 1, 0, 7, 6, 4],
        },
        Reference {
            seed: 42,
            int: -1170105035,
            int10: 3,
            int100: 48,
            int1000: 884,
            int16: 4,
            int7e8: 623087525,
            long: 5111195811822994797,
            boolean: true,
            float_bits: 1035667552,
            double_bits: 4606312073534385342,
            gaussian_bits: [-4617835941257819195, -4614403294974258598],
            shuffled: [5, 4, 0, 1, 3, 7, 2, 8, 9, 6],
        },
        Reference {
            seed: -7,
            int: 1155869324,
            int10: 7,
            int100: 90,
            int1000: 707,
            int16: 8,
            int7e8: 314447087,
            long: -2732364192705608872,
            boolean: false,
            float_bits: 1045094876,
            double_bits: 4601962560766891450,
            gaussian_bits: [4611422097317650229, 4609483444790506685],
            shuffled: [2, 5, 7, 8, 4, 0, 9, 3, 6, 1],
        },
        Reference {
            seed: 123456789012345,
            int: -1421006947,
            int10: 4,
            int100: 19,
            int1000: 652,
            int16: 14,
            int7e8: 402514868,
            long: 1596561836080580602,
            boolean: true,
            float_bits: 1053464974,
            double_bits: 4605887243577338685,
            gaussian_bits: [4606921244293114075, -4616772616007536428],
            shuffled: [9, 3, 0, 6, 4, 1, 8, 5, 2, 7],
        },
    ];

    #[test]
    fn matches_java_util_random() {
        for r in &REFERENCES {
            let mut random = JavaRandom::new(r.seed);
            assert_eq!(random.next_int(), r.int, "seed {}", r.seed);
            assert_eq!(random.next_int_bounded(10), r.int10);
            assert_eq!(random.next_int_bounded(100), r.int100);
            assert_eq!(random.next_int_bounded(1000), r.int1000);
            assert_eq!(random.next_int_bounded(16), r.int16);
            assert_eq!(random.next_int_bounded(700_000_000), r.int7e8);
            assert_eq!(random.next_long(), r.long);
            assert_eq!(random.next_boolean(), r.boolean);
            assert_eq!(random.next_float().to_bits(), r.float_bits);
            assert_eq!(random.next_double().to_bits(), r.double_bits);
            assert_eq!(random.next_gaussian().to_bits() as i64, r.gaussian_bits[0]);
            assert_eq!(random.next_gaussian().to_bits() as i64, r.gaussian_bits[1]);
            let mut list: Vec<i32> = (0..10).collect();
            random.shuffle(&mut list);
            assert_eq!(list, r.shuffled);
        }
    }

    #[test]
    fn rounds_like_java() {
        let cases = [(2.5, 3), (-2.5, -2), (0.49999999999999994, 0), (-0.5, 0), (1e20, i64::MAX), (f64::NAN, 0), (-1.5, -1), (7.5, 8)];
        for (x, expected) in cases {
            assert_eq!(java_round(x), expected, "{x}");
        }
        for (x, expected) in [(2.5f32, 3), (-2.5, -2), (-0.5, 0), (3.4999998, 3)] {
            assert_eq!(java_round_f32(x), expected, "{x}");
        }
    }

    #[test]
    fn deterministic_source_counts_seeds() {
        let mut a = RandomSource::deterministic(100);
        let mut b = RandomSource::deterministic(100);
        assert_eq!(a.new_random(), JavaRandom::new(100));
        assert_eq!(a.fresh_int(1000), JavaRandom::new(101).next_int_bounded(1000));
        b.new_random();
        b.new_random();
        assert_eq!(a.math_random(), b.math_random());
    }
}
