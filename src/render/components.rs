use crate::markdown;
use crate::model::Component;
use crate::model::Emoji;
use crate::model::UnfurledMediaItem;
use crate::render::html::esc;
use crate::render::html::write_blocks;
use crate::render::html::RenderCtx;

pub(crate) fn write_components(components: &[Component], out: &mut String, ctx: &RenderCtx) {
    if components.is_empty() {
        return;
    }
    out.push_str("<div class=\"eg-components\">");
    for component in components {
        write_component(component, out, ctx);
    }
    out.push_str("</div>");
}

fn write_component(component: &Component, out: &mut String, ctx: &RenderCtx) {
    match component {
        Component::ActionRow { components } => {
            out.push_str("<div class=\"eg-action-row\">");
            for child in components {
                write_component(child, out, ctx);
            }
            out.push_str("</div>");
        }
        Component::Button {
            style,
            label,
            emoji,
            url,
            disabled,
        } => {
            let mut classes = format!("eg-btn {}", button_class(*style));
            if *disabled {
                classes.push_str(" is-disabled");
            }
            let href = url.as_deref().filter(|_| !disabled).map(|u| u.to_owned());
            match href {
                Some(href) => {
                    out.push_str(&format!(
                        "<a class=\"{classes}\" href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                        esc(&href)
                    ));
                    push_button_content(label.as_deref(), emoji.as_ref(), out);
                    out.push_str("</a>");
                }
                None => {
                    out.push_str(&format!(
                        "<button type=\"button\" class=\"{classes}\"{}>",
                        if *disabled { " disabled" } else { "" }
                    ));
                    push_button_content(label.as_deref(), emoji.as_ref(), out);
                    out.push_str("</button>");
                }
            }
        }
        Component::SelectMenu {
            placeholder,
            options,
            ..
        } => {
            let shown = placeholder
                .as_deref()
                .or_else(|| options.first().map(|o| o.label.as_str()))
                .unwrap_or("");
            out.push_str("<div class=\"eg-select\">");
            out.push_str(&format!("<span>{}</span>", esc(shown)));
            out.push_str("<span class=\"eg-select-chevron\"></span>");
            out.push_str("</div>");
        }
        Component::TextDisplay { content } => {
            out.push_str("<div class=\"eg-text-display\">");
            write_blocks(&markdown::parse(content), out, ctx);
            out.push_str("</div>");
        }
        Component::Section {
            components,
            accessory,
        } => {
            out.push_str("<div class=\"eg-section\">");
            out.push_str("<div class=\"eg-section-text\">");
            for child in components {
                write_component(child, out, ctx);
            }
            out.push_str("</div>");
            write_accessory(accessory, out, ctx);
            out.push_str("</div>");
        }
        Component::Thumbnail {
            media,
            description,
            spoiler,
        } => {
            push_media_figure(media, description.as_deref(), *spoiler, "eg-thumb", out);
        }
        Component::MediaGallery { items } => {
            let cols = items.len().clamp(1, 3);
            out.push_str(&format!(
                "<div class=\"eg-gallery\" style=\"grid-template-columns:repeat({cols},1fr)\">"
            ));
            for item in items {
                push_media_figure(
                    &item.media,
                    item.description.as_deref(),
                    item.spoiler,
                    "eg-gallery-item",
                    out,
                );
            }
            out.push_str("</div>");
        }
        Component::File { file, spoiler } => {
            let name = file_name(&file.url);
            out.push_str("<div class=\"eg-file-wrap");
            if *spoiler {
                out.push_str(" eg-has-spoiler");
            }
            out.push_str("\"><a class=\"eg-file-card\" href=\"#\"><span class=\"eg-file-name\">");
            out.push_str(&esc(&name));
            out.push_str("</span><span class=\"eg-file-download\"></span></a>");
            if *spoiler {
                out.push_str("<div class=\"spoiler-overlay\"></div>");
            }
            out.push_str("</div>");
        }
        Component::Separator { divider, spacing } => {
            let size = match spacing.unwrap_or(1) {
                1 => "sm",
                _ => "lg",
            };
            match *divider {
                false => out.push_str(&format!("<div class=\"eg-sep-spacer-{size}\"></div>")),
                true => out.push_str(&format!("<hr class=\"eg-separator eg-sep-{size}\">")),
            }
        }
        Component::Container {
            components,
            accent_color,
            spoiler,
        } => {
            out.push_str("<div class=\"eg-container");
            if *spoiler {
                out.push_str(" eg-has-spoiler");
            }
            out.push_str("\">");
            if let Some(color) = accent_color {
                out.push_str(&format!(
                    "<div class=\"eg-container-bar\" style=\"background:#{:06x}\"></div>",
                    (*color).clamp(0, 0xFF_FFFF)
                ));
            }
            out.push_str("<div class=\"eg-container-inner\">");
            for child in components {
                write_component(child, out, ctx);
            }
            out.push_str("</div>");
            if *spoiler {
                out.push_str("<div class=\"spoiler-overlay\"></div>");
            }
            out.push_str("</div>");
        }
    }
}

