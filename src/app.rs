use crate::config;
use crate::urlsource;

pub(crate) fn run(config: &config::Config) {
    let _url_source = urlsource::url_source(config);

}