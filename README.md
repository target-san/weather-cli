# Weather CLI application

Implementation of [Weather CLI demo application](https://gist.github.com/anelson/0029f620105a19702b5eed5935880a28)

Notes and limitations:
* `AerisWeather` requires complex OAuth-based authentication and application registration, so it's omitted intentionally
* `OpenWeather` provides only 24h forecast on free plans, so custom date isn't supported
* `AccuWeather`'s historical data is available only on enterprise plans, so custom date isn't supported either

## Development

Project uses `cargo-make` for automating certain tasks:
* `cargo make ci` - run all necessary checks the way CI runs them; includes:
    * `cargo make ci-fmt` - doesn't actually format code but rather checks if it's formatted correctly;
        use `cargo fmt` to run formatter
    * `cargo make ci-lint` - runs `cargo clippy` with additional settings
    * `cargo make ci-test` - runs `cargo test`

## Possible future optimizations

Some of these are just byte-crunching, yet left for reference

* Use statically generated dispatch functions for provider registry
    * Remove need for dynamic registry
    * Generate provider-specific config readers and verifiers at compile-time
    * Generate enum for providers set
    * Remove futures boxing
* Manually start async event loop only when preparatory work is done.
