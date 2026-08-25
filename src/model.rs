use pwr_ext::prelude::CreateMessageDe;
use serde::Deserialize;

/// A message-parsing failure, either from malformed JSON or from payload
/// shapes upstream serenity builders cannot represent (spec D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parses webhook-execution JSON into a [`ParsedMessage`] — the render-side
/// [`Message`] view together with the canonical body it was loaded from.
///
/// Pipeline per spec D1: `str -> Value -> pwr-ext wrappers (strict,
/// upstream-canonical) -> serde_json::to_value -> Message::from_value`.
/// Unknown component types, invalid select-menu kinds, and other
/// non-builder-shaped payloads error here before any rendering happens.
pub fn parse_message(json: &str) -> Result<ParsedMessage, ParseError> {
    json.parse::<ParsedMessage>()
}

/// A parsed message together with the canonical payload the view was loaded
/// from.
///
/// The webhook send path forwards `canonical` verbatim (spec D3), so
/// non-rendered wire fields — button `custom_id`, select-menu kinds, option
/// values, `min_values`/`max_values` — survive the trip even though the
/// render-side [`Message`] view drops them.
#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub message: Message,
    pub canonical: serde_json::Value,
}

impl std::str::FromStr for ParsedMessage {
    type Err = ParseError;

    fn from_str(json: &str) -> Result<Self, ParseError> {
        let raw: serde_json::Value =
            serde_json::from_str(json).map_err(|error| ParseError(error.to_string()))?;
        message_from_value(raw)
    }
}

impl ParsedMessage {
    pub fn from_value(raw: &serde_json::Value) -> Result<Self, ParseError> {
        message_from_value(raw.clone())
    }
}

fn message_from_value(mut raw: serde_json::Value) -> Result<ParsedMessage, ParseError> {
    let object = raw
        .as_object_mut()
        .ok_or_else(|| ParseError("expected a JSON object".to_owned()))?;

    // Webhook-execution identity fields live beside the message body; serenity's
    // `CreateMessage` does not carry them, so keep them out of strict parsing
    // and splice them back after canonicalization.
    let username = object.remove("username");
    let avatar_url = object.remove("avatar_url");

    // Required by the upstream builder shape but optional in Discord's
    // webhook API; absent means false.
    object
        .entry("tts".to_owned())
        .or_insert(serde_json::Value::Bool(false));
    object
        .entry("enforce_nonce".to_owned())
        .or_insert(serde_json::Value::Bool(false));
    if let Some(embeds) = object
        .get_mut("embeds")
        .and_then(serde_json::Value::as_array_mut)
    {
        for embed in embeds {
            let Some(fields) = embed
                .get_mut("fields")
                .and_then(serde_json::Value::as_array_mut)
            else {
                continue;
            };
            for field in fields {
                if let Some(field) = field.as_object_mut() {
                    field
                        .entry("inline".to_owned())
                        .or_insert(serde_json::Value::Bool(false));
                }
            }
        }
    }

    let wrapper: CreateMessageDe =
        serde_json::from_value(raw).map_err(|error| ParseError(format!("parse: {error}")))?;
    let mut canonical = wrapper
        .into_canonical_value()
        .map_err(|error| ParseError(format!("parse: {error}")))?;

    if let Some(canonical) = canonical.as_object_mut() {
        if let Some(username) = username {
            canonical.insert("username".to_owned(), username);
        }
        if let Some(avatar_url) = avatar_url {
            canonical.insert("avatar_url".to_owned(), avatar_url);
        }
    }

    let message: Message =
        serde_json::from_value(canonical.clone()).map_err(|error| ParseError(error.to_string()))?;
    Ok(ParsedMessage { message, canonical })
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub tts: bool,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub flags: Option<i64>,
    #[serde(default)]
    pub components: Vec<Component>,
}

