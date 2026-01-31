use axum::{
    body::Bytes,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let addr: SocketAddr = bind_addr.parse().expect("invalid BIND_ADDR");

    let app = Router::new()
        .route("/", get(|| async { "ok\n" }))
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/echo", post(echo));

    eprintln!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn echo(headers: HeaderMap, body: Bytes) -> Response {
    let ct = headers
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"));

    (StatusCode::OK, [(header::CONTENT_TYPE, ct)], body).into_response()
}
