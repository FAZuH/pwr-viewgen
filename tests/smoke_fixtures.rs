//! Builder-based fixture generators for the large-payload smoke suite
//! (spec D1): every fixture is constructed through real serenity builders,
//! serialized to JSON, and consumed by `parse_message` → render. The
//! `expected_counts` metadata pins structural facts (HTML marker, occurrence
//! count) that `smoke_golden.rs` asserts per fixture.
//!
//! Limit values come from `pwr_viewgen::validate` constants so Discord's
//! limits have a single source of truth (spec D3).

use std::borrow::Cow;
use std::time::Duration;

use pwr_viewgen::validate::MAX_ACTION_ROW_CHILDREN;
use pwr_viewgen::validate::MAX_AUTHOR_NAME_CHARS;
use pwr_viewgen::validate::MAX_CONTENT_CHARS;
use pwr_viewgen::validate::MAX_DESCRIPTION_CHARS;
use pwr_viewgen::validate::MAX_EMBEDS;
use pwr_viewgen::validate::MAX_FIELDS;
use pwr_viewgen::validate::MAX_FIELD_NAME_CHARS;
use pwr_viewgen::validate::MAX_FIELD_VALUE_CHARS;
use pwr_viewgen::validate::MAX_FOOTER_TEXT_CHARS;
use pwr_viewgen::validate::MAX_TITLE_CHARS;
use serenity::builder::CreateActionRow;
use serenity::builder::CreateButton;
use serenity::builder::CreateComponent;
use serenity::builder::CreateContainer;
use serenity::builder::CreateContainerComponent;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use serenity::builder::CreateEmbedFooter;
use serenity::builder::CreateFile;
use serenity::builder::CreateMediaGallery;
use serenity::builder::CreateMediaGalleryItem;
use serenity::builder::CreateMessage;
use serenity::builder::CreatePoll;
use serenity::builder::CreatePollAnswer;
use serenity::builder::CreateSection;
use serenity::builder::CreateSectionAccessory;
use serenity::builder::CreateSectionComponent;
use serenity::builder::CreateSelectMenu;
use serenity::builder::CreateSelectMenuKind;
use serenity::builder::CreateSelectMenuOption;
use serenity::builder::CreateSeparator;
use serenity::builder::CreateTextDisplay;
use serenity::builder::CreateThumbnail;
use serenity::builder::CreateUnfurledMediaItem;
use serenity::model::application::ButtonStyle;
use serenity::model::application::SeparatorSpacingSize;
use serenity::model::channel::MessageFlags;
use serenity::model::channel::PollLayoutType;
use serenity::model::channel::ReactionType;
use serenity::model::Colour;
use serenity::model::Timestamp;

const V2_FLAG: u16 = MessageFlags::IS_COMPONENTS_V2.bits();

// HTML markers used as structural-fact keys in `Fixture::expected_counts`.
pub const EMBED: &str = r#"<div class="eg-embed">"#;
pub const EMBED_BAR: &str = r#"<div class="eg-embed-bar""#;
pub const FIELD: &str = r#"<div class="eg-field"><div class="eg-field-name">"#;
pub const FOOTER_ICON: &str = r#"<img class="eg-footer-icon""#;
pub const CONTENT: &str = r#"<div class="eg-content">"#;
pub const ACTION_ROW: &str = r#"<div class="eg-action-row">"#;
pub const BUTTON: &str = r#"class="eg-btn "#;
pub const SELECT_PILL: &str = r#"<div class="eg-select">"#;
pub const TEXT_DISPLAY: &str = r#"<div class="eg-text-display">"#;
pub const SECTION: &str = r#"<div class="eg-section">"#;
pub const CONTAINER: &str = r#"<div class="eg-container">"#;
pub const GALLERY_ITEM: &str = r#"class="eg-gallery-item""#;
pub const FILE_CARD: &str = r#"class="eg-file-card""#;
pub const SEPARATOR_RULE: &str = r#"<hr class="eg-separator"#;
pub const SPOILER_OVERLAY: &str = r#"<div class="spoiler-overlay"></div>"#;
pub const MARKDOWN_SPOILER_SPAN: &str = r#"<span class="spoiler">"#;
pub const EMOJI_IMG: &str = r#"<img class="emoji""#;

