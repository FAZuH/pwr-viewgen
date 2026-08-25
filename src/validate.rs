use crate::model::Component;
use crate::model::Embed;
use crate::model::Footer;
use crate::model::Message;

pub const MAX_CONTENT_CHARS: usize = 2000;
pub const MAX_EMBEDS: usize = 10;
pub const MAX_TITLE_CHARS: usize = 256;
pub const MAX_DESCRIPTION_CHARS: usize = 4096;
pub const MAX_FIELDS: usize = 25;
pub const MAX_FIELD_NAME_CHARS: usize = 256;
pub const MAX_FIELD_VALUE_CHARS: usize = 1024;
pub const MAX_FOOTER_TEXT_CHARS: usize = 2048;
pub const MAX_AUTHOR_NAME_CHARS: usize = 256;
pub const MIN_COLOR: i64 = 0;
pub const MAX_COLOR: i64 = 0xFF_FFFF;
pub const MAX_ACTION_ROW_CHILDREN: usize = 5;
pub const MAX_TEXT_DISPLAY_CHARS: usize = 4000;
pub const MAX_COMBINED_TEXT_CHARS: usize = 4000;
pub const MAX_GALLERY_ITEMS: usize = 10;
const BUTTON_STYLE_RANGE: std::ops::RangeInclusive<u8> = 1..=5;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("{path} must be at most {limit} characters, got {actual}")]
    TooLong {
        path: String,
        limit: usize,
        actual: usize,
    },
    #[error("{path} must contain at most {limit} items, got {actual}")]
    TooManyItems {
        path: String,
        limit: usize,
        actual: usize,
    },
    #[error("{path} must be between {min} and {max}, got {actual}")]
    OutOfRange {
        path: String,
        min: i64,
        max: i64,
        actual: i64,
    },
    #[error("{path}: not allowed when the components v2 flag is set")]
    ForbiddenWithComponentsV2 { path: String },
    #[error("{path}: at least one component is required when the components v2 flag is set")]
    MissingComponentsForV2 { path: String },
    #[error("{path}: action rows allow at most {limit} child components, got {actual}")]
    ActionRowTooLarge {
        path: String,
        limit: usize,
        actual: usize,
    },
    #[error(
        "content and text display contents must total at most {limit} characters, got {actual}"
    )]
    CombinedTextTooLong { limit: usize, actual: usize },
}

pub fn validate(message: &Message) -> Result<(), ValidationError> {
    let components_v2 = message.is_components_v2();
    if components_v2 {
        if !message.content.is_empty() {
            return Err(ValidationError::ForbiddenWithComponentsV2 {
                path: "content".into(),
            });
        }
        if !message.embeds.is_empty() {
            return Err(ValidationError::ForbiddenWithComponentsV2 {
                path: "embeds".into(),
            });
        }
        if message.components.is_empty() {
            return Err(ValidationError::MissingComponentsForV2 {
                path: "components".into(),
            });
        }
    } else if char_len(&message.content) > MAX_CONTENT_CHARS {
        return Err(ValidationError::TooLong {
            path: "content".into(),
            limit: MAX_CONTENT_CHARS,
            actual: char_len(&message.content),
        });
    }

    if message.embeds.len() > MAX_EMBEDS {
        return Err(ValidationError::TooManyItems {
            path: "embeds".into(),
            limit: MAX_EMBEDS,
            actual: message.embeds.len(),
        });
    }
    for (index, embed) in message.embeds.iter().enumerate() {
        validate_embed(embed, &format!("embeds[{index}]"))?;
    }

    for (index, component) in message.components.iter().enumerate() {
        validate_component(component, &format!("components[{index}]"))?;
    }

    let combined_chars = char_len(&message.content)
        + message
            .components
            .iter()
            .map(text_display_char_count)
            .sum::<usize>();
    if combined_chars > MAX_COMBINED_TEXT_CHARS {
        return Err(ValidationError::CombinedTextTooLong {
            limit: MAX_COMBINED_TEXT_CHARS,
            actual: combined_chars,
        });
    }
    Ok(())
}