impl Message {
    /// True when the payload carries the components v2 flag (bit 15).
    pub fn is_components_v2(&self) -> bool {
        self.flags.is_some_and(|flags| flags & (1 << 15) != 0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Embed {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub color: Option<i64>,
    #[serde(default)]
    pub footer: Option<Footer>,
    #[serde(default)]
    pub image: Option<EmbedImage>,
    #[serde(default)]
    pub thumbnail: Option<EmbedImage>,
    #[serde(default)]
    pub author: Option<Author>,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Footer {
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct EmbedImage {
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Field {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Emoji {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub animated: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct SelectOption {
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub emoji: Option<Emoji>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct UnfurledMediaItem {
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct MediaGalleryItem {
    pub media: UnfurledMediaItem,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub spoiler: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Component {
    ActionRow {
        components: Vec<Component>,
    },
    Button {
        style: u8,
        label: Option<String>,
        emoji: Option<Emoji>,
        url: Option<String>,
        disabled: bool,
    },
    SelectMenu {
        kind: u8,
        placeholder: Option<String>,
        disabled: bool,
        options: Vec<SelectOption>,
    },
    TextDisplay {
        content: String,
    },
    Section {
        components: Vec<Component>,
        accessory: Box<Component>,
    },
    Thumbnail {
        media: UnfurledMediaItem,
        description: Option<String>,
        spoiler: bool,
    },
    MediaGallery {
        items: Vec<MediaGalleryItem>,
    },
    File {
        file: UnfurledMediaItem,
        spoiler: bool,
    },
    Separator {
        divider: bool,
        spacing: Option<u8>,
    },
    Container {
        components: Vec<Component>,
        accent_color: Option<i64>,
        spoiler: bool,
    },
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::from_json(value).map_err(serde::de::Error::custom)
    }
}

impl Component {
    fn from_json(value: serde_json::Value) -> Result<Self, String> {
        let ty = value
            .get("type")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| "missing numeric \"type\" field on component".to_string())?;
        match ty {
            1 => parse_body::<ActionRowBody>(value).map(|b| Component::ActionRow {
                components: b.components,
            }),
            2 => parse_body::<ButtonBody>(value).map(|b| Component::Button {
                style: b.style,
                label: b.label,
                emoji: b.emoji,
                url: b.url,
                disabled: b.disabled,
            }),
            3 | 5 | 6 | 7 | 8 => {
                parse_body::<SelectMenuBody>(value).map(|b| Component::SelectMenu {
                    kind: ty as u8,
                    placeholder: b.placeholder,
                    disabled: b.disabled,
                    options: b.options,
                })
            }
            9 => parse_body::<SectionBody>(value).map(|b| Component::Section {
                components: b.components,
                accessory: b.accessory,
            }),
            10 => parse_body::<TextDisplayBody>(value)
                .map(|b| Component::TextDisplay { content: b.content }),
            11 => parse_body::<ThumbnailBody>(value).map(|b| Component::Thumbnail {
                media: b.media,
                description: b.description,
                spoiler: b.spoiler,
            }),
            12 => parse_body::<MediaGalleryBody>(value)
                .map(|b| Component::MediaGallery { items: b.items }),
            13 => parse_body::<FileBody>(value).map(|b| Component::File {
                file: b.file,
                spoiler: b.spoiler,
            }),
            14 => parse_body::<SeparatorBody>(value).map(|b| Component::Separator {
                divider: b.divider,
                spacing: b.spacing,
            }),
            17 => parse_body::<ContainerBody>(value).map(|b| Component::Container {
                components: b.components,
                accent_color: b.accent_color,
                spoiler: b.spoiler,
            }),
            other => Err(format!("unknown component type {other}")),
        }
    }
}

fn parse_body<B: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<B, String> {
    serde_json::from_value(value).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct ActionRowBody {
    #[serde(default)]
    components: Vec<Component>,
}

#[derive(Deserialize)]
struct ButtonBody {
    style: u8,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    emoji: Option<Emoji>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    disabled: bool,
}

#[derive(Deserialize)]
struct SelectMenuBody {
    #[serde(default)]
    placeholder: Option<String>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    options: Vec<SelectOption>,
}

#[derive(Deserialize)]
struct TextDisplayBody {
    content: String,
}

#[derive(Deserialize)]
struct SectionBody {
    #[serde(default)]
    components: Vec<Component>,
    accessory: Box<Component>,
}

#[derive(Deserialize)]
struct ThumbnailBody {
    media: UnfurledMediaItem,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    spoiler: bool,
}

#[derive(Deserialize)]
struct MediaGalleryBody {
    items: Vec<MediaGalleryItem>,
}

#[derive(Deserialize)]
struct FileBody {
    file: UnfurledMediaItem,
    #[serde(default)]
    spoiler: bool,
}

#[derive(Deserialize)]
struct SeparatorBody {
    #[serde(default)]
    divider: bool,
    #[serde(default)]
    spacing: Option<u8>,
}

#[derive(Deserialize)]
struct ContainerBody {
    #[serde(default)]
    components: Vec<Component>,
    #[serde(default)]
    accent_color: Option<i64>,
    #[serde(default)]
    spoiler: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_content_object_parses_with_defaults() {
        let msg = parse_message(r#"{"content": "hello world"}"#).expect("bare payload parses").message;
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.username, None);
        assert_eq!(msg.avatar_url, None);
        assert!(!msg.tts);
        assert!(msg.embeds.is_empty());
        assert_eq!(msg.flags, None);
        assert!(msg.components.is_empty());
    }

    #[test]
    fn full_webhook_payload_parses_ignoring_unknown_fields() {
        let raw = r#"{
            "id": "9999",
            "channel_id": "1234",
            "webhook_id": "42",
            "content": "with embed",
            "username": "Notifier",
            "avatar_url": "https://cdn.example.test/a.png",
            "tts": true,
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
                    "author": { "name": "Ada", "url": "https://example.test/~ada", "icon_url": "https://cdn.example.test/ada.png" },
                    "fields": [
                        { "name": "Branch", "value": "main", "inline": true },
                        { "name": "Duration", "value": "3m 12s" }
                    ]
                }
            ]
        }"#;
        let msg = parse_message(raw).expect("full payload parses").message;

        assert_eq!(msg.username.as_deref(), Some("Notifier"));
        assert!(msg.tts);
        assert_eq!(msg.flags, Some(0));
        let [embed] = msg.embeds.as_slice() else {
            panic!("expected exactly one embed");
        };
        assert_eq!(embed.title.as_deref(), Some("Deploy finished"));
        assert_eq!(embed.description.as_deref(), Some("All systems nominal"));
        assert_eq!(embed.url.as_deref(), Some("https://example.test/deploys/7"));
        // canonicalized through the builder (".000" milliseconds dropped); renders identically
        assert_eq!(embed.timestamp.as_deref(), Some("2026-08-22T12:00:00Z"));
        assert_eq!(embed.color, Some(3066993));
        assert_eq!(
            embed.footer.as_ref().map(|f| f.text.as_str()),
            Some("CI bot")
        );
        assert_eq!(
            embed.footer.as_ref().and_then(|f| f.icon_url.as_deref()),
            Some("https://cdn.example.test/ci.png")
        );
        assert_eq!(
            embed.image.as_ref().map(|i| i.url.as_str()),
            Some("https://cdn.example.test/chart.png")
        );
        assert_eq!(
            embed.thumbnail.as_ref().map(|i| i.url.as_str()),
            Some("https://cdn.example.test/logo.png")
        );
        let author = embed.author.as_ref().expect("author present");
        assert_eq!(author.name, "Ada");
        assert_eq!(author.url.as_deref(), Some("https://example.test/~ada"));
        assert_eq!(
            author.icon_url.as_deref(),
            Some("https://cdn.example.test/ada.png")
        );
        assert_eq!(embed.fields.len(), 2);
        assert_eq!(embed.fields[0].name, "Branch");
        assert_eq!(embed.fields[0].value, "main");
        assert!(embed.fields[0].inline);
        assert!(!embed.fields[1].inline);
    }

    #[test]
    fn components_v1_action_row_button_and_select_parse() {
        let raw = r#"{
            "content": "Pick one",
            "components": [
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 2,
                            "style": 5,
                            "label": "Docs",
                            "emoji": { "id": null, "name": "📚", "animated": false },
                            "url": "https://example.test/docs",
                            "disabled": false
                        },
                        {
                            "type": 2,
                            "style": 1,
                            "label": "Danger zone",
                            "disabled": true
                        }
                    ]
                },
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 3,
                            "custom_id": "color_pick",
                            "placeholder": "Choose a color…",
                            "options": [
                                { "label": "Red", "value": "red", "description": "The loud one", "emoji": { "name": "🔴" } },
                                { "label": "Blue", "value": "blue" }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let msg = parse_message(raw).expect("v1 components parse").message;

        let [row, menu] = msg.components.as_slice() else {
            panic!("expected action row + select menu");
        };
        let Component::ActionRow { components } = row else {
            panic!("first component should be an action row");
        };
        let [button_danger, button_link] = components.as_slice() else {
            panic!("expected two buttons");
        };
        let Component::Button {
            style,
            label,
            emoji,
            url,
            disabled,
        } = button_danger
        else {
            panic!("expected button");
        };
        assert_eq!(*style, 5);
        assert_eq!(label.as_deref(), Some("Docs"));
        assert_eq!(emoji.as_ref().map(|e| e.name.as_str()), Some("📚"));
        assert_eq!(url.as_deref(), Some("https://example.test/docs"));
        assert!(!disabled);

        let Component::Button {
            style, disabled, ..
        } = button_link
        else {
            panic!("expected button");
        };
        assert_eq!(*style, 1);
        assert!(*disabled);

        let Component::ActionRow {
            components: menu_row,
        } = menu
        else {
            panic!("second component should be a select-menu action row");
        };
        let [Component::SelectMenu {
            placeholder,
            options,
            ..
        }] = menu_row.as_slice()
        else {
            panic!("row should hold one select menu");
        };
        assert_eq!(placeholder.as_deref(), Some("Choose a color…"));
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Red");
        assert_eq!(options[0].description.as_deref(), Some("The loud one"));
        assert_eq!(options[0].emoji.as_ref().unwrap().name, "🔴");
        assert_eq!(options[1].label, "Blue");
        assert!(options[1].description.is_none());
    }

    #[test]
    fn components_v2_payload_parses_all_numeric_types() {
        let raw = r##"{
            "flags": 32768,
            "components": [
                { "type": 10, "content": "# Welcome\nplain text display" },
                {
                    "type": 9,
                    "components": [{ "type": 10, "content": "section body" }],
                    "accessory": {
                        "type": 11,
                        "media": { "url": "https://cdn.example.test/thumb.png" },
                        "description": "a thumbnail",
                        "spoiler": true
                    }
                },
                {
                    "type": 12,
                    "items": [
                        { "media": { "url": "https://cdn.example.test/a.png" } },
                        {
                            "media": { "url": "https://cdn.example.test/b.png" },
                            "description": "second",
                            "spoiler": true
                        }
                    ]
                },
                { "type": 13, "file": { "url": "attachment://notes.pdf" }, "spoiler": true },
                { "type": 14, "divider": true, "spacing": 2 },
                {
                    "type": 17,
                    "accent_color": 8912896,
                    "spoiler": false,
                    "components": [
                        { "type": 10, "content": "inside container" },
                        {
                            "type": 1,
                            "components": [
                                { "type": 2, "style": 3, "label": "Inner button" }
                            ]
                        }
                    ]
                }
            ]
        }"##;
        let msg = parse_message(raw).expect("v2 components parse").message;

        assert_eq!(msg.flags, Some(1 << 15));
        assert_eq!(
            msg.components.len(),
            6,
            "text display, section, gallery, file, separator, container"
        );
        assert!(matches!(
            msg.components[0],
            Component::TextDisplay { ref content } if content.contains("Welcome")
        ));
        let Component::Section {
            components,
            accessory,
        } = &msg.components[1]
        else {
            panic!("expected section");
        };
        assert!(matches!(
            components.as_slice(),
            [Component::TextDisplay { .. }]
        ));
        assert!(
            matches!(accessory.as_ref(), Component::Thumbnail { media, spoiler: true, .. }
            if media.url == "https://cdn.example.test/thumb.png")
        );
        let Component::MediaGallery { items } = &msg.components[2] else {
            panic!("expected media gallery");
        };
        assert_eq!(items.len(), 2);
        assert!(items[0].description.is_none() && !items[0].spoiler);
        assert_eq!(items[1].description.as_deref(), Some("second"));
        assert!(items[1].spoiler);
        assert!(
            matches!(msg.components[3], Component::File { ref file, spoiler: true }
            if file.url == "attachment://notes.pdf")
        );
        assert!(matches!(
            msg.components[4],
            Component::Separator {
                divider: true,
                spacing: Some(2)
            }
        ));
        let Component::Container {
            components,
            accent_color,
            spoiler,
        } = &msg.components[5]
        else {
            panic!("expected container");
        };
        assert_eq!(*accent_color, Some(8912896));
        assert!(!spoiler);
        assert_eq!(components.len(), 2);
        assert!(matches!(components[1], Component::ActionRow { .. }));
    }

    #[test]
    fn unknown_component_type_is_rejected() {
        let raw = r#"{"content":"x","components":[{"type":42,"content":"nope"}]}"#;
        let err = parse_message(raw).expect_err("unknown type must fail");
        assert!(
            err.to_string().contains("component type"),
            "error should mention component type, got: {err}"
        );
    }

    #[test]
    fn component_missing_type_field_is_rejected() {
        let raw = r#"{"content":"x","components":[{"content":"no type here"}]}"#;
        let err = parse_message(raw).expect_err("missing type must fail");
        assert!(
            err.to_string().contains("\"type\""),
            "error should mention the missing type field, got: {err}"
        );
    }
}