/// A generated fixture: the payload JSON plus the structural facts a golden
/// test may assert against the rendered HTML.
pub struct Fixture {
    pub name: &'static str,
    pub json: String,
    pub expected_counts: &'static [(&'static str, usize)],
}

impl Fixture {
    /// Renders one `(marker, count)` pair into an assertion message.
    pub fn count_message(&self, marker: &str, expected: usize, actual: usize) -> String {
        format!(
            "fixture `{}` expects {expected} occurrences of {marker}, got {actual}",
            self.name
        )
    }
}

// ---- Max-limit stress class ----

/// 10 embeds at every documented embed limit simultaneously: max-length
/// titles/descriptions/footer/author, 25 max-length fields each, max content.
pub fn max_limit_embed_stress() -> Fixture {
    let timestamp = "2025-08-22T12:00:00Z"
        .parse::<Timestamp>()
        .expect("valid RFC 3339");
    let embeds: Vec<CreateEmbed> = (0..MAX_EMBEDS)
        .map(|i| {
            let mut embed = CreateEmbed::new()
                .title(sized_text(&format!("embed-{i}-title · "), MAX_TITLE_CHARS))
                .url("https://example.test/changelog")
                .description(sized_text(
                    "Lorem ipsum dolor sit amet. ",
                    MAX_DESCRIPTION_CHARS,
                ))
                .colour(Colour::from(0xFF_FFFF))
                .timestamp(timestamp)
                .author(
                    CreateEmbedAuthor::new(sized_text("author-", MAX_AUTHOR_NAME_CHARS))
                        .icon_url("https://cdn.example.test/author.png"),
                )
                .footer(
                    CreateEmbedFooter::new(sized_text("footer text · ", MAX_FOOTER_TEXT_CHARS))
                        .icon_url("https://cdn.example.test/footer.png"),
                )
                .thumbnail("https://cdn.example.test/thumb.png", None)
                .image("https://cdn.example.test/chart.png", None);
            for j in 0..MAX_FIELDS {
                embed = embed.field(
                    sized_text(&format!("e{i}f{j}·name·"), MAX_FIELD_NAME_CHARS),
                    sized_text(&format!("e{i}f{j}·value·"), MAX_FIELD_VALUE_CHARS),
                    j % 4 != 3,
                );
            }
            embed
        })
        .collect();

    let json = serialize(
        CreateMessage::new()
            .content(sized_text("stress content line. ", MAX_CONTENT_CHARS))
            .embeds(embeds),
    );

    Fixture {
        name: "max_limit_embed_stress",
        json,
        expected_counts: &[
            (CONTENT, 1),
            (EMBED_BAR, MAX_EMBEDS),
            (FOOTER_ICON, MAX_EMBEDS),
            (FIELD, MAX_EMBEDS * MAX_FIELDS),
        ],
    }
}

