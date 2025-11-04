#!/usr/bin/env bash
# Usage: ./run_rust_benchmark.sh [num_runs]

set -euo pipefail

NUM_RUNS=10         # Number of runs (change as needed)
RUST_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUST_EXE="$RUST_DIR/target/release/nanovna-saver"


if [[ ! -x "$RUST_EXE" ]]; then
  echo "Building Rust project..."
  cargo build --release --manifest-path "$RUST_DIR/Cargo.toml"
fi


BYTES_TOTAL=0
RUN_TIMES=()
BYTES_PER_RUN=()
START_TIME=$(date +%s.%N)

for ((i=1; i<=NUM_RUNS; i++)); do
  RUN_START=$(date +%s.%N)
  OUT=$("$RUST_EXE")
  RUN_END=$(date +%s.%N)
  RUN_TIME=$(awk -v s="$RUN_START" -v e="$RUN_END" 'BEGIN { printf "%.6f", e-s }')
  BYTES=$(echo "$OUT" | grep -Eo 'Read[[:space:]]+[0-9]+[[:space:]]+bytes' | awk '{print $2}' | tail -n1)
  BYTES=${BYTES:-0}
  BYTES_TOTAL=$((BYTES_TOTAL + BYTES))
  RUN_TIMES+=("$RUN_TIME")
  BYTES_PER_RUN+=("$BYTES")



END_TIME=$(date +%s.%N)
ELAPSED=$(awk -v s="$START_TIME" -v e="$END_TIME" 'BEGIN { printf "%.6f", e-s }')

# Calculate mean and stddev
MEAN_TIME=$(awk '{s+=$1}END{printf "%.6f",s/NR}' <<< "${RUN_TIMES[*]}")
STD_TIME=$(awk -v m="$MEAN_TIME" '{s+=($1-m)^2}END{printf "%.6f", sqrt(s/NR)}' <<< "${RUN_TIMES[*]}")
AVG_TIME_PER_READING="$MEAN_TIME"
THROUGHPUT=$(awk -v b="$BYTES_TOTAL" -v t="$ELAPSED" 'BEGIN { printf "%.2f", (b/t)/1024 }')

printf '\nBENCHMARK RESULTS\n'
printf 'Runs:                %d\n' "$NUM_RUNS"
printf 'Total Time:          %.6f seconds\n' "$ELAPSED"
printf 'Mean Run Time:       %.6f seconds\n' "$MEAN_TIME"
printf 'Stddev Run Time:     %.6f seconds\n' "$STD_TIME"
printf 'Time per Reading:    %.6f seconds\n' "$AVG_TIME_PER_READING"
printf 'Total Bytes:         %d\n' "$BYTES_TOTAL"
printf 'Throughput:          %.2f KB/s\n' "$THROUGHPUT"
