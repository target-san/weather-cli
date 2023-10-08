# Weather CLI application

CLI application which allows user to fetch weather data at specific location from one of providers.
Providers supported:

* [https://www.accuweather.com/]. Provides data on current weather conditions.
    Doesn't support forecast on specific date.
* [https://openweathermap.org/]. Provides data on current weather conditions.
    Doesn't support forecast on specific date.
* [https://www.weatherapi.com/]. May provide weather data for specific date, depending on
    user's subscription plan.

Please note that using any of these providers requires registration and possibly subscription
to paid plan.

Features:

* `weather configure` - configure specific forecast provider, either in interactive mode
    or by passing all necessary parameters via command line
* `weather get` - get weather info for current provider - or pick another provider
    and optionally make it current one
* `weather clear` - clear configuration for specific or all forecast providers
* `weather list` - list more detailed information on all supported forecast providers

See application's CLI help for more details

## Development

Project uses `cargo-make` for automating certain tasks:
* `cargo make ci` - run all necessary checks the way CI runs them; includes:
    * `cargo make ci-fmt` - doesn't actually format code but rather checks if it's formatted correctly;
        use `cargo fmt` to run formatter
    * `cargo make ci-lint` - runs `cargo clippy` with additional settings
    * `cargo make ci-test` - runs `cargo test`

CI executes all these checks, so ensure your change complies with project style
by running `cargo make ci` 

## Notes and limitations

Implementation of [Weather CLI demo application](https://gist.github.com/anelson/0029f620105a19702b5eed5935880a28)

* `AerisWeather` requires complex OAuth-based authentication and application registration, so it's omitted intentionally
* `OpenWeather` provides only 24h forecast on free plans, so custom date isn't supported
* `AccuWeather`'s historical data is available only on enterprise plans, so custom date isn't supported either
* `AccuWeather` is excluded from CI. Its free trial is extremely limited,
    allowing either 50 requests per day (or 50 requests in total?). This includes location requests, so we get
    effectively 25 weather requests.
* INI is intentionally used as config format. More complicated formats like TOML or JSON would simply stand in way
    because they would need more data type checking and conversions than actually needed.
* Most application code uses normal blocking IO, although async IO is used for network queries.
    Async processing is actually excessive in such a small demo application.
    Yet since it's a demo, there would've been questions why async isn't used,
    so hybrid solution was adopted. This also saves a bit of performance,
    though it's negligible on such scale.

### Possible changes and optimizations

* Use statically generated dispatch functions for provider registry
    * Removes need for dynamic registry
    * Generate provider-specific config readers and verifiers at compile-time
    * Generate enum for providers set
    * Remove futures boxing
