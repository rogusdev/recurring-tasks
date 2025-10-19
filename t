#!/bin/bash
cargo fmt
RUST_LOG=debug cargo test --no-default-features -F system $@
# ./t -- --nocapture
# ./t -- --exact 'tests::half_offset'
# ./t -- --nocapture --exact 'tests::run_cancelled'