fn validate_embed(embed: &Embed, base: &str) -> Result<(), ValidationError> {
    check_len(
        embed.title.as_deref(),
        &format!("{base}.title"),
        MAX_TITLE_CHARS,
    )?;
    check_len(
        embed.description.as_deref(),
        &format!("{base}.description"),
        MAX_DESCRIPTION_CHARS,
    )?;
    if let Some(color) = embed.color {
        if !(MIN_COLOR..=MAX_COLOR).contains(&color) {
            return Err(ValidationError::OutOfRange {
                path: format!("{base}.color"),
                min: MIN_COLOR,
                max: MAX_COLOR,
                actual: color,
            });
        }
    }
    if let Some(Footer { text, .. }) = &embed.footer {
        check_len(
            Some(text),
            &format!("{base}.footer.text"),
            MAX_FOOTER_TEXT_CHARS,
        )?;
    }
    if let Some(author) = &embed.author {
        check_len(
            Some(&author.name),
            &format!("{base}.author.name"),
            MAX_AUTHOR_NAME_CHARS,
        )?;
    }
    if embed.fields.len() > MAX_FIELDS {
        return Err(ValidationError::TooManyItems {
            path: format!("{base}.fields"),
            limit: MAX_FIELDS,
            actual: embed.fields.len(),
        });
    }
    for (index, field) in embed.fields.iter().enumerate() {
        let base = format!("{base}.fields[{index}]");
        check_len(
            Some(&field.name),
            &format!("{base}.name"),
            MAX_FIELD_NAME_CHARS,
        )?;
        check_len(
            Some(&field.value),
            &format!("{base}.value"),
            MAX_FIELD_VALUE_CHARS,
        )?;
    }
    Ok(())
}

