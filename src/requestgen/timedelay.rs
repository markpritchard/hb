use std::time::Duration;

use rand::{Rng, thread_rng};

use crate::config::DelayDistribution;

/// Creates a time delay supplier based on the requested delay etc
pub(crate) fn create_supplier(delay_ms: &u32, distrib: &DelayDistribution) -> Box<dyn TimeDelaySupplier> {
    let delay_us = *delay_ms as u64 * 1000u64;
    match distrib {
        DelayDistribution::Constant => Box::new(ConstantDelay::new(delay_us)),
        DelayDistribution::Uniform => Box::new(UniformDelay::new(delay_us)),
        DelayDistribution::NegativeExponential => Box::new(NegativeExponentialDelay::new(delay_us)),
    }
}

/// Generates a time delay from the underlying distribution
pub(crate) trait TimeDelaySupplier {
    fn next_delay(&self) -> Duration;
}

// A fixed delay
struct ConstantDelay {
    delay: Duration
}

impl ConstantDelay {
    fn new(delay_us: u64) -> ConstantDelay {
        let delay = Duration::from_micros(delay_us);
        ConstantDelay { delay }
    }
}

impl TimeDelaySupplier for ConstantDelay {
    fn next_delay(&self) -> Duration {
        self.delay
    }
}

// An exponential distribution better models the bursts/clumping behaviour in traffic
// http://perfdynamics.blogspot.com/2012/05/load-testing-with-uniform-vs.html
struct NegativeExponentialDelay {
    z_neg: f64,
}

impl NegativeExponentialDelay {
    fn new(delay_us: u64) -> NegativeExponentialDelay {
        let z_neg = -1f64 * delay_us as f64;
        NegativeExponentialDelay { z_neg }
    }
}

impl TimeDelaySupplier for NegativeExponentialDelay {
    // http://perfdynamics.blogspot.com/2012/03/how-to-generate-exponential-delays.html
    fn next_delay(&self) -> Duration {
        // Generate next delay time
        let u = thread_rng().gen_range(0f64, 1f64);
        let t = (self.z_neg * u.ln()) as u64;
        Duration::from_micros(t)
    }
}

// The classic "random" delay where we choose a random delay given an upper bound
struct UniformDelay {
    bound_us: u64,
}

impl UniformDelay {
    fn new(delay_us: u64) -> UniformDelay {
        UniformDelay { bound_us: delay_us }
    }
}

impl TimeDelaySupplier for UniformDelay {
    fn next_delay(&self) -> Duration {
        let delay_us = thread_rng().gen_range(0, self.bound_us);
        Duration::from_micros(delay_us as u64)
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    // Verifies the constant delay supplier returns the expected value
    #[test]
    fn test_constant() {
        const DELAY_US: u64 = 5 * 1000;

        let duration = Duration::from_micros(DELAY_US);
        let time_delay = ConstantDelay::new(DELAY_US);
        assert_eq!(duration, time_delay.next_delay());
    }

    // Verifies that the exponential supplier generates a set of delays consistent with the distribution
    #[test]
    fn test_negative_exponential() {
        const DELAY_US: u64 = 30 * 1000;
        const TEST_ITERS: usize = 10000;

        let time_delay = NegativeExponentialDelay::new(DELAY_US);
        let mut sum_us = 0;
        for _i in 0..TEST_ITERS {
            let delay_us = time_delay.next_delay().as_micros() as u64;
            sum_us += delay_us;
        }
        let avg = sum_us as f64 / TEST_ITERS as f64;

        assert_approx_eq!(DELAY_US as f64, avg, 2000f64);
    }

    // Verifies that the uniform supplier generates the correct distribution
    #[test]
    fn test_uniform() {
        const DELAY_US: u64 = 10 * 1000;
        const TEST_ITERS: usize = 10000;

        let time_delay = UniformDelay::new(DELAY_US);
        let mut histo = [0u32; DELAY_US as usize];
        for _i in 0..TEST_ITERS {
            let delay_us = time_delay.next_delay().as_micros() as u64;
            histo[delay_us as usize] += 1;
        }

        let expected_avg = (TEST_ITERS as u64 / DELAY_US) as f64;
        let sum: f64 = histo.iter().map(|v| *v as f64).sum();
        let actual_avg = sum / histo.len() as f64;
        assert_approx_eq!(expected_avg, actual_avg, 0.00000001f64);
    }
}
