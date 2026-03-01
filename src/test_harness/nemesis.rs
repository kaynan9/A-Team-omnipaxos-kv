use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::time::{Duration, sleep};

// ---------------------------------------------------------------------------
// Docker helpers — network partitioning
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Docker helpers — crash (stop/start)
// ---------------------------------------------------------------------------

/// Kills a Docker container with SIGKILL (immediate crash, no graceful shutdown).
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

/// Restarts a stopped Docker container.
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

// ---------------------------------------------------------------------------
// Cleanup functions
// ---------------------------------------------------------------------------

async fn heal_all(network: &str, partitioned: &mut Vec<String>) {
    for container in partitioned.drain(..) {
        println!("NEMESIS: final heal — reconnecting {container}");
        match docker_network_connect(network, &container).await {
            Ok(()) => println!("NEMESIS: {container} reconnected"),
            Err(e) => eprintln!("NEMESIS: ERROR in final heal for {container}: {e}"),
        }
    }
}

async fn restore_all(stopped: &mut Vec<String>) {
    for container in stopped.drain(..) {
        println!("NEMESIS: final restore — restarting {container}");
        match docker_start_container(&container).await {
            Ok(()) => println!("NEMESIS: {container} restarted"),
            Err(e) => eprintln!("NEMESIS: ERROR in final restore for {container}: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Fault schedules
// ---------------------------------------------------------------------------

async fn run_partition_faults(network: &str, containers: &[String], partitioned: &mut Vec<String>) {
    println!("NEMESIS: waiting 2s for cluster to stabilize...");
    sleep(Duration::from_secs(2)).await;

    for container in containers.iter() {
        // --- Inject partition ---
        println!("NEMESIS: disconnecting {container} from network {network}");
        match docker_network_disconnect(network, container).await {
            Ok(()) => {
                partitioned.push(container.clone());
                println!("NEMESIS: {container} is now partitioned");
            }
            Err(e) => {
                eprintln!("NEMESIS: ERROR partitioning {container}: {e}");
                continue; // skip heal step for this container
            }
        }

        // Hold partition for 10s — enough for OmniPaxos election timeout + stabilisation
        println!("NEMESIS: holding partition for 10s...");
        sleep(Duration::from_secs(10)).await;

        // --- Heal partition ---
        println!("NEMESIS: healing {container}");
        match docker_network_connect(network, container).await {
            Ok(()) => {
                partitioned.retain(|c| c != container);
                println!("NEMESIS: {container} reconnected");
            }
            Err(e) => {
                eprintln!("NEMESIS: ERROR reconnecting {container}: {e}");
                // Leave in `partitioned`; heal_all() will retry
            }
        }

        // Recovery window before next fault
        println!("NEMESIS: recovery window 5s...");
        sleep(Duration::from_secs(5)).await;
    }
}

async fn run_crash_faults(containers: &[String], stopped: &mut Vec<String>) {
    if containers.is_empty() {
        return;
    }

    let mut rng = StdRng::from_entropy();

    println!("NEMESIS [crash]: waiting 2s for cluster to stabilize...");
    sleep(Duration::from_secs(2)).await;

    // Perform crash-restart cycles on randomly selected nodes.
    // We do `containers.len()` rounds so each node is targeted on average once.
    let rounds = containers.len();

    for round in 0..rounds {
        // Pick a random container that is NOT already stopped
        let alive: Vec<&String> = containers
            .iter()
            .filter(|c| !stopped.contains(c))
            .collect();

        if alive.is_empty() {
            eprintln!("NEMESIS [crash]: all containers stopped — skipping round {round}");
            continue;
        }

        let target = alive[rng.gen_range(0..alive.len())].clone();

        // --- Kill the node ---
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

        // Hold the crash for a random duration between 5–15s
        let down_secs = rng.gen_range(5u64..=15);
        println!("NEMESIS [crash]: {target} down for {down_secs}s...");
        sleep(Duration::from_secs(down_secs)).await;

        // --- Restart the node ---
        println!("NEMESIS [crash]: restarting {target}");
        match docker_start_container(&target).await {
            Ok(()) => {
                stopped.retain(|c| c != &target);
                println!("NEMESIS [crash]: {target} restarted");
            }
            Err(e) => {
                eprintln!("NEMESIS [crash]: ERROR restarting {target}: {e}");
                // Leave in `stopped`; restore_all() will retry
            }
        }

        // Recovery window before next crash
        println!("NEMESIS [crash]: recovery window 5s...");
        sleep(Duration::from_secs(5)).await;
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub async fn run_nemesis_schedule(
    network: String,
    containers: Vec<String>,
    enable_crash: bool,
) {
    let mut partitioned: Vec<String> = Vec::new();
    let mut stopped: Vec<String> = Vec::new();

    // Phase 1: network partition faults
    run_partition_faults(&network, &containers, &mut partitioned).await;

    // Phase 2: crash + restart faults (if enabled)
    if enable_crash {
        run_crash_faults(&containers, &mut stopped).await;
    }

    // Unconditional cleanup — always restore any remaining containers
    heal_all(&network, &mut partitioned).await;
    restore_all(&mut stopped).await;
    println!("NEMESIS: done — all containers restored");
}
