//! WebSocket handler implementation

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::task_manager::TaskEvent;
use crate::types::{ClientMessage, ServerMessage};
use crate::api::ApiState;

/// WebSocket connection state
#[derive(Debug, Default)]
struct ConnectionState {
    /// Subscribed task IDs (empty = all)
    subscribed_tasks: HashSet<i64>,
    /// Subscribe to all tasks
    subscribe_all: bool,
}

/// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: ApiState) {
    let conn_state = Arc::new(Mutex::new(ConnectionState::default()));

    // Subscribe to task events
    let mut event_rx = state.task_manager.subscribe();

    info!("WebSocket client connected");

    // Simple message handling loop
    loop {
        tokio::select! {
            // Receive from client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received WebSocket message: {}", text);
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                handle_client_message(&conn_state, client_msg).await;
                            }
                            Err(e) => {
                                warn!("Failed to parse client message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }

            // Receive events from task manager
            event = event_rx.recv() => {
                if let Ok(event) = event {
                    let should_send = {
                        let state = conn_state.lock().await;
                        should_send_event(&state, &event)
                    };

                    if should_send {
                        let msg = event_to_message(&event);
                        if socket.send(Message::Text(msg)).await.is_err() {
                            warn!("Failed to send event to client");
                            break;
                        }
                    }
                }
            }
        }
    }

    info!("WebSocket connection closed");
}

/// Check if we should send an event to this client
fn should_send_event(state: &ConnectionState, event: &TaskEvent) -> bool {
    if state.subscribe_all {
        return true;
    }

    let task_id = match event {
        TaskEvent::Created { task_id } => *task_id,
        TaskEvent::StatusChanged { task_id, .. } => *task_id,
        TaskEvent::Progress { task_id, .. } => *task_id,
        TaskEvent::CrashFound { task_id, .. } => *task_id,
        TaskEvent::Finished { task_id, .. } => *task_id,
        TaskEvent::Error { task_id, .. } => *task_id,
    };

    state.subscribed_tasks.contains(&task_id)
}

/// Convert task event to WebSocket message
fn event_to_message(event: &TaskEvent) -> String {
    let msg = match event {
        TaskEvent::Created { task_id } => {
            // Use a simple JSON structure for task created
            return serde_json::to_string(&serde_json::json!({
                "type": "task_created",
                "task_id": task_id
            })).unwrap_or_default();
        }
        TaskEvent::StatusChanged { task_id, status } => {
            ServerMessage::Status {
                task_id: *task_id,
                status: *status,
            }
        }
        TaskEvent::Progress { task_id, iteration, crashes, speed, current_mutator } => {
            ServerMessage::Progress {
                task_id: *task_id,
                iteration: *iteration,
                crashes: *crashes,
                speed: *speed,
                current_mutator: current_mutator.clone(),
            }
        }
        TaskEvent::CrashFound { task_id, crash_id, crash_type, iteration } => {
            ServerMessage::Crash {
                task_id: *task_id,
                crash_id: *crash_id,
                crash_type: crash_type.clone(),
                iteration: *iteration,
            }
        }
        TaskEvent::Finished { task_id, status } => {
            ServerMessage::Status {
                task_id: *task_id,
                status: *status,
            }
        }
        TaskEvent::Error { task_id, message } => {
            ServerMessage::Error {
                message: format!("Task {}: {}", task_id, message),
            }
        }
    };

    serde_json::to_string(&msg).unwrap_or_default()
}

/// Handle client message
async fn handle_client_message(conn_state: &Arc<Mutex<ConnectionState>>, msg: ClientMessage) {
    let mut state = conn_state.lock().await;

    match msg {
        ClientMessage::Subscribe { task_id } => {
            state.subscribed_tasks.insert(task_id);
            state.subscribe_all = false;
            debug!("Client subscribed to task {}", task_id);
        }
        ClientMessage::Unsubscribe { task_id } => {
            state.subscribed_tasks.remove(&task_id);
            debug!("Client unsubscribed from task {}", task_id);
        }
        ClientMessage::SubscribeAll => {
            state.subscribed_tasks.clear();
            state.subscribe_all = true;
            debug!("Client subscribed to all tasks");
        }
    }
}
