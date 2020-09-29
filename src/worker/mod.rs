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

        // If we have a delay between requests, then sleep
        if request.sleep.as_nanos() > 0 {
            thread::sleep(request.sleep);
        }

        // Execute the request note the request latency
        let start = Instant::now();
        let response = client.get(request.url).send();

        // Track response code statistics
        let mut duration = 0;
        match response {
            Ok(response) => {
                let count = stats.status.entry(response.status().as_u16()).or_insert(0);
                *count += 1;

                // Read the response and track errors
                if let Err(e) = response.bytes() {
                    stats.response_errors += 1;
                    warn!("Error retrieving response for {}: {}", request.url, e);
                }

                let end = Instant::now();
                duration = end.duration_since(start).as_micros() as u64;
            }
            Err(e) => {
                stats.request_errors += 1;
                warn!("Hit error processing {}: {}", request.url, e);
            }
        }

        // Update the latency histogram
        stats.latency += duration;
    }

    // Accumulate bench result
    let mut summary = summary.lock().unwrap();
    summary.accumulate(stats);
}
