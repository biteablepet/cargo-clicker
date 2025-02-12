# `cargo-clicker`

[![crates.io](https://img.shields.io/crates/v/cargo-clicker.svg)](https://crates.io/crates/cargo-clicker)
[![Rust CI](https://github.com/biteablepet/cargo-clicker/workflows/Release/badge.svg?branch=main)](https://github.com/biteablepet/cargo-clicker/actions/workflows/release.yml)

haven't you ever wanted some positive reinforcement when you finally get those tests to pass?

# installation

you can `cargo install cargo-clicker`.

# usage

add "clicker" immediately after cargo and program with extra reinforcement

```
$ cargo clicker test
```

# configuration

* `CARGO_CLICKER_SILENCE` - never play sound when this variable is set
* `CARGO_CLICKER_RESPONSES` - directory containing "Positive" and "Negative" subdirectories with custom reinforcement~

`cargo-clicker` comes with some built in clicks, but if you want special treatment then set `CARGO_CLICKER_RESPONSES`
appropriately and have at it. if the appropriate directory is empty, no sound be played.

# thanks

thanks to [`cargo-mommy`](https://crates.io/crates/cargo-mommy) for the inspiration 🥺

# licensing

dual-licensed MIT and Apache 2.0
