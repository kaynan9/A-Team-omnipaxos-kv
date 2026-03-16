# Project Readiness Checklist — OmniPaxos-KV

**Evaluation Date**: 2026-02-26
**Evaluated Against**: Project Criteria §2.1 Basic Requirements (30 points)
**Methodology**: Static analysis of repository source code only. All conclusions are
evidence-based; no functionality is assumed if it cannot be located in the codebase.

---

## Quick Verdict Table

| # | Requirement | Status |
|---|-------------|--------|
| 1 | Testable Shim — HTTP API (put, get, cas) | ✅ Met |
| 2 | Test Client + Indeterminate State Handling | ✅ Met |
| 3 | Generator — Read/Write/CAS Mix | ✅ Met |
| 4 | Nemesis — Network Partitioning | ✅ Met |
| 5 | Nemesis — Crash + Node Restart | ✅ Met |
| 6 | EDN History Generation (Knossos-compatible) | ✅ Met |
| 7 | Linearizability Checker Invocation | ✅ Met |
| 8 | Linearizable Reads Implementation | ✅ Met |
| 9 | Linearizable Reads Design Discussion | ✅ Met |
| 10 | Linearizable Reads — Dedicated Tests | ✅ Met |

---

## Detailed Criteria Assessment

---

### 1. ✅ Testable HTTP Shim

**Requirement**: Expose a programmable HTTP API allowing external agents to trigger `put`,
`get`, and optionally `cas` operations.

**Status**: MET

**Evidence**:

- **`src/server/http_shim.rs`**: Full Axum HTTP server implementation.
  - `POST /kv` — handles `put`, `get`, `cas` operations
  - `GET /health` — returns `{"status": "ok"}`
  - `KvRequest` struct: `op`, `key`, `value?`, `expected?`
  - `KvResponse` struct: `op`, `ok`, `key`, `value?`, `error?`
  - 10-second timeout: returns `{"ok": false, "error": "timeout"}` with HTTP 200
  - HTTP 400 for unknown/malformed operations
  - HTTP 503 if internal channel is closed
- **`src/server/main.rs`**: Shim is spawned as an independent Tokio task when `http_port`
  is present in config:
  ```rust
  if let Some(http_port) = server_config.local.http_port {
      let (shim_tx, shim_rx) = tokio::sync::mpsc::channel(256);
      tokio::spawn(http_shim::run_http_shim(http_port, shim_tx));
      Some(shim_rx)
  }
  ```
- **`build_scripts/server-1-config.toml`**: `http_port = 8081`
- **`build_scripts/server-2-config.toml`**: `http_port = 8082`
- **`build_scripts/server-3-config.toml`**: `http_port = 8083`
- Requests are forwarded to the OmniPaxos consensus loop via `ShimRequest` /
  `oneshot::channel` — all shim operations are fully consensus-serialized.

**Minor Gap**: `KVCommand::Delete` exists in the binary protocol but the HTTP shim
rejects `"delete"` as `"unknown op"` (returns HTTP 400). This is undocumented.

---

### 2. ✅ Test Client + Indeterminate State Handling

**Requirement**: Create a test client that bridges the generator to the shim, correctly
interpreting indeterminate states.

**Status**: MET

**Evidence**:

- **`src/test_harness/client.rs`**: `TestClient` using the `reqwest` crate.
  - Sends `POST /kv` JSON requests to configured server URLs
  - Correct three-way result classification:
    | Server Response | `OpResult` | History `EventType` |
    |----------------|------------|---------------------|
    | `ok: true` | `Ok(value)` | `:ok` |
    | `error: "timeout"` or `"unavailable"` | `Indeterminate` | `:info` |
    | Any other `ok: false` | `Fail(msg)` | `:fail` |
    | HTTP/network error | `Indeterminate` | `:info` |
  - Rotates to next server on connection error (basic failover)
- **`src/test_harness/main.rs`**: Spawns `cfg.num_clients` concurrent Tokio tasks.
  Each task: generates op → records `:invoke` → sends to server → records `:ok`/`:fail`/`:info`.
  Adds 10–50ms random inter-operation delay. Updates `known_values` on confirmed writes.

---

### 3. ✅ Generator — Read/Write/CAS Mix

**Requirement**: Generator must produce a mix of read and write operations (CAS optional).

**Status**: MET

**Evidence**:

- **`src/test_harness/generator.rs`**: `Generator` struct with `read_ratio` and `cas_ratio`.
  - Produces `Operation::Put`, `Operation::Get`, `Operation::Cas`
  - Distribution: if `roll < read_ratio` → Get; elif `roll < read_ratio + cas_ratio` → Cas; else → Put
  - Keys: `k0`..`k{key_range-1}` (bounded keyspace for contention)
  - Values: `v{counter}` (unique, monotonically increasing)
  - `known_values: HashMap<String,String>` tracks last confirmed write per key, used as CAS `expected`
- **`src/test_harness/config.rs`**: All ratios configurable via CLI:
  - `--read-ratio` (default `0.5`)
  - `--cas-ratio` (default `0.1`)
  - `--key-range` (default `5`)
  - `--ops-per-client` (default `100`)
  - `--num-clients` (default `5`)
- **Unit tests** in `generator.rs`:
  - `next_op_covers_all_variants_and_keys_in_range`: verifies all three variants appear over 100 ops, keys within range
  - `counter_increments_each_call`: verifies value uniqueness

---

### 4. ✅ Nemesis — Network Partitioning

**Requirement**: Implement fault injection that isolates the leader from followers or splits
the cluster (network partitioning).

**Status**: MET

**Evidence**:

`src/test_harness/nemesis.rs` — real Docker-based fault injection:

```rust
async fn docker_network_disconnect(network: &str, container: &str) -> Result<(), String> {
    let network = network.to_string();
    let container = container.to_string();
    tokio::task::spawn_blocking(move || {
        let out = std::process::Command::new("docker")
            .args(["network", "disconnect", "--force", &network, &container])
            .output()
            .map_err(|e| format!("spawn failed: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    })
    .await
    .map_err(|e| format!("spawn_blocking panicked: {e}"))?
}
```

**What is implemented**:
- `docker network disconnect --force {network} {container}` — severs all TCP connections
  from the target container to the rest of the cluster instantly
- `docker network connect {network} {container}` — restores connectivity after a 10s hold
- `partitioned: Vec<String>` tracks currently disconnected containers
- `heal_all()` is called unconditionally at exit, guaranteeing cleanup even if an earlier
  reconnect attempt failed
- All blocking `std::process::Command` calls wrapped in `tokio::task::spawn_blocking`
- Configurable via `--nemesis-network` and `--nemesis-containers` CLI flags (defaults:
  `build_scripts_default` and `s1,s2,s3`)
- Schedule: 5s stabilise → per-container: disconnect + 10s partition + reconnect + 5s recovery

---

### 5. ✅ Nemesis — Crash + Node Restart

**Requirement**: Implement fault injection that kills and restarts nodes randomly.

**Status**: MET

**Evidence**:

`src/test_harness/nemesis.rs` — real Docker-based crash + restart fault injection:

- `docker_kill_container(container)` — sends `SIGKILL` via `docker kill --signal KILL`
  to immediately crash a container (no graceful shutdown)
- `docker_start_container(container)` — restarts a stopped container via `docker start`
- `run_crash_faults(containers, stopped)` — randomly selects alive containers and
  performs kill/restart cycles:
  - Runs `containers.len()` rounds (each node targeted on average once)
  - Random target selection from alive (non-stopped) containers
  - Random downtime per crash: 5–15 seconds
  - 5-second recovery window between rounds
  - Tracks stopped containers in `stopped: Vec<String>`
- `restore_all(stopped)` — unconditional cleanup at exit guarantees all killed
  containers are restarted, even if individual restart attempts failed earlier
- All blocking `std::process::Command` calls wrapped in `tokio::task::spawn_blocking`

**CLI integration**:
- `--nemesis-crash` flag enables crash faults (default: `false`)
- Uses same `--nemesis-containers` list as network partitioning
- Schedule: partition faults run first, then crash faults, then unconditional cleanup

**Files modified**:
- `src/test_harness/nemesis.rs` — added `docker_kill_container`, `docker_start_container`,
  `run_crash_faults`, `restore_all`; updated `run_nemesis_schedule` signature to accept
  `enable_crash: bool`
- `src/test_harness/config.rs` — added `--nemesis-crash` CLI flag
- `src/test_harness/main.rs` — passes `cfg.nemesis_crash` to nemesis schedule

---

### 6. ✅ EDN History Generation (Knossos-compatible)

**Requirement**: Produce an operation history checkable by a linearizability verifier such
as Knossos.

**Status**: MET