fn validate_component(component: &Component, path: &str) -> Result<(), ValidationError> {
    match component {
        Component::ActionRow { components } => {
            if components.len() > MAX_ACTION_ROW_CHILDREN {
                return Err(ValidationError::ActionRowTooLarge {
                    path: path.to_owned(),
                    limit: MAX_ACTION_ROW_CHILDREN,
                    actual: components.len(),
                });
            }
            walk_children(components, path)
        }
        Component::Button { style, .. } => {
            if !BUTTON_STYLE_RANGE.contains(style) {
                return Err(ValidationError::OutOfRange {
                    path: format!("{path}.style"),
                    min: 1,
                    max: 5,
                    actual: *style as i64,
                });
            }
            Ok(())
        }
        Component::Section {
            components,
            accessory,
        } => {
            walk_children(components, path)?;
            validate_component(accessory, &format!("{path}.accessory"))
        }
        Component::Container { components, .. } => walk_children(components, path),
        Component::TextDisplay { content } => check_len(
            Some(content),
            &format!("{path}.content"),
            MAX_TEXT_DISPLAY_CHARS,
        ),
        Component::MediaGallery { items } => {
            if items.len() > MAX_GALLERY_ITEMS {
                return Err(ValidationError::TooManyItems {
                    path: path.to_owned(),
                    limit: MAX_GALLERY_ITEMS,
                    actual: items.len(),
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn walk_children(components: &[Component], parent_path: &str) -> Result<(), ValidationError> {
    for (index, child) in components.iter().enumerate() {
        validate_component(child, &format!("{parent_path}.components[{index}]"))?;
    }
    Ok(())
}

fn text_display_char_count(component: &Component) -> usize {
    match component {
        Component::TextDisplay { content } => char_len(content),
        Component::ActionRow { components } => text_display_char_count_in(components),
        Component::Section {
            components,
            accessory,
        } => text_display_char_count_in(components) + text_display_char_count(accessory),
        Component::Container { components, .. } => text_display_char_count_in(components),
        _ => 0,
    }
}

fn text_display_char_count_in(components: &[Component]) -> usize {
    components.iter().map(text_display_char_count).sum()
}

fn check_len(value: Option<&str>, path: &str, limit: usize) -> Result<(), ValidationError> {
    let Some(text) = value else {
        return Ok(());
    };
    let len = char_len(text);
    if len > limit {
        Err(ValidationError::TooLong {
            path: path.to_owned(),
            limit,
            actual: len,
        })
    } else {
        Ok(())
    }
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Field;
    use crate::model::MediaGalleryItem;
    use crate::model::UnfurledMediaItem;

    fn plain_msg() -> Message {
        Message {
            content: "hello".into(),
            ..Message::default()
        }
    }

    fn sized_string(len: usize, ch: char) -> String {
        std::iter::repeat_n(ch, len).collect()
    }

    fn embed_with_title(len: usize) -> Embed {
        Embed {
            title: Some(sized_string(len, 'x')),
            ..Embed::default()
        }
    }

    #[test]
    fn valid_message_with_embed_and_components_passes() {
        let raw = r#"{
            "content": "hello",
            "username": "Bot",
            "embeds": [
                {
                    "title": "t",
                    "description": "d",
                    "color": 16711680,
                    "footer": { "text": "ft" },
                    "author": { "name": "ada" },
                    "fields": [ { "name": "k", "value": "v", "inline": true } ]
                }
            ],
            "components": [
                { "type": 1, "components": [ { "type": 2, "style": 3, "label": "go" } ] }
            ]
        }"#;
        let msg: Message = serde_json::from_str(raw).expect("parses");
        assert_eq!(validate(&msg), Ok(()));
    }

    #[test]
    fn all_exact_limits_pass() {
        let mut msg = plain_msg();
        msg.content = sized_string(2000, 'a');
        msg.embeds = vec![Embed {
            title: Some(sized_string(256, 't')),
            description: Some(sized_string(4096, 'd')),
            color: Some(0xFF_FFFF),
            footer: Some(Footer {
                text: sized_string(2048, 'f'),
                icon_url: None,
            }),
            author: Some(crate::model::Author {
                name: sized_string(256, 'n'),
                url: None,
                icon_url: None,
            }),
            fields: (0..25)
                .map(|_| Field {
                    name: sized_string(256, 'n'),
                    value: sized_string(1024, 'v'),
                    inline: false,
                })
                .collect(),
            ..Embed::default()
        }];
        msg.components = vec![Component::ActionRow {
            components: (1..=5)
                .map(|style| Component::Button {
                    style,
                    label: Some("b".into()),
                    emoji: None,
                    url: None,
                    disabled: false,
                })
                .collect(),
        }];
        assert_eq!(validate(&msg), Ok(()));
    }

    #[test]
    fn content_over_2000_is_rejected_at_content_path() {
        let mut msg = plain_msg();
        msg.content = sized_string(2001, 'a');
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooLong {
                path: "content".into(),
                limit: 2000,
                actual: 2001
            })
        );
    }

    #[test]
    fn eleven_embeds_are_rejected_at_embeds_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![Embed::default(); 11];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooManyItems {
                path: "embeds".into(),
                limit: 10,
                actual: 11
            })
        );
    }

    #[test]
    fn oversized_title_reports_embed_indexed_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![embed_with_title(257)];
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
    fn oversized_field_name_reports_ticket_example_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![Embed::default(); 3];
        msg.embeds[2].fields = vec![Field {
            name: sized_string(257, 'n'),
            value: "v".into(),
            inline: false,
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooLong {
                path: "embeds[2].fields[0].name".into(),
                limit: 256,
                actual: 257
            })
        );
    }

    #[test]
    fn oversized_field_value_reports_value_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![Embed {
            fields: vec![Field {
                name: "n".into(),
                value: sized_string(1025, 'v'),
                inline: false,
            }],
            ..Embed::default()
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooLong {
                path: "embeds[0].fields[0].value".into(),
                limit: 1024,
                actual: 1025
            })
        );
    }

    #[test]
    fn twenty_six_fields_are_rejected_at_fields_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![Embed {
            fields: vec![
                Field {
                    name: "n".into(),
                    value: "v".into(),
                    inline: false
                };
                26
            ],
            ..Embed::default()
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooManyItems {
                path: "embeds[0].fields".into(),
                limit: 25,
                actual: 26
            })
        );
    }

    #[test]
    fn oversized_footer_text_reports_footer_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![Embed {
            footer: Some(Footer {
                text: sized_string(2049, 'f'),
                icon_url: None,
            }),
            ..Embed::default()
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooLong {
                path: "embeds[0].footer.text".into(),
                limit: 2048,
                actual: 2049
            })
        );
    }

    #[test]
    fn oversized_author_name_reports_author_path() {
        let mut msg = plain_msg();
        msg.embeds = vec![Embed {
            author: Some(crate::model::Author {
                name: sized_string(257, 'n'),
                url: None,
                icon_url: None,
            }),
            ..Embed::default()
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooLong {
                path: "embeds[0].author.name".into(),
                limit: 256,
                actual: 257
            })
        );
    }

    #[test]
    fn color_out_of_range_is_rejected_at_color_path() {
        for color in [-1, 0x100_0000] {
            let mut msg = plain_msg();
            msg.embeds = vec![Embed {
                color: Some(color),
                ..Embed::default()
            }];
            assert_eq!(
                validate(&msg),
                Err(ValidationError::OutOfRange {
                    path: "embeds[0].color".into(),
                    min: 0,
                    max: 0xFF_FFFF,
                    actual: color
                }),
                "color {color} should be rejected"
            );
        }
    }

    #[test]
    fn v2_flag_forbids_nonempty_content() {
        let mut msg = plain_msg();
        msg.flags = Some(1 << 15);
        msg.components = vec![text_display("hi")];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::ForbiddenWithComponentsV2 {
                path: "content".into()
            })
        );
    }

    #[test]
    fn v2_flag_forbids_embeds() {
        let mut msg = plain_msg();
        msg.flags = Some(1 << 15);
        msg.content.clear();
        msg.embeds = vec![Embed::default()];
        msg.components = vec![text_display("hi")];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::ForbiddenWithComponentsV2 {
                path: "embeds".into()
            })
        );
    }

    #[test]
    fn v2_flag_requires_at_least_one_component() {
        let mut msg = plain_msg();
        msg.flags = Some(1 << 15);
        msg.content.clear();
        assert_eq!(
            validate(&msg),
            Err(ValidationError::MissingComponentsForV2 {
                path: "components".into()
            })
        );
    }

    #[test]
    fn action_row_over_five_children_is_rejected_with_path() {
        let mut msg = plain_msg();
        msg.components = vec![Component::Container {
            components: vec![Component::ActionRow {
                components: (0..6).map(|_| button(1)).collect(),
            }],
            accent_color: None,
            spoiler: false,
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::ActionRowTooLarge {
                path: "components[0].components[0]".into(),
                limit: 5,
                actual: 6
            })
        );
    }

    #[test]
    fn button_style_outside_1_to_5_is_rejected() {
        let mut msg = plain_msg();
        msg.components = vec![Component::ActionRow {
            components: vec![button(0)],
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::OutOfRange {
                path: "components[0].components[0].style".into(),
                min: 1,
                max: 5,
                actual: 0
            })
        );
    }

    #[test]
    fn text_display_at_exact_char_limit_passes() {
        let msg = v2_message(vec![text_display(&sized_string(
            MAX_TEXT_DISPLAY_CHARS,
            'x',
        ))]);
        assert_eq!(validate(&msg), Ok(()));
    }

    #[test]
    fn oversized_text_display_reports_component_content_path() {
        let msg = v2_message(vec![text_display(&sized_string(
            MAX_TEXT_DISPLAY_CHARS + 1,
            'x',
        ))]);
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooLong {
                path: "components[0].content".into(),
                limit: MAX_TEXT_DISPLAY_CHARS,
                actual: MAX_TEXT_DISPLAY_CHARS + 1
            })
        );
    }

    #[test]
    fn two_text_displays_at_combined_limit_pass() {
        let msg = v2_message(vec![
            text_display(&sized_string(2000, 'a')),
            text_display(&sized_string(2000, 'b')),
        ]);
        assert_eq!(validate(&msg), Ok(()));
    }

    #[test]
    fn text_displays_over_combined_budget_are_rejected_without_single_overflow() {
        let msg = v2_message(vec![
            text_display(&sized_string(2000, 'a')),
            text_display(&sized_string(2001, 'b')),
        ]);
        assert_eq!(
            validate(&msg),
            Err(ValidationError::CombinedTextTooLong {
                limit: MAX_COMBINED_TEXT_CHARS,
                actual: MAX_COMBINED_TEXT_CHARS + 1
            })
        );
    }

    #[test]
    fn message_content_counts_toward_the_combined_text_budget() {
        let mut msg = plain_msg();
        msg.content = sized_string(1000, 'c');
        msg.components = vec![Component::Container {
            components: vec![text_display(&sized_string(3001, 't'))],
            accent_color: None,
            spoiler: false,
        }];
        assert_eq!(
            validate(&msg),
            Err(ValidationError::CombinedTextTooLong {
                limit: MAX_COMBINED_TEXT_CHARS,
                actual: MAX_COMBINED_TEXT_CHARS + 1
            })
        );
    }

    #[test]
    fn content_only_message_at_content_limit_passes_combined_check() {
        let mut msg = plain_msg();
        msg.content = sized_string(MAX_CONTENT_CHARS, 'a');
        assert_eq!(validate(&msg), Ok(()));
    }

    #[test]
    fn media_gallery_at_item_limit_passes() {
        let msg = v2_message(vec![media_gallery(MAX_GALLERY_ITEMS)]);
        assert_eq!(validate(&msg), Ok(()));
    }

    #[test]
    fn eleven_gallery_items_are_rejected_at_gallery_path() {
        let msg = v2_message(vec![media_gallery(MAX_GALLERY_ITEMS + 1)]);
        assert_eq!(
            validate(&msg),
            Err(ValidationError::TooManyItems {
                path: "components[0]".into(),
                limit: MAX_GALLERY_ITEMS,
                actual: MAX_GALLERY_ITEMS + 1
            })
        );
    }

    fn button(style: u8) -> Component {
        Component::Button {
            style,
            label: Some("b".into()),
            emoji: None,
            url: None,
            disabled: false,
        }
    }

    fn text_display(content: &str) -> Component {
        Component::TextDisplay {
            content: content.into(),
        }
    }

    fn v2_message(components: Vec<Component>) -> Message {
        Message {
            flags: Some(1 << 15),
            components,
            ..Message::default()
        }
    }

    fn media_gallery(item_count: usize) -> Component {
        Component::MediaGallery {
            items: std::iter::repeat_n(
                MediaGalleryItem {
                    media: UnfurledMediaItem {
                        url: "https://cdn.example.test/item.png".into(),
                    },
                    description: None,
                    spoiler: false,
                },
                item_count,
            )
            .collect(),
        }
    }
}
