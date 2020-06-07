use crate::config;
use crate::timedelay::time_delay;
use crate::urlsource::url_source;

pub(crate) fn run(config: &config::Config) {
    let _url_source = url_source(config);
    let _time_delay = time_delay(config);
}