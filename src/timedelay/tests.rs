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
