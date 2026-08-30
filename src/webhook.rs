use std::time::Duration;

use crate::model::ParsedMessage;
use crate::validate::validate;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub const WAIT_PARAM: &str = "wait=true";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    #[error("invalid webhook URL {url:?}: must be an http(s) URL")]
    InvalidUrl { url: String },
    #[error("message failed validation: {0}")]
    Validation(#[from] crate::validate::ValidationError),
    #[error("webhook request failed: {0}")]
    Transport(String),
    #[error("Discord returned {status}: {body}")]
    Discord { status: u16, body: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendResult {
    pub status: u16,
    pub message_id: Option<String>,
}

pub fn build_request(
    webhook_url: &str,
    body_json: &str,
    wait: bool,
) -> Result<WebhookRequest, SendError> {
    if !webhook_url.starts_with("http://") && !webhook_url.starts_with("https://") {
        return Err(SendError::InvalidUrl {
            url: webhook_url.to_owned(),
        });
    }
    let mut url = webhook_url.to_owned();
    if wait {
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(WAIT_PARAM);
    }
    Ok(WebhookRequest {
        method: "POST",
        url,
        headers: vec![("Content-Type", "application/json".to_owned())],
        body: body_json.to_owned(),
    })
}

pub fn prepare(
    webhook_url: &str,
    parsed: &ParsedMessage,
    wait: bool,
) -> Result<WebhookRequest, SendError> {
    validate(&parsed.message)?;
    build_request(webhook_url, &parsed.canonical.to_string(), wait)
}

pub fn extract_message_id(response_body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(response_body)
        .ok()?
        .get("id")?
        .as_str()
        .map(str::to_owned)
}

pub fn send(
    webhook_url: &str,
    parsed: &ParsedMessage,
    wait: bool,
) -> Result<SendResult, SendError> {
    let request = prepare(webhook_url, parsed, wait)?;
    let agent = ureq::AgentBuilder::new().timeout(REQUEST_TIMEOUT).build();
    post(&agent, &request)
}

fn map_send_response(status: u16, body: String) -> Result<SendResult, SendError> {
    if (200..300).contains(&status) {
        Ok(SendResult {
            status,
            message_id: extract_message_id(&body),
        })
    } else {
        Err(SendError::Discord { status, body })
    }
}

fn post(agent: &ureq::Agent, request: &WebhookRequest) -> Result<SendResult, SendError> {
    let mut req = agent.post(&request.url);
    for (name, value) in &request.headers {
        req = req.set(name, value);
    }
    match req.send_string(&request.body) {
        Ok(response) | Err(ureq::Error::Status(_, response)) => {
            let status = response.status();
            let body = response.into_string().unwrap_or_default();
            map_send_response(status, body)
        }
        Err(ureq::Error::Transport(transport)) => Err(SendError::Transport(transport.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Message;

    fn plain_message() -> Message {
        Message {
            content: "hello".into(),
            ..Message::default()
        }
    }

    fn parsed_with(message: Message, canonical: &str) -> ParsedMessage {
        ParsedMessage {
            message,
            canonical: serde_json::from_str(canonical).expect("canonical is json"),
        }
    }

    #[test]
    fn wait_appends_query_to_plain_url() {
        let req = build_request("https://discord.com/api/webhooks/1/abc", "{}", true).unwrap();
        assert_eq!(req.url, "https://discord.com/api/webhooks/1/abc?wait=true");
    }

    #[test]
    fn wait_appends_to_existing_query_with_ampersand() {
        let req = build_request(
            "https://discord.com/api/webhooks/1/abc?thread_id=42",
            "{}",
            true,
        )
        .unwrap();
        assert_eq!(
            req.url,
            "https://discord.com/api/webhooks/1/abc?thread_id=42&wait=true"
        );
    }

    #[test]
    fn without_wait_url_is_unchanged() {
        let req = build_request("https://discord.com/api/webhooks/1/abc", "{}", false).unwrap();
        assert_eq!(req.url, "https://discord.com/api/webhooks/1/abc");
    }

    #[test]
    fn request_is_json_post() {
        let req = build_request(
            "https://discord.com/api/webhooks/1/abc",
            "{\"content\":\"hi\"}",
            false,
        )
        .unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(
            req.headers,
            vec![("Content-Type", "application/json".to_owned())]
        );
        assert_eq!(req.body, "{\"content\":\"hi\"}");
    }

    #[test]
    fn non_http_scheme_is_rejected() {
        assert_eq!(
            build_request("ftp://example.com/hook", "{}", false),
            Err(SendError::InvalidUrl {
                url: "ftp://example.com/hook".into(),
            })
        );
    }

    #[test]
    fn empty_url_is_rejected() {
        assert_eq!(
            build_request("", "{}", false),
            Err(SendError::InvalidUrl { url: String::new() })
        );
    }

    #[test]
    fn prepare_forwards_canonical_body_verbatim_after_validation() {
        let parsed = parsed_with(plain_message(), r#"{"content":"hello","custom_id":"kept"}"#);
        let req = prepare("https://discord.com/api/webhooks/1/abc", &parsed, false).unwrap();
        assert_eq!(
            req.body,
            parsed.canonical.to_string(),
            "canonical body is forwarded verbatim"
        );
        assert_eq!(req.body, r#"{"content":"hello","custom_id":"kept"}"#);
    }

    #[test]
    fn forwarded_body_keeps_button_custom_id_select_kind_and_option_value() {
        let raw = r#"{
            "content": "pick one",
            "components": [
                { "type": 1, "components": [
                    { "type": 2, "style": 1, "label": "Ping", "custom_id": "do_thing" }
                ]},
                { "type": 1, "components": [
                    { "type": 5, "custom_id": "user_pick", "placeholder": "who?" }
                ]},
                { "type": 1, "components": [
                    { "type": 3, "custom_id": "color", "options": [
                        { "label": "Red", "value": "red" }
                    ]}
                ]}
            ]
        }"#;
        let parsed = raw
            .parse::<crate::model::ParsedMessage>()
            .expect("payload parses");
        let req = prepare("https://discord.com/api/webhooks/1/abc", &parsed, false).unwrap();

        assert!(
            req.body.contains(r#""custom_id":"do_thing""#),
            "button custom_id must survive, got: {}",
            req.body
        );
        assert!(
            req.body.contains(r#""type":5"#),
            "user-select kind must keep its numeric type, got: {}",
            req.body
        );
        assert!(
            req.body.contains(r#""custom_id":"user_pick""#),
            "select custom_id must survive, got: {}",
            req.body
        );
        assert!(
            req.body.contains(r#""value":"red""#),
            "option value must survive, got: {}",
            req.body
        );
    }

    #[test]
    fn oversized_content_still_fails_before_request_is_built() {
        let mut message = plain_message();
        message.content = "x".repeat(2001);
        let parsed = parsed_with(message, "{}");
        assert!(matches!(
            prepare("https://discord.com/api/webhooks/1/abc", &parsed, false),
            Err(SendError::Validation(_))
        ));
    }

    #[test]
    fn message_id_is_extracted_from_wait_response() {
        let body = r#"{"id":"1199368822186135553","channel_id":"1","content":"hello"}"#;
        assert_eq!(
            extract_message_id(body),
            Some("1199368822186135553".to_owned())
        );
    }

    #[test]
    fn empty_response_yields_no_message_id() {
        assert_eq!(extract_message_id(""), None);
        assert_eq!(extract_message_id("not json"), None);
        assert_eq!(extract_message_id(r#"{"content":"no id"}"#), None);
    }

    #[test]
    fn success_response_with_id_maps_to_ok_result() {
        let body = r#"{"id":"1199368822186135553","channel_id":"1","content":"hello"}"#.to_owned();
        assert_eq!(
            map_send_response(200, body),
            Ok(SendResult {
                status: 200,
                message_id: Some("1199368822186135553".to_owned()),
            })
        );
    }

    #[test]
    fn rate_limit_response_surfaces_discord_429_info_as_an_error() {
        let body = r#"{"retry_after": 1.234, "global": false,
            "message": "You are being rate limited.", "code": 0}"#
            .to_owned();
        assert_eq!(
            map_send_response(429, body),
            Err(SendError::Discord {
                status: 429,
                body: r#"{"retry_after": 1.234, "global": false,
            "message": "You are being rate limited.", "code": 0}"#
                    .to_owned(),
            })
        );
    }

    #[test]
    fn non_2xx_response_maps_discord_error_body_to_discord_error() {
        let body = r#"{"message": "Unknown Webhook", "code": 10015}"#.to_owned();
        assert_eq!(
            map_send_response(404, body),
            Err(SendError::Discord {
                status: 404,
                body: r#"{"message": "Unknown Webhook", "code": 10015}"#.to_owned(),
            })
        );
    }
}
