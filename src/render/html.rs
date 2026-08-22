use crate::markdown::ir::Block;
use crate::markdown::ir::MentionKind;
use crate::markdown::ir::Span;
use crate::markdown::timestamp;
use crate::model::Embed;
use crate::model::Field;

pub(crate) struct RenderCtx {
    pub now_unix: i64,
}

pub(crate) fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(crate) fn write_blocks(blocks: &[Block], out: &mut String, ctx: &RenderCtx) {
    for block in blocks {
        write_block(block, out, ctx);
    }
}

fn write_block(block: &Block, out: &mut String, ctx: &RenderCtx) {
    match block {
        Block::Para(spans) => {
            out.push_str("<p>");
            write_spans(spans, out, ctx);
            out.push_str("</p>");
        }
        Block::Quote { wide, blocks } => {
            if *wide {
                out.push_str("<blockquote class=\"eg-quote-wide\">");
            } else {
                out.push_str("<blockquote class=\"eg-quote\">");
            }
            write_blocks(blocks, out, ctx);
            out.push_str("</blockquote>");
        }
        Block::Code { lang, body } => {
            out.push_str("<pre><code");
            if !lang.is_empty() {
                out.push_str(&format!(" class=\"language-{}\"", esc(lang)));
            }
            out.push('>');
            out.push_str(&esc(body));
            out.push_str("</code></pre>");
        }
        Block::Heading(level, spans) => {
            let level = (*level).clamp(1, 3);
            out.push_str(&format!("<h{level}>"));
            write_spans(spans, out, ctx);
            out.push_str(&format!("</h{level}>"));
        }
        Block::Subtext(spans) => {
            out.push_str("<div class=\"eg-subtext\">");
            write_spans(spans, out, ctx);
            out.push_str("</div>");
        }
        Block::List { ordered, items } => {
            out.push_str(if *ordered { "<ol>" } else { "<ul>" });
            for item in items {
                out.push_str("<li>");
                write_spans(item, out, ctx);
                out.push_str("</li>");
            }
            out.push_str(if *ordered { "</ol>" } else { "</ul>" });
        }
    }
}

pub(crate) fn write_spans(spans: &[Span], out: &mut String, ctx: &RenderCtx) {
    for span in spans {
        write_span(span, out, ctx);
    }
}

fn write_span(span: &Span, out: &mut String, ctx: &RenderCtx) {
    match span {
        Span::Text(text) => out.push_str(&esc(text)),
        Span::Bold(inner) => wrap(out, "strong", inner, ctx),
        Span::Italic(inner) => wrap(out, "em", inner, ctx),
        Span::Underline(inner) => wrap(out, "u", inner, ctx),
        Span::Strike(inner) => wrap(out, "s", inner, ctx),
        Span::Spoiler(inner) => wrap_class(out, "spoiler", inner, ctx),
        Span::Code(body) => {
            out.push_str("<code>");
            out.push_str(&esc(body));
            out.push_str("</code>");
        }
        Span::Link { url, label } => {
            out.push_str(&format!(
                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                esc(url)
            ));
            write_spans(label, out, ctx);
            out.push_str("</a>");
        }
        Span::Autolink(url) => out.push_str(&format!(
            "<a href=\"{0}\" target=\"_blank\" rel=\"noopener noreferrer\">{0}</a>",
            esc(url)
        )),
        Span::Emoji { id, name, animated } => {
            let ext = if *animated { "gif" } else { "png" };
            out.push_str(&format!(
                "<img class=\"emoji\" alt=\":{}:\" src=\"https://cdn.discordapp.com/emojis/{}.{ext}\">",
                esc(name),
                esc(id)
            ));
        }
        Span::Mention(kind) => {
            let label = match kind {
                MentionKind::User(id) | MentionKind::Role(id) => format!("@{}", esc(id)),
                MentionKind::Channel(id) => format!("#{}", esc(id)),
                MentionKind::Everyone => "@everyone".to_owned(),
                MentionKind::Here => "@here".to_owned(),
            };
            out.push_str(&format!("<span class=\"mention\">{label}</span>"));
        }
        Span::Timestamp { unix, style } => {
            let formatted = timestamp::fmt_with(*unix, *style, ctx.now_unix);
            out.push_str(&format!(
                "<span class=\"timestamp\">{}</span>",
                esc(&formatted)
            ));
        }
    }
}

