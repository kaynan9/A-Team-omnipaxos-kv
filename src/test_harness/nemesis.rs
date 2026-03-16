use futures::future::join_all;
use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::time::{Duration, sleep};



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

async fn docker_network_connect(network: &str, container: &str) -> Result<(), String> {
    let network = network.to_string();
    let container = container.to_string();
    tokio::task::spawn_blocking(move || {
        let out = std::process::Command::new("docker")
            .args(["network", "connect", &network, &container])
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


async fn docker_kill_container(container: &str) -> Result<(), String> {
    let container = container.to_string();
    tokio::task::spawn_blocking(move || {
        let out = std::process::Command::new("docker")
            .args(["kill", "--signal", "KILL", &container])
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

async fn docker_start_container(container: &str) -> Result<(), String> {
    let container = container.to_string();
    tokio::task::spawn_blocking(move || {
        let out = std::process::Command::new("docker")
            .args(["start", &container])
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

async fn docker_network_disconnect_group(network: &str, containers: &[String]) -> Vec<String> {
    let futures: Vec<_> = containers
        .iter()
        .map(|c| {
            let net = network.to_string();
            let container = c.clone();
            async move {
                let result = docker_network_disconnect(&net, &container).await;
                (container, result)
            }
        })
        .collect();

    let results = join_all(futures).await;
    let mut disconnected = Vec::new();
    for (container, result) in results {
        match result {
            Ok(()) => {
                println!("NEMESIS [partition]: {container} disconnected");
                disconnected.push(container);
            }
            Err(e) => {
                eprintln!("NEMESIS [partition]: ERROR disconnecting {container}: {e}");
            }
        }
    }
    disconnected
}

async fn docker_network_connect_group(network: &str, containers: &[String]) -> Vec<String> {
    let futures: Vec<_> = containers
        .iter()
        .map(|c| {
            let net = network.to_string();
            let container = c.clone();
            async move {
                let result = docker_network_connect(&net, &container).await;
                (container, result)
            }
        })
        .collect();

    let results = join_all(futures).await;
    let mut reconnected = Vec::new();
    for (container, result) in results {
        match result {
            Ok(()) => {
                println!("NEMESIS [partition]: {container} reconnected");
                reconnected.push(container);
            }
            Err(e) => {
                eprintln!("NEMESIS [partition]: ERROR reconnecting {container}: {e}");
            }
        }
    }
    reconnected
}


async fn heal_all(network: &str, partitioned: &[String]) {
    for container in partitioned {
        println!("NEMESIS: final heal — reconnecting {container}");
        match docker_network_connect(network, container).await {
            Ok(()) => println!("NEMESIS: {container} reconnected"),
            Err(e) => eprintln!("NEMESIS: ERROR in final heal for {container}: {e}"),
        }
    }
}

async fn restore_all(stopped: &[String]) {
    for container in stopped {
        println!("NEMESIS: final restore — restarting {container}");
        match docker_start_container(container).await {
            Ok(()) => println!("NEMESIS: {container} restarted"),
            Err(e) => eprintln!("NEMESIS: ERROR in final restore for {container}: {e}"),
        }
    }
}


async fn run_partition_faults(network: String, containers: Vec<String>) -> Vec<String> {
    if containers.is_empty() {
        return Vec::new();
    }

    let mut rng = StdRng::from_entropy();
    let mut partitioned: Vec<String> = Vec::new();

    println!("NEMESIS [partition]: waiting 2s for cluster to stabilize...");
    sleep(Duration::from_secs(2)).await;

    let rounds = containers.len();
    for round in 0..rounds {
        let set_size = rng.gen_range(1..containers.len().max(2));
        let mut candidates = containers.clone();
        let mut partition_set: Vec<String> = Vec::with_capacity(set_size);

        for _ in 0..set_size {
            if candidates.is_empty() {
                break;
            }
            let idx = rng.gen_range(0..candidates.len());
            partition_set.push(candidates.remove(idx));
        }

        println!(
            "NEMESIS [partition]: round {round} — disconnecting group: {:?}",
            partition_set
        );

        let disconnected = docker_network_disconnect_group(&network, &partition_set).await;

        if let Some(leader) = containers.first() {
            if disconnected.contains(leader) {
                println!(
                    "NEMESIS: NOTE — {} is the initial leader (per cluster config). \
                     This partition tests leader isolation.",
                    leader
                );
            }
        }

        partitioned.extend(disconnected);

        println!("NEMESIS [partition]: holding partition for 10s...");
        sleep(Duration::from_secs(10)).await;

        println!(
            "NEMESIS [partition]: healing group: {:?}",
            partition_set
        );
        let reconnected = docker_network_connect_group(&network, &partition_set).await;
        for c in &reconnected {
            partitioned.retain(|p| p != c);
        }

        println!("NEMESIS [partition]: recovery window 5s...");
        sleep(Duration::from_secs(5)).await;
    }

    partitioned
}

async fn run_crash_faults(containers: Vec<String>) -> Vec<String> {
    if containers.is_empty() {
        return Vec::new();
    }

    let mut rng = StdRng::from_entropy();
    let mut stopped: Vec<String> = Vec::new();

    println!("NEMESIS [crash]: waiting 2s for cluster to stabilize...");
    sleep(Duration::from_secs(2)).await;

    let rounds = containers.len();
    for round in 0..rounds {
        let alive: Vec<&String> = containers
            .iter()
            .filter(|c| !stopped.contains(c))
            .collect();

        if alive.is_empty() {
            eprintln!("NEMESIS [crash]: all containers stopped — skipping round {round}");
            continue;
        }

        let target = alive[rng.gen_range(0..alive.len())].clone();

        println!("NEMESIS [crash]: killing {target} (round {round})");
        match docker_kill_container(&target).await {
            Ok(()) => {
                stopped.push(target.clone());
                println!("NEMESIS [crash]: {target} killed");
            }
            Err(e) => {
                eprintln!("NEMESIS [crash]: ERROR killing {target}: {e}");
                continue;
            }
        }

        let down_secs = rng.gen_range(5u64..=15);
        println!("NEMESIS [crash]: {target} down for {down_secs}s...");
        sleep(Duration::from_secs(down_secs)).await;

        println!("NEMESIS [crash]: restarting {target}");
        match docker_start_container(&target).await {
            Ok(()) => {
                stopped.retain(|c| c != &target);
                println!("NEMESIS [crash]: {target} restarted");
            }
            Err(e) => {
                eprintln!("NEMESIS [crash]: ERROR restarting {target}: {e}");
            }
        }

        println!("NEMESIS [crash]: recovery window 5s...");
        sleep(Duration::from_secs(5)).await;
    }

    stopped
}


async fn run_quorum_loss_faults(containers: Vec<String>) -> Vec<String> {
    if containers.is_empty() {
        return Vec::new();
    }

    let mut rng = StdRng::from_entropy();
    let mut stopped: Vec<String> = Vec::new();
    let majority = containers.len() / 2 + 1;

    println!("NEMESIS [quorum-loss]: waiting 2s for cluster to stabilize...");
    sleep(Duration::from_secs(2)).await;

    let rounds = containers.len();
    for round in 0..rounds {
        let alive: Vec<String> = containers
            .iter()
            .filter(|c| !stopped.contains(c))
            .cloned()
            .collect();

        if alive.len() < majority {
            eprintln!(
                "NEMESIS [quorum-loss]: only {} alive, need {} for quorum loss — skipping round {round}",
                alive.len(),
                majority
            );
            continue;
        }

        let mut candidates = alive;
        let mut kill_set: Vec<String> = Vec::with_capacity(majority);
        for _ in 0..majority {
            let idx = rng.gen_range(0..candidates.len());
            kill_set.push(candidates.remove(idx));
        }

        println!(
            "NEMESIS [quorum-loss]: round {round} — killing majority: {:?}",
            kill_set
        );

        let kill_futures: Vec<_> = kill_set
            .iter()
            .map(|c| {
                let container = c.clone();
                async move {
                    let result = docker_kill_container(&container).await;
                    (container, result)
                }
            })
            .collect();

        let kill_results = join_all(kill_futures).await;
        let mut killed_this_round: Vec<String> = Vec::new();
        for (container, result) in kill_results {
            match result {
                Ok(()) => {
                    println!("NEMESIS [quorum-loss]: {container} killed");
                    stopped.push(container.clone());
                    killed_this_round.push(container);
                }
                Err(e) => {
                    eprintln!("NEMESIS [quorum-loss]: ERROR killing {container}: {e}");
                }
            }
        }

        let down_secs = rng.gen_range(5u64..=15);
        println!("NEMESIS [quorum-loss]: cluster has no quorum — holding for {down_secs}s...");
        sleep(Duration::from_secs(down_secs)).await;

        let start_futures: Vec<_> = killed_this_round
            .iter()
            .map(|c| {
                let container = c.clone();
                async move {
                    let result = docker_start_container(&container).await;
                    (container, result)
                }
            })
            .collect();

        let start_results = join_all(start_futures).await;
        for (container, result) in start_results {
            match result {
                Ok(()) => {
                    println!("NEMESIS [quorum-loss]: {container} restarted");
                    stopped.retain(|c| c != &container);
                }
                Err(e) => {
                    eprintln!("NEMESIS [quorum-loss]: ERROR restarting {container}: {e}");
                }
            }
        }

        println!("NEMESIS [quorum-loss]: recovery window 5s...");
        sleep(Duration::from_secs(5)).await;
    }

    stopped
}



pub async fn run_nemesis_schedule(
    network: String,
    containers: Vec<String>,
    enable_crash: bool,
    enable_quorum_loss: bool,
) {
    let mut all_partitioned: Vec<String> = Vec::new();
    let mut all_stopped: Vec<String> = Vec::new();

    let mut fault_handles: Vec<tokio::task::JoinHandle<(Vec<String>, Vec<String>)>> = Vec::new();

    {
        let net = network.clone();
        let ctrs = containers.clone();
        fault_handles.push(tokio::spawn(async move {
            let partitioned = run_partition_faults(net, ctrs).await;
            (partitioned, Vec::new())
        }));
    }

    if enable_crash {
        let ctrs = containers.clone();
        fault_handles.push(tokio::spawn(async move {
            let stopped = run_crash_faults(ctrs).await;
            (Vec::new(), stopped)
        }));
    }

    if enable_quorum_loss {
        let ctrs = containers.clone();
        fault_handles.push(tokio::spawn(async move {
            let stopped = run_quorum_loss_faults(ctrs).await;
            (Vec::new(), stopped)
        }));
    }

    for handle in fault_handles {
        match handle.await {
            Ok((partitioned, stopped)) => {
                all_partitioned.extend(partitioned);
                all_stopped.extend(stopped);
            }
            Err(e) => {
                eprintln!("NEMESIS: fault task panicked: {e}");
            }
        }
    }

    heal_all(&network, &all_partitioned).await;
    restore_all(&all_stopped).await;
    println!("NEMESIS: done — all containers restored");
}
