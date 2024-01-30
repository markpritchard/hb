use std::collections::HashMap;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{io, thread};

use hdrhistogram::Histogram;
use ureq::{Agent, Error};

use crate::config::HttpMethod;
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
pub(crate) fn run_test(
    agent: Agent,
    http_method: HttpMethod,
    header_map: Option<HashMap<String, String>>,
    concurrency: u16,
    request_generator: RequestGenerator,
    urls: &'static [String],
    payloads: &'static [String],
) -> BenchResult {
    let request_generator = Arc::new(request_generator);
    let results = Arc::new(Mutex::new(Vec::new()));

    info!("Starting test with {} workers", concurrency);

    let mut workers = Vec::new();
    for worker_id in 0..concurrency {
        let request_generator = request_generator.clone();
        let results = results.clone();
        let header_map = header_map.clone();
        let agent = agent.clone();
        let worker = thread::spawn(move || {
            let result = run_worker(
                worker_id,
                request_generator,
                agent,
                http_method,
                header_map,
                urls,
                payloads,
            );
            let mut results = results.lock().unwrap();
            results.push(result);
        });
        workers.push(worker);
    }

    // Wait for workers to complete
    info!("Waiting for workers to complete");
    for worker in workers {
        worker.join().unwrap();
    }

    // Combine all the individual test results
    let mut merged = BenchResult::new();
    let mut results = results.lock().unwrap();
    info!(
        "Merging {} results from {} workers",
        results.len(),
        concurrency
    );
    for result in results.iter_mut() {
        result.add_to(&mut merged);
    }

    merged
}

fn run_worker(
    worker_id: u16,
    request_generator: Arc<RequestGenerator>,
    agent: Agent,
    http_method: HttpMethod,
    header_map: Option<HashMap<String, String>>,
    urls: &'static [String],
    payloads: &'static [String],
) -> BenchResult {
    let mut result = BenchResult::new();

    // Execute requests until we are done
    while let Some(hb_request) = request_generator.next() {
        trace!("{} -> {:?}", worker_id, hb_request);

        // If we have a delay between requests, then sleep
        if hb_request.sleep.as_nanos() > 0 {
            thread::sleep(hb_request.sleep);
        }

        // Initialise the request
        // When testing POST or PUT only one url is provided
        let url = match http_method {
            HttpMethod::Post | HttpMethod::Put => urls[0].as_str(),
            _ => urls[hb_request.url_index].as_str(),
        };
        let mut ureq_request = agent.request(http_method.as_str(), url);

        // Add the headers
        if let Some(ref hm) = header_map {
            for (header, value) in hm {
                ureq_request = ureq_request.set(header, value);
            }
        }

        // Execute the request
        let start = Instant::now();
        let ureq_response = if http_method == HttpMethod::Post || http_method == HttpMethod::Put {
            let payload: &'static str = &payloads[hb_request.url_index];

            // TODO: allow user to override POST request content-type, setting it to json for now
            ureq_request
                .set("Content-Type", "application/json")
                .send_string(payload)
        } else {
            ureq_request.call()
        };

        // Track response code statistics
        let mut duration = 0;
        match ureq_response {
            Ok(response) => {
                let count = result.status.entry(response.status()).or_insert(0);
                *count += 1;

                // Read the response and track errors
                let mut reader = BufReader::new(response.into_reader());
                let mut sink = io::empty();
                if let Err(e) = io::copy(&mut reader, &mut sink) {
                    result.response_errors += 1;
                    warn!("Error retrieving response for {}: {}", url, e);
                }

                let end = Instant::now();
                duration = end.duration_since(start).as_millis() as u64;
            }
            Err(Error::Status(code, response)) => {
                result.request_errors += 1;
                warn!("Hit error processing {}: {} {:?}", url, code, response);
            }
            Err(Error::Transport(transport)) => {
                panic!("Hit transport layer error {}: {}", url, transport);
            }
        }

        // Update the latency histogram
        result.latency += duration;

        // Track the per-request latency too
        result.request_times.push((hb_request.url_index, duration));
    }

    result
}
