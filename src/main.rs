#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant};

use crate::config::{HttpMethod, LoadTestContext};
use crate::workers::BenchResult;

mod config;
mod requestgen;
mod workers;

/// Parses command line arguments, launches the workers, consolidates results
fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse the command line and read in the set of URLs we use to test
    let LoadTestContext {
        config,
        urls,
        payloads,
    } = config::Config::from_cmdline()?;

    // When testing POST or PUT, the total number of distinct requests should be the size of payloads list
    let distinct_requests_count = match config.http_method {
        HttpMethod::Post | HttpMethod::Put => payloads.len(),
        _ => urls.len(),
    };

    // Initialise the request generator from the config
    let request_generator = requestgen::RequestGenerator::new(&config, distinct_requests_count);

    // Launch the workers
    let bench_start = Instant::now();
    info!("Running test");
    let result_summary = workers::run_test(
        config.http_method,
        config.concurrency,
        &request_generator,
        urls,
        payloads,
    );
    let bench_end = Instant::now();

    // Print the results of the benchmark
    let bench_duration = bench_end.duration_since(bench_start);
    print_results(bench_duration, &result_summary);

    // Generate a report if required
    if let Some(slow_percentile) = config.slow_percentile {
        print_slow_report(result_summary, urls, slow_percentile);
    }

    Ok(())
}

// Output the report
fn print_slow_report(summary: BenchResult, urls: &[String], slow_percentile: f64) {
    // Collect all the durations by URL
    let mut url_stats = HashMap::new();
    for (url_index, duration) in summary.request_times {
        let url = urls[url_index].as_str();
        url_stats.entry(url).or_insert_with(Vec::new).push(duration);
    }

    // Determine the lower latency bound for the request to be included in the slow requests report
    let lower_bound = summary.latency.value_at_percentile(slow_percentile);

    // Compute the summary stats by URL and filter to those that exceed the smallest latency cutoff
    let mut lines = url_stats
        .iter()
        .filter_map(|(url, durations)| {
            // Compute the basic stats we need for the report line
            let (min, max, sum) = durations.iter().fold((u64::MAX, 0, 0), |state, &duration| {
                (
                    state.0.min(duration),
                    state.1.max(duration),
                    state.2 + duration,
                )
            });

            // If the max latency didn't exceed our lower bound then we just ignore this URL for the report
            if max < lower_bound {
                return None;
            }

            let count = durations.len();
            let avg = sum / count as u64;
            Some(ReportLine {
                url,
                count,
                min,
                max,
                avg,
            })
        })
        .collect::<Vec<ReportLine>>();

    // Sort by latency in descending order and dump out the report
    lines.sort_by(|l, r| r.max.cmp(&l.max));

    println!(
        "\nSlow requests ({}%'ile -> {}ms):\nmax\tavg\tmin\tcount\trequest",
        slow_percentile, lower_bound
    );
    for line in lines {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            line.max, line.avg, line.min, line.count, line.url
        );
    }
}

// Output the benchmark results
fn print_results(bench_duration: Duration, summary: &BenchResult) {
    // Note errors if they occurred
    if summary.request_errors > 0 {
        warn!("*** {} request errors", summary.request_errors);
    }
    if summary.response_errors > 0 {
        warn!("*** {} response errors", summary.response_errors);
    }

    // Dump the status codes
    let mut codes = summary.status.keys().copied().collect::<Vec<u16>>();
    codes.sort_unstable();
    println!("\nHTTP responses:");
    for code in codes {
        println!("{}\t{}", code, summary.status.get(&code).unwrap());
    }

    // Dump the latency
    println!(
        "\nBenchmark run time {}s.\nLatency:",
        bench_duration.as_secs_f32()
    );
    for p in &[
        50f64, 75f64, 95f64, 99f64, 99.9f64, 99.99f64, 99.999f64, 100f64,
    ] {
        let millis = &summary.latency.value_at_percentile(*p);
        println!("{}%\t{}ms", p, millis);
    }
}

struct ReportLine<'a> {
    url: &'a str,
    count: usize,
    min: u64,
    max: u64,
    avg: u64,
}
