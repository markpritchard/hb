use std::fs;
use std::io;
use std::io::BufRead;

pub(crate) struct Config {
    pub urls: Vec<String>,
    pub concurrency: usize,
    pub requests: usize,
    pub order: RequestOrder,
}

pub(crate) enum RequestOrder {
    SEQUENTIAL,
    RANDOM,
}

impl Config {
    pub(crate) fn from_cmdline() -> Config {
        let matches = clap::App::new("httpbench")
            .version("0.1.0")
            .about("HTTP/S load testing tool")

            // Number of concurrent requests / workers
            .arg(clap::Arg::with_name("concurrency")
                .short("c")
                .value_name("concurrency")
                .default_value("10")
                .help("number of workers generating load"))

            // Number of requests to execute
            .arg(clap::Arg::with_name("requests")
                .short("n")
                .value_name("requests")
                .default_value("100")
                .help("number of requests to execute"))

            // Order of requests
            .arg(clap::Arg::with_name("order")
                .short("o")
                .value_name("order")
                .possible_values(&["r", "s"])
                .default_value("r")
                .help("order in which to request URLs: r=random, s=sequential"))

            // URLs we test with - in a file, or passed as command-line args
            .arg(clap::Arg::with_name("urlfile")
                .short("f")
                .long("file")
                .value_name("file")
                .required_unless("urls")
                .conflicts_with("urls")
                .help("file containing URLs to request"))
            .arg(clap::Arg::with_name("urls")
                .index(1)
                .min_values(0)
                .value_name("URL"))

            .get_matches();

        // Extract the URls
        let urls = load_urls(matches.value_of("urlfile"), matches.values_of("urls"));

        // Grab basic params
        // TODO cleanup parsing of these arguments
        let concurrency: usize = matches.value_of("concurrency").unwrap().parse::<>().unwrap();
        let requests: usize = matches.value_of("requests").unwrap().parse::<>().unwrap();
        let order = match matches.value_of("order").unwrap() {
            "s" => RequestOrder::SEQUENTIAL,
            _ => RequestOrder::RANDOM
        };

        Config { urls, concurrency, requests, order }
    }
}

fn load_urls(url_file: Option<&str>, urls: Option<clap::Values>) -> Vec<String> {
    // Read from a file, or just collect the URLs on the command line
    if let Some(url_file) = url_file {
        info!("Loading URLs from {}", url_file);
        // TODO better error handling
        let file = fs::File::open(url_file).unwrap();
        io::BufReader::new(file).lines().map(|l| l.unwrap()).collect()
    } else {
        urls.unwrap().map(|s| s.to_owned()).collect()
    }
}