fn write_accessory(accessory: &Component, out: &mut String, ctx: &RenderCtx) {
    match accessory {
        Component::Thumbnail {
            media,
            description,
            spoiler,
        } => {
            out.push_str("<div class=\"eg-accessory\">");
            push_media_figure(
                media,
                description.as_deref(),
                *spoiler,
                "eg-accessory-thumbnail",
                out,
            );
            out.push_str("</div>");
        }
        other => {
            out.push_str("<div class=\"eg-accessory\">");
            write_component(other, out, ctx);
            out.push_str("</div>");
        }
    }
}

fn push_button_content(label: Option<&str>, emoji: Option<&Emoji>, out: &mut String) {
    if let Some(emoji) = emoji {
        push_emoji(emoji, out);
    }
    if let Some(label) = label {
        out.push_str(&format!("<span>{}</span>", esc(label)));
    }
}

fn push_media_figure(
    media: &UnfurledMediaItem,
    description: Option<&str>,
    spoiler: bool,
    class: &str,
    out: &mut String,
) {
    out.push_str("<figure class=\"eg-media");
    if spoiler {
        out.push_str(" eg-has-spoiler");
    }
    out.push_str("\">");
    let alt = description.unwrap_or("");
    out.push_str(&format!(
        "<img class=\"{class}\" src=\"{}\" alt=\"{}\">",
        esc(&media.url),
        esc(alt)
    ));
    if spoiler {
        out.push_str("<div class=\"spoiler-overlay\"></div>");
    }
    out.push_str("</figure>");
}

fn push_emoji(emoji: &Emoji, out: &mut String) {
    let ext = if emoji.animated { "gif" } else { "png" };
    let id = emoji.id.clone().unwrap_or_default();
    out.push_str(&format!(
        "<img class=\"emoji\" alt=\":{}:\" src=\"https://cdn.discordapp.com/emojis/{}.{ext}\">",
        esc(&emoji.name),
        esc(&id)
    ));
}

fn button_class(style: u8) -> &'static str {
    match style {
        1 => "eg-btn-primary",
        2 => "eg-btn-secondary",
        3 => "eg-btn-success",
        4 => "eg-btn-danger",
        _ => "eg-btn-link",
    }
}

