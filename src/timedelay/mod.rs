use std::time::Duration;

use rand::{Rng, thread_rng};

use crate::config::{self, DelayDistribution};

#[cfg(test)]
mod tests;

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