fn wrap(out: &mut String, tag: &str, inner: &[Span], ctx: &RenderCtx) {
    out.push('<');
    out.push_str(tag);
    out.push('>');
    write_spans(inner, out, ctx);
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

fn wrap_class(out: &mut String, class: &str, inner: &[Span], ctx: &RenderCtx) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    write_spans(inner, out, ctx);
    out.push_str("</span>");
}

pub(crate) fn write_embed(embed: &Embed, out: &mut String, ctx: &RenderCtx) {
    out.push_str("<div class=\"eg-embed\">");
    if let Some(color) = embed.color {
        let hex = color.clamp(0, 0xFF_FFFF);
        out.push_str(&format!(
            "<div class=\"eg-embed-bar\" style=\"background:#{hex:06x}\"></div>"
        ));
    }
    out.push_str("<div class=\"eg-embed-inner\">");

    if let Some(thumbnail) = &embed.thumbnail {
        out.push_str(&format!(
            "<img class=\"eg-thumbnail\" src=\"{}\" alt=\"\">",
            esc(&thumbnail.url)
        ));
    }

    if let Some(author) = &embed.author {
        out.push_str("<div class=\"eg-author-row\">");
        if let Some(icon_url) = &author.icon_url {
            out.push_str(&format!(
                "<img class=\"eg-author-icon\" src=\"{}\" alt=\"\">",
                esc(icon_url)
            ));
        }
        out.push_str(&format!(
            "<span class=\"eg-author-name\">{}</span>",
            esc(&author.name)
        ));
        out.push_str("</div>");
    }

    if let Some(title) = &embed.title {
        out.push_str("<div class=\"eg-title\">");
        match embed.url.as_deref() {
            Some(link_url) => {
                out.push_str(&format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                    esc(link_url)
                ));
                write_blocks(&crate::markdown::parse(title), out, ctx);
                out.push_str("</a>");
            }
            None => write_blocks(&crate::markdown::parse(title), out, ctx),
        }
        out.push_str("</div>");
    }

    if let Some(description) = &embed.description {
        out.push_str("<div class=\"eg-description\">");
        write_blocks(&crate::markdown::parse(description), out, ctx);
        out.push_str("</div>");
    }

    if !embed.fields.is_empty() {
        render_fields(&embed.fields, out);
    }

    if let Some(image) = &embed.image {
        out.push_str(&format!(
            "<img class=\"eg-image\" src=\"{}\" alt=\"\">",
            esc(&image.url)
        ));
    }

    let footer_ts = embed.timestamp.clone();
    if footer_ts.is_some() || embed.footer.is_some() {
        out.push_str("<div class=\"eg-footer\">");
        if let Some(footer) = &embed.footer {
            if let Some(icon_url) = &footer.icon_url {
                out.push_str(&format!(
                    "<img class=\"eg-footer-icon\" src=\"{}\" alt=\"\">",
                    esc(icon_url)
                ));
            }
            out.push_str(&format!("<span>{}</span>", esc(&footer.text)));
        }
        if let Some(ts) = &footer_ts {
            let formatted = fmt_embed_timestamp(ts, ctx);
            out.push_str(&format!("<span>&bull; {}</span>", esc(&formatted)));
        }
        out.push_str("</div>");
    }

    out.push_str("</div></div>");
}

fn fmt_embed_timestamp(ts: &str, ctx: &RenderCtx) -> String {
    match chrono::DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => timestamp::fmt_with(dt.timestamp(), 'f', ctx.now_unix),
        Err(_) => ts.to_owned(),
    }
}

fn render_fields(fields: &[Field], out: &mut String) {
    out.push_str("<div class=\"eg-fields\">");
    let mut i = 0;
    while i < fields.len() {
        let run_end = if fields[i].inline {
            let mut j = i + 1;
            while j < fields.len() && j - i < 3 && fields[j].inline {
                j += 1;
            }
            j
        } else {
            i + 1
        };
        let cols = run_end - i;
        out.push_str(&format!(
            "<div class=\"eg-field-group\" style=\"grid-template-columns:repeat({cols},1fr)\">"
        ));
        for field in &fields[i..run_end] {
            push_field(field, out);
        }
        out.push_str("</div>");
        i = run_end;
    }
    out.push_str("</div>");
}

