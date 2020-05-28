#[macro_use]
extern crate log;

mod app;
mod config;
mod urlsource;

fn main() {
    env_logger::init();

    // Parse the command line and run the app
    let config = config::Config::from_cmdline();
    app::run(&config);
}
