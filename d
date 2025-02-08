#!/bin/bash
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc
echo '*** http://localhost:5000/recurring_tasks/ ***'
caddy file-server --root ./target/doc --listen :5000
