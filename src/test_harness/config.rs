use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "OmniPaxos KV test harness")]
pub struct TestConfig {
    #[arg(long, default_value = "http://localhost:8081,http://localhost:8082,http://localhost:8083")]
    pub servers: String,

    #[arg(long, default_value_t = 5)]
    pub num_clients: usize,

    #[arg(long, default_value_t = 1500)]
    pub ops_per_client: usize,

    #[arg(long, default_value_t = 5)]
    pub key_range: usize,

    #[arg(long, default_value_t = 0.5)]
    pub read_ratio: f64,

    #[arg(long, default_value_t = 0.1)]
    pub cas_ratio: f64,

    #[arg(long, default_value = "history.edn")]
    pub output: String,

    #[arg(long, default_value_t = false)]
    pub nemesis: bool,

    #[arg(long, default_value = "omnipaxos-net")]
    pub nemesis_network: String,

    #[arg(long, default_value = "s1,s2,s3")]
    pub nemesis_containers: String,

    #[arg(long, default_value_t = false)]
    pub nemesis_crash: bool,

    #[arg(long, default_value_t = false)]
    pub nemesis_quorum_loss: bool,

    #[arg(long, default_value_t = false)]
    pub convergence_check: bool,
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
