use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone)]
struct AppState {
    // 将来ログや設定を入れたい時のための枠（今は未使用）
    _dummy: Arc<()>,
}

#[tokio::main]
async fn main() {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let addr: SocketAddr = bind_addr.parse().expect("invalid BIND_ADDR");

    let state = AppState { _dummy: Arc::new(()) };

    let app = Router::new()
        .route("/", get(root))
        .route("/healthz", get(healthz))
        .route("/echo", post(echo))
        .with_state(state);

    eprintln!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> impl IntoResponse {
    "ok\n"
}

async fn healthz() -> impl IntoResponse {
    StatusCode::OK
}

async fn echo(State(_state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    // Content-Typeはクライアントの値をできるだけ保持
    let ct = headers
        .get(axum::http::header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"));

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(axum::http::header::CONTENT_TYPE, ct);

    (StatusCode::OK, resp_headers, body).into_response()
}

