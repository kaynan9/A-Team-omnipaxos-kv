mod client;
mod config;
mod generator;
mod history;
mod nemesis;

use std::{
    fmt::Write as FmtWrite,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
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

                let (event_type, result_value, error_msg) = match (&result, &func) {
                    // Read :ok — use the value the server returned.
                    (OpResult::Ok(v), FunctionType::Read) => (EventType::Ok, v.clone(), None),
                    // Write / CAS :ok — echo back the value from the original op.
                    (OpResult::Ok(_), _) => (EventType::Ok, invoke_value.clone(), None),
                    (OpResult::PreconditionFailed(msg), _) => (EventType::Fail, invoke_value.clone(), Some(msg.clone())),
                    (OpResult::SystemError(msg), _) => (EventType::Fail, invoke_value.clone(), Some(msg.clone())),
                    // Indeterminate: record a visible :info event so the EDN file has a
                    // paired completion for every invoke, then advance to a fresh process ID
                    // so the next op does not reuse this process slot.
                    (OpResult::Indeterminate, _) => (EventType::Info, invoke_value.clone(), None),
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

                // Advance to a fresh process ID after an indeterminate result so that
                // subsequent ops on this client do not reuse the same process slot.
                if matches!(result, OpResult::Indeterminate) {
                    process_id += num_clients;
                }

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
    let (invokes, oks, fails, infos) = history.type_counts();
    let (precondition_fails, system_errors) = history.error_counts();
    println!("\n--- summary ---");
    println!("total events:  {}", total);
    println!("  :invoke      {}", invokes);
    println!("  :ok          {}", oks);
    println!("  :fail        {} (CAS Precondition: {}, System Error: {})", fails, precondition_fails, system_errors);
    println!("  :info        {} (indeterminate)", infos);
    println!("history written to: {}", cfg.output);

    // --- Write summary file ---
    let summary_path = if cfg.output.ends_with(".edn") {
        format!("{}.summary.txt", &cfg.output[..cfg.output.len() - 4])
    } else {
        format!("{}.summary.txt", cfg.output)
    };

    let mut summary = String::new();
    writeln!(summary, "=== OmniPaxos KV Test Harness — Run Summary ===").unwrap();
    writeln!(summary).unwrap();
    writeln!(summary, "--- config ---").unwrap();
    writeln!(summary, "servers:             {}", cfg.servers).unwrap();
    writeln!(summary, "num_clients:         {}", cfg.num_clients).unwrap();
    writeln!(summary, "ops_per_client:      {}", cfg.ops_per_client).unwrap();
    writeln!(summary, "key_range:           {}", cfg.key_range).unwrap();
    writeln!(summary, "read_ratio:          {}", cfg.read_ratio).unwrap();
    writeln!(summary, "cas_ratio:           {}", cfg.cas_ratio).unwrap();
    writeln!(summary, "output:              {}", cfg.output).unwrap();
    writeln!(summary, "nemesis:             {}", cfg.nemesis).unwrap();
    if cfg.nemesis {
        writeln!(summary, "nemesis_network:     {}", cfg.nemesis_network).unwrap();
        writeln!(summary, "nemesis_containers:  {}", cfg.nemesis_containers).unwrap();
        writeln!(summary, "nemesis_crash:       {}", cfg.nemesis_crash).unwrap();
        writeln!(summary, "nemesis_quorum_loss: {}", cfg.nemesis_quorum_loss).unwrap();
    }
    writeln!(summary, "convergence_check:   {}", cfg.convergence_check).unwrap();
    writeln!(summary).unwrap();
    writeln!(summary, "--- event counts ---").unwrap();
    writeln!(summary, "total:   {}", total).unwrap();
    writeln!(summary, ":invoke  {}", invokes).unwrap();
    writeln!(summary, ":ok      {}", oks).unwrap();
    writeln!(summary, ":fail    {} (precondition-failed: {}, system-error: {})",
             fails, precondition_fails, system_errors).unwrap();
    writeln!(summary, ":info    {} (indeterminate)", infos).unwrap();
    if cfg.nemesis {
        writeln!(summary).unwrap();
        writeln!(summary, "--- nemesis ---").unwrap();
        writeln!(summary, "Fault injection was enabled during this run.").unwrap();
        writeln!(summary, "Network: {}  Containers: {}",
                 cfg.nemesis_network, cfg.nemesis_containers).unwrap();
        if cfg.nemesis_crash       { writeln!(summary, "  crash faults:       enabled").unwrap(); }
        if cfg.nemesis_quorum_loss { writeln!(summary, "  quorum-loss faults: enabled").unwrap(); }
        if let Some(leader) = cfg.nemesis_containers_list().into_iter().next() {
            writeln!(summary,
                "NOTE: {} is the initial leader (per cluster config). Leader isolation was tested.",
                leader
            ).unwrap();
        }
    }

    std::fs::write(&summary_path, &summary).expect("failed to write summary file");
    println!("summary written to:  {}", summary_path);

    // --- Convergence check (post-fault liveness verification) ---
    // For each server, write a sentinel key then read it back to confirm
    // that server can serve both writes and reads independently.
    if cfg.convergence_check {
        println!("\n--- convergence check ---");

        for (idx, server_url) in servers.iter().enumerate() {
            // Pin this client to a single server by giving it a one-element list.
            let mut conv_client = TestClient::new(vec![server_url.clone()], 0);

            let sentinel_key = format!("__conv_check_{idx}__");
            let sentinel_val = format!("probe_{idx}");

            // PUT
            let put_op = Operation::Put {
                key: sentinel_key.clone(),
                value: sentinel_val.clone(),
            };
            let put_ok = matches!(conv_client.send_op(&put_op).await, OpResult::Ok(_));

            // GET
            let get_op = Operation::Get { key: sentinel_key.clone() };
            let get_ok = matches!(conv_client.send_op(&get_op).await, OpResult::Ok(_));

            if put_ok && get_ok {
                println!("CONVERGENCE CHECK: {} — OK", server_url);
            } else {
                println!(
                    "CONVERGENCE CHECK: {} — FAIL (put={}, get={})",
                    server_url,
                    if put_ok { "ok" } else { "fail" },
                    if get_ok { "ok" } else { "fail" },
                );
            }
        }
    }
}
