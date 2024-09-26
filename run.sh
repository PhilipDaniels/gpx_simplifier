#!/bin/bash
cd "$(dirname "$0")"

# Remove all output files.
rm -f target/release/*simp*.gpx && rm -f target/release/*.xlsx

# A run that just does stage detection.
RUST_LOG=DEBUG cargo run --release -- --detect-stages --write-trackpoints  # --write-trackpoint-hyperlinks

# A run that just does simplification.
#cargo run --release -- --metres=10
