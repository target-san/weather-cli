# Weather CLI application

## Development

Project uses `cargo-make` for automating certain tasks:
* `cargo make ci` - run all necessary checks the way CI runs them; includes:
    * `cargo make ci-fmt` - doesn't actually format code but rather checks if it's formatted correctly;
        use `cargo fmt` to run formatter
    * `cargo make ci-lint` - runs `cargo clippy` with additional settings
    * `cargo make ci-test` - runs `cargo test`

## Notes and limitations

Implementation of [Weather CLI demo application](https://gist.github.com/anelson/0029f620105a19702b5eed5935880a28)

* `AerisWeather` requires complex OAuth-based authentication and application registration, so it's omitted intentionally
* `OpenWeather` provides only 24h forecast on free plans, so custom date isn't supported
* `AccuWeather`'s historical data is available only on enterprise plans, so custom date isn't supported either
* `AccuWeather` isn't tested on CI - it has ridiculous limitation of 50 requests per day on free/trial plans.
    Since this limit includes location requests, effective number of queries is 25 RPD.
* INI is intentionally used as config format. More complicated formats like TOML or JSON would simply stand in way
    because they would need more data type checking and conversions than actually needed.
* Most application code uses normal blocking IO, although async IO is used for network queries.
    Async processing is actually excessive in such a small demo application.
    Yet since it's a demo, there would've been questions why async isn't used,
    so hybrid solution was adopted. This also saves a bit of performance,
    though it's negligible on such scale.

### Possible changes and optimizations

* Use statically generated dispatch functions for provider registry
    * Remove need for dynamic registry
    * Generate provider-specific config readers and verifiers at compile-time
    * Generate enum for providers set
    * Remove futures boxing
