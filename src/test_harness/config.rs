use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "OmniPaxos KV test harness")]
pub struct TestConfig {
    /// Comma-separated list of server base URLs
    #[arg(long, default_value = "http://localhost:8081,http://localhost:8082,http://localhost:8083")]
    pub servers: String,

    /// Number of concurrent clients
    #[arg(long, default_value_t = 5)]
    pub num_clients: usize,

    /// Number of operations per client
    #[arg(long, default_value_t = 1500)]
    pub ops_per_client: usize,

    /// Number of distinct keys in the keyspace
    #[arg(long, default_value_t = 5)]
    pub key_range: usize,

    /// Fraction of operations that are reads (0.0–1.0)
    #[arg(long, default_value_t = 0.5)]
    pub read_ratio: f64,

    /// Fraction of operations that are compare-and-swap (0.0–1.0)
    #[arg(long, default_value_t = 0.1)]
    pub cas_ratio: f64,

    /// Path to the output history file
    #[arg(long, default_value = "history.edn")]
    pub output: String,

    /// Enable the nemesis (fault injection)
    #[arg(long, default_value_t = false)]
    pub nemesis: bool,

    /// Docker network name to partition (default matches docker-compose in build_scripts/)
    #[arg(long, default_value = "omnipaxos-net")]
    pub nemesis_network: String,

    /// Comma-separated Docker container names the nemesis may partition
    #[arg(long, default_value = "s1,s2,s3")]
    pub nemesis_containers: String,

    /// Enable crash + restart fault injection (kill and restart Docker containers)
    #[arg(long, default_value_t = false)]
    pub nemesis_crash: bool,
}

impl TestConfig {
    pub fn server_list(&self) -> Vec<String> {
        self.servers.split(',').map(|s| s.trim().to_string()).collect()
    }

    pub fn nemesis_containers_list(&self) -> Vec<String> {
        self.nemesis_containers
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
