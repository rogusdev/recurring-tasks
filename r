#!/bin/bash
cargo fmt
RUST_LOG=debug cargo run -p db $@
