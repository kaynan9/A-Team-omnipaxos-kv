use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use log::warn;
use omnipaxos_kv::common::{kv::KVCommand, messages::ServerMessage};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};

use crate::server::ShimRequest;

static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

fn next_command_id() -> usize {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone)]
pub struct ShimState {
    pub shim_sender: mpsc::Sender<ShimRequest>,
}

#[derive(Debug, Deserialize)]
pub struct KvRequest {
    pub op: String,
    pub key: String,
    pub value: Option<String>,
    pub expected: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KvResponse {
    pub op: String,
    pub ok: bool,
    pub key: String,
    pub value: Option<String>,
    pub error: Option<String>,
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

async fn kv_handler(
    State(state): State<ShimState>,
    Json(req): Json<KvRequest>,
) -> impl IntoResponse {
    let kv_cmd = match req.op.as_str() {
        "put" => {
            let value = match &req.value {
                Some(v) => v.clone(),
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(KvResponse {
                            op: req.op,
                            ok: false,
                            key: req.key,
                            value: None,
                            error: Some("missing 'value' for put".to_string()),
                        }),
                    );
                }
            };
            KVCommand::Put(req.key.clone(), value)
        }
        "get" => KVCommand::Get(req.key.clone()),
        "cas" => {
            let new_value = match &req.value {
                Some(v) => v.clone(),
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(KvResponse {
                            op: req.op,
                            ok: false,
                            key: req.key,
                            value: None,
                            error: Some("missing 'value' for cas".to_string()),
                        }),
                    );
                }
            };
            KVCommand::Cas(req.key.clone(), req.expected.clone(), new_value)
        }
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(KvResponse {
                    op: other.to_string(),
                    ok: false,
                    key: req.key,
                    value: None,
                    error: Some(format!("unknown op: {}", other)),
                }),
            );
        }
    };

    let command_id = next_command_id();
    let (tx, rx) = oneshot::channel::<ServerMessage>();

    let shim_req = ShimRequest {
        command_id,
        kv_command: kv_cmd,
        responder: tx,
    };

    if state.shim_sender.send(shim_req).await.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(KvResponse {
                op: req.op,
                ok: false,
                key: req.key,
                value: None,
                error: Some("unavailable".to_string()),
            }),
        );
    }

    let server_msg = match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(msg)) => msg,
        Ok(Err(_)) => {
            return (
                StatusCode::OK,
                Json(KvResponse {
                    op: req.op,
                    ok: false,
                    key: req.key,
                    value: None,
                    error: Some("unavailable".to_string()),
                }),
            );
        }
        Err(_) => {
            return (
                StatusCode::OK,
                Json(KvResponse {
                    op: req.op,
                    ok: false,
                    key: req.key,
                    value: None,
                    error: Some("timeout".to_string()),
                }),
            );
        }
    };

    let response = match server_msg {
        ServerMessage::Write(_) => KvResponse {
            op: req.op,
            ok: true,
            key: req.key,
            value: None,
            error: None,
        },
        ServerMessage::Read(_, value) => KvResponse {
            op: req.op,
            ok: true,
            key: req.key,
            value,
            error: None,
        },
        ServerMessage::CasOk(_) => KvResponse {
            op: req.op,
            ok: true,
            key: req.key,
            value: None,
            error: None,
        },
        ServerMessage::CasFailed(_, current) => KvResponse {
            op: req.op,
            ok: false,
            key: req.key,
            value: current,
            error: Some("precondition-failed".to_string()),
        },
        ServerMessage::Error(_, msg) => KvResponse {
            op: req.op,
            ok: false,
            key: req.key,
            value: None,
            error: Some(msg),
        },
        ServerMessage::StartSignal(_) => {
            warn!("Unexpected StartSignal routed to shim");
            KvResponse {
                op: req.op,
                ok: false,
                key: req.key,
                value: None,
                error: Some("unavailable".to_string()),
            }
        }
    };

    (StatusCode::OK, Json(response))
}

pub async fn run_http_shim(port: u16, shim_sender: mpsc::Sender<ShimRequest>) {
    let state = ShimState { shim_sender };
    let app = Router::new()
        .route("/health", get(health))
        .route("/kv", post(kv_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind HTTP shim port");
    log::info!("HTTP shim listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .expect("HTTP shim server failed");
}
