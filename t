#!/bin/bash
cargo fmt
RUST_LOG=debug cargo test $@
# ./t -- --nocapture
# ./t -- --exact 'half_offset'