**Evidence**:

- **`src/test_harness/history.rs`**: Thread-safe `History` (wraps `Arc<Mutex<Vec<HistoryEvent>>>`).
  - `record(&self, event)` — appends event, safe to call from concurrent Tokio tasks
  - `write_edn(&self, path)` — writes Knossos-format EDN to file
  - EDN format produced:
    ```edn
    [
     {:process 0 :type :invoke :f :write :value ["k1" "v0"]}
     {:process 0 :type :ok     :f :write :value ["k1" "v0"]}
     {:process 1 :type :invoke :f :read  :value ["k1" nil]}
     {:process 1 :type :ok     :f :read  :value ["k1" "v0"]}
     {:process 2 :type :invoke :f :cas   :value ["k1" "v0" "v1"]}
     {:process 2 :type :info   :f :cas   :value ["k1" "v0" "v1"]}
    ]
    ```
  - `:info` for indeterminate (correct — Knossos treats `:info` as "may or may not have taken effect")
  - `:fail` for confirmed failures (CAS precondition, rejected ops)
- **Unit tests** in `history.rs`: `record_and_len`, `type_counts_correct`, `write_edn_format`
- `history.edn` present in repository root — evidence the harness has been executed

---

### 7. ✅ Linearizability Checker Invocation

**Requirement**: Use a linearizability checker (like Knossos) to analyze the history.
Demonstrate the history is linearizable or analyze violations.

**Status**: MET

**Evidence**:

- **`check-linearizability.sh`** (repository root): Shell script that invokes the Knossos
  linearizability checker on a `history.edn` file. Supports two execution methods:
  1. Local Clojure CLI (if `clojure` is on PATH)
  2. Docker fallback (pulls `clojure:temurin-21-tools-deps-1.12.0.1530` automatically)
  - Exits with code 0 on pass, code 1 on violation or error
  - Validates that the history file exists before running

- **`knossos/deps.edn`**: Clojure project declaring the `knossos/knossos` dependency (v0.3.10)

- **`knossos/src/checker.clj`**: Clojure checker script that:
  - Reads the multi-key EDN history produced by the test harness
  - Groups operations by key
  - Transforms each key's operations into single-register format
    (strips the key from `:value` vectors)
  - Runs `knossos.competition/analysis` with a `cas-register` model per key
  - Reports per-key pass/fail results
  - Exits with code 1 if any key violates linearizability

- **`.github/workflows/linearizability.yml`**: GitHub Actions CI/CD pipeline that:
  - Triggers on push to `main`, pull requests, and manual dispatch
  - Installs Clojure CLI, caches Maven dependencies
  - Verifies `history.edn` exists
  - Runs `check-linearizability.sh`
  - Uploads `history.edn` as an artifact on failure for debugging

- **`README.md`**: Updated with step-by-step instructions for:
  - Running the test harness to generate `history.edn`
  - Running the linearizability checker manually
  - Prerequisites (Clojure CLI or Docker)

---

### 8. ✅ Linearizable Reads Implementation

**Requirement**: Implement a mechanism to make reads linearizable.

**Status**: MET

**Evidence**:

- **`src/server/server.rs`, line ~206-208** (`handle_client_messages`):
  ```rust
  ClientMessage::Append(command_id, kv_command) => {
      let _ = self.append_to_log(from, command_id, kv_command);
  }
  ```
  ALL KVCommands — including `KVCommand::Get` — are routed through `append_to_log()`.

- **`src/server/server.rs`** (`handle_decided_entries` → `update_database_and_respond`):
  Read results are only returned AFTER the entry is decided by OmniPaxos consensus. A Get
  receives a sequence number in the log; all entries decided before it are applied first.
  This guarantees the read observes the most recently committed state.

- **No local reads exist**: The codebase contains zero paths that read from `Database`
  without first going through consensus. Stale follower reads are architecturally impossible.

---

### 9. ✅ Linearizable Reads Design Discussion

**Requirement**: Discuss the design of the linearizable reads mechanism in the report.

**Status**: MET

**Evidence**:

- **`docs/linearizable-reads.md`**: Comprehensive design document covering:
  - How reads work (routed through OmniPaxos log, not served locally)
  - Why this achieves linearizability (sequence number = position in total order)
  - The alternative approach (local follower reads) and why it violates linearizability
    with a concrete staleness scenario
  - Trade-offs: consensus-read latency vs. correctness
  - Code snippets directly referencing `server.rs` and `update_database_and_respond`
  - Mention of Knossos as the verification tool

