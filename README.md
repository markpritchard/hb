# hb

`hb` is an endpoint focused HTTP load testing / benchmark tool.

## Description

The goal of `hb` is to provide a simple, robust tool to apply load against an endpoint (or set of endpoints). For example, it could replay load against a web-server, or evaluate the performance of REST APIs.
It does not attempt to model workflows or user journeys.

It is similar to many other tools such as:
* [ApacheBench](https://httpd.apache.org/docs/current/programs/ab.html)
* [Siege](https://github.com/JoeDog/siege)
* [wrk](https://github.com/wg/wrk)
* [wrk2](https://github.com/giltene/wrk2)

### Features

`hb` supports the following features:
* a large number (millions) of URLs (i.e. can test multiple endpoints or resources)
* variable load concurrency (i.e. N worker threads)
* variable request rate (N/unit of time) with optional distribution (uniform, constant, negative exponential)
* reports latency based on percentiles
* tracks the slowest N percentile of requests, and dumps a report after the run

Future features include:
* ability to replay from a load-balancer log file at a time scaling multiple ([link](https://github.com/markpritchard/hb/issues/2))
* track delays due to coordinated omission ([link](https://github.com/markpritchard/hb/issues/3))

### Why?

Why another load testing tool? In my experience, while excellent, the above tools have various problems:
* ApacheBench reports are extremely useful, but it has a limited feature set (e.g. only a single URL or request)
* siege supports multiple requests (e.g. calls to various REST services behind an LB), but appears to crash with large volumes of URLs or concurrent requests
* wrk has a good mix of features but suffers from [coordinated omission](https://www.youtube.com/watch?v=lJ8ydIuPFeU)
* wrk2 is a fantastic improvement on wrk, but has limited load generation mechanisms such as [negative exponential delays](http://perfdynamics.blogspot.com/2012/05/load-testing-with-uniform-vs.html) 

A new tool also provides an opportunity to use a modern language ([Rust](https://www.rust-lang.org/)) to support multiple platforms easily, whilst maintaining security, reliability and performance.

## Getting Started

### Building

At the moment the only platform supported is Linux. Future releases will include pre-built artifacts for MacOS and Windows.

#### Linux, static build, x86_64

This uses a [Docker container](https://github.com/emk/rust-musl-builder) to build a statically linked binary that can be run on any reasonable x86_64 environment.

```
alias rust-musl-builder='docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder'
rust-musl-builder cargo build --release
```

### Running

Execute `./hb -h` to view usage.

## License

This project is licensed under the [MIT license](LICENSE.txt).

## Acknowledgments

My gratitude to my employer [SEEK](seek.com.au) who provided time and hardware to build the initial version of this tool, and an environment where we can engage in interesting work and tackle cool problems.
