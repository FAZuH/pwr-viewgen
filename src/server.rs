use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;

use crate::render::{self};

#[cfg(feature = "png")]
const DEFAULT_SCALE: u32 = 2;

pub fn router() -> axum::Router {
    use axum::routing::get;
    use axum::routing::post;
    axum::Router::new()
        .route("/", get(index))
        .route("/api/render", post(api_render))
        .route("/api/png", post(api_png))
        .route("/api/send", post(api_send))
}

pub async fn serve(port: u16) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|error| format!("failed to bind 127.0.0.1:{port}: {error}"))?;
    println!("pwr-viewgen serving on http://127.0.0.1:{port}");
    axum::serve(listener, router())
        .await
        .map_err(|error| format!("server error: {error}"))
}

pub fn run_server(port: u16) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to start async runtime: {error}"))?;
    runtime.block_on(serve(port))
}

async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("server.html"),
    )
}

#[derive(serde::Deserialize)]
struct RenderRequest {
    json: serde_json::Value,
    width: Option<u32>,
}

#[derive(serde::Deserialize)]
#[cfg_attr(not(feature = "png"), allow(dead_code))]
struct PngRequest {
    json: serde_json::Value,
    width: Option<u32>,
    scale: Option<u32>,
}

#[derive(serde::Deserialize)]
struct SendRequest {
    json: serde_json::Value,
    url: String,
}

async fn api_render(
    Json(request): Json<RenderRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let parsed = parse_message(&request.json)?;
    let now_unix = chrono::Utc::now().timestamp();
    let width = request.width.unwrap_or(render::DEFAULT_CONTENT_WIDTH);
    Ok(Json(serde_json::json!({
        "html": render::render_html_with_width(&parsed.message, now_unix, width),
    })))
}

async fn api_png(Json(request): Json<PngRequest>) -> Result<Response, ApiError> {
    #[cfg(feature = "png")]
    {
        let parsed = parse_message(&request.json)?;
        let now_unix = chrono::Utc::now().timestamp();
        let width = request.width.unwrap_or(render::DEFAULT_CONTENT_WIDTH);
        let scale = request.scale.unwrap_or(DEFAULT_SCALE);
        let document = render::render_html_with_width(&parsed.message, now_unix, width);
        let captured = tokio::task::spawn_blocking(move || {
            crate::shot::capture_html(&document, width, scale as f32)
        })
        .await
        .map_err(|error| internal(format!("capture task failed: {error}")))?
        .map_err(|error| internal(error.to_string()))?;
        Ok(([(header::CONTENT_TYPE, "image/png")], captured).into_response())
    }
    #[cfg(not(feature = "png"))]
    {
        let _ = request;
        Err(internal(
            "PNG export unavailable: binary was built without the `png` feature".to_owned(),
        ))
    }
}

async fn api_send(Json(request): Json<SendRequest>) -> Result<Json<serde_json::Value>, ApiError> {
    let parsed = parse_message(&request.json)?;
    let url = request.url;
    let result = tokio::task::spawn_blocking(move || crate::webhook::send(&url, &parsed, false))
        .await
        .map_err(|error| internal(format!("send task failed: {error}")))?;
    Ok(Json(match result {
        Ok(_) => serde_json::json!({ "ok": true }),
        Err(error) => serde_json::json!({ "ok": false, "error": error.to_string() }),
    }))
}

fn parse_message(value: &serde_json::Value) -> Result<crate::model::ParsedMessage, ApiError> {
    match value {
        serde_json::Value::String(raw) => raw.parse::<crate::model::ParsedMessage>(),
        _ => crate::model::ParsedMessage::from_value(value),
    }
    .map_err(|error| bad_request(format!("invalid message JSON: {error}")))
}

fn bad_request(message: String) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message)
}

fn internal(message: String) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, message)
}

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}
