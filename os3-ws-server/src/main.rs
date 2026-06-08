use axum::{
    Router, extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State}, response::IntoResponse, routing::get
};
use futures::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, path::Path, sync::Arc, time::{Duration, Instant}
};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    sync::{RwLock, Mutex},
    time::sleep,
};
use uuid::Uuid;
use tower_http::services::ServeDir;

/// ---------- 1️⃣  Message types ----------
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Incoming {
    #[serde(rename = "keepalive")]
    KeepAlive,
    #[serde(rename = "initialize_response")]
    InitializeResponse {
        channel: String,
        client_type: String, // "phone" or "screen"
    },
    #[serde(rename = "scroll_y")]
    ScrollY { value: f64 },
    #[serde(rename = "list_clients")]
    ListClients,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Outgoing {
    #[serde(rename = "initialize")]
    Initialize { client_id: String },

    #[serde(rename = "scroll_y")]
    ScrollY {
        client_id: String,
        value: f64,
    },

    #[serde(rename = "list_clients")]
    ListClients { clients: Vec<String> },
}

/// ---------- 2️⃣  Client representation ----------
#[derive(Debug, Clone, PartialEq)]
enum ClientType {
    Phone,
    Screen,
}

impl ClientType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "phone" => Some(ClientType::Phone),
            "screen" => Some(ClientType::Screen),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Client {
    id: String,
    client_type: ClientType,
    channel: String,
    tx: UnboundedSender<Message>,
    last_heartbeat: Instant,
}

impl Client {
    fn new(id: String, tx: UnboundedSender<Message>) -> Self {
        Self {
            id,
            client_type: ClientType::Phone, // default, will be overwritten
            channel: String::new(),
            tx,
            last_heartbeat: Instant::now(),
        }
    }
}

/// ---------- 3️⃣  Shared state ----------
#[derive(Debug)]
struct AppState {
    /// client_id -> Client
    clients: HashMap<String, Client>,
    /// channel -> list of client ids (only screen)
    screen_clients_in_channel_memo: HashMap<String, Vec<String>>
}

impl AppState {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            screen_clients_in_channel_memo: HashMap::new(),
        }
    }

    /// Add or replace a client (used after initialize_response)
    fn upsert_client(&mut self, client: Client) {
        let is_screen = client.client_type == ClientType::Screen;
        let channel_copy = client.channel.clone();
        
        self.clients.insert(client.id.clone(), client);

        if is_screen {
            self.screen_clients_in_channel_memo.insert(channel_copy.clone(),
                self.screen_clients_in_channel(&channel_copy)
            );
        }
    }

    /// Remove a client
    fn remove_client(&mut self, id: &str) {
        self.clients.remove(id);
    }

    /// update heartbeat
    fn heartbeat(&mut self, id: &str) {
        if let Some(c) = self.clients.get_mut(id) {
            c.last_heartbeat = Instant::now();
        }
    }

    /// Return all screen clients that share the same channel
    fn screen_clients_in_channel(&self, channel: &str) -> Vec<String> {
        self.clients
            .values()
            .filter(|c| c.channel == channel && matches!(c.client_type, ClientType::Screen))
            .map(|x|x.id.clone())
            .collect()
    }

    fn broadcast_to_screen(&self, out_text: &str, channel: &str) {
                        if let Some(memo) = self.screen_clients_in_channel_memo.get(channel) {
                            memo.iter().for_each(|screen_id| {
                                let screen = self.clients.get(screen_id);
                                if let Some(screen) = screen {
                                    let _ = screen.tx.send(Message::Text(out_text.clone().into()));
                                }
                            })
                        }
    }

}

