use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use crate::BenchResult;
use crate::requestgen::RequestGenerator;

/// Starts a worker thread that executes the benchmark
pub (crate) fn start(worker_id: u16, request_generator: &Arc<RequestGenerator>, summary: &Arc<Mutex<BenchResult>>) -> JoinHandle<()> {
    let request_generator = request_generator.clone();
    let result_summary = summary.clone();
    thread::spawn(move || {
        run(worker_id, request_generator, result_summary);
    })
}

// Pulls requests from the generator, runs them and tracks benchmark statistics
fn run(worker_id: u16, request_generator: Arc<RequestGenerator>, summary: Arc<Mutex<BenchResult>>) {
    // Record into local statistics then accumulate once complete
    let mut stats = BenchResult::new();

    // Execute requests until we are done
    let client = reqwest::blocking::Client::new();
    while let Some(request) = request_generator.next() {
        debug!("{} -> {:?}", worker_id, request);

        // Execute the request note the request latency
        let start = Instant::now();
        let response = client.get(request.url).send();
        let end = Instant::now();
        let duration = end.duration_since(start).as_micros() as u64;

        // Update the latency histogram
        stats.latency += duration;

        // Track response code statistics
        match response {
            Ok(response) => {
                let count = stats.status.entry(response.status().as_u16()).or_insert(0);
                *count += 1;
            }
            Err(e) => {
                stats.errors += 1;
                warn!("Hit error processing {}: {}", request.url, e);
            }
        }
    }

    // Accumulate bench result
    let mut summary = summary.lock().unwrap();
    summary.accumulate(stats);
}
