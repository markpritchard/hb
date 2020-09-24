use crate::config;
use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Mutex;

mod indexseq;
mod timedelay;

/// Generates requests from the source URLs according to the configured order, time delay etc
pub(crate) struct RequestGenerator {
    urls: Vec<String>,
    url_index_supplier: Box<dyn indexseq::IndexSupplier>,
    time_delay_supplier: Box<dyn timedelay::TimeDelaySupplier>,
    pub(crate) progress: Mutex<ProgressBar>,
}

impl RequestGenerator {
    /// Create a new generator from the config
    /// NOTE: mutable reference since we want to own the URLs so we can hand out references across threads
    pub(crate) fn new(config: &mut config::Config) -> RequestGenerator {
        // Grab the URLs read from the input file etc
        let urls = std::mem::replace(&mut config.urls, Vec::<String>::new());

        // Create the index supplier (used to select the next URL from the test set)
        let index_limit = urls.len();
        let num_requests = config.requests;
        let url_index_supplier = indexseq::create_supplier(&config.order, index_limit, num_requests);

        // Create the time delay supplier used to schedule the next request
        let time_delay_supplier = timedelay::create_supplier(&config.delay_ms, &config.delay_distrib);

        // Initialise the request generator
        let progress = ProgressBar::new(num_requests as u64);
        progress.set_style(ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .progress_chars("#>-"));

        RequestGenerator { urls, url_index_supplier, time_delay_supplier, progress: Mutex::new(progress) }
    }

    /// Return the next request to execute or None if no more requests need to be executed
    pub(crate) fn next(&self) -> Option<Request> {
        self.url_index_supplier.next_index()
            .map(move |i| {
                // Extract the URL corresponding to the selected index
                let url = self.urls[i].as_str();

                // Determine the time delay for this request
                let sleep = self.time_delay_supplier.next_delay();

                // Bump progress
                let progress = self.progress.lock().unwrap();
                progress.inc(1);

                Request { url, sleep }
            })
    }
}

/// A request to execute
#[derive(Debug, PartialEq)]
pub(crate) struct Request<'a> {
    pub url: &'a str,
    pub sleep: Duration,
}

// Need to share the generator across threads
unsafe impl Send for RequestGenerator {}

// Need to share a reference to the generator across threads (i.e. the Arc::clone calls)
unsafe impl Sync for RequestGenerator {}

#[cfg(test)]
mod tests {
    use crate::config::{DelayDistribution, RequestOrder};

    use super::*;

    // Verify the generator emits requests with a simple end-to-end configuration
    #[test]
    fn request_generator() {
        let mut config = config::Config {
            urls: vec!("http://one".to_string(), "http://two".to_string(), "http://three".to_string()),
            concurrency: 1,
            requests: 3,
            order: RequestOrder::Sequential,
            delay_ms: 1,
            delay_distrib: DelayDistribution::Constant,
        };

        let generator = RequestGenerator::new(&mut config);
        assert_eq!("http://one", generator.next().unwrap().url);
        assert_eq!("http://two", generator.next().unwrap().url);
        assert_eq!("http://three", generator.next().unwrap().url);
        assert_eq!(None, generator.next());
    }
}
