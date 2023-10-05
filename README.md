# Weather CLI application

## Development

Project uses `cargo-make` for automating certain tasks:
* `cargo make ci` - run all necessary checks the way CI runs them; includes:
    * `cargo make ci-fmt` - doesn't actually format code but rather checks if it's formatted correctly;
        use `cargo fmt` to run formatter
    * `cargo make ci-lint` - runs `cargo clippy` with additional settings
    * `cargo make ci-test` - runs `cargo test`
