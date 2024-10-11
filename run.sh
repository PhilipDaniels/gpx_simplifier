#!/bin/bash
cd "$(dirname "$0")"

# Remove all output files.
rm -vf target/release/*simp*.gpx && rm -vf target/release/*.xlsx

# Build separately so that globbing works in the next command.
cargo build --release

RUST_LOG=DEBUG target/release/gapix -f --metres=5 --analyse target/release/*.gpx
