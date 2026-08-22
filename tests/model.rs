use pwr_viewgen::model::Component;
use pwr_viewgen::model::Message;
use pwr_viewgen::validate::validate;
use pwr_viewgen::validate::ValidationError;

const SIMPLE_JSON: &str = r#"{"content": "gm"}"#;

const FULL_JSON: &str = r#"{
    "id": "9999",
    "webhook_id": "42",
    "application_id": "77",
    "content": "Deploy report",
    "username": "Notifier",
    "avatar_url": "https://cdn.example.test/avatar.png",
    "tts": false,
    "flags": 0,
    "embeds": [
        {
            "title": "Deploy finished",
            "description": "All systems nominal",
            "url": "https://example.test/deploys/7",
            "timestamp": "2026-08-22T12:00:00.000Z",
            "color": 3066993,
            "footer": { "text": "CI bot", "icon_url": "https://cdn.example.test/ci.png" },
            "image": { "url": "https://cdn.example.test/chart.png" },
            "thumbnail": { "url": "https://cdn.example.test/logo.png" },
            "author": { "name": "Ada", "icon_url": "https://cdn.example.test/ada.png" },
            "fields": [
                { "name": "Branch", "value": "main", "inline": true },
                { "name": "Duration", "value": "3m 12s", "inline": true },
                { "name": "Commit", "value": "abc1234" }
            ]
        }
    ]
}"#;

const COMPONENTS_V1_JSON: &str = r#"{
    "content": "Pick one:",
    "components": [
        {
            "type": 1,
            "components": [
                { "type": 2, "style": 5, "label": "Docs", "url": "https://example.test/docs" },
                { "type": 2, "style": 1, "label": "Ping", "disabled": true }
            ]
        },
        {
            "type": 1,
            "components": [
                {
                    "type": 3,
                    "placeholder": "Choose…",
                    "options": [
                        { "label": "Red", "description": "loud", "emoji": { "name": "🔴" } },
                        { "label": "Blue", "emoji": { "name": "🔵" } }
                    ]
                }
            ]
        }
    ]
}"#;

const COMPONENTS_V2_JSON: &str = r##"{
    "flags": 32768,
    "components": [
        { "type": 10, "content": "# Release notes\n- one\n- two" },
        {
            "type": 9,
            "components": [{ "type": 10, "content": "highlight body" }],
            "accessory": {
                "type": 11,
                "media": { "url": "https://cdn.example.test/thumb.png" },
                "spoiler": false
            }
        },
        {
            "type": 12,
            "items": [ { "media": { "url": "https://cdn.example.test/a.png" } } ]
        },
        { "type": 13, "file": { "url": "attachment://notes.pdf" }, "spoiler": true },
        { "type": 14, "divider": true, "spacing": 1 },
        {
            "type": 17,
            "accent_color": 8912896,
            "components": [
                { "type": 10, "content": "container text" },
                { "type": 1, "components": [ { "type": 2, "style": 3, "label": "OK" } ] }
            ]
        }
    ]
}"##;

#[test]
fn simple_payload_round_trips_through_parse_and_validate() {
    let msg: Message = serde_json::from_str(SIMPLE_JSON).expect("simple payload parses");
    assert_eq!(msg.content, "gm");
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn full_payload_round_trips_through_parse_and_validate() {
    let msg: Message = serde_json::from_str(FULL_JSON).expect("full payload parses");
    assert_eq!(msg.username.as_deref(), Some("Notifier"));
    let [embed] = msg.embeds.as_slice() else {
        panic!("expected one embed");
    };
    assert_eq!(embed.fields.len(), 3);
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn components_v1_fixture_parses_and_validates() {
    let msg: Message = serde_json::from_str(COMPONENTS_V1_JSON).expect("v1 fixture parses");
    assert_eq!(msg.components.len(), 2);
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn components_v2_fixture_parses_and_validates() {
    let msg: Message = serde_json::from_str(COMPONENTS_V2_JSON).expect("v2 fixture parses");
    assert_eq!(msg.flags, Some(1 << 15));
    assert_eq!(msg.components.len(), 6);
    assert!(matches!(
        msg.components.first(),
        Some(Component::TextDisplay { .. })
    ));
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn unknown_fields_are_ignored_at_every_nesting_level() {
    let raw = r#"{
        "unknown_top": { "deeply": ["nested"] },
        "content": "x",
        "embeds": [ { "title": "t", "mystery_number": 7,
                      "footer": { "text": "f", "legacy": null } } ],
        "components": [ { "type": 1, "brand_new_field": true,
                          "components": [ { "type": 2, "style": 1,
                                            "future_feature": "ignored" } ] } ]
    }"#;
    let msg: Message = serde_json::from_str(raw).expect("unknown fields tolerated");
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn oversized_title_passes_parsing_but_fails_validation_with_path() {
    let title = "a".repeat(257);
    let raw = format!(r#"{{ "embeds": [ {{ "title": "{title}" }} ] }}"#);
    let msg: Message = serde_json::from_str(&raw).expect("parsing does not enforce limits");
    assert_eq!(
        validate(&msg),
        Err(ValidationError::TooLong {
            path: "embeds[0].title".into(),
            limit: 256,
            actual: 257
        })
    );
}

#[test]
fn v2_flag_alongside_content_is_rejected() {
    let raw = r#"{ "content": "nope", "flags": 32768,
                   "components": [ { "type": 10, "content": "hi" } ] }"#;
    let msg: Message = serde_json::from_str(raw).expect("parses");
    assert_eq!(
        validate(&msg),
        Err(ValidationError::ForbiddenWithComponentsV2 {
            path: "content".into()
        })
    );
}

#[test]
fn component_serialization_preserves_numeric_type_tags() {
    let msg: Message = serde_json::from_str(COMPONENTS_V2_JSON).expect("v2 fixture parses");
    let serialized = serde_json::to_string(&msg).expect("serializes");
    let reparsed: Message = serde_json::from_str(&serialized).expect("re-parses");
    assert_eq!(reparsed, msg);
    assert!(
        serialized.contains(r#""type":17"#) && serialized.contains(r#""type":10"#),
        "numeric tags must survive serialization, got: {serialized}"
    );
}
