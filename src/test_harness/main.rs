mod client;
mod config;
mod generator;
mod history;
mod nemesis;

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

    let history = History::new();
    let servers = cfg.server_list();
    let num_servers = servers.len();

    let mut handles = Vec::new();

    for i in 0..cfg.num_clients {
        let history = history.clone();
        let servers = servers.clone();
        let ops_per_client = cfg.ops_per_client;
        let key_range = cfg.key_range;
        let read_ratio = cfg.read_ratio;
        let cas_ratio = cfg.cas_ratio;

        let handle = tokio::spawn(async move {
            let mut gen = Generator::new(key_range, read_ratio, cas_ratio);
            let mut client = TestClient::new(servers, i % num_servers);
            let mut rng = StdRng::from_entropy();

            for _ in 0..ops_per_client {
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
                    process: i,
                    event_type: EventType::Invoke,
                    function: func.clone(),
                    key: key.clone(),
                    value: invoke_value.clone(),
                    expected: invoke_expected.clone(),
                });

                let result = client.send_op(&op).await;

                let (event_type, result_value) = match (&result, &func) {
                    // Read :ok — use the value the server returned.
                    (OpResult::Ok(v), FunctionType::Read) => (EventType::Ok, v.clone()),
                    // Write / CAS :ok — echo back the value from the original op.
                    (OpResult::Ok(_), _)                  => (EventType::Ok, invoke_value.clone()),
                    (OpResult::Fail(_), _)                => (EventType::Fail, invoke_value.clone()),
                    (OpResult::Indeterminate, _)          => (EventType::Info, invoke_value.clone()),
                };

                history.record(HistoryEvent {
                    process: i,
                    event_type,
                    function: func,
                    key: key.clone(),
                    value: result_value,
                    expected: invoke_expected,
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
        let nemesis_handle = tokio::spawn(nemesis::run_nemesis_schedule(network, containers, crash));
        for handle in handles {
            let _ = handle.await;
        }
        let _ = nemesis_handle.await;
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
    println!("\n--- summary ---");
    println!("total events:  {}", total);
    println!("  :invoke      {}", invokes);
    println!("  :ok          {}", oks);
    println!("  :fail        {}", fails);
    println!("  :info        {}", infos);
    println!("history written to: {}", cfg.output);
}
