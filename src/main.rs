#[macro_use]
extern crate log;

use std::sync;

use std::collections::HashMap;
use hdrhistogram::Histogram;
use std::sync::{Mutex, Arc};
use std::error::Error;
use std::time::{Instant, Duration};
use crate::requestgen::RequestGenerator;
use std::thread::JoinHandle;

mod config;
mod requestgen;
mod worker;

/// Parses command line arguments, launches the workers, consolidates results
fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse the command line and read in the set of URLs we use to test
    let mut config = config::Config::from_cmdline()?;

    // Initialise the request generator from the config
    // Note that it consumes the URLs from config (hence the mutable ref)
    let request_generator = sync::Arc::new(requestgen::RequestGenerator::new(&mut config));

    // Initialise the summary bench result each worker will accumulate into
    let result_summary = Arc::new(Mutex::new(BenchResult::new()));

    // Launch the workers
    let bench_start = Instant::now();
    info!("Starting {} workers", config.concurrency);
    let mut workers = Vec::new();
    for id in 0..config.concurrency {
        workers.push(worker::start(id, &request_generator, &result_summary));
    }

    // Wait for them to complete
    wait_for_workers(request_generator, workers);
    let bench_end = Instant::now();

    // Print the results of the benchmark
    let bench_duration = bench_end.duration_since(bench_start);
    print_results(bench_duration, result_summary);

    Ok(())
}

/// Statistics we generate during the benchmark process
pub(crate) struct BenchResult {
    status: HashMap<u16, u32>,
    request_errors: u32,
    response_errors: u32,
    latency: Histogram<u64>,
}

impl BenchResult {
    /// Initialise a new benchmark result
    pub fn new() -> BenchResult {
        BenchResult {
            status: HashMap::new(),
            request_errors: 0,
            response_errors: 0,
            // We measure latency in microseconds, so configure the histogram to track 1 microsecond to 10 seconds
            latency: Histogram::<u64>::new_with_bounds(1, 1000 * 1000 * 10, 2).unwrap(),
        }
    }

    /// Accumulate another benchmark result into this one (i.e. from a worker thread into the summary).
    pub fn accumulate(&mut self, other: BenchResult) {
        for (code, count) in other.status {
            let total = self.status.entry(code).or_insert(0);
            *total += count;
        }

        self.request_errors += other.request_errors;
        self.response_errors += other.response_errors;

        self.latency.add(other.latency).unwrap();
    }
}

// Output the benchmark results
fn print_results(bench_duration: Duration, summary: Arc<Mutex<BenchResult>>) {
    let summary = summary.lock().unwrap();

    // Note errors if they occurred
    if summary.request_errors > 0 {
        warn!("*** {} request errors", summary.request_errors);
    }
    if summary.response_errors > 0 {
        warn!("*** {} response errors", summary.response_errors);
    }

    // Dump the status codes
    let mut codes = summary.status.keys()
        .copied()
        .collect::<Vec<u16>>();
    codes.sort();
    println!("\nHTTP responses:");
    for code in codes {
        println!("{}\t{}", code, summary.status.get(&code).unwrap());
    }

    // Dump the latency
    println!("\nBenchmark run time {}s.\nLatency:", bench_duration.as_secs_f32());
    for p in &[50f64, 75f64, 95f64, 99f64, 99.9f64, 99.99f64, 99.999f64, 100f64] {
        let micros = &summary.latency.value_at_percentile(*p);
        let millis = *micros as f64 / 1000f64;
        println!("{}%\t{}ms", p, millis);
    }
}

fn wait_for_workers(request_generator: Arc<RequestGenerator>, workers: Vec<JoinHandle<()>>) {
    info!("Waiting for test to complete");
    for worker in workers {
        worker.join().unwrap()
    }

    // Note we are done
    let progress = request_generator.progress.lock().unwrap();
    progress.finish_with_message("done")
}