/// Deepest legal components-v2 nesting: container → section → thumbnail
/// accessory, with a full-width gallery, file card, separator, and a maxed
/// five-button action row inside the same container.
///
/// Discord documents no per-container children cap; the binding constraint
/// is the message-wide budget of 40 components, and this tree uses 18 of it.
pub fn deep_v2_nesting_stress() -> Fixture {
    let buttons: Vec<CreateButton> = (0..MAX_ACTION_ROW_CHILDREN)
        .map(|i| {
            let style = [
                ButtonStyle::Primary,
                ButtonStyle::Secondary,
                ButtonStyle::Success,
                ButtonStyle::Danger,
            ][i % 4];
            CreateButton::new(format!("deep:btn:{i}"))
                .label(format!("Action {i}"))
                .style(style)
                .disabled(i == MAX_ACTION_ROW_CHILDREN - 1)
        })
        .collect();

    let tree = CreateComponent::Container(
        CreateContainer::new(vec![
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                "# Deep nesting stress\nEvery v2 region in one container.",
            )),
            CreateContainerComponent::Section(CreateSection::new(
                vec![
                    CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                        "Section body **one**.",
                    )),
                    CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                        "Section body *two*.",
                    )),
                ],
                CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
                    CreateUnfurledMediaItem::new("https://cdn.example.test/deep-thumb.png"),
                )),
            )),
            CreateContainerComponent::MediaGallery(CreateMediaGallery::new(vec![
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "https://cdn.example.test/g0.png",
                ))
                .description("gallery item zero"),
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "https://cdn.example.test/g1.png",
                )),
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "https://cdn.example.test/g2.png",
                )),
            ])),
            CreateContainerComponent::File(CreateFile::new(CreateUnfurledMediaItem::new(
                "attachment://deep-nesting.pdf",
            ))),
            CreateContainerComponent::Separator(
                CreateSeparator::new()
                    .divider(true)
                    .spacing(SeparatorSpacingSize::Large),
            ),
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(buttons)),
        ])
        .accent_colour(0x58_65_F2),
    );

    let json = serialize(
        CreateMessage::new()
            .flags(MessageFlags::from_bits_truncate(V2_FLAG))
            .components(vec![tree]),
    );

    Fixture {
        name: "deep_v2_nesting_stress",
        json,
        expected_counts: &[
            (CONTAINER, 1),
            (SECTION, 1),
            (TEXT_DISPLAY, 3),
            (GALLERY_ITEM, 3),
            (FILE_CARD, 1),
            (SEPARATOR_RULE, 1),
            (ACTION_ROW, 1),
            (BUTTON, MAX_ACTION_ROW_CHILDREN),
        ],
    }
}

// ---- Realistic-large class ----

/// A release-notes announcement: two rich embeds plus link buttons and a
/// string select for older versions.
pub fn release_notes_post() -> Fixture {
    let changelog = CreateEmbed::new()
        .title("pwr-viewgen 0.4.0")
        .url("https://example.test/releases/0.4.0")
        .description("- faster markdown pass\n- components v2 regions\n- fixed footer icons")
        .colour(Colour::from(0x2E_CC_71))
        .author(CreateEmbedAuthor::new("Release Bot"))
        .footer(CreateEmbedFooter::new("released just now"))
        .field("Added", "components v2 rendering", true)
        .field("Fixed", "footer icon spacing", true)
        .field("Changed", "width flag default", true);
    let upgrade = CreateEmbed::new()
        .title("Upgrade guide")
        .description("```\ncargo install pwr-viewgen\n```")
        .colour(Colour::from(0x58_65_F2));

    let version_select = CreateSelectMenu::new(
        "release:pick-version",
        CreateSelectMenuKind::String {
            options: Cow::Owned(vec![
                CreateSelectMenuOption::new("0.4.0", "v0.4.0").default_selection(true),
                CreateSelectMenuOption::new("0.3.2", "v0.3.2"),
                CreateSelectMenuOption::new("0.3.1", "v0.3.1"),
            ]),
        },
    )
    .placeholder("Pick a version…");

    let json = serialize(
        CreateMessage::new()
            .content("New release is out! 🎉 Highlights below.")
            .embeds(vec![changelog, upgrade])
            .components(vec![
                CreateComponent::ActionRow(CreateActionRow::buttons(vec![
                    CreateButton::new_link("https://example.test/docs").label("Docs"),
                    CreateButton::new_link("https://example.test/crate").label("Crate"),
                ])),
                CreateComponent::ActionRow(CreateActionRow::select_menu(version_select)),
            ]),
    );

    Fixture {
        name: "release_notes_post",
        json,
        expected_counts: &[
            (CONTENT, 1),
            (EMBED, 2),
            (FIELD, 3),
            (BUTTON, 2),
            (SELECT_PILL, 1),
        ],
    }
}

