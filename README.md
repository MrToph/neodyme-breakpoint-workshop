# Solana Security Workshop

Welcome to our Solana Security Workshop!

All details are in the docs. To check it out online, visit [https://workshop.neodyme.io](https://workshop.neodyme.io).

To build it yourself, install mdbook (`cargo install mdbook`) and run `mdbook serve`.

# My notes

```bash
# quick start
# compile all contracts
cargo build-bpf --workspace

# run level0 exploit in pocs/src/bin/level0.rs
RUST_BACKTRACE=1 cargo run --bin level0
```