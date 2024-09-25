#!/bin/bash
cd "$(dirname "$0")"

# Remove all output files.
rm -f target/release/*simp*.gpx && rm -f target/release/*.xlsx

# A run that just does stage detection.
#RUST_LOG=TRACE cargo run --release -- --detect-stages --min-stop-time=5 --resume-speed=15 --write-trackpoints --write-trackpoint-hyperlinks
RUST_LOG=TRACE cargo run --release -- --detect-stages --min-stop-time=5 --resume-speed=15 --write-trackpoints

# A run that just does simplification.
#cargo run --release -- --metres=10
