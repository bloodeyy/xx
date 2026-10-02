// ============================================
// Discord RPC Sidecar — Main Entry
// ============================================

mod activity;
mod session;

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

// ============================================
// IPC Message Types
// ============================================

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum IncomingMessage {
    #[serde(rename = "init")]
    Init { user_id: String, token: String },

    #[serde(rename = "set_activity")]
    SetActivity { config: serde_json::Value },

    #[serde(rename = "clear_activity")]
    ClearActivity,

    #[serde(rename = "set_status")]
    SetStatus { status: String },

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "shutdown")]
    Shutdown,
}

#[derive(Debug, Serialize)]
struct OutgoingMessage {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl OutgoingMessage {
    fn ready(user_id: &str) -> Self {
        Self {
            status: "ready".to_string(),
            message: None,
            user_id: Some(user_id.to_string()),
            name: None,
            error: None,
        }
    }

    fn activity_set(name: &str) -> Self {
        Self {
            status: "activity_set".to_string(),
            message: None,
            user_id: None,
            name: Some(name.to_string()),
            error: None,
        }
    }

    fn error(msg: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: None,
            user_id: None,
            name: None,
            error: Some(msg.to_string()),
        }
    }

    fn pong() -> Self {
        Self {
            status: "pong".to_string(),
            message: None,
            user_id: None,
            name: None,
            error: None,
        }
    }
}

// ============================================
// Output Helper
// ============================================

fn send_message(msg: OutgoingMessage) {
    let json = serde_json::to_string(&msg).unwrap_or_else(|_| {
        r#"{"status":"error","error":"serialization failed"}"#.to_string()
    });

    let mut stdout = io::stdout();
    let _ = writeln!(stdout, "{}", json);
    let _ = stdout.flush();
}

// ============================================
// Main Loop
// ============================================

fn main() {
    eprintln!("[sidecar] Starting...");

    let session: Arc<Mutex<Option<session::RpcSession>>> = Arc::new(Mutex::new(None));

    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[sidecar] stdin error: {}", e);
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let msg: IncomingMessage = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[sidecar] invalid JSON: {} — {}", line, e);
                send_message(OutgoingMessage::error(&format!("Invalid JSON: {}", e)));
                continue;
            }
        };

        match msg {
            IncomingMessage::Init { user_id, token } => {
                eprintln!("[sidecar] Init for user {}", user_id);

                match session::RpcSession::new(&user_id, &token) {
                    Ok(new_session) => {
                        let rpc_user = new_session.user_name().unwrap_or_default();
                        let mut guard = session.lock().unwrap();
                        *guard = Some(new_session);

                        eprintln!("[sidecar] Connected as {}", rpc_user);
                        send_message(OutgoingMessage::ready(&rpc_user));
                    }
                    Err(e) => {
                        eprintln!("[sidecar] Init failed: {}", e);
                        send_message(OutgoingMessage::error(&format!("Init failed: {}", e)));
                    }
                }
            }

            IncomingMessage::SetActivity { config } => {
                let mut guard = session.lock().unwrap();

                match guard.as_mut() {
                    Some(s) => {
                        let name = config
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Custom Activity")
                            .to_string();

                        match s.set_activity_from_config(&config) {
                            Ok(_) => {
                                eprintln!("[sidecar] Activity set: {}", name);
                                send_message(OutgoingMessage::activity_set(&name));
                            }
                            Err(e) => {
                                eprintln!("[sidecar] Activity error: {}", e);
                                send_message(OutgoingMessage::error(&format!(
                                    "Activity failed: {}",
                                    e
                                )));
                            }
                        }
                    }
                    None => {
                        send_message(OutgoingMessage::error(
                            "Session not initialized. Send 'init' first.",
                        ));
                    }
                }
            }

            IncomingMessage::ClearActivity => {
                let mut guard = session.lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    let _ = s.clear_activity();
                    eprintln!("[sidecar] Activity cleared");
                }
                send_message(OutgoingMessage::activity_set(""));
            }

            IncomingMessage::SetStatus { status } => {
                let mut guard = session.lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    let _ = s.set_status(&status);
                    eprintln!("[sidecar] Status set: {}", status);
                }

                send_message(OutgoingMessage {
                    status: "status_set".to_string(),
                    message: Some(status),
                    user_id: None,
                    name: None,
                    error: None,
                });
            }

            IncomingMessage::Ping => {
                send_message(OutgoingMessage::pong());
            }

            IncomingMessage::Shutdown => {
                eprintln!("[sidecar] Shutdown requested");

                let mut guard = session.lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    let _ = s.disconnect();
                }
                break;
            }
        }
    }

    eprintln!("[sidecar] Exiting");
}