fn push_field(field: &Field, out: &mut String) {
    out.push_str("<div class=\"eg-field\"><div class=\"eg-field-name\">");
    out.push_str(&esc(&field.name));
    out.push_str("</div><div class=\"eg-field-value\">");
    out.push_str(&esc(&field.value));
    out.push_str("</div></div>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;

    const NOW: i64 = 1_755_878_400;

    fn render_inline(input: &str) -> String {
        let blocks = parse(input);
        let mut out = String::new();
        write_blocks(&blocks, &mut out, &RenderCtx { now_unix: NOW });
        out
    }

    #[test]
    fn strict_escaping_covers_all_five_characters() {
        assert_eq!(esc("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
        assert_eq!(esc("plain"), "plain");
    }

    #[test]
    fn emphasis_and_code_map_to_expected_tags() {
        assert_eq!(render_inline("**b**"), "<p><strong>b</strong></p>");
        assert_eq!(render_inline("*i*"), "<p><em>i</em></p>");
        assert_eq!(render_inline("__u__"), "<p><u>u</u></p>");
        assert_eq!(render_inline("~~s~~"), "<p><s>s</s></p>");
        assert_eq!(
            render_inline("||x||"),
            "<p><span class=\"spoiler\">x</span></p>"
        );
        assert_eq!(render_inline("`c`"), "<p><code>c</code></p>");
        assert_eq!(
            render_inline("<script>alert(1)</script>"),
            "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>"
        );
    }

    #[test]
    fn links_open_in_new_tab_with_noopener() {
        assert_eq!(
            render_inline("[go](https://x.test)"),
            "<p><a href=\"https://x.test\" target=\"_blank\" rel=\"noopener noreferrer\">go</a></p>"
        );
        assert_eq!(
            render_inline("https://x.test"),
            "<p><a href=\"https://x.test\" target=\"_blank\" rel=\"noopener noreferrer\">https://x.test</a></p>"
        );
    }

    #[test]
    fn custom_emoji_becomes_cdn_image_with_gif_or_png() {
        assert_eq!(
            render_inline("<a:dance:42>"),
            "<p><img class=\"emoji\" alt=\":dance:\" src=\"https://cdn.discordapp.com/emojis/42.gif\"></p>"
        );
        assert_eq!(
            render_inline("<:tada:7>"),
            "<p><img class=\"emoji\" alt=\":tada:\" src=\"https://cdn.discordapp.com/emojis/7.png\"></p>"
        );
    }

    #[test]
    fn mentions_render_as_pills() {
        assert_eq!(
            render_inline("<@123>"),
            "<p><span class=\"mention\">@123</span></p>"
        );
        assert_eq!(
            render_inline("<@&9>"),
            "<p><span class=\"mention\">@9</span></p>"
        );
        assert_eq!(
            render_inline("<#5>"),
            "<p><span class=\"mention\">#5</span></p>"
        );
        assert_eq!(
            render_inline("@everyone @here"),
            "<p><span class=\"mention\">@everyone</span> <span class=\"mention\">@here</span></p>"
        );
    }

    #[test]
    fn timestamp_is_formatted_with_injected_clock() {
        // 1755878400 = Fri Aug 22 2025 16:00:00 UTC (verified via date(1))
        assert_eq!(
            render_inline("<t:0:f>"),
            "<p><span class=\"timestamp\">January 1, 1970 12:00 AM</span></p>"
        );
        assert_eq!(
            render_inline("<t:1755870000:r>"),
            "<p>&lt;t:1755870000:r&gt;</p>",
            "lowercase r is not a style; the rule declines and text remains"
        );
        assert_eq!(
            render_inline("<t:1755871200:R>"),
            "<p><span class=\"timestamp\">2 hours ago</span></p>",
            "relative style uses the injected now"
        );
    }

    #[test]
    fn block_structures_render_discord_chrome() {
        assert_eq!(render_inline("# Title\nbody"), "<h1>Title</h1><p>body</p>");
        assert_eq!(render_inline("- a\n- b"), "<ul><li>a</li><li>b</li></ul>");
        assert_eq!(render_inline("1. x\n2. y"), "<ol><li>x</li><li>y</li></ol>");
        assert_eq!(
            render_inline("-# note"),
            "<div class=\"eg-subtext\">note</div>"
        );
        assert_eq!(
            render_inline("> quoted"),
            "<blockquote class=\"eg-quote\"><p>quoted</p></blockquote>"
        );
        assert_eq!(
            render_inline("```\na<b\n```"),
            "<pre><code>a&lt;b</code></pre>",
            "code bodies are escaped but not markdown-parsed"
        );
    }

    #[test]
    fn field_grid_chunks_contiguous_inline_runs_by_three() {
        let field = |name: &str, inline: bool| Field {
            name: name.to_owned(),
            value: "v".to_owned(),
            inline,
        };
        let fields = vec![
            field("a", true),
            field("b", true),
            field("c", true),
            field("d", true),
            field("full", false),
            field("e", true),
        ];
        let mut out = String::new();
        render_fields(&fields, &mut out);
        assert_eq!(
            out,
            concat!(
                "<div class=\"eg-fields\">",
                "<div class=\"eg-field-group\" style=\"grid-template-columns:repeat(3,1fr)\">",
                "<div class=\"eg-field\"><div class=\"eg-field-name\">a</div><div class=\"eg-field-value\">v</div></div>",
                "<div class=\"eg-field\"><div class=\"eg-field-name\">b</div><div class=\"eg-field-value\">v</div></div>",
                "<div class=\"eg-field\"><div class=\"eg-field-name\">c</div><div class=\"eg-field-value\">v</div></div>",
                "</div>",
                "<div class=\"eg-field-group\" style=\"grid-template-columns:repeat(1,1fr)\">",
                "<div class=\"eg-field\"><div class=\"eg-field-name\">d</div><div class=\"eg-field-value\">v</div></div>",
                "</div>",
                "<div class=\"eg-field-group\" style=\"grid-template-columns:repeat(1,1fr)\">",
                "<div class=\"eg-field\"><div class=\"eg-field-name\">full</div><div class=\"eg-field-value\">v</div></div>",
                "</div>",
                "<div class=\"eg-field-group\" style=\"grid-template-columns:repeat(1,1fr)\">",
                "<div class=\"eg-field\"><div class=\"eg-field-name\">e</div><div class=\"eg-field-value\">v</div></div>",
                "</div>",
                "</div>"
            )
        );
    }

    #[test]
    fn embed_color_renders_as_hex_bar_only_when_set() {
        let colored = Embed {
            color: Some(0x58_65_f2),
            ..Embed::default()
        };
        let mut out = String::new();
        write_embed(&colored, &mut out, &RenderCtx { now_unix: NOW });
        assert!(
            out.contains("<div class=\"eg-embed-bar\" style=\"background:#5865f2\"></div>"),
            "got: {out}"
        );

        let bare = Embed::default();
        let mut out = String::new();
        write_embed(&bare, &mut out, &RenderCtx { now_unix: NOW });
        assert!(!out.contains("eg-embed-bar"), "no bar without color: {out}");
    }

    #[test]
    fn embed_timestamp_formats_like_the_timestamp_markdown_rule() {
        let pinned = Embed {
            timestamp: Some("2025-08-22T12:00:00.000Z".to_owned()),
            ..Embed::default()
        };
        let mut out = String::new();
        write_embed(&pinned, &mut out, &RenderCtx { now_unix: NOW });
        assert!(
            out.contains(
                "<div class=\"eg-footer\"><span>&bull; August 22, 2025 12:00 PM</span></div>"
            ),
            "got: {out}"
        );
    }

    #[test]
    fn unparseable_embed_timestamp_falls_back_to_raw_text() {
        let odd = Embed {
            timestamp: Some("not-a-date".to_owned()),
            ..Embed::default()
        };
        let mut out = String::new();
        write_embed(&odd, &mut out, &RenderCtx { now_unix: NOW });
        assert!(out.contains("<span>&bull; not-a-date</span>"), "got: {out}");
    }
}
