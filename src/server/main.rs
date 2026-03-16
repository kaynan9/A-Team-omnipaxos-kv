use crate::{configs::OmniPaxosKVConfig, server::OmniPaxosServer};
use env_logger;

mod configs;
mod database;
mod http_shim;
mod network;
mod server;

#[tokio::main]
pub async fn main() {
    env_logger::init();
    let server_config = match OmniPaxosKVConfig::new() {
        Ok(parsed_config) => parsed_config,
        Err(e) => panic!("{e}"),
    };

    let shim_receiver = if let Some(http_port) = server_config.local.http_port {
        let (shim_tx, shim_rx) = tokio::sync::mpsc::channel(256);
        tokio::spawn(http_shim::run_http_shim(http_port, shim_tx));
        Some(shim_rx)
    } else {
        None
    };

    let mut server = OmniPaxosServer::new(server_config, shim_receiver).await;
    server.run().await;
}
