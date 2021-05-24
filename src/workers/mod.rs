use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use hdrhistogram::Histogram;

use crate::requestgen::RequestGenerator;

/// Statistics we generate during the benchmark process
pub(crate) struct BenchResult {
    pub status: HashMap<u16, u32>,
    pub request_errors: u32,
    pub response_errors: u32,
    pub latency: Histogram<u64>,
    pub request_times: Vec<(usize, u64)>,
}

impl BenchResult {
    /// Initialise a new benchmark result
    pub fn new() -> BenchResult {
        BenchResult {
            status: HashMap::new(),
            request_errors: 0,
            response_errors: 0,
            // We measure latency in milliseconds, so configure the histogram to track 1 millisecond to 100 seconds
            latency: Histogram::<u64>::new_with_bounds(1, 1000 * 100, 2).unwrap(),
            request_times: Vec::new(),
        }
    }

    // Accumulate this result into the summary
    fn add_to(&mut self, summary: &mut BenchResult) {
        for (code, count) in &self.status {
            let total = summary.status.entry(*code).or_insert(0);
            *total += count;
        }

        summary.request_errors += self.request_errors;
        summary.response_errors += self.response_errors;

        let latency = std::mem::replace(&mut self.latency, Histogram::<u64>::new(1).unwrap());
        summary.latency += latency;

        summary.request_times.append(&mut self.request_times);
    }
}

/// Starts workers that pull requests from the generator, runs them and tracks benchmark statistics
pub(crate) fn run_test(concurrency: u16, request_generator: RequestGenerator, urls: &Arc<Vec<String>>) -> BenchResult {
    let request_generator = Arc::new(request_generator);

    let mut workers = Vec::new();
    for worker_id in 0..concurrency {
        let request_generator = request_generator.clone();
        let urls = urls.clone();
        workers.push(thread::spawn(move || {
            run_worker(worker_id, request_generator, urls)
        }));
    }

    // Combine all the individual test results
    let mut merged = BenchResult::new();
    for worker in workers {
        let mut result = worker.join().unwrap();
        result.add_to(&mut merged);
    }

    merged
}

fn run_worker(worker_id: u16, request_generator: Arc<RequestGenerator>, urls: Arc<Vec<String>>) -> BenchResult {
    let mut result = BenchResult::new();

    // Execute requests until we are done
    let client = reqwest::blocking::Client::new();
    while let Some(request) = request_generator.next() {
        debug!("{} -> {:?}", worker_id, request);

        // If we have a delay between requests, then sleep
        if request.sleep.as_nanos() > 0 {
            thread::sleep(request.sleep);
        }

        // Execute the request note the request latency
        let url = urls[request.url_index].as_str();
        let start = Instant::now();
        let response = client.get(url).send();

        // Track response code statistics
        let mut duration = 0;
        match response {
            Ok(response) => {
                let count = result.status.entry(response.status().as_u16()).or_insert(0);
                *count += 1;

                // Read the response and track errors
                if let Err(e) = response.bytes() {
                    result.response_errors += 1;
                    warn!("Error retrieving response for {}: {}", url, e);
                }

                let end = Instant::now();
                duration = end.duration_since(start).as_millis() as u64;
            }
            Err(e) => {
                result.request_errors += 1;
                warn!("Hit error processing {}: {}", url, e);
            }
        }

        // Update the latency histogram
        result.latency += duration;

        // Track the per-request latency too
        result.request_times.push((request.url_index, duration));
    }

    result
}
