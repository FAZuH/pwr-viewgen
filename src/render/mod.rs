pub mod html;

use html::esc;
use html::RenderCtx;

use crate::markdown;
use crate::model::Message;

mod components;

const STYLESHEET: &str = include_str!("../assets/message.css");

pub fn render_html(message: &Message, now_unix: i64) -> String {
    render_html_with_width(message, now_unix, DEFAULT_CONTENT_WIDTH)
}

pub const DEFAULT_CONTENT_WIDTH: u32 = 600;

pub fn render_html_with_width(message: &Message, now_unix: i64, content_width: u32) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>{STYLESHEET}</style></head><body>{}\
</body></html>",
        render_message_html_with_width(message, now_unix, Some(content_width))
    )
}

fn render_message_html_with_width(
    message: &Message,
    now_unix: i64,
    content_width: Option<u32>,
) -> String {
    let ctx = RenderCtx { now_unix };
    let mut out = String::with_capacity(2048);
    match content_width {
        Some(width) => out.push_str(&format!("<div id=\"wrap\" style=\"--eg-width:{width}px\">")),
        None => out.push_str("<div id=\"wrap\">"),
    }

    if message.username.is_some() || message.avatar_url.is_some() {
        out.push_str("<div class=\"eg-header\">");
        match message.avatar_url.as_deref() {
            Some(url) => out.push_str(&format!(
                "<img class=\"eg-avatar\" src=\"{}\" alt=\"\">",
                esc(url)
            )),
            None => out.push_str("<span class=\"eg-avatar-placeholder\"></span>"),
        }
        if let Some(username) = &message.username {
            out.push_str(&format!(
                "<span class=\"eg-username\">{}</span>",
                esc(username)
            ));
        }
        out.push_str("</div>");
    }

    let has_body = !message.content.trim().is_empty()
        || !message.embeds.is_empty()
        || !message.components.is_empty();
    if has_body {
        out.push_str("<div class=\"eg-body\">");
    }

    let components_v2 = message.is_components_v2();
    if !components_v2 && !message.content.trim().is_empty() {
        out.push_str("<div class=\"eg-content\">");
        html::write_blocks(&markdown::parse(&message.content), &mut out, &ctx);
        out.push_str("</div>");
    }

    if !components_v2 && !message.embeds.is_empty() {
        out.push_str("<div class=\"eg-embeds\">");
        for embed in &message.embeds {
            html::write_embed(embed, &mut out, &ctx);
        }
        out.push_str("</div>");
    }

    components::write_components(&message.components, &mut out, &ctx);

    if has_body {
        out.push_str("</div>");
    }

    out.push_str("</div>");
    out
}

pub fn render_message_html(message: &Message, now_unix: i64) -> String {
    render_message_html_with_width(message, now_unix, None)
}
#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_755_878_400;

    #[test]
    fn bare_content_renders_content_only_without_header() {
        let msg: Message = serde_json::from_str(r#"{"content":"gm"}"#).unwrap();
        let html = render_html(&msg, NOW);
        assert!(html.contains(r#"<div id="wrap" style="--eg-width:600px">"#));
        assert!(
            !html.contains("class=\"eg-header"),
            "no header without username: {}",
            html.rfind("<body").map(|i| &html[i..]).unwrap_or(&html)
        );
        assert_eq!(
            extract(&html, r#"<div class="eg-content">"#, "</div>"),
            "<p>gm</p>"
        );
        assert!(!html.contains("class=\"eg-embed"));
    }

    #[test]
    fn header_shows_username_and_avatar_when_present() {
        let msg: Message = serde_json::from_str(
            r#"{"content":"hi","username":"Ada","avatar_url":"https://x.test/a.png"}"#,
        )
        .unwrap();
        let html = render_html(&msg, NOW);
        assert!(html.contains("<img class=\"eg-avatar\" src=\"https://x.test/a.png\" alt=\"\">"));
        assert!(html.contains("<span class=\"eg-username\">Ada</span>"));
    }

    #[test]
    fn stylesheet_is_embedded_once_in_head() {
        let msg = Message::default();
        let html = render_html(&msg, NOW);
        assert_eq!(html.matches(STYLESHEET).count(), 1);
    }

    fn extract(html: &str, start_marker: &str, end_marker: &str) -> String {
        let start = html.find(start_marker).expect("start marker") + start_marker.len();
        let end = start + html[start..].find(end_marker).expect("end marker");
        html[start..end].to_owned()
    }
}