/// A bot status panel: containers + section + text displays + media gallery +
/// separators + file cards + selects, all inside one components-v2 tree.
pub fn bot_status_panel() -> Fixture {
    let panel = CreateComponent::Container(
        CreateContainer::new(vec![
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                "# Service status\nAll regions nominal.",
            )),
            CreateContainerComponent::Section(CreateSection::new(
                vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                    "Uptime **99.98%** over the last 30 days.",
                ))],
                CreateSectionAccessory::Thumbnail(
                    CreateThumbnail::new(CreateUnfurledMediaItem::new(
                        "https://cdn.example.test/status-logo.png",
                    ))
                    .description("Status logo"),
                ),
            )),
            CreateContainerComponent::MediaGallery(CreateMediaGallery::new(
                ["latency", "errors", "traffic"]
                    .into_iter()
                    .map(|name| {
                        CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(format!(
                            "https://cdn.example.test/{name}.png"
                        )))
                    })
                    .collect::<Vec<_>>(),
            )),
            CreateContainerComponent::Separator(
                CreateSeparator::new()
                    .divider(true)
                    .spacing(SeparatorSpacingSize::Small),
            ),
            CreateContainerComponent::File(CreateFile::new(CreateUnfurledMediaItem::new(
                "attachment://postmortem.md",
            ))),
            CreateContainerComponent::Separator(
                CreateSeparator::new()
                    .divider(true)
                    .spacing(SeparatorSpacingSize::Small),
            ),
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
                CreateButton::new_link("https://status.example.test").label("Status page"),
                CreateButton::new("subscribe:alerts")
                    .label("Subscribe")
                    .style(ButtonStyle::Success),
                CreateButton::new("mute:alerts")
                    .label("Mute")
                    .style(ButtonStyle::Secondary),
            ])),
            CreateContainerComponent::ActionRow(CreateActionRow::select_menu(
                CreateSelectMenu::new(
                    "status:region-pick",
                    CreateSelectMenuKind::String {
                        options: Cow::Owned(
                            ["eu-central", "us-east", "ap-south"]
                                .into_iter()
                                .map(|region| CreateSelectMenuOption::new(region, region))
                                .collect(),
                        ),
                    },
                )
                .placeholder("Filter by region…"),
            )),
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                "-# Updated <t:1755878400:R>",
            )),
        ])
        .accent_colour(0x2E_CC_71),
    );

    let json = serialize(
        CreateMessage::new()
            .flags(MessageFlags::from_bits_truncate(V2_FLAG))
            .components(vec![panel]),
    );

    Fixture {
        name: "bot_status_panel",
        json,
        expected_counts: &[
            (CONTAINER, 1),
            (SECTION, 1),
            (TEXT_DISPLAY, 3),
            (GALLERY_ITEM, 3),
            (FILE_CARD, 1),
            (SEPARATOR_RULE, 2),
            (ACTION_ROW, 2),
            (BUTTON, 3),
            (SELECT_PILL, 1),
        ],
    }
}

/// A poll-bearing message: the poll must survive strict parsing even though
/// the render-side view does not display it.
pub fn poll_announcement() -> Fixture {
    let poll = CreatePoll::new()
        .question("Best deploy window?")
        .answers(vec![
            CreatePollAnswer::new().text("Monday morning"),
            CreatePollAnswer::new().text("Wednesday afternoon"),
            CreatePollAnswer::new().text("Friday 🎉"),
        ])
        .duration(Duration::from_secs(48 * 60 * 60))
        .layout_type(PollLayoutType::Default);

    let json = serialize(
        CreateMessage::new()
            .content("Please vote below!")
            .poll(poll),
    );

    Fixture {
        name: "poll_announcement",
        json,
        expected_counts: &[(CONTENT, 1)],
    }
}

// ---- Edge/adversarial class ----

/// Unicode storms: ZWJ emoji, regional indicators, combining marks, CJK and
/// RTL overrides across content, button emoji, and select options.
pub fn zwj_emoji_storm() -> Fixture {
    let json = serialize(
        CreateMessage::new()
            .content(concat!(
                "family: 👨‍👩‍👧‍👦 flag: 🏳️‍🌈 cafe\u{301}\u{202E}gnitirw-rtl\u{202C} ",
                "日本語テキスト custom: <:tada:7>",
            ))
            .components(vec![CreateComponent::ActionRow(CreateActionRow::buttons(
                vec![CreateButton::new("react:family").label("Family").emoji(
                    ReactionType::Unicode("👨‍👩‍👧‍👦".to_string().try_into().expect("non-empty")),
                )],
            ))]),
    );

    Fixture {
        name: "zwj_emoji_storm",
        json,
        expected_counts: &[(CONTENT, 1), (ACTION_ROW, 1), (EMOJI_IMG, 2)],
    }
}