fn file_name(url: &str) -> String {
    let no_fragment = url.split('#').next().unwrap_or(url);
    let no_query = no_fragment.split('?').next().unwrap_or(no_fragment);
    no_query
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("file")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MediaGalleryItem;
    use crate::model::SelectOption;

    fn render(component: &Component) -> String {
        let mut out = String::new();
        write_component(component, &mut out, &RenderCtx { now_unix: 0 });
        out
    }

    fn button(style: u8) -> Component {
        Component::Button {
            style,
            label: Some("B".into()),
            emoji: None,
            url: None,
            disabled: false,
        }
    }

    #[test]
    fn button_styles_map_to_discord_color_classes() {
        assert_eq!(
            render(&button(1)),
            "<button type=\"button\" class=\"eg-btn eg-btn-primary\"><span>B</span></button>"
        );
        assert!(render(&button(2)).contains("eg-btn-secondary"));
        assert!(render(&button(3)).contains("eg-btn-success"));
        assert!(render(&button(4)).contains("eg-btn-danger"));
        assert!(render(&button(5)).contains("eg-btn-link"));
    }

    #[test]
    fn link_button_with_url_becomes_anchor_with_external_glyph() {
        let btn = Component::Button {
            style: 5,
            label: Some("Docs".into()),
            emoji: None,
            url: Some("https://x.test/d".into()),
            disabled: false,
        };
        assert_eq!(
            render(&btn),
            concat!(
                "<a class=\"eg-btn eg-btn-link\" href=\"https://x.test/d\" ",
                "target=\"_blank\" rel=\"noopener noreferrer\"><span>Docs</span></a>"
            ),
            "the ↗ glyph comes from CSS ::after"
        );
    }

    #[test]
    fn disabled_link_button_never_renders_as_anchor() {
        let btn = Component::Button {
            style: 5,
            label: Some("Docs".into()),
            emoji: None,
            url: Some("https://x.test/d".into()),
            disabled: true,
        };
        let html = render(&btn);
        assert!(html.starts_with("<button type=\"button\""));
        assert!(html.contains("is-disabled"));
        assert!(html.contains("disabled>"));
        assert!(!html.contains("<a "));
    }

    #[test]
    fn select_menu_closed_shows_placeholder_then_first_option() {
        let menu = |placeholder: Option<&str>| Component::SelectMenu {
            placeholder: placeholder.map(str::to_owned),
            disabled: false,
            options: vec![SelectOption {
                label: "First".into(),
                description: None,
                emoji: None,
            }],
        };
        assert_eq!(
            render(&menu(Some("Pick one…"))),
            "<div class=\"eg-select\"><span>Pick one…</span><span class=\"eg-select-chevron\"></span></div>"
        );
        assert!(render(&menu(None)).contains("<span>First</span>"));
    }

    #[test]
    fn file_name_uses_url_basename_without_query_or_fragment() {
        for (url, expected) in [
            ("attachment://notes.pdf", "notes.pdf"),
            ("attachment://notes.pdf?v=2#frag", "notes.pdf"),
            ("https://cdn.example.test/a/b/img.png", "img.png"),
            ("https://cdn.example.test/", "file"),
        ] {
            assert_eq!(file_name(url), expected, "url {url}");
        }
    }

    #[test]
    fn spoilers_cover_media_files_and_containers() {
        let overlay = "<div class=\"spoiler-overlay\"></div>";
        let thumb = Component::Thumbnail {
            media: UnfurledMediaItem {
                url: "https://x.test/a.png".into(),
            },
            description: None,
            spoiler: true,
        };
        let html = render(&thumb);
        assert!(html.contains("eg-has-spoiler") && html.contains(overlay));

        let file = Component::File {
            file: UnfurledMediaItem {
                url: "attachment://f.zip".into(),
            },
            spoiler: true,
        };
        assert!(render(&file).contains(overlay));

        let container = Component::Container {
            components: vec![],
            accent_color: None,
            spoiler: true,
        };
        let html = render(&container);
        assert!(html.contains("eg-has-spoiler") && html.contains(overlay));
    }

    #[test]
    fn separator_divider_and_spacing_variants() {
        let sep = |divider: bool, spacing: Option<u8>| Component::Separator { divider, spacing };
        assert_eq!(
            render(&sep(true, Some(1))),
            "<hr class=\"eg-separator eg-sep-sm\">"
        );
        assert_eq!(
            render(&sep(true, None)),
            "<hr class=\"eg-separator eg-sep-sm\">",
            "absent spacing defaults to small"
        );
        assert_eq!(
            render(&sep(true, Some(2))),
            "<hr class=\"eg-separator eg-sep-lg\">"
        );
        assert_eq!(
            render(&sep(false, Some(1))),
            "<div class=\"eg-sep-spacer-sm\"></div>"
        );
        assert_eq!(
            render(&sep(false, Some(2))),
            "<div class=\"eg-sep-spacer-lg\"></div>",
            "divider false renders an invisible spacer"
        );
    }

    #[test]
    fn media_gallery_grid_columns_capped_at_three() {
        let gallery = |count: usize| Component::MediaGallery {
            items: (0..count)
                .map(|i| MediaGalleryItem {
                    media: UnfurledMediaItem {
                        url: format!("https://x.test/{i}.png"),
                    },
                    description: None,
                    spoiler: false,
                })
                .collect(),
        };
        assert!(render(&gallery(2)).contains("repeat(2,1fr)"));
        assert!(render(&gallery(5)).contains("repeat(3,1fr)"), "capped at 3");
    }

    #[test]
    fn text_display_parses_markdown_and_timestamps_flow_through_ctx() {
        let td = Component::TextDisplay {
            content: "# Hi <t:0:f>".into(),
        };
        let mut out = String::new();
        write_component(
            &td,
            &mut out,
            &RenderCtx {
                now_unix: 1_755_878_400,
            },
        );
        assert_eq!(
            out,
            "<div class=\"eg-text-display\"><h1>Hi <span class=\"timestamp\">January 1, 1970 12:00 AM</span></h1></div>",
            "timestamps render inline wherever markdown puts them"
        );
    }

    #[test]
    fn section_places_text_column_before_right_accessory() {
        let section = Component::Section {
            components: vec![Component::TextDisplay {
                content: "body text".into(),
            }],
            accessory: Box::new(Component::Thumbnail {
                media: UnfurledMediaItem {
                    url: "https://x.test/t.png".into(),
                },
                description: None,
                spoiler: false,
            }),
        };
        let html = render(&section);
        let text_pos = html.find("eg-section-text").expect("text column");
        let acc_pos = html.find("eg-accessory-thumbnail").expect("accessory");
        assert!(
            text_pos < acc_pos,
            "accessory must come after the text column"
        );
    }

    #[test]
    fn container_has_accent_bar_when_color_present() {
        let container = Component::Container {
            components: vec![],
            accent_color: Some(0x58_65_f2),
            spoiler: false,
        };
        let html = render(&container);
        assert!(html.contains("style=\"background:#5865f2\""));

        let bare = Component::Container {
            components: vec![],
            accent_color: None,
            spoiler: false,
        };
        assert!(!render(&bare).contains("eg-container-bar"));
    }
}
