//! Async WebSocket gateway client for Nexus.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn, error};

use crate::error::{NexusError, Result};

const DEFAULT_GW: &str = "ws://localhost:3001";

/// Gateway opcodes.
pub mod op {
    pub const DISPATCH: u8 = 0;
    pub const HEARTBEAT: u8 = 1;
    pub const IDENTIFY: u8 = 2;
    pub const RESUME: u8 = 6;
    pub const RECONNECT: u8 = 7;
    pub const HEARTBEAT_ACK: u8 = 11;
}

/// A raw gateway event.
#[derive(Debug, Clone)]
pub struct GatewayEvent {
    pub event: Option<String>,
    pub data: Value,
}

/// Async gateway client with auto-reconnect and heartbeat.
///
/// ```rust,no_run
/// use nexus_sdk::gateway::GatewayClient;
///
/// #[tokio::main]
/// async fn main() -> nexus_sdk::Result<()> {
///     let mut gw = GatewayClient::new("Bot mytoken", None);
///     let mut events = gw.subscribe();
///     gw.connect().await?;  // spawns background task, returns immediately
///     while let Ok(event) = events.recv().await {
///         if let Some(name) = &event.event {
///             println!("{name}: {:?}", event.data);
///         }
///     }
///     Ok(())
/// }
/// ```
pub struct GatewayClient {
    token: String,
    gateway_url: String,
    heartbeat_interval: Duration,
    max_reconnect: u32,
    sender: broadcast::Sender<GatewayEvent>,
    session_id: Arc<Mutex<Option<String>>>,
    seq: Arc<Mutex<Option<u64>>>,
}

impl GatewayClient {
    pub fn new(token: impl Into<String>, gateway_url: Option<&str>) -> Self {
        let token = {
            let t = token.into();
            if t.starts_with("Bot ") { t } else { format!("Bot {t}") }
        };
        let (sender, _) = broadcast::channel(256);
        Self {
            token,
            gateway_url: gateway_url.unwrap_or(DEFAULT_GW).to_owned(),
            heartbeat_interval: Duration::from_secs(30),
            max_reconnect: 10,
            sender,
            session_id: Arc::new(Mutex::new(None)),
            seq: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Subscribe to broadcast gateway events.
    pub fn subscribe(&self) -> broadcast::Receiver<GatewayEvent> {
        self.sender.subscribe()
    }

    /// Spawns a background task that maintains the gateway connection.
    /// Returns immediately; use [`subscribe`] to receive events.
    pub async fn connect(&self) -> Result<()> {
        let token = self.token.clone();
        let url = self.gateway_url.clone();
        let hb_interval = self.heartbeat_interval;
        let max_reconnect = self.max_reconnect;
        let tx = self.sender.clone();
        let session_id = Arc::clone(&self.session_id);
        let seq = Arc::clone(&self.seq);

        tokio::spawn(async move {
            let mut attempts = 0u32;
            loop {
                match run_once(&token, &url, hb_interval, &tx, &session_id, &seq).await {
                    Ok(()) => { attempts = 0; }
                    Err(e) => {
                        attempts += 1;
                        if attempts > max_reconnect {
                            error!("Gateway: max reconnect attempts reached: {e}");
                            break;
                        }
                        let delay = Duration::from_secs(u64::min(2u64.pow(attempts), 30));
                        warn!("Gateway: disconnected ({e}), reconnecting in {delay:?} (attempt {attempts})");
                        let _ = tx.send(GatewayEvent {
                            event: Some("RECONNECTING".into()),
                            data: json!({ "attempt": attempts }),
                        });
                        sleep(delay).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Destroy the client (stops the background task indirectly by dropping the sender).
    pub fn destroy(self) {
        drop(self.sender);
    }
}

async fn run_once(
    token: &str,
    url: &str,
    hb_interval: Duration,
    tx: &broadcast::Sender<GatewayEvent>,
    session_id: &Mutex<Option<String>>,
    seq: &Mutex<Option<u64>>,
) -> Result<()> {
    let (ws, _) = connect_async(url).await?;
    let (mut sink, mut stream) = ws.split();

    // Identify or resume
    let sid = session_id.lock().await.clone();
    let s = *seq.lock().await;
    let identify = if let (Some(sid), Some(s)) = (sid, s) {
        json!({ "op": op::RESUME, "d": { "token": token, "session_id": sid, "seq": s } })
    } else {
        json!({ "op": op::IDENTIFY, "d": { "token": token, "properties": { "$os": "rust" } } })
    };
    sink.send(Message::Text(identify.to_string().into())).await?;

    // Heartbeat task
    let sink = Arc::new(Mutex::new(sink));
    let sink_hb = Arc::clone(&sink);
    let seq_hb = Arc::clone(&Arc::new(Mutex::new(s)));
    let hb_task = tokio::spawn(async move {
        loop {
            sleep(hb_interval).await;
            let seq_val = *seq_hb.lock().await;
            let msg = json!({ "op": op::HEARTBEAT, "d": seq_val }).to_string();
            if sink_hb.lock().await.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let result = async {
        while let Some(msg) = stream.next().await {
            let msg = msg?;
            let text = match &msg {
                Message::Text(t) => t.as_str().to_owned(),
                Message::Close(_) => return Ok(()),
                _ => continue,
            };
            let payload: Value = serde_json::from_str(&text)?;
            let op_code = payload["op"].as_u64().unwrap_or(255) as u8;
            let data = payload.get("d").cloned().unwrap_or(Value::Null);
            let event_name = payload.get("t").and_then(|v| v.as_str()).map(str::to_owned);
            if let Some(s) = payload.get("s").and_then(|v| v.as_u64()) {
                *seq.lock().await = Some(s);
            }

            match op_code {
                op::DISPATCH => {
                    if let Some(ref name) = event_name {
                        if name == "READY" {
                            if let Some(sid) = data.get("session_id").and_then(|v| v.as_str()) {
                                *session_id.lock().await = Some(sid.to_owned());
                            }
                        }
                    }
                    let _ = tx.send(GatewayEvent { event: event_name, data });
                }
                op::HEARTBEAT => {
                    let s = *seq.lock().await;
                    let msg = json!({ "op": op::HEARTBEAT, "d": s }).to_string();
                    sink.lock().await.send(Message::Text(msg.into())).await?;
                }
                op::RECONNECT => {
                    info!("Gateway: server requested reconnect");
                    return Ok(());
                }
                op::HEARTBEAT_ACK => debug!("Gateway: heartbeat ack"),
                _ => {}
            }
        }
        Ok::<(), NexusError>(())
    }.await;

    hb_task.abort();
    result
}
