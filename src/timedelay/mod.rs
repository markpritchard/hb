use std::time::Duration;

use rand::{thread_rng, Rng};

use crate::config::{self, DelayDistribution};

pub(crate) fn time_delay(config: &config::Config) -> Box<dyn Iterator<Item=Duration>> {
    // Create the requested time delay generator
    let delay_ms = config.delay_ms;
    match config.delay_distrib {
        DelayDistribution::Constant => Box::new(ConstantDelay::new(delay_ms)),
        DelayDistribution::Uniform => Box::new(UniformDelay::new(delay_ms)),
        DelayDistribution::NegativeExponential => Box::new(NegativeExponentialDelay::new(delay_ms)),
    }
}

struct ConstantDelay {
    delay: Duration
}

impl ConstantDelay {
    fn new(delay_ms: u32) -> ConstantDelay {
        let delay = Duration::from_millis(delay_ms as u64);
        ConstantDelay { delay }
    }
}

impl Iterator for ConstantDelay {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.delay)
    }
}

// http://perfdynamics.blogspot.com/2012/05/load-testing-with-uniform-vs.html
struct NegativeExponentialDelay {
    z_neg: f32,
}

impl NegativeExponentialDelay {
    fn new(delay_ms: u32) -> NegativeExponentialDelay {
        let z_neg = -1f32 * delay_ms as f32;
        NegativeExponentialDelay { z_neg }
    }
}

impl Iterator for NegativeExponentialDelay {
    type Item = Duration;

    // http://perfdynamics.blogspot.com/2012/03/how-to-generate-exponential-delays.html
    fn next(&mut self) -> Option<Self::Item> {
        // Generate next delay time
        let u = thread_rng().gen_range(0f32, 1f32);
        let t = (self.z_neg * u.ln()) as u64;
        Some(Duration::from_millis(t))
    }
}

struct UniformDelay {
    bound_ms: u32,
}

impl UniformDelay {
    fn new(delay_ms: u32) -> UniformDelay {
        UniformDelay { bound_ms: delay_ms }
    }
}

impl Iterator for UniformDelay {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let delay_ms = thread_rng().gen_range(0, self.bound_ms);
        Some(Duration::from_millis(delay_ms as u64))
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use super::*;

    #[test]
    fn test_constant() {
        const DELAY_MS: u32 = 5;

        let duration = Duration::from_millis(DELAY_MS as u64);
        let mut time_delay = ConstantDelay::new(DELAY_MS);
        assert_eq!(duration, time_delay.next().unwrap());
    }

    #[test]
    fn test_negative_exponential() {
        const DELAY_MS: u32 = 30;
        const TEST_ITERS: u32 = 10000;

        let time_delay = NegativeExponentialDelay::new(DELAY_MS);
        let sum: u32 = time_delay
            .take(TEST_ITERS as usize)
            .map(|d| d.as_millis() as u32)
            .sum();
        let avg = sum as f32 / TEST_ITERS as f32;

        assert_approx_eq!(DELAY_MS as f32, avg, 2f32);
    }

    #[test]
    fn test_uniform() {
        const DELAY_MS: u32 = 10;
        const TEST_ITERS: u32 = 10000;

        let time_delay = UniformDelay::new(DELAY_MS);
        let mut histo = [0u32; DELAY_MS as usize];
        time_delay
            .take(TEST_ITERS as usize)
            .for_each(|d| {
                let ms = d.as_millis() as usize;
                histo[ms] += 1;
            });

        let expected = (TEST_ITERS / DELAY_MS) as f32;
        let epsilon = expected / 10f32;
        for freq in histo.iter() {
            assert_approx_eq!(*freq as f32, expected, epsilon);
        }
    }
}