/// Every spoilerable region spoiled at once: container, accessory thumbnail,
/// both gallery items, both file cards — plus a ||markdown|| spoiler span.
pub fn spoiler_everything() -> Fixture {
    let tree = CreateComponent::Container(
        CreateContainer::new(vec![
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                "plot twist: ||the butler did it||",
            )),
            CreateContainerComponent::Section(CreateSection::new(
                vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                    "spoilered evidence below.",
                ))],
                CreateSectionAccessory::Thumbnail(
                    CreateThumbnail::new(CreateUnfurledMediaItem::new(
                        "https://cdn.example.test/evidence.png",
                    ))
                    .spoiler(true),
                ),
            )),
            CreateContainerComponent::MediaGallery(CreateMediaGallery::new(vec![
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "https://cdn.example.test/s0.png",
                ))
                .spoiler(true),
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    "https://cdn.example.test/s1.png",
                ))
                .spoiler(true),
            ])),
            CreateContainerComponent::File(
                CreateFile::new(CreateUnfurledMediaItem::new("attachment://ending.zip"))
                    .spoiler(true),
            ),
        ])
        .accent_colour(0x88_00_00)
        .spoiler(true),
    );
    let top_file = CreateComponent::File(
        CreateFile::new(CreateUnfurledMediaItem::new("attachment://alt.zip")).spoiler(true),
    );

    let json = serialize(
        CreateMessage::new()
            .flags(MessageFlags::from_bits_truncate(V2_FLAG))
            .components(vec![tree, top_file]),
    );

    Fixture {
        name: "spoiler_everything",
        json,
        expected_counts: &[(SPOILER_OVERLAY, 6), (MARKDOWN_SPOILER_SPAN, 1)],
    }
}

/// Legal minimums everywhere: empty text display, empty section, empty
/// unaccented container, divider-less separator, single bare gallery item.
pub fn legal_minimum_empties() -> Fixture {
    let json = serialize(
        CreateMessage::new()
            .flags(MessageFlags::from_bits_truncate(V2_FLAG))
            .components(vec![
                CreateComponent::TextDisplay(CreateTextDisplay::new("")),
                CreateComponent::Separator(CreateSeparator::new()),
                CreateComponent::Section(CreateSection::new(
                    vec![],
                    CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
                        CreateUnfurledMediaItem::new("https://cdn.example.test/min.png"),
                    )),
                )),
                CreateComponent::MediaGallery(CreateMediaGallery::new(vec![
                    CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                        "https://cdn.example.test/only.png",
                    )),
                ])),
                CreateComponent::Container(CreateContainer::new(vec![])),
            ]),
    );

    Fixture {
        name: "legal_minimum_empties",
        json,
        expected_counts: &[
            (TEXT_DISPLAY, 1),
            (SECTION, 1),
            (CONTAINER, 1),
            (GALLERY_ITEM, 1),
        ],
    }
}

/// v1-style action rows living inside a components-v2 tree, at the top level
/// and nested in a container.
pub fn mixed_v1_rows_in_v2_tree() -> Fixture {
    let row = CreateActionRow::buttons(vec![
        CreateButton::new("mixed:ack")
            .label("Acknowledge")
            .style(ButtonStyle::Success),
        CreateButton::new_link("https://example.test/mixed").label("Details"),
    ]);
    let select_row = CreateActionRow::select_menu(CreateSelectMenu::new(
        "mixed:pick",
        CreateSelectMenuKind::String {
            options: Cow::Owned(vec![CreateSelectMenuOption::new("One", "one")]),
        },
    ));

    let json = serialize(
        CreateMessage::new()
            .flags(MessageFlags::from_bits_truncate(V2_FLAG))
            .components(vec![
                CreateComponent::TextDisplay(CreateTextDisplay::new("Mixed tree.")),
                CreateComponent::ActionRow(row),
                CreateComponent::Container(CreateContainer::new(vec![
                    CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
                        "Inside the container.",
                    )),
                    CreateContainerComponent::ActionRow(select_row),
                ])),
            ]),
    );

    Fixture {
        name: "mixed_v1_rows_in_v2_tree",
        json,
        expected_counts: &[
            (TEXT_DISPLAY, 2),
            (ACTION_ROW, 2),
            (BUTTON, 2),
            (SELECT_PILL, 1),
            (CONTAINER, 1),
        ],
    }
}