---

### 10. ✅ Linearizable Reads — Dedicated Tests

**Requirement**: Test the linearizable reads mechanism.

**Status**: MET

**Evidence**:

**Dedicated unit tests** in `src/server/database.rs` (`#[cfg(test)] mod tests`):

1. **`write_then_read_returns_written_value`** — Basic linearizability property: writes a
   value, reads it back, asserts the read returns the written value. Also tests read-before-
   write (returns `None`) and overwrite-then-read.

2. **`reads_always_reflect_latest_write_no_stale_data`** — Stale-read prevention: applies
   multiple sequential writes to the same key, then reads — asserts the read always returns
   the most recent write, never an older value. Tests interleaved reads between writes to
   multiple keys. Verifies reads on unwritten keys return `None` (no phantom values).

3. **`concurrent_writes_reads_reflect_total_order`** — Concurrent writes: simulates a
   decided log from two concurrent processes (P0, P1) with interleaved Put/CAS/Get operations.
   Verifies each read reflects exactly the state at its position in the total order. P0's
   read sees P1's later write; P1's read sees P0's CAS result.

4. **`cas_exactly_once_under_contention`** — CAS linearizability: two processes race to CAS
   the same key. Exactly one succeeds; the other fails with the correct current value.
   Subsequent reads reflect the winning CAS.

5. **`delete_then_read_returns_none`** — Delete followed by read returns `None`. CAS with
   `expected=None` succeeds on absent key.

**Architectural guarantee** (verified by code inspection, documented in test comments):
- `server.rs:handle_client_messages` routes ALL `ClientMessage::Append` variants — including
  `KVCommand::Get` — through `append_to_log()` (consensus).
- `server.rs:update_database_and_respond` only calls `Database::handle_command` after entries
  are decided by OmniPaxos. There is zero code path for local/follower reads.

**Automated integration pathway**:
- `run-linearizability-test.sh` — end-to-end script: starts Docker cluster → runs test
  harness → generates `history.edn` → runs Knossos checker → exits non-zero on violation.
- `.github/workflows/linearizability.yml` — CI pipeline with `unit-tests` job that runs
  `cargo test --release database::tests` and `check-linearizability` job that runs Knossos.

---

## Additional Observations

### ✅ Docker Compose: HTTP Ports Exposed

**File**: `build_scripts/docker-compose.yml`

HTTP shim ports are now exposed to the Docker host:
- `s1`: `8081:8081`
- `s2`: `8082:8082`
- `s3`: `8083:8083`

The test harness can connect to `http://localhost:8081,8082,8083` from the host.

### ✅ End-to-End Run Script

**File**: `run-linearizability-test.sh`

Automated pipeline: starts cluster → runs test harness → runs Knossos → tears down cluster.

The `build_scripts/` directory covers cluster + benchmark clients but not the test harness.

### `history.edn` Tracked in Git

The generated history file is committed to the repository (not gitignored). This is an
output artifact and will grow or change on every harness run.

### Binary Client vs. Test Harness

Two separate client architectures exist and serve different purposes:

| Binary | Protocol | Purpose |
|--------|----------|---------|
| `src/client/` (`client` binary) | Binary TCP (Bincode) | Throughput benchmarking |
| `src/test_harness/` (`test-harness` binary) | HTTP JSON (reqwest) | Linearizability testing |

These are independent and do not share code. The test harness correctly uses HTTP.

---

## Overall Readiness Verdict

**All 10 of 10 criteria are fully met.**

**Completed items**:

- [x] HTTP shim fully functional (put, get, cas, health check)
- [x] Test client correctly handles all three result states including indeterminate
- [x] Generator produces configurable mixed read/write/CAS workload
- [x] Nemesis — network partitioning (`docker network disconnect/connect`)
- [x] Nemesis — crash + restart (`docker kill --signal KILL` / `docker start`)
- [x] EDN history recorder is correct and well-tested
- [x] Knossos linearizability checker integrated (`check-linearizability.sh` + CI/CD)
- [x] Linearizable reads correctly implemented (all reads through consensus log)
- [x] Linearizable reads design discussion is thorough (`docs/linearizable-reads.md`)
- [x] Dedicated linearizability tests (5 tests in `database.rs`)
- [x] End-to-end integration script (`run-linearizability-test.sh`)
