use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use crate::generator::Operation;

#[derive(Debug)]
pub enum OpResult {
    Ok(Option<String>),
    #[allow(dead_code)]
    Fail(String),
    Indeterminate,
}

pub struct TestClient {
    http_client: Client,
    servers: Vec<String>,
    current: usize,
}

impl TestClient {
    pub fn new(servers: Vec<String>, start_index: usize) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");
        Self {
            http_client,
            servers,
            current: start_index,
        }
    }

    fn rotate(&mut self) {
        self.current = (self.current + 1) % self.servers.len();
    }

    pub async fn send_op(&mut self, op: &Operation) -> OpResult {
        let url = format!("{}/kv", self.servers[self.current]);

        let body = match op {
            Operation::Put { key, value } => json!({
                "op": "put",
                "key": key,
                "value": value,
            }),
            Operation::Get { key } => json!({
                "op": "get",
                "key": key,
            }),
            Operation::Cas { key, expected, value } => json!({
                "op": "cas",
                "key": key,
                "value": value,
                "expected": expected,
            }),
        };

        let resp = match self.http_client.post(&url).json(&body).send().await {
            Result::Ok(r) => r,
            Err(_) => {
                self.rotate();
                return OpResult::Indeterminate;
            }
        };

        let json: Value = match resp.json().await {
            Result::Ok(j) => j,
            Err(_) => {
                self.rotate();
                return OpResult::Indeterminate;
            }
        };

        if json["ok"].as_bool().unwrap_or(false) {
            let value = json["value"].as_str().map(|s| s.to_string());
            return OpResult::Ok(value);
        }

        match json["error"].as_str().unwrap_or("") {
            "timeout" | "unavailable" => {
                self.rotate();
                OpResult::Indeterminate
            }
            other => OpResult::Fail(other.to_string()),
        }
    }
}
