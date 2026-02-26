# Omnipaxos-kv
This is an example repo showcasing the use of the [Omnipaxos](https://omnipaxos.com) consensus library to create a simple distributed key-value store. The source can be used to build server and client binaries which communicate over TCP. The repo also contains a benchmarking example which delploys Omnipaxos servers and clients onto [GCP](https://cloud.google.com) instances and runs an experiment collecting client response latencies (see `benchmarks/README.md`).

# Prerequisites
 - [Rust](https://www.rust-lang.org/tools/install)
 - [Docker](https://www.docker.com/)

# How to run
The `build_scripts` directory contains various utilities for configuring and running OmniPaxos clients and servers. Also contains examples of TOML file configuration.
 - `run-local-client.sh` runs two clients in separate local processes. Configuration such as which server to connect to defined in TOML files.
 - `run-local-cluster.sh` runs a 3 server cluster in separate local processes.
 - `docker-compose.yml` docker compose for a 3 server cluster.
 - See `benchmarks/README.md` for benchmarking scripts

# Linearizability Testing

The test harness generates a Knossos-compatible `history.edn` file that records all client
operations. This history can be verified for linearizability using the included checker.

## Running the Test Harness

```bash
# 1. Start the cluster (Docker Compose)
cd build_scripts && docker compose build && docker compose up -d

# 2. Run the test harness to generate history.edn
cargo run --release --bin test-harness -- \
  --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 \
  --num-clients 5 \
  --ops-per-client 100 \
  --output history.edn

# 3. (Optional) Run with fault injection (nemesis)
cargo run --release --bin test-harness -- \
  --nemesis \
  --nemesis-crash \
  --nemesis-containers s1,s2,s3 \
  --output history.edn
```

## Verifying Linearizability

The `check-linearizability.sh` script runs [Knossos](https://github.com/jepsen-io/knossos)
on the generated history. It groups operations by key and checks each key independently
as a CAS register.

```bash
# Verify the history (auto-detects Clojure CLI or Docker)
./check-linearizability.sh history.edn
```

**Prerequisites** (one of the following):
- [Clojure CLI](https://clojure.org/guides/install_clojure) installed locally
- [Docker](https://docs.docker.com/get-docker/) installed (the script pulls `clojure:temurin-21-tools-deps` automatically)

The script exits with code 0 if the history is linearizable, or code 1 if a violation is
detected. In CI, the GitHub Actions workflow (`.github/workflows/linearizability.yml`) runs
this check automatically on every push to `main` and on pull requests.
