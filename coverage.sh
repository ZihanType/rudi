#!/bin/bash

# rustup component add llvm-tools
# cargo install grcov
# cargo install cargo-nextest --locked

TMPDIR=`pwd`
cargo clean
CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='%t/target/coverage/profraw/%p-%m.profraw' cargo nextest run -p rudi
grcov $TMPDIR/target/coverage/profraw/ --binary-path $TMPDIR/target/debug/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o $TMPDIR/target/coverage/
grcov $TMPDIR/target/coverage/profraw/ --binary-path $TMPDIR/target/debug/ -s . -t lcov --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o $TMPDIR/target/coverage/lcov.info
