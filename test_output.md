PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> docker info 2>$null | Out-Null; if ($LASTEXITCODE -eq 0) { "OK: Docker" } else { "FAIL: Start Docker" }
OK: Docker
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> cargo --version; if ($LASTEXITCODE -eq 0) { "OK: Rust" } else { "FAIL: Install Rust" }
cargo 1.85.0 (d73d2caf9 2024-12-31)
OK: Rust
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> if (Test-Path "Cargo.toml") { "OK: Root" } else { "FAIL: cd to project root" }
OK: Root
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> cargo build --release --bin server --bin test-harness
    Finished `release` profile [optimized] target(s) in 0.41s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> docker compose build
[+] Building 0.0s (0/0)  docker:default
[+] Building 8.2s (32/32) FINISHED                                                                                         docker:default
 => [c1 internal] load build definition from client.dockerfile                                                                       0.0s
 => => transferring dockerfile: 828B                                                                                                 0.0s
 => [c1 internal] load .dockerignore                                                                                                 0.0s
 => => transferring context: 123B                                                                                                    0.0s
 => [s1 internal] load .dockerignore                                                                                                 0.0s
 => => transferring context: 123B                                                                                                    0.0s
 => [s1 internal] load build definition from server.dockerfile                                                                       0.0s
 => => transferring dockerfile: 828B                                                                                                 0.0s
 => [s1 internal] load metadata for docker.io/library/debian:bookworm-slim                                                           1.4s
 => [s1 internal] load metadata for docker.io/library/rust:1.85                                                                      1.4s
 => [c1 auth] library/rust:pull token for registry-1.docker.io                                                                       0.0s
 => [c1 auth] library/debian:pull token for registry-1.docker.io                                                                     0.0s
 => [c1 chef 1/4] FROM docker.io/library/rust:1.85@sha256:e51d0265072d2d9d5d320f6a44dde6b9ef13653b035098febd68cce8fa7c0bc4           0.0s
 => [c1 runtime 1/3] FROM docker.io/library/debian:bookworm-slim@sha256:74d56e3931e0d5a1dd51f8c8a2466d21de84a271cd3b5a733b803aa91ab  0.0s
 => [s1 internal] load build context                                                                                                 0.0s
 => => transferring context: 15.77kB                                                                                                 0.0s
 => [c1 internal] load build context                                                                                                 0.0s
 => => transferring context: 15.77kB                                                                                                 0.0s
 => CACHED [c1 chef 2/4] RUN set -eux                                                                                                0.0s
 => CACHED [c1 chef 3/4] RUN cargo install cargo-chef                                                                                0.0s
 => CACHED [c1 chef 4/4] WORKDIR /app                                                                                                0.0s
 => [s1 planner 1/2] COPY . .                                                                                                        0.0s
 => [s1 planner 2/2] RUN cargo chef prepare --recipe-path recipe.json                                                                0.3s
 => CACHED [c1 planner 1/2] COPY . .                                                                                                 0.0s
 => CACHED [c1 planner 2/2] RUN cargo chef prepare --recipe-path recipe.json                                                         0.0s
 => CACHED [c1 builder 1/4] COPY --from=planner /app/recipe.json recipe.json                                                         0.0s
 => CACHED [c1 builder 2/4] RUN cargo chef cook --release --recipe-path recipe.json                                                  0.0s
 => [c1 builder 3/4] COPY . .                                                                                                        0.0s
 => CACHED [s1 builder 1/4] COPY --from=planner /app/recipe.json recipe.json                                                         0.0s
 => CACHED [s1 builder 2/4] RUN cargo chef cook --release --recipe-path recipe.json                                                  0.0s
 => CACHED [s1 builder 3/4] COPY . .                                                                                                 0.0s
 => [s1 builder 4/4] RUN cargo build --release --bin server                                                                          6.2s
 => [c1 builder 4/4] RUN cargo build --release --bin client                                                                          3.8s
 => CACHED [s1 runtime 2/3] WORKDIR /app                                                                                             0.0s
 => CACHED [c1 runtime 3/3] COPY --from=builder /app/target/release/client /usr/local/bin                                            0.0s
 => [c1] exporting to image                                                                                                          0.0s
 => => exporting layers                                                                                                              0.0s
 => => writing image sha256:c470172abecac840186770c399e893dcf6afc46810243b6865ec7c6d678af235                                         0.0s
 => => naming to docker.io/library/omnipaxos-client                                                                                  0.0s
 => CACHED [s1 runtime 3/3] COPY --from=builder /app/target/release/server /usr/local/bin                                            0.0s
 => [s1] exporting to image                                                                                                          0.0s
 => => exporting layers                                                                                                              0.0s
 => => writing image sha256:868b379340ab0be4069446fe6d84fed3220fb19d13d7c6ca272ae5b4e96d48e0                                         0.0s
 => => naming to docker.io/library/omnipaxos-server                                                                                  0.0s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> Pop-Location
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> cargo test
   Compiling omnipaxos-kv v0.1.0 (C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.78s
     Running unittests src\lib.rs (target\debug\deps\omnipaxos_kv-9fb966c27f917776.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\client\main.rs (target\debug\deps\client-bddf0b5a1d1651dd.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\server\main.rs (target\debug\deps\server-54b73feaa2c9101e.exe)

running 5 tests
test database::tests::cas_exactly_once_under_contention ... ok
test database::tests::concurrent_writes_reads_reflect_total_order ... ok
test database::tests::delete_then_read_returns_none ... ok
test database::tests::write_then_read_returns_written_value ... ok
test database::tests::reads_always_reflect_latest_write_no_stale_data ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\test_harness\main.rs (target\debug\deps\test_harness-6c4d91d864cf91f8.exe)

running 5 tests
test history::tests::record_and_len ... ok
test history::tests::type_counts_correct ... ok
test generator::tests::counter_increments_each_call ... ok
test generator::tests::next_op_covers_all_variants_and_keys_in_range ... ok
test history::tests::write_edn_format ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests omnipaxos_kv

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> docker compose up -d
[+] Running 6/6
 ✔ Network omnipaxos-net  Created                                                                                                    0.0s
 ✔ Container s2           Started                                                                                                    0.1s
 ✔ Container s3           Started                                                                                                    0.1s
 ✔ Container c1           Started                                                                                                    0.1s
 ✔ Container c2           Started                                                                                                    0.1s
 ✔ Container s1           Started                                                                                                    0.1s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> Start-Sleep -Seconds 10
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> docker compose ps
NAME      IMAGE              COMMAND                  SERVICE   CREATED          STATUS          PORTS
c1        omnipaxos-client   "/usr/local/bin/clie…"   c1        11 seconds ago   Up 10 seconds   8000/tcp
c2        omnipaxos-client   "/usr/local/bin/clie…"   c2        11 seconds ago   Up 10 seconds   8000/tcp
s1        omnipaxos-server   "/usr/local/bin/serv…"   s1        11 seconds ago   Up 10 seconds   8000/tcp, 0.0.0.0:8081->8081/tcp
s2        omnipaxos-server   "/usr/local/bin/serv…"   s2        11 seconds ago   Up 11 seconds   8000/tcp, 0.0.0.0:8082->8082/tcp
s3        omnipaxos-server   "/usr/local/bin/serv…"   s3        11 seconds ago   Up 10 seconds   8000/tcp, 0.0.0.0:8083->8083/tcp
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> Pop-Location
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> cargo run --release --bin test-harness -- `
>>   --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 `
>>   --num-clients 5 --ops-per-client 100 --key-range 5 --read-ratio 0.4 --cas-ratio 0.2 `
>>   --output history-no-faults.edn
    Finished `release` profile [optimized] target(s) in 0.34s
     Running `target\release\test-harness.exe --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 --num-clients 5 --ops-per-client 100 --key-range 5 --read-ratio 0.4 --cas-ratio 0.2 --output history-no-faults.edn`
test harness starting
servers:        ["http://localhost:8081", "http://localhost:8082", "http://localhost:8083"]
num_clients:    5
ops_per_client: 100
key_range:      5
read_ratio:     0.4
cas_ratio:      0.2
output:         history-no-faults.edn
nemesis:        false
nemesis_network:    omnipaxos-net
nemesis_containers: s1,s2,s3
nemesis_crash:      false

--- summary ---
total events:  1000
  :invoke      500
  :ok          420
  :fail        80
  :info        0 (indeterminate, resolved by Knossos)
history written to: history-no-faults.edn
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location knossos
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\knossos> clojure -M -m checker ..\history-no-faults.edn
Knossos linearizability checker
Reading history from: ..\\history-no-faults.edn
Found 1000 operations across 5 keys: ("k0" "k1" "k2" "k3" "k4")
  Checking key "k0"... OK
  Checking key "k1"... Mar 01, 2026 8:01:10 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k2"... Mar 01, 2026 8:01:10 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k3"... OK
  Checking key "k4"... Mar 01, 2026 8:01:10 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK

PASS: All 5 keys are linearizable.
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\knossos> Pop-Location
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> docker compose logs > ..\logs-no-faults.txt 2>&1
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> docker compose down
[+] Running 6/6
 ✔ Container c2           Removed                                                                                                   10.8s
 ✔ Container s2           Removed                                                                                                   10.9s
 ✔ Container s1           Removed                                                                                                   11.1s
 ✔ Container s3           Removed                                                                                                   11.2s
 ✔ Container c1           Removed                                                                                                   11.4s
 ✔ Network omnipaxos-net  Removed                                                                                                    0.3s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv\build_scripts> Pop-Location
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts; docker compose up -d; Pop-Location
[+] Running 6/6
 ✔ Network omnipaxos-net  Created                                                                                                    0.0s
 ✔ Container c2           Started                                                                                                    0.1s
 ✔ Container s1           Started                                                                                                    0.1s
 ✔ Container s3           Started                                                                                                    0.1s
 ✔ Container c1           Started                                                                                                    0.1s
 ✔ Container s2           Started                                                                                                    0.1s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Start-Sleep -Seconds 10
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv>
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> cargo run --release --bin test-harness -- `
>>   --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 `
>>   --num-clients 5 --ops-per-client 1500 --key-range 5 --read-ratio 0.4 --cas-ratio 0.2 `
>>   --nemesis --output history-partitions.edn 2>&1 | Tee-Object -FilePath harness-partitions.log
cargo :     Finished `release` profile [optimized] target(s) in 0.32s
At line:1 char:1
+ cargo run --release --bin test-harness -- `
+ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: (    Finished `r...get(s) in 0.32s:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError

     Running `target\release\test-harness.exe --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 --num-clients
5 --ops-per-client 1500 --key-range 5 --read-ratio 0.4 --cas-ratio 0.2 --nemesis --output history-partitions.edn`
test harness starting
servers:        ["http://localhost:8081", "http://localhost:8082", "http://localhost:8083"]
num_clients:    5
ops_per_client: 1500
key_range:      5
read_ratio:     0.4
cas_ratio:      0.2
output:         history-partitions.edn
nemesis:        true
nemesis_network:    omnipaxos-net
nemesis_containers: s1,s2,s3
nemesis_crash:      false
NEMESIS: waiting 2s for cluster to stabilize...
NEMESIS: disconnecting s1 from network omnipaxos-net
NEMESIS: s1 is now partitioned
NEMESIS: holding partition for 10s...
NEMESIS: healing s1
NEMESIS: s1 reconnected
NEMESIS: recovery window 5s...
NEMESIS: disconnecting s2 from network omnipaxos-net
NEMESIS: s2 is now partitioned
NEMESIS: holding partition for 10s...
NEMESIS: healing s2
NEMESIS: s2 reconnected
NEMESIS: recovery window 5s...
NEMESIS: disconnecting s3 from network omnipaxos-net
NEMESIS: s3 is now partitioned
NEMESIS: holding partition for 10s...
NEMESIS: healing s3
NEMESIS: s3 reconnected
NEMESIS: recovery window 5s...
NEMESIS: done ÔÇö all containers restored

--- summary ---
total events:  10188
  :invoke      5102
  :ok          4201
  :fail        885
  :info        16 (indeterminate, resolved by Knossos)
history written to: history-partitions.edn
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location knossos; clojure -M -m checker ..\history-partitions.edn; Pop-Location

Knossos linearizability checker
Reading history from: ..\\history-partitions.edn
Found 10188 operations across 5 keys: ("k0" "k1" "k2" "k3" "k4")
  Checking key "k0"... Mar 01, 2026 8:02:57 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k1"... Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k2"... Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k3"... Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k4"... Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:02:58 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK

PASS: All 5 keys are linearizable.
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts; docker compose logs > ..\logs-partitions.txt 2>&1; docker compose down; Pop-Location
[+] Running 6/6
 ✔ Container s2           Removed                                                                                                   11.2s
 ✔ Container s3           Removed                                                                                                   11.0s
 ✔ Container s1           Removed                                                                                                   11.1s
 ✔ Container c2           Removed                                                                                                   11.4s
 ✔ Container c1           Removed                                                                                                   10.9s
 ✔ Network omnipaxos-net  Removed                                                                                                    0.2s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts; docker compose up -d; Pop-Location
[+] Running 6/6
 ✔ Network omnipaxos-net  Created                                                                                                    0.0s
 ✔ Container s3           Started                                                                                                    0.1s
 ✔ Container s1           Started                                                                                                    0.1s
 ✔ Container c2           Started                                                                                                    0.1s
 ✔ Container s2           Started                                                                                                    0.1s
 ✔ Container c1           Started                                                                                                    0.1s
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Start-Sleep -Seconds 10
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv>
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> cargo run --release --bin test-harness -- `
>>   --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 `
>>   --num-clients 5 --ops-per-client 1500 --key-range 5 --read-ratio 0.4 --cas-ratio 0.2 `
>>   --nemesis --nemesis-crash --output history-partitions-crashes.edn 2>&1 | Tee-Object -FilePath harness-partitions-crashes.log
cargo :     Finished `release` profile [optimized] target(s) in 0.32s
At line:1 char:1
+ cargo run --release --bin test-harness -- `
+ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: (    Finished `r...get(s) in 0.32s:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError

     Running `target\release\test-harness.exe --servers http://localhost:8081,http://localhost:8082,http://localhost:8083 --num-clients
5 --ops-per-client 1500 --key-range 5 --read-ratio 0.4 --cas-ratio 0.2 --nemesis --nemesis-crash --output history-partitions-crashes.edn`
test harness starting
servers:        ["http://localhost:8081", "http://localhost:8082", "http://localhost:8083"]
num_clients:    5
ops_per_client: 1500
key_range:      5
read_ratio:     0.4
cas_ratio:      0.2
output:         history-partitions-crashes.edn
nemesis:        true
nemesis_network:    omnipaxos-net
nemesis_containers: s1,s2,s3
nemesis_crash:      true
NEMESIS: waiting 2s for cluster to stabilize...
NEMESIS: disconnecting s1 from network omnipaxos-net
NEMESIS: s1 is now partitioned
NEMESIS: holding partition for 10s...
NEMESIS: healing s1
NEMESIS: s1 reconnected
NEMESIS: recovery window 5s...
NEMESIS: disconnecting s2 from network omnipaxos-net
NEMESIS: s2 is now partitioned
NEMESIS: holding partition for 10s...
NEMESIS: healing s2
NEMESIS: s2 reconnected
NEMESIS: recovery window 5s...
NEMESIS: disconnecting s3 from network omnipaxos-net
NEMESIS: s3 is now partitioned
NEMESIS: holding partition for 10s...
NEMESIS: healing s3
NEMESIS: s3 reconnected
NEMESIS: recovery window 5s...
NEMESIS [crash]: waiting 2s for cluster to stabilize...
NEMESIS [crash]: killing s1 (round 0)
NEMESIS [crash]: s1 killed
NEMESIS [crash]: s1 down for 9s...
NEMESIS [crash]: restarting s1
NEMESIS [crash]: s1 restarted
NEMESIS [crash]: recovery window 5s...
NEMESIS [crash]: killing s3 (round 1)
NEMESIS [crash]: s3 killed
NEMESIS [crash]: s3 down for 13s...
NEMESIS [crash]: restarting s3
NEMESIS [crash]: s3 restarted
NEMESIS [crash]: recovery window 5s...
NEMESIS [crash]: killing s2 (round 2)
NEMESIS [crash]: s2 killed
NEMESIS [crash]: s2 down for 8s...
NEMESIS [crash]: restarting s2
NEMESIS [crash]: s2 restarted
NEMESIS [crash]: recovery window 5s...
NEMESIS: done ÔÇö all containers restored

--- summary ---
total events:  14591
  :invoke      7316
  :ok          6144
  :fail        1131
  :info        41 (indeterminate, resolved by Knossos)
history written to: history-partitions-crashes.edn
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location knossos; clojure -M -m checker ..\history-partitions-crashes.edn; Pop-Location
Knossos linearizability checker
Reading history from: ..\\history-partitions-crashes.edn
Found 14591 operations across 5 keys: ("k0" "k1" "k2" "k3" "k4")
  Checking key "k0"... Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k1"... Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k2"... Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k3"... Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK
  Checking key "k4"... Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
Mar 01, 2026 8:05:32 PM clojure.tools.logging$eval142$fn__145 invoke
INFO: More than 1024 reachable models; not memoizing models for this search
OK

PASS: All 5 keys are linearizable.
PS C:\Users\Abdul Rahman\Desktop\A-Team-omnipaxos-kv> Push-Location build_scripts; docker compose logs > ..\logs-partitions-crashes.txt 2>&1; docker compose down; Pop-Location
[+] Running 6/6
 ✔ Container c2           Removed                                                                                                   11.0s
 ✔ Container s3           Removed                                                                                                   11.0s
 ✔ Container s1           Removed                                                                                                   11.2s
 ✔ Container c1           Removed                                                                                                   11.3s
 ✔ Container s2           Removed                                                                                                   11.6s
 ✔ Network omnipaxos-net  Removed   