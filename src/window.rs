//! Seeded simulation of reading policy: single reading vs k-reading
//! majority window per gate, folded with the unchanged max-rank rule.

use crate::{Decision, Outcome};
use serde::Serialize;

struct SplitMix(u64);

impl SplitMix {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct WindowReport {
    pub runs: u32,
    pub gates: u32,
    pub p_bad: f64,
    pub window: u32,
    pub majority: u32,
    pub flips_k1: u32,
    pub flips_k5: u32,
    pub flip_rate_k1: f64,
    pub flip_rate_k5: f64,
    pub analytic_k1: f64,
}

fn fold(outcomes: &[Outcome]) -> Decision {
    outcomes
        .iter()
        .copied()
        .max_by_key(|o| o.rank())
        .map(Decision::from)
        .unwrap_or(Decision::Pass)
}

pub fn window_sim(runs: u32, seed: u64, gates: u32, p_bad: f64, window: u32, majority: u32) -> WindowReport {
    let mut rng = SplitMix(seed);
    let (mut f1, mut fk) = (0u32, 0u32);
    for _ in 0..runs {
        let mut single = Vec::new();
        let mut windowed = Vec::new();
        for _ in 0..gates {
            let readings: Vec<bool> = (0..window).map(|_| rng.unit() < p_bad).collect();
            single.push(if readings[0] { Outcome::Block } else { Outcome::Pass });
            let bad = readings.iter().filter(|b| **b).count() as u32;
            windowed.push(if bad >= majority { Outcome::Block } else { Outcome::Pass });
        }
        if fold(&single) == Decision::Block {
            f1 += 1;
        }
        if fold(&windowed) == Decision::Block {
            fk += 1;
        }
    }
    let r = runs.max(1) as f64;
    WindowReport {
        runs,
        gates,
        p_bad,
        window,
        majority,
        flips_k1: f1,
        flips_k5: fk,
        flip_rate_k1: f1 as f64 / r,
        flip_rate_k5: fk as f64 / r,
        analytic_k1: 1.0 - (1.0 - p_bad).powi(gates as i32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(window_sim(200, 7, 3, 0.05, 5, 3), window_sim(200, 7, 3, 0.05, 5, 3));
    }

    #[test]
    fn zero_noise_never_flips() {
        let r = window_sim(100, 1, 3, 0.0, 5, 3);
        assert_eq!(r.flips_k1 + r.flips_k5, 0);
    }

    #[test]
    fn always_bad_always_flips() {
        let r = window_sim(100, 1, 3, 1.0, 5, 3);
        assert_eq!((r.flips_k1, r.flips_k5), (100, 100));
    }

    #[test]
    fn window_reduces_flips() {
        let r = window_sim(1000, 7, 3, 0.05, 5, 3);
        assert!(r.flip_rate_k5 < r.flip_rate_k1);
    }
}
