use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{
        self,
        ws::{Message, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    headers,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router, TypedHeader,
};
use parking_lot::Mutex;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use tokio::time;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{info, Level};

#[derive(Debug, Clone)]
struct State {
    map: HashMap<String, f64>,
    results: HashMap<String, f64>,
}

impl State {
    pub fn update_results(&mut self) {
        for kv in self.results.iter_mut() {
            *kv.1 = self.map.get(kv.0).unwrap().sin();
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    // let state = State {
    //     map: HashMap::from_iter((0..8).map(|i| (format!("{i}"), i as f64))),
    //     results: HashMap::from_iter((0..8).map(|i| (format!("{i}"), (i as f64).sin()))),
    // };

    let state = State {
        map: HashMap::new(),
        results: HashMap::new(),
    };

    let app = Router::new()
        .route("/series", post(add_series))
        .route("/ws", get(ws_handler))
        .with_state(Arc::new(Mutex::new(state)))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

#[derive(Debug, Deserialize)]
struct Series {
    name: String,
}

async fn add_series(
    axum::extract::State(state): axum::extract::State<Arc<Mutex<State>>>,
    axum::extract::Json(payload): extract::Json<Series>,
) -> impl IntoResponse {
    let mut guard = state.lock();
    let initial = thread_rng().gen::<f64>();
    guard.map.insert(payload.name.clone(), initial);
    guard.results.insert(payload.name, initial.sin());
    StatusCode::OK
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::extract::State(state): axum::extract::State<Arc<Mutex<State>>>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr, state: Arc<Mutex<State>>) {
    info!("Set up websocket for {}", who);

    let mut interval = time::interval(Duration::from_millis(10));

    loop {
        let results = {
            let guard = state.lock();
            guard.results.clone()
        };
        let msg = serde_json::to_string(&results).unwrap();
        socket.send(Message::Text(msg)).await.unwrap();

        {
            let mut state_lock = state.lock();
            for v in state_lock.map.values_mut() {
                *v += 0.005;
            }
            state_lock.update_results();
        }

        interval.tick().await;
    }
}
