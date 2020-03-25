# `spi-memory`

[![crates.io](https://img.shields.io/crates/v/spi-memory.svg)](https://crates.io/crates/spi-memory)
[![docs.rs](https://docs.rs/spi-memory/badge.svg)](https://docs.rs/spi-memory/)
[![Build Status](https://travis-ci.org/jonas-schievink/spi-memory.svg?branch=master)](https://travis-ci.org/jonas-schievink/spi-memory)

This crate provides a generic [`embedded-hal`]-based driver for different
families of SPI Flash and EEPROM chips.

Right now, only 25-series Flash chips are supported. Feel free to send PRs to
support other families though!

Please refer to the [changelog](CHANGELOG.md) to see what changed in the last
releases.

[`embedded-hal`]: https://github.com/rust-embedded/embedded-hal

## Usage

Add an entry to your `Cargo.toml`:

```toml
[dependencies]
spi-memory = "0.2.0"
```

Check the [API Documentation](https://docs.rs/spi-memory/) for how to use the
crate's functionality.