/// ---------- 4️⃣  WebSocket handler ----------
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<RwLock<AppState>>) {
    // 1️⃣  Split socket into sender/receiver
    let (ws_tx, mut ws_rx) = socket.split();
    let ws_tx: Arc<Mutex<SplitSink<WebSocket, Message>>> = Arc::new(Mutex::new(ws_tx));

    // 2️⃣  Generate a random UUID for this client
    let client_id = Uuid::new_v4().to_string();

    // 3️⃣  Prepare a channel for outgoing messages
    let (out_tx, mut out_rx): (
        UnboundedSender<Message>,
        UnboundedReceiver<Message>,
    ) = unbounded_channel();

    // 4️⃣  Spawn a task that forwards messages from out_rx to the websocket
    let ws_tx_2 = ws_tx.clone();
    let write_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_tx_2.lock().await.send(msg).await.is_err() {
                // Connection closed
                break;
            }
        }
    });

    // 5️⃣  Send the initial "initialize" message
    let init_msg = Outgoing::Initialize {
        client_id: client_id.clone(),
    };
    let init_text = serde_json::to_string(&init_msg).unwrap();
    if ws_tx.lock().await.send(Message::Text(init_text.into())).await.is_err() {
        return; // Connection closed before we could send
    }

    // 6️⃣  Create a placeholder client entry
    let mut client = Client::new(client_id.clone(), out_tx.clone());

    // 7️⃣  Read messages from the websocket
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(txt) => {
                // Parse the incoming JSON
                let parsed: Result<Incoming, _> = serde_json::from_str(&txt);
                match parsed {
                    Ok(Incoming::ListClients) => {
                        if !matches!(client.client_type, ClientType::Screen) {
                            continue;
                        }
                        // Broadcast to all screen clients in the same channel
                        let guard = state.read().await;
                        let out_msg = Outgoing::ListClients { clients: guard.clients.iter().filter(|(_, v)| v.client_type == ClientType::Phone).map(|(x, _)|{x.clone()}).collect() };
                        let out_text = serde_json::to_string(&out_msg).unwrap();
                        guard.broadcast_to_screen(&out_text, &client.channel);
                    }
                    Ok(Incoming::KeepAlive) => {
                        // Update heartbeat
                        let mut guard = state.write().await;
                        guard.heartbeat(&client_id);
                    }
                    Ok(Incoming::InitializeResponse { channel, client_type }) => {
                        // Store the type and channel
                        client.channel = channel;
                        client.client_type = match ClientType::from_str(&client_type) {
                            Some(t) => t,
                            None => {
                                // Invalid type – drop connection
                                return;
                            }
                        };
                        // Add/replace in the global state
                        let mut guard = state.write().await;
                        guard.upsert_client(client.clone());
                    }
                    Ok(Incoming::ScrollY { value }) => {
                        // Only phones are allowed to send scroll_y
                        if !matches!(client.client_type, ClientType::Phone) {
                            continue;
                        }
                        // Build the outgoing message
                        let out_msg = Outgoing::ScrollY {
                            client_id: client.id.clone(),
                            value,
                        };
                        let out_text = serde_json::to_string(&out_msg).unwrap();

                        // Broadcast to all screen clients in the same channel
                        let guard = state.read().await;
                        guard.broadcast_to_screen(&out_text, &client.channel);
                    }
                    Err(_) => todo!(),
                }
            }
            Message::Close(_) => {
                // Client closed th.getn
                break;
            }
            _ => {}
        }
    }

    // 8️⃣  Clean up: remove client from state
    let mut guard = state.write().await;
    guard.remove_client(&client_id);

    // 9️⃣  Ensure the write task finishes
    write_task.abort();
}

/// ---------- 5️⃣  Periodic inactivity checker ----------
async fn inactivity_checker(state: Arc<RwLock<AppState>>) {
    loop {
        sleep(Duration::from_secs(1)).await;
        let now = Instant::now();
        let mut guard = state.write().await;

        // Collect ids that have timed out
        let timed_out: Vec<String> = guard
            .clients
            .iter()
            .filter_map(|(id, client)| {
                if now.duration_since(client.last_heartbeat) > Duration::from_secs(5) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        for id in timed_out {
            let _ = guard.clients.get(&id).unwrap().tx.send(Message::Close(None));
            guard.remove_client(&id);
            println!("Removed due to inactivity: {}", &id);
        }
    }
}

/// ---------- 6️⃣  Server entry point ----------
#[tokio::main]
async fn main() {
    let shared_state = Arc::new(RwLock::new(AppState::new()));

    // Spawn the inactivity checker in the background
    let state_clone = shared_state.clone();
    tokio::spawn(async move { inactivity_checker(state_clone).await });

    let app = Router::new()
        .route("/doomscroll_web/ws", get(ws_handler))
        .fallback_service(ServeDir::new(Path::new("../os3yew/wasm/doomscroll/")))
        .with_state(shared_state);

    // Listen on 0.0.0.0:3000
    axum::serve(tokio::net::TcpListener::bind("127.0.0.1:6543").await.unwrap(), app.into_make_service()).await.unwrap();
}

