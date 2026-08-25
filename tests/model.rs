use pwr_viewgen::model::parse_message;
use pwr_viewgen::model::Component;
use pwr_viewgen::model::Message;
use pwr_viewgen::validate::validate;
use pwr_viewgen::validate::ValidationError;
use pwr_viewgen::validate::MAX_COMBINED_TEXT_CHARS;
use pwr_viewgen::validate::MAX_GALLERY_ITEMS;
use pwr_viewgen::validate::MAX_TEXT_DISPLAY_CHARS;

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
                    "custom_id":"color_pick",
                    "placeholder": "Choose…",
                    "options": [
                        { "label": "Red", "value":"red", "description": "loud", "emoji": { "name": "🔴" } },
                        { "label": "Blue", "value":"blue", "emoji": { "name": "🔵" } }
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
    let msg: Message = parse_message(SIMPLE_JSON).expect("simple payload parses").message;
    assert_eq!(msg.content, "gm");
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn full_payload_round_trips_through_parse_and_validate() {
    let msg: Message = parse_message(FULL_JSON).expect("full payload parses").message;
    assert_eq!(msg.username.as_deref(), Some("Notifier"));
    let [embed] = msg.embeds.as_slice() else {
        panic!("expected one embed");
    };
    assert_eq!(embed.fields.len(), 3);
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn components_v1_fixture_parses_and_validates() {
    let msg: Message = parse_message(COMPONENTS_V1_JSON).expect("v1 fixture parses").message;
    assert_eq!(msg.components.len(), 2);
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn components_v2_fixture_parses_and_validates() {
    let msg: Message = parse_message(COMPONENTS_V2_JSON).expect("v2 fixture parses").message;
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
    let msg: Message = parse_message(raw).expect("unknown fields tolerated").message;
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn oversized_title_passes_parsing_but_fails_validation_with_path() {
    let title = "a".repeat(257);
    let raw = format!(r#"{{ "embeds": [ {{ "title": "{title}" }} ] }}"#);
    let msg: Message = parse_message(&raw).expect("parsing does not enforce limits").message;
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
    let msg: Message = parse_message(raw).expect("parses").message;
    assert_eq!(
        validate(&msg),
        Err(ValidationError::ForbiddenWithComponentsV2 {
            path: "content".into()
        })
    );
}

#[test]
fn parse_is_idempotent_through_the_canonical_form() {
    let once = parse_message(COMPONENTS_V2_JSON).expect("first parse");
    let serialized = once.canonical.to_string();
    let twice = parse_message(&serialized).expect("second parse");
    assert_eq!(once.message, twice.message);
}

const SELECT_KINDS_JSON: &str = r#"{
    "components": [
        { "type": 1, "components": [ { "type": 5, "custom_id": "u", "placeholder": "Pick user" } ] },
        { "type": 1, "components": [ { "type": 6, "custom_id": "r" } ] },
        { "type": 1, "components": [ { "type": 7, "custom_id": "m", "disabled": true } ] },
        { "type": 1, "components": [ { "type": 8, "custom_id": "c", "channel_types": [0, 2] } ] }
    ]
}"#;

#[test]
fn select_kinds_user_role_mentionable_channel_parse_with_their_kind() {
    let msg: Message = parse_message(SELECT_KINDS_JSON).expect("typed selects parse").message;
    assert_eq!(msg.components.len(), 4);
    for (index, kind) in [5u8, 6, 7, 8].into_iter().enumerate() {
        let Component::ActionRow { components } = &msg.components[index] else {
            panic!("row {index} should be an action row");
        };
        let [menu] = components.as_slice() else {
            panic!("row {index} should hold one menu");
        };
        let Component::SelectMenu {
            kind: parsed_kind,
            placeholder,
            options,
            disabled,
        } = menu
        else {
            panic!("expected a select menu in row {index}");
        };
        assert_eq!(*parsed_kind, kind);
        assert!(options.is_empty(), "non-string selects carry no options");
        match index {
            0 => assert_eq!(placeholder.as_deref(), Some("Pick user")),
            1 => assert_eq!(placeholder.as_deref(), None),
            2 => assert!(*disabled),
            _ => {}
        }
    }
}

#[test]
fn string_select_keeps_options_and_reports_kind_three() {
    let msg: Message = parse_message(COMPONENTS_V1_JSON).expect("v1 fixture parses").message;
    let Component::ActionRow { components } = &msg.components[1] else {
        panic!("second row should be an action row");
    };
    let Some(Component::SelectMenu {
        kind,
        placeholder,
        options,
        ..
    }) = components.first()
    else {
        panic!("expected a string select");
    };
    assert_eq!(*kind, 3);
    assert_eq!(placeholder.as_deref(), Some("Choose…"));
    assert_eq!(options.len(), 2);
    assert_eq!(options[0].label, "Red");
}

#[test]
fn premium_button_parses_with_style_six_without_url() {
    let raw = r#"{
        "components": [
            { "type": 1, "components": [
                { "type": 2, "style": 6, "sku_id": "123456789012345678" }
            ]}
        ]
    }"#;
    let msg: Message = parse_message(raw).expect("premium button parses").message;
    let Component::ActionRow { components } = &msg.components[0] else {
        panic!("expected action row");
    };
    let [Component::Button {
        style,
        url,
        disabled,
        ..
    }] = components.as_slice()
    else {
        panic!("expected one button");
    };
    assert_eq!(*style, 6);
    assert_eq!(url.as_deref(), None);
    assert!(!*disabled);
}

#[test]
fn webhook_identity_fields_survive_the_parse_pipeline() {
    let msg: Message = parse_message(FULL_JSON).expect("full payload parses").message;
    assert_eq!(msg.username.as_deref(), Some("Notifier"));
    assert_eq!(
        msg.avatar_url.as_deref(),
        Some("https://cdn.example.test/avatar.png")
    );
}

#[test]
fn components_v2_flag_bit_survives_parse_pipeline() {
    let msg: Message = parse_message(COMPONENTS_V2_JSON).expect("v2 fixture parses").message;
    assert_eq!(
        msg.flags,
        Some(1 << 15),
        "IS_COMPONENTS_V2 must survive from_bits_truncate"
    );
}

#[test]
fn flag_bits_undefined_upstream_are_truncated_at_parse_time() {
    let raw = r#"{"flags": 32772, "content": "x"}"#;
    let msg: Message = parse_message(raw).expect("parses").message;
    assert_eq!(
        msg.flags,
        Some(32772),
        "defined bits (suppress embeds) stay"
    );

    let raw_unknown = format!(r#"{{"flags": {}, "content": "x"}}"#, 1 << 20 | 32768);
    let msg: Message = parse_message(&raw_unknown).expect("parses").message;
    assert_eq!(
        msg.flags,
        Some(32768),
        "undefined bit 1<<20 is dropped exactly like upstream from_bits_truncate"
    );
}

#[test]
fn unknown_component_type_is_reported_in_a_parse_context() {
    let err = parse_message(r#"{"content":"x","components":[{"type":42,"content":"nope"}]}"#)
        .expect_err("unknown type must fail");
    assert!(
        err.to_string().contains("component type"),
        "error should mention component type, got: {err}"
    );
}

#[test]
fn missing_component_type_field_is_reported_in_a_parse_context() {
    let err = parse_message(r#"{"content":"x","components":[{"content":"no type here"}]}"#)
        .expect_err("missing type must fail");
    assert!(
        err.to_string().contains("\"type\""),
        "error should mention the missing type field, got: {err}"
    );
}

#[test]
fn v2_tree_with_text_display_and_full_gallery_parses_and_validates() {
    let items: Vec<String> = (0..MAX_GALLERY_ITEMS)
        .map(|i| format!(r#"{{ "media": {{ "url": "https://cdn.example.test/g{i}.png" }} }}"#))
        .collect();
    let raw = format!(
        r##"{{ "flags": 32768, "components": [
            {{ "type": 10, "content": "# Release notes\nAll systems go." }},
            {{ "type": 12, "items": [{}] }}
        ] }}"##,
        items.join(",")
    );
    let msg: Message = parse_message(&raw).expect("v2 tree parses").message;
    assert_eq!(validate(&msg), Ok(()));
}

#[test]
fn oversized_text_display_rejects_with_a_message_naming_the_limit() {
    let content = "x".repeat(MAX_TEXT_DISPLAY_CHARS + 1);
    let raw = format!(
        r#"{{ "flags": 32768, "components": [ {{ "type": 10, "content": "{content}" }} ] }}"#
    );
    let msg: Message = parse_message(&raw).expect("parsing does not enforce limits").message;
    let error = validate(&msg).expect_err("oversized text display must be rejected");
    assert_eq!(
        error.to_string(),
        format!(
            "components[0].content must be at most {MAX_TEXT_DISPLAY_CHARS} characters, got {}",
            MAX_TEXT_DISPLAY_CHARS + 1
        )
    );
}

#[test]
fn content_plus_two_text_displays_over_combined_budget_is_rejected() {
    let content = "c".repeat(100);
    let display = "t".repeat(2000);
    let raw = format!(
        r#"{{
            "content": "{content}",
            "components": [
                {{ "type": 10, "content": "{display}" }},
                {{ "type": 10, "content": "{display}" }}
            ]
        }}"#
    );
    let msg: Message = parse_message(&raw).expect("parsing does not enforce combined budget").message;
    assert_eq!(
        validate(&msg),
        Err(ValidationError::CombinedTextTooLong {
            limit: MAX_COMBINED_TEXT_CHARS,
            actual: MAX_COMBINED_TEXT_CHARS + 100
        })
    );
}

#[test]
fn eleven_item_gallery_is_rejected_after_parsing() {
    let item_count = MAX_GALLERY_ITEMS + 1;
    let items: Vec<String> = (0..item_count)
        .map(|i| format!(r#"{{ "media": {{ "url": "https://cdn.example.test/g{i}.png" }} }}"#))
        .collect();
    let raw = format!(
        r#"{{ "flags": 32768, "components": [ {{ "type": 12, "items": [{}] }} ] }}"#,
        items.join(",")
    );
    let msg: Message = parse_message(&raw).expect("parsing does not enforce gallery limits").message;
    assert_eq!(
        validate(&msg),
        Err(ValidationError::TooManyItems {
            path: "components[0]".into(),
            limit: MAX_GALLERY_ITEMS,
            actual: item_count
        })
    );
}

#[test]
fn section_body_text_display_counts_toward_the_combined_budget() {
    let content = "c".repeat(100);
    let display = "t".repeat(MAX_TEXT_DISPLAY_CHARS);
    let raw = format!(
        r#"{{ "content": "{content}", "components": [
            {{
                "type": 9,
                "components": [ {{ "type": 10, "content": "{display}" }} ],
                "accessory": {{ "type": 11, "media": {{ "url": "https://cdn.example.test/s.png" }} }}
            }}
        ] }}"#
    );
    let msg: Message = parse_message(&raw).expect("section tree parses").message;
    assert_eq!(
        validate(&msg),
        Err(ValidationError::CombinedTextTooLong {
            limit: MAX_COMBINED_TEXT_CHARS,
            actual: MAX_TEXT_DISPLAY_CHARS + 100
        })
    );
}

#[test]
fn container_text_display_counts_toward_the_combined_budget() {
    let content = "c".repeat(100);
    let display = "t".repeat(MAX_TEXT_DISPLAY_CHARS);
    let raw = format!(
        r#"{{ "content": "{content}", "components": [
            {{
                "type": 17,
                "components": [ {{ "type": 10, "content": "{display}" }} ]
            }}
        ] }}"#
    );
    let msg: Message = parse_message(&raw).expect("container tree parses").message;
    assert_eq!(
        validate(&msg),
        Err(ValidationError::CombinedTextTooLong {
            limit: MAX_COMBINED_TEXT_CHARS,
            actual: MAX_TEXT_DISPLAY_CHARS + 100
        })
    );
}

#[test]
fn single_oversized_section_display_reports_deep_path_before_combined_budget() {
    let display = "x".repeat(MAX_TEXT_DISPLAY_CHARS + 1);
    let raw = format!(
        r#"{{ "components": [
            {{
                "type": 9,
                "components": [ {{ "type": 10, "content": "{display}" }} ],
                "accessory": {{ "type": 11, "media": {{ "url": "https://cdn.example.test/s.png" }} }}
            }}
        ] }}"#
    );
    let msg: Message = parse_message(&raw).expect("parsing does not enforce limits").message;
    assert_eq!(
        validate(&msg),
        Err(ValidationError::TooLong {
            path: "components[0].components[0].content".into(),
            limit: MAX_TEXT_DISPLAY_CHARS,
            actual: MAX_TEXT_DISPLAY_CHARS + 1
        })
    );
}
