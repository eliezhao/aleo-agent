# Aleo Network Rust Agent Repository
**aleo-agent is a simple-to-use library to interact with the Aleo Network in Rust.
The current repository is in version 1.0.0-alpha, indicating that it is still in the development phase.  
Examples available for the current version can be found in the examples folder of this repository.  
The current version of the agent is compatible with the `Testnet3` test network.**

## Building
I use `cargo` to build this repo. Make sure you have rust stable installed. To build the repo:

```shell
cargo build
```

## Docs

## Release
To release, increase the version number in all crates and run `cargo build` to update the lock file.

## Docs
Generate usage documentation using cargo or check [crates.io]()

```shell
cargo doc --no-deps --open
```