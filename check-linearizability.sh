#!/usr/bin/env bash
# check-linearizability.sh — Run the Knossos linearizability checker on a history.edn file.
#
# Usage:
#   ./check-linearizability.sh [path/to/history.edn]
#
# If no path is given, defaults to ./history.edn.
#
# Methods (tried in order):
#   1. Local Clojure CLI (`clojure` on PATH)
#   2. Docker (`clojure:temurin-21-tools-deps-1.12.0.1530` image)
#
# Exit codes:
#   0 — history is linearizable
#   1 — linearizability violation detected OR runtime error

set -euo pipefail

HISTORY_FILE="${1:-history.edn}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KNOSSOS_DIR="${SCRIPT_DIR}/knossos"

# ---- Validate input --------------------------------------------------------

if [ ! -f "$HISTORY_FILE" ]; then
    echo "ERROR: History file not found: $HISTORY_FILE"
    echo "Run the test harness first to generate it:"
    echo "  cargo run --release --bin test-harness -- --output history.edn"
    exit 1
fi

# Convert to absolute path for Docker volume mount
HISTORY_ABS="$(cd "$(dirname "$HISTORY_FILE")" && pwd)/$(basename "$HISTORY_FILE")"

echo "=== Knossos Linearizability Checker ==="
echo "History file: $HISTORY_ABS"
echo ""

# ---- Method 1: Local Clojure CLI -------------------------------------------

if command -v clojure &>/dev/null; then
    echo "Using local Clojure CLI..."
    cd "$KNOSSOS_DIR"
    exec clojure -M -m checker "$HISTORY_ABS"
fi

# ---- Method 2: Docker ------------------------------------------------------

if command -v docker &>/dev/null; then
    echo "Clojure CLI not found. Using Docker..."
    DOCKER_IMAGE="clojure:temurin-21-tools-deps-1.12.0.1530"

    docker run --rm \
        -v "$KNOSSOS_DIR:/knossos:ro" \
        -v "$HISTORY_ABS:/data/history.edn:ro" \
        -w /knossos \
        "$DOCKER_IMAGE" \
        clojure -M -m checker /data/history.edn

    exit $?
fi

# ---- Neither available ------------------------------------------------------

echo "ERROR: Neither 'clojure' nor 'docker' found on PATH."
echo ""
echo "Install one of the following:"
echo "  - Clojure CLI: https://clojure.org/guides/install_clojure"
echo "  - Docker:      https://docs.docker.com/get-docker/"
exit 1
