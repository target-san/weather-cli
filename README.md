# Weather CLI application

## Development

Project uses `cargo-make` for automating certain tasks:
* `cargo make ci` - run all necessary checks the way CI runs them; includes:
    * `cargo make ci-fmt` - doesn't actually format code but rather checks if it's formatted correctly;
        use `cargo fmt` to run formatter
    * `cargo make ci-lint` - runs `cargo clippy` with additional settings
    * `cargo make ci-test` - runs `cargo test`

## Possible future optimizations

Some of these are just byte-crunching, yet left for reference

* Use statically generated dispatch functions for provider registry.
    Since set of providers is known at compile-time, all necessary functions
    can be generated via macro. This would remove need for dynamic registry and
    futures boxing, and allow generate nice enums for provider names for CLAP.
* Manually start async event loop only when preparatory work is done.
