//! 乱数は rand クレート（PCG）を使う。ここではシード値の組み立てと、
//! よく使う操作の薄いヘルパーだけを提供する。

use rand::distr::{weighted::WeightedIndex, Distribution};
use rand::seq::IndexedRandom;
use rand::{RngExt, SeedableRng};

pub type Rng = rand_pcg::Pcg64Mcg;

/// 複数の値から 64bit のシード値を作る（splitmix64 の混合関数）
pub fn mix(parts: &[u64]) -> u64 {
    parts.iter().fold(0x5EED_1EAC_0DA7_A000u64, |h, &p| {
        let mut z = (h ^ p).wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    })
}

pub fn rng(parts: &[u64]) -> Rng {
    Rng::seed_from_u64(mix(parts))
}

/// 引数から決まる [0, 1) の値。流出判定などを状態を持たずに再現するために使う
pub fn hash01(parts: &[u64]) -> f64 {
    rng(parts).random::<f64>()
}

pub fn hash_str(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

pub trait RngHelpers: RngExt {
    fn chance(&mut self, p: f64) -> bool {
        if p.is_nan() || p <= 0.0 {
            false
        } else {
            self.random_bool(p.min(1.0))
        }
    }

    fn pick<'a, T>(&mut self, s: &'a [T]) -> &'a T {
        s.choose(self).expect("pick from empty slice")
    }

    fn weighted(&mut self, w: &[f64]) -> usize {
        match WeightedIndex::new(w.iter()) {
            Ok(d) => d.sample(self),
            Err(_) => self.random_range(0..w.len().max(1)),
        }
    }

    fn rangef(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.random::<f64>()
    }

    /// 標準正規分布（Box-Muller）
    fn normal(&mut self) -> f64 {
        let u1 = self.random::<f64>().max(1e-12);
        let u2 = self.random::<f64>();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.random::<f64>()).exp()
    }

    fn poisson(&mut self, lambda: f64) -> u32 {
        let l = (-lambda).exp();
        let (mut k, mut p) = (0u32, 1.0f64);
        loop {
            p *= self.random::<f64>();
            if p <= l || k > 1000 {
                return k;
            }
            k += 1;
        }
    }

    fn digits(&mut self, n: usize) -> String {
        (0..n).map(|_| char::from(b'0' + self.random_range(0..10u8))).collect()
    }
}

impl<R: RngExt + ?Sized> RngHelpers for R {}