/// Flag bits serenity's `MessageFlags` does not define, spliced into the
/// serialized builder output (builders cannot express them). Canonicalization
/// must drop exactly those bits; see the golden assertion on the canonical
/// flags value. Bits 9–11 are unnamed in upstream's u16 bitflags.
pub fn unknown_flag_bits() -> Fixture {
    let mut json = unknown_flag_bits_before_splice();
    splice_flags(&mut json, (1 << 9) | (1 << 10) | (1 << 11));

    Fixture {
        name: "unknown_flag_bits",
        json,
        expected_counts: &[(TEXT_DISPLAY, 1), (CONTENT, 0)],
    }
}

/// The same builder output before the unknown-bit splice, for golden
/// comparisons proving the extra bits do not affect rendering.
pub fn unknown_flag_bits_before_splice() -> String {
    serialize(
        CreateMessage::new()
            .flags(MessageFlags::IS_COMPONENTS_V2 | MessageFlags::SUPPRESS_EMBEDS)
            .components(vec![CreateComponent::TextDisplay(CreateTextDisplay::new(
                "flags probe.",
            ))]),
    )
}

fn splice_flags(json: &mut String, extra_bits: u16) {
    use serde_json::Value;

    let mut value: Value = serde_json::from_str(json).expect("serialized builder is valid JSON");
    let flags = value
        .get("flags")
        .and_then(Value::as_u64)
        .expect("builder serialized a flags number");
    value["flags"] = serde_json::to_value(flags | u64::from(extra_bits)).expect("number");
    *json = value.to_string();
}

/// Every fixture, grouped by spec class.
pub fn all_fixtures() -> Vec<Fixture> {
    vec![
        max_limit_embed_stress(),
        deep_v2_nesting_stress(),
        release_notes_post(),
        bot_status_panel(),
        poll_announcement(),
        zwj_emoji_storm(),
        spoiler_everything(),
        legal_minimum_empties(),
        mixed_v1_rows_in_v2_tree(),
        unknown_flag_bits(),
    ]
}

fn serialize(message: CreateMessage<'static>) -> String {
    serde_json::to_string(&message).expect("serenity builder serializes to canonical JSON")
}

fn sized_text(pattern: &str, chars: usize) -> String {
    let mut out = String::with_capacity(chars * pattern.len());
    while out.chars().count() < chars {
        let remaining = chars - out.chars().count();
        out.extend(pattern.chars().take(remaining));
    }
    out
}

#[cfg(test)]
mod tests {
    use pwr_viewgen::model::parse_message;
    use pwr_viewgen::model::ParsedMessage;
    use pwr_viewgen::validate;

    use super::all_fixtures;
    use super::unknown_flag_bits;

    #[test]
    fn every_generated_fixture_parses_through_the_public_pipeline() {
        for fixture in all_fixtures() {
            parse_message(&fixture.json)
                .unwrap_or_else(|error| panic!("fixture `{}` must parse: {error}", fixture.name));
        }
    }

    #[test]
    fn every_generated_fixture_passes_validation_at_the_public_limits() {
        for fixture in all_fixtures() {
            let message = parse_message(&fixture.json).expect("fixture parses").message;
            assert_eq!(
                validate::validate(&message),
                Ok(()),
                "fixture `{}` must respect validate.rs limits",
                fixture.name
            );
        }
    }

    #[test]
    fn unknown_flag_bits_normalize_to_known_bits_after_canonicalization() {
        let fixture = unknown_flag_bits();
        let parsed: ParsedMessage = fixture.json.parse().expect("fixture parses");
        assert_eq!(parsed.message.flags, Some((1 << 15) | (1 << 2)));
        assert_eq!(
            parsed
                .canonical
                .get("flags")
                .and_then(serde_json::Value::as_i64),
            Some((1 << 15) | (1 << 2)),
            "canonical payload must carry only bits defined on MessageFlags"
        );
    }
}
