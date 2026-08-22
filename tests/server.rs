#![cfg(feature = "serve")]

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use pwr_viewgen::server;
use tower::ServiceExt;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture exists")
}

async fn post_json(uri: &str, body: &str) -> axum::response::Response {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .expect("request builds");
    server::router()
        .oneshot(request)
        .await
        .expect("infallible service")
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

#[tokio::test]
async fn index_serves_single_page_shell() {
    let response = server::router()
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("infallible service");
    assert_eq!(response.status(), StatusCode::OK);
    let page = body_string(response).await;
    assert!(page.contains(r#"id="json""#), "JSON textarea expected");
    assert!(page.contains("/api/render"), "render call expected");
    assert!(page.contains("/api/png"), "png call expected");
    assert!(page.contains("/api/send"), "send call expected");
}

#[tokio::test]
async fn render_returns_full_document_with_chrome_markers() {
    let payload = format!(
        r#"{{"json": {}, "width": 600}}"#,
        fixture("full.json").trim()
    );
    let response = post_json("/api/render", &payload).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let value: serde_json::Value = serde_json::from_str(&body).expect("json response");
    let html = value["html"].as_str().expect("html field");
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains(r#"<span class="eg-username">Notifier</span>"#));
}

#[tokio::test]
async fn render_rejects_invalid_message_json_with_bad_request() {
    let response = post_json("/api/render", r#"{"json": "{oops", "width": 600}"#).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_string(response).await;
    assert!(body.contains("invalid"), "error context expected: {body}");
}

#[tokio::test]
async fn render_accepts_string_form_json_payload() {
    let fixture = fixture("full.json");
    let payload = format!(
        r#"{{"json": {}, "width": 600}}"#,
        serde_json::to_string(&fixture.trim()).expect("fixture is json text")
    );
    let response = post_json("/api/render", &payload).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let value: serde_json::Value = serde_json::from_str(&body).expect("json response");
    let html = value["html"].as_str().expect("html field");
    assert!(html.contains(r#"<span class="eg-username">Notifier</span>"#));
}

#[tokio::test]
async fn png_accepts_string_form_json_past_payload_parsing() {
    let oversized = format!(r#"{{"content": "{}"}}"#, "x".repeat(2001));
    let raw = serde_json::to_string(&oversized).expect("oversized is json text");
    let payload = format!(r#"{{"json": {raw}, "width": 600, "scale": 2}}"#);
    let response = post_json("/api/png", &payload).await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "string form must reach payload validation (chrome not needed to fail here)"
    );
    let body = body_string(response).await;
    assert!(
        body.contains("content must be at most 2000 characters"),
        "validation error expected (not a type mismatch), got: {body}"
    );
}

#[tokio::test]
async fn send_accepts_string_form_json_past_payload_parsing() {
    let oversized = format!(r#"{{"content": "{}"}}"#, "x".repeat(2001));
    let raw = serde_json::to_string(&oversized).expect("oversized is json text");
    let payload = format!(r#"{{"json": {raw}, "url": "https://discord.com/api/webhooks/1/abc"}}"#);
    let response = post_json("/api/send", &payload).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_string(response).await;
    assert!(
        body.contains("content must be at most 2000 characters") && !body.contains("invalid type"),
        "string must be parsed into a Message, got: {body}"
    );
}

#[tokio::test]
async fn send_reports_validation_failure_without_network() {
    let oversized = format!(r#"{{"content": "{}"}}"#, "x".repeat(2001));
    let payload =
        format!(r#"{{"json": {oversized}, "url": "https://discord.com/api/webhooks/1/abc"}}"#);
    let response = post_json("/api/send", &payload).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_string(response).await;
    assert!(
        body.contains("message failed validation")
            && body.contains("content must be at most 2000 characters"),
        "validation detail expected, got: {body}"
    );
}

#[tokio::test]
#[ignore = "requires a local Chrome/Chromium binary; run with: cargo test --features serve -- --ignored"]
async fn png_endpoint_captures_png_bytes() {
    let payload = format!(
        r#"{{"json": {}, "width": 600, "scale": 2}}"#,
        fixture("simple.json").trim()
    );
    let response = post_json("/api/png", &payload).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        axum::http::header::HeaderValue::from_static("image/png")
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "PNG magic bytes");
}
