use std::error::Error;
use std::fs;
use std::io;
use std::io::BufRead;

use clap::builder::PossibleValuesParser;
use clap::{value_parser, Arg};
use url::Url;

pub(crate) struct Config {
    pub concurrency: u16,
    pub requests: usize,
    pub order: RequestOrder,
    pub delay_ms: u32,
    pub delay_distrib: DelayDistribution,
    pub slow_percentile: Option<f64>,
}

pub(crate) enum RequestOrder {
    Sequential,
    Random,
}

pub(crate) enum DelayDistribution {
    Constant,
    Uniform,
    NegativeExponential,
}

impl Config {
    pub(crate) fn from_cmdline() -> Result<(Config, Vec<String>), Box<dyn Error>> {
        let matches = clap::Command::new("httpbench")
            .version("0.1.0")
            .about("HTTP/S load testing tool")
            // Number of concurrent requests / workers
            .arg(Arg::new("concurrency")
                .value_parser(value_parser!(u16))
                .short('c')
                .value_name("concurrency")
                .default_value("10")
                .help("number of workers generating load"))

            // Number of requests to execute
            .arg(Arg::new("requests")
                .value_parser(value_parser!(usize))
                .short('n')
                .value_name("requests")
                .default_value("100")
                .help("number of requests to execute"))

            // Order of requests
            .arg(Arg::new("order")
                .value_parser(PossibleValuesParser::new(["r", "s"]))
                .short('o')
                .value_name("order")
                .default_value("r")
                .help("order in which to request URLs: r=random, s=sequential"))

            // Time delay between request *dispatch*
            .arg(Arg::new("delay")
                .value_parser(value_parser!(u32))
                .short('t')
                .long("delay-time")
                .value_name("ms")
                .default_value("0")
                .help("time between requests (NB: includes response time)"))

            .arg(Arg::new("delaydist")
                .short('d')
                .long("delay-dist")
                .value_name("distribution")
                .value_parser(PossibleValuesParser::new(["c", "u", "ne"]))
                .default_value("c")
                .requires("delay")
                .help("distribution of delay times: c=constant, u=uniform, ne=negative exponential"))

            // URLs we test with - in a file, or passed as command-line args
            .arg(Arg::new("urlfile")
                .short('f')
                .long("file")
                .value_name("file")
                .required_unless_present("urls")
                .conflicts_with("urls")
                .help("file containing URLs to request"))
            .arg(Arg::new("urls")
                .index(1)
                .value_name("URL"))

            // Prefix for URLs
            .arg(Arg::new("urlprefix")
                .short('p')
                .long("prefix")
                .value_name("urlprefix")
                .help("Prefix to automatically add to URLs (e.g. if your URL file contains just paths+query strings such as from a load-balancer log"))

            // Generate a slow queries report - anything over the nominated latency
            .arg(Arg::new("reportslow")
                .value_parser(value_parser!(f64))
                .short('s')
                .long("reportslow")
                .value_name("percentile")
                .help("Generate a report of requests over a given latency"))

            .get_matches();

        // Extract the URLs
        let url_prefix = matches.get_one("urlprefix").copied();
        let url_file = matches.get_one("urlfile").copied();
        let args_urls: Option<Vec<&str>> = matches.get_many("urls").map(|v| v.copied().collect());
        let urls = load_urls(url_prefix, url_file, args_urls)?;

        // Grab basic params
        // TODO cleanup parsing of these arguments
        let concurrency: u16 = *matches.get_one("concurrency").unwrap();
        let requests: usize = *matches.get_one("requests").unwrap();
        let order = match matches.get_one("order").copied().unwrap() {
            "s" => RequestOrder::Sequential,
            _ => RequestOrder::Random,
        };
        let delay_ms: u32 = *matches.get_one("delay").unwrap();
        let delay_distrib = match matches.get_one("delaydist").copied().unwrap() {
            "u" => DelayDistribution::Uniform,
            "ne" => DelayDistribution::NegativeExponential,
            _ => DelayDistribution::Constant,
        };
        let slow_percentile = *matches.get_one("reportslow").unwrap();

        let result = (
            Config {
                concurrency,
                requests,
                order,
                delay_ms,
                delay_distrib,
                slow_percentile,
            },
            urls,
        );
        Ok(result)
    }
}

fn load_urls(
    url_prefix: Option<&str>,
    url_file: Option<&str>,
    args_urls: Option<Vec<&str>>,
) -> Result<Vec<String>, Box<dyn Error>> {
    // Read from a file, or just collect the URLs on the command line
    let mut urls: Vec<String> = if let Some(url_file) = url_file {
        info!("Loading URLs from {}", url_file);
        // TODO better error handling
        let file = fs::File::open(url_file).unwrap();
        io::BufReader::new(file)
            .lines()
            .map(|l| l.unwrap())
            .collect()
    } else {
        args_urls.unwrap().iter().map(|s| (*s).to_owned()).collect()
    };

    // Prefix as required
    if let Some(url_prefix) = url_prefix {
        info!("Applying prefixes");
        let base = Url::parse(url_prefix)?;
        for url in urls.iter_mut() {
            match Url::parse(url) {
                // Nothing required in the OK case
                Ok(_) => {}
                // If no base, then fix
                Err(url::ParseError::RelativeUrlWithoutBase) => match base.join(url) {
                    Ok(prefixed) => *url = prefixed.into(),
                    Err(e) => warn!("URL {} is invalid: {}", url, e),
                },
                Err(e) => warn!("URL {} is invalid: {}", url, e),
            }
        }
    }

    Ok(urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify that we prepend the URL prefix to any urls not currently prefixed with a valid scheme, host etc
    #[test]
    fn url_prefix() {
        let prefix = "http://localhost:8070/";
        let expected = "http://localhost:8070/abc123?def=456";
        let urls = vec![expected, "abc123?def=456", "/abc123?def=456"];

        let loaded = load_urls(Some(prefix), None, Some(urls)).unwrap();
        for test in loaded {
            assert_eq!(expected, test);
        }
    }
}
