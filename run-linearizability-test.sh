#!/usr/bin/env bash
# run-linearizability-test.sh — End-to-end linearizability verification pipeline.
#
# Runs the test harness against a live cluster, then verifies the generated
# history with the Knossos linearizability checker.
#
# Usage:
#   ./run-linearizability-test.sh [--nemesis] [--nemesis-crash]
#
# Prerequisites:
#   - Docker (for the cluster and optionally for Knossos)
#   - Rust toolchain (for building the test harness)
#   - Clojure CLI or Docker (for Knossos — Docker is used if clojure is absent)
#
# Exit codes:
#   0 — history is linearizable
#   1 — linearizability violation detected, test harness failure, or setup error

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HISTORY_FILE="${SCRIPT_DIR}/history.edn"
COMPOSE_DIR="${SCRIPT_DIR}/build_scripts"

NEMESIS_FLAGS=""
for arg in "$@"; do
    case "$arg" in
        --nemesis)       NEMESIS_FLAGS="$NEMESIS_FLAGS --nemesis" ;;
        --nemesis-crash) NEMESIS_FLAGS="$NEMESIS_FLAGS --nemesis-crash" ;;
        *)               echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

cleanup() {
    echo ""
    echo "=== Tearing down cluster ==="
    cd "$COMPOSE_DIR" && docker compose down --timeout 5 2>/dev/null || true
}
trap cleanup EXIT

# ---- Step 1: Build --------------------------------------------------------

echo "=== Step 1: Building server image and test harness ==="
cd "$COMPOSE_DIR" && docker compose build --quiet
cd "$SCRIPT_DIR"  && cargo build --release --bin test-harness 2>&1 | tail -3

# ---- Step 2: Start cluster ------------------------------------------------

echo ""
echo "=== Step 2: Starting 3-node cluster ==="
cd "$COMPOSE_DIR" && docker compose up -d
echo "Waiting 10s for cluster to elect leader and stabilize..."
sleep 10

# ---- Step 3: Run test harness ---------------------------------------------

echo ""
echo "=== Step 3: Running test harness ==="
cd "$SCRIPT_DIR"
cargo run --release --bin test-harness -- \
    --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 \
    --num-clients 5 \
    --ops-per-client 50 \
    --key-range 3 \
    --read-ratio 0.4 \
    --cas-ratio 0.2 \
    --output "$HISTORY_FILE" \
    $NEMESIS_FLAGS

if [ ! -f "$HISTORY_FILE" ]; then
    echo "ERROR: Test harness did not produce $HISTORY_FILE"
    exit 1
fi

echo ""
echo "=== Step 4: Verifying linearizability with Knossos ==="
"${SCRIPT_DIR}/check-linearizability.sh" "$HISTORY_FILE"
