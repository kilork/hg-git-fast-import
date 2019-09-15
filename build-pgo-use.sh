#!/usr/bin/env bash

PATH=$PATH:~/.rustup/toolchains/`rustup show active-toolchain | cut -f1 -d" "`/lib/rustlib/x86_64-apple-darwin/bin/

# STEP 3: Merge the `.profraw` files into a `.profdata` file
llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data

# STEP 4: Use the `.profdata` file for guiding optimizations
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" \
    cargo build --release