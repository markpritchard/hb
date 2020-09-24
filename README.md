
## Release builds

### No external dependencies

This uses a [Docker container](https://github.com/emk/rust-musl-builder) to build a statically linked binary that can be run on any reasonable x86_64 environment.

```
alias rust-musl-builder='docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder'
rust-musl-builder cargo build --release
```
