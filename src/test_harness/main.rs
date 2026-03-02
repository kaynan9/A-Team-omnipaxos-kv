mod client;
mod config;
mod generator;
mod history;
mod nemesis;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use clap::Parser;
use client::{OpResult, TestClient};
use config::TestConfig;
use generator::{Generator, Operation};
use history::{EventType, FunctionType, History, HistoryEvent};
use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("test harness starting");

    let cfg = TestConfig::parse();
    println!("servers:        {:?}", cfg.server_list());
    println!("num_clients:    {}", cfg.num_clients);
    println!("ops_per_client: {}", cfg.ops_per_client);
    println!("key_range:      {}", cfg.key_range);
    println!("read_ratio:     {}", cfg.read_ratio);
    println!("cas_ratio:      {}", cfg.cas_ratio);
    println!("output:         {}", cfg.output);
    println!("nemesis:        {}", cfg.nemesis);
    println!("nemesis_network:    {}", cfg.nemesis_network);
    println!("nemesis_containers: {}", cfg.nemesis_containers);
    println!("nemesis_crash:      {}", cfg.nemesis_crash);
    println!("nemesis_quorum_loss: {}", cfg.nemesis_quorum_loss);
    println!("convergence_check:  {}", cfg.convergence_check);

    let history = History::new();
    let servers = cfg.server_list();
    let num_servers = servers.len();
    let num_clients = cfg.num_clients;
    let cancel = Arc::new(AtomicBool::new(false));

    let mut handles = Vec::new();

    for i in 0..cfg.num_clients {
        let history = history.clone();
        let servers = servers.clone();
        let ops_per_client = cfg.ops_per_client;
        let key_range = cfg.key_range;
        let read_ratio = cfg.read_ratio;
        let cas_ratio = cfg.cas_ratio;
        let cancel = cancel.clone();

        let handle = tokio::spawn(async move {
            let mut gen = Generator::new(key_range, read_ratio, cas_ratio);
            let mut client = TestClient::new(servers, i % num_servers);
            let mut rng = StdRng::from_entropy();
            // Each client starts with its own process ID and steps by num_clients
            // on each indeterminate result, so IDs across clients never collide:
            //   client 0 → 0, 5, 10, ...  client 1 → 1, 6, 11, ...
            let mut process_id = i;

            for _ in 0..ops_per_client {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let op = gen.next_op();

                let (func, invoke_value, invoke_expected) = match &op {
                    Operation::Put { value, .. } => {
                        (FunctionType::Write, Some(value.clone()), None)
                    }
                    Operation::Get { .. } => (FunctionType::Read, None, None),
                    Operation::Cas { expected, value, .. } => {
                        (FunctionType::Cas, Some(value.clone()), expected.clone())
                    }
                };

                let key = match &op {
                    Operation::Put { key, .. } => key.clone(),
                    Operation::Get { key }     => key.clone(),
                    Operation::Cas { key, .. } => key.clone(),
                };

                history.record(HistoryEvent {
                    process: process_id,
                    event_type: EventType::Invoke,
                    function: func.clone(),
                    key: key.clone(),
                    value: invoke_value.clone(),
                    expected: invoke_expected.clone(),
                    error: None,
                });

                let result = client.send_op(&op).await;

                // Indeterminate result: the invoke is left without a completion event.
                // Knossos 0.3.10 does not treat :info as a completion, so we leave the
                // invoke dangling. Knossos's complete() converts dangling invokes to :info
                // at the end. We advance to a fresh process ID so the next op doesn't
                // collide with the still-open invoke.
                if matches!(result, OpResult::Indeterminate) {
                    process_id += num_clients;
                    let delay_ms = rng.gen_range(10u64..=50);
                    sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }

                let (event_type, result_value, error_msg) = match (&result, &func) {
                    // Read :ok — use the value the server returned.
                    (OpResult::Ok(v), FunctionType::Read) => (EventType::Ok, v.clone(), None),
                    // Write / CAS :ok — echo back the value from the original op.
                    (OpResult::Ok(_), _)   => (EventType::Ok, invoke_value.clone(), None),
                    (OpResult::PreconditionFailed(msg), _) => (EventType::Fail, invoke_value.clone(), Some(msg.clone())),
                    (OpResult::SystemError(msg), _) => (EventType::Fail, invoke_value.clone(), Some(msg.clone())),
                    (OpResult::Indeterminate, _) => unreachable!(),
                };

                history.record(HistoryEvent {
                    process: process_id,
                    event_type,
                    function: func,
                    key: key.clone(),
                    value: result_value,
                    expected: invoke_expected,
                    error: error_msg,
                });

                // Update known_values on confirmed write success.
                match (&result, &op) {
                    (OpResult::Ok(_), Operation::Put { key, value }) => {
                        gen.known_values.insert(key.clone(), value.clone());
                    }
                    (OpResult::Ok(_), Operation::Cas { key, value, .. }) => {
                        gen.known_values.insert(key.clone(), value.clone());
                    }
                    _ => {}
                }

                let delay_ms = rng.gen_range(10u64..=50);
                sleep(Duration::from_millis(delay_ms)).await;
            }
        });

        handles.push(handle);
    }

    if cfg.nemesis {
        let network = cfg.nemesis_network.clone();
        let containers = cfg.nemesis_containers_list();
        let crash = cfg.nemesis_crash;
        let quorum_loss = cfg.nemesis_quorum_loss;
        let nemesis_handle = tokio::spawn(nemesis::run_nemesis_schedule(network, containers, crash, quorum_loss));
        let _ = nemesis_handle.await;

        println!("\nNEMESIS: faults finished. Waiting 15s for quiescent period...");
        sleep(Duration::from_secs(15)).await;

        cancel.store(true, Ordering::Relaxed);
        for handle in handles {
            let _ = handle.await;
        }
    } else {
        for handle in handles {
            let _ = handle.await;
        }
    }

    history
        .write_edn(&cfg.output)
        .expect("failed to write history");

    let total = history.len();
    let (invokes, oks, fails, _) = history.type_counts();
    let (precondition_fails, system_errors) = history.error_counts();
    // Indeterminate ops leave a dangling :invoke with no paired result.
    // Knossos converts these to :info internally during linearizability checking.
    let indeterminate = invokes - oks - fails;
    println!("\n--- summary ---");
    println!("total events:  {}", total);
    println!("  :invoke      {}", invokes);
    println!("  :ok          {}", oks);
    println!("  :fail        {} (CAS Precondition: {}, System Error: {})", fails, precondition_fails, system_errors);
    println!("  :info        {} (indeterminate, resolved by Knossos)", indeterminate);
    println!("history written to: {}", cfg.output);

    // --- Convergence check (post-fault liveness verification) ---
    if cfg.convergence_check {
        println!("\n--- convergence check ---");
        let mut conv_client = TestClient::new(servers.clone(), 0);
        let mut passed = 0usize;
        let mut failed_keys: Vec<String> = Vec::new();

        for i in 0..cfg.key_range {
            let key = format!("k{i}");
            let op = Operation::Get { key: key.clone() };
            let mut success = false;

            for attempt in 0..3 {
                match conv_client.send_op(&op).await {
                    OpResult::Ok(_) | OpResult::PreconditionFailed(_) | OpResult::SystemError(_) => {
                        success = true;
                        break;
                    }
                    OpResult::Indeterminate => {
                        if attempt < 2 {
                            println!("Convergence: key {key} — attempt {attempt} indeterminate, retrying...");
                        }
                    }
                }
            }

            if success {
                passed += 1;
            } else {
                failed_keys.push(key);
            }
        }

        if failed_keys.is_empty() {
            println!("Convergence check: PASSED — all {} keys readable", passed);
        } else {
            println!(
                "Convergence check: FAILED — {} keys unreachable: {:?}",
                failed_keys.len(),
                failed_keys
            );
        }
    }
}
