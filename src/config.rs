use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::io::BufRead;
use std::str::FromStr;

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
    pub http_method: HttpMethod,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum HttpMethod {
    Get,
    Post,
    Put,
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let upper_case = s.to_uppercase();
        match upper_case.as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            _ => Err(()),
        }
    }
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

pub(crate) struct LoadTestContext {
    pub(crate) config: Config,
    pub(crate) urls: &'static Vec<String>,
    pub(crate) payloads: &'static Vec<String>,
}

impl Config {
    pub(crate) fn from_cmdline<I, T>(args: I) -> Result<LoadTestContext, Box<dyn Error>>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
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
                .value_parser(PossibleValuesParser::new(["c", "u", "ne"]))
                .short('d')
                .long("delay-dist")
                .value_name("distribution")
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

            .arg(Arg::new("httpmethod")
                .value_parser(PossibleValuesParser::new(["GET", "POST", "PUT"]))
                .short('m')
                .long("method")
                .value_name("httpmethod")
                .default_value("GET")
                .help("The HTTP method used for this test. Only GET, POST, and PUT are currently supported. \
                          When [http_method] is set to POST or PUT only the first url is used for all requests, and you must \
                          also supply 'payloads' argument."))

            .arg(Arg::new("payloads")
                .long("payloads")
                .value_name("payload file path")
                .help("The payload for POST and PUT requests. Each request in the test takes one line in this file as payload."))

            .get_matches_from(args);

        // Extract the URLs
        let url_prefix = matches.get_one::<String>("urlprefix");
        let url_file = matches.get_one::<String>("urlfile");
        let args_urls: Option<Vec<String>> = matches
            .get_many::<String>("urls")
            .map(|v| v.into_iter().cloned().collect());
        let urls = Box::leak(Box::new(load_urls(url_prefix, url_file, args_urls)?));

        // Grab basic params
        // TODO cleanup parsing of these arguments
        let concurrency: u16 = *matches.get_one("concurrency").unwrap();
        let requests: usize = *matches.get_one("requests").unwrap();
        let order = matches.get_one::<String>("order").unwrap();
        let order = match order.as_str() {
            "s" => RequestOrder::Sequential,
            _ => RequestOrder::Random,
        };
        let delay_ms: u32 = *matches.get_one("delay").unwrap();
        let delay_distrib = matches.get_one::<String>("delaydist").unwrap();
        let delay_distrib = match delay_distrib.as_str() {
            "u" => DelayDistribution::Uniform,
            "ne" => DelayDistribution::NegativeExponential,
            _ => DelayDistribution::Constant,
        };
        let slow_percentile = matches.get_one::<f64>("reportslow").copied();

        let http_method = matches.get_one::<String>("httpmethod").unwrap();
        let http_method = HttpMethod::from_str(http_method).expect("Unsupported http method");

        let payloads = if let Some(payloads_file) = matches.get_one::<String>("payloads") {
            info!("Loading payloads from {}", payloads_file);
            let file = fs::File::open(payloads_file);
            match file {
                Ok(file) => io::BufReader::new(file)
                    .lines()
                    .map(|l| l.unwrap())
                    .collect(),
                // If we are unable to load 'payloads' file simply exit
                Err(error) => panic!("Unable to open file: {:?}", error),
            }
        } else {
            vec![]
        };
        let payloads = Box::leak(Box::new(payloads));

        // If we are running POST or PUT, we need to have payloads and can only have a single URL as the endpoint
        match http_method {
            HttpMethod::Post | HttpMethod::Put => {
                assert!(
                    !payloads.is_empty(),
                    "Payloads must be supplied when http_method is set to POST or PUT"
                );
                assert_eq!(urls.len(), 1, "Must only have a single URL for POST or PUT");
            }
            _ => {}
        }

        Ok(LoadTestContext {
            config: Config {
                concurrency,
                requests,
                order,
                delay_ms,
                delay_distrib,
                slow_percentile,
                http_method,
            },
            urls,
            payloads,
        })
    }
}

fn load_urls(
    url_prefix: Option<&String>,
    url_file: Option<&String>,
    args_urls: Option<Vec<String>>,
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

    // Verify we can parse the URL prefix from the command line
    #[test]
    fn argparse_url_prefix() {
        let args = vec!["hb", "-p", "http://localhost", "/test"];
        let context = Config::from_cmdline(args).unwrap();
        assert_eq!(&vec!("http://localhost/test".to_string()), context.urls);
    }

    // Verify we can parse the concurrency from the command line
    #[test]
    fn argparse_concurrency() {
        let args = vec!["hb", "-c", "42", "http://test"];
        let context = Config::from_cmdline(args).unwrap();
        assert_eq!(42, context.config.concurrency);
    }

    // Verify that we prepend the URL prefix to any urls not currently prefixed with a valid scheme, host etc
    #[test]
    fn url_prefix_prepended() {
        let prefix = "http://localhost:8070/".to_string();
        let expected = "http://localhost:8070/abc123?def=456".to_string();
        let urls = vec![
            expected.clone(),
            "abc123?def=456".to_string(),
            "/abc123?def=456".to_string(),
        ];

        let loaded = load_urls(Some(&prefix), None, Some(urls)).unwrap();
        for test in loaded {
            assert_eq!(expected, test);
        }
    }
}
