use pwr_viewgen::model::Message;
use pwr_viewgen::render::render_html;
use pwr_viewgen::render::render_message_html;

const NOW: i64 = 1_755_878_400;

const SIMPLE_JSON: &str = r#"{"content": "**gm** world 🌍"}"#;

const FULL_JSON: &str = r#"{
    "content": "Deploy **finished**",
    "username": "Notifier",
    "avatar_url": "https://cdn.example.test/avatar.png",
    "embeds": [{
        "title": "Release notes",
        "url": "https://example.test/rel/7",
        "description": "All systems *nominal*",
        "color": 3066993,
        "author": { "name": "Ada", "icon_url": "https://cdn.example.test/ada.png" },
        "thumbnail": { "url": "https://cdn.example.test/thumb.png" },
        "image": { "url": "https://cdn.example.test/chart.png" },
        "footer": { "text": "CI bot", "icon_url": "https://cdn.example.test/ci.png" },
        "timestamp": "2025-08-22T12:00:00.000Z",
        "fields": [
            { "name": "Branch", "value": "main", "inline": true },
            { "name": "Duration", "value": "3m 12s", "inline": true },
            { "name": "Commit", "value": "abc1234", "inline": true },
            { "name": "Notes", "value": "see <docs>" }
        ]
    }]
}"#;

const COMPONENTS_V1_JSON: &str = r#"{
    "content": "Pick one:",
    "components": [
        { "type": 1, "components": [
            { "type": 2, "style": 1, "label": "Primary" },
            { "type": 2, "style": 2, "label": "Grey" },
            { "type": 2, "style": 3, "label": "Green" },
            { "type": 2, "style": 4, "label": "Red", "disabled": true },
            { "type": 2, "style": 5, "label": "Docs", "url": "https://example.test/docs" }
        ]},
        { "type": 1, "components": [
            { "type": 3, "placeholder": "Choose…", "options": [ { "label": "Red" }, { "label": "Blue" } ] }
        ]}
    ]
}"#;

const COMPONENTS_V2_JSON: &str = r##"{
    "flags": 32768,
    "components": [
        { "type": 10, "content": "# Release notes\n- one\n- two" },
        { "type": 14, "divider": true, "spacing": 2 },
        {
            "type": 9,
            "components": [{ "type": 10, "content": "section *body*" }],
            "accessory": {
                "type": 11,
                "media": { "url": "https://cdn.example.test/thumb.png" },
                "spoiler": true
            }
        },
        {
            "type": 12,
            "items": [
                { "media": { "url": "https://cdn.example.test/a.png" } },
                { "media": { "url": "https://cdn.example.test/b.png" } }
            ]
        },
        { "type": 13, "file": { "url": "attachment://notes.pdf?v=2" } },
        {
            "type": 17,
            "accent_color": 8912896,
            "components": [
                { "type": 10, "content": "inside **container**" },
                { "type": 1, "components": [ { "type": 2, "style": 3, "label": "OK" } ] },
                { "type": 14, "divider": false, "spacing": 1 }
            ]
        }
    ]
}"##;

#[test]
fn golden_simple() {
    let msg: Message = serde_json::from_str(SIMPLE_JSON).unwrap();
    assert_eq!(
        render_message_html(&msg, NOW),
        concat!(
            r#"<div id="wrap">"#,
            r#"<div class="eg-body">"#,
            r#"<div class="eg-content"><p><strong>gm</strong> world 🌍</p></div>"#,
            r#"</div>"#,
            r#"</div>"#
        )
    );
}

#[test]
fn golden_full_embed() {
    let msg: Message = serde_json::from_str(FULL_JSON).unwrap();
    assert_eq!(
        render_message_html(&msg, NOW),
        concat!(
            r#"<div id="wrap">"#,
            r#"<div class="eg-header">"#,
            r#"<img class="eg-avatar" src="https://cdn.example.test/avatar.png" alt="">"#,
            r#"<span class="eg-username">Notifier</span>"#,
            r#"</div>"#,
            r#"<div class="eg-body">"#,
            r#"<div class="eg-content"><p>Deploy <strong>finished</strong></p></div>"#,
            r#"<div class="eg-embeds">"#,
            r#"<div class="eg-embed">"#,
            r#"<div class="eg-embed-bar" style="background:#2ecc71"></div>"#,
            r#"<div class="eg-embed-inner">"#,
            r#"<img class="eg-thumbnail" src="https://cdn.example.test/thumb.png" alt="">"#,
            r#"<div class="eg-author-row">"#,
            r#"<img class="eg-author-icon" src="https://cdn.example.test/ada.png" alt="">"#,
            r#"<span class="eg-author-name">Ada</span>"#,
            r#"</div>"#,
            r#"<div class="eg-title"><a href="https://example.test/rel/7" target="_blank" rel="noopener noreferrer"><p>Release notes</p></a></div>"#,
            r#"<div class="eg-description"><p>All systems <em>nominal</em></p></div>"#,
            r#"<div class="eg-fields">"#,
            r#"<div class="eg-field-group" style="grid-template-columns:repeat(3,1fr)">"#,
            r#"<div class="eg-field"><div class="eg-field-name">Branch</div><div class="eg-field-value">main</div></div>"#,
            r#"<div class="eg-field"><div class="eg-field-name">Duration</div><div class="eg-field-value">3m 12s</div></div>"#,
            r#"<div class="eg-field"><div class="eg-field-name">Commit</div><div class="eg-field-value">abc1234</div></div>"#,
            r#"</div>"#,
            r#"<div class="eg-field-group" style="grid-template-columns:repeat(1,1fr)">"#,
            r#"<div class="eg-field"><div class="eg-field-name">Notes</div><div class="eg-field-value">see &lt;docs&gt;</div></div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"<img class="eg-image" src="https://cdn.example.test/chart.png" alt="">"#,
            r#"<div class="eg-footer">"#,
            r#"<img class="eg-footer-icon" src="https://cdn.example.test/ci.png" alt="">"#,
            r#"<span>CI bot</span>"#,
            r#"<span>&bull; August 22, 2025 12:00 PM</span>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#
        )
    );
}

#[test]
fn golden_components_v1_buttons_and_select() {
    let msg: Message = serde_json::from_str(COMPONENTS_V1_JSON).unwrap();
    assert_eq!(
        render_message_html(&msg, NOW),
        concat!(
            r#"<div id="wrap">"#,
            r#"<div class="eg-body">"#,
            r#"<div class="eg-content"><p>Pick one:</p></div>"#,
            r#"<div class="eg-components">"#,
            r#"<div class="eg-action-row">"#,
            r#"<button type="button" class="eg-btn eg-btn-primary"><span>Primary</span></button>"#,
            r#"<button type="button" class="eg-btn eg-btn-secondary"><span>Grey</span></button>"#,
            r#"<button type="button" class="eg-btn eg-btn-success"><span>Green</span></button>"#,
            r#"<button type="button" class="eg-btn eg-btn-danger is-disabled" disabled><span>Red</span></button>"#,
            r#"<a class="eg-btn eg-btn-link" href="https://example.test/docs" target="_blank" rel="noopener noreferrer"><span>Docs</span></a>"#,
            r#"</div>"#,
            r#"<div class="eg-action-row">"#,
            r#"<div class="eg-select"><span>Choose…</span><span class="eg-select-chevron"></span></div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#
        )
    );
}

#[test]
fn golden_components_v2_renders_all_container_children() {
    let msg: Message = serde_json::from_str(COMPONENTS_V2_JSON).unwrap();
    assert_eq!(
        render_message_html(&msg, NOW),
        concat!(
            r#"<div id="wrap">"#,
            r#"<div class="eg-body">"#,
            r#"<div class="eg-components">"#,
            r#"<div class="eg-text-display"><h1>Release notes</h1><ul><li>one</li><li>two</li></ul></div>"#,
            r#"<hr class="eg-separator eg-sep-lg">"#,
            r#"<div class="eg-section">"#,
            r#"<div class="eg-section-text">"#,
            r#"<div class="eg-text-display"><p>section <em>body</em></p></div>"#,
            r#"</div>"#,
            r#"<div class="eg-accessory">"#,
            r#"<figure class="eg-media eg-has-spoiler">"#,
            r#"<img class="eg-accessory-thumbnail" src="https://cdn.example.test/thumb.png" alt="">"#,
            r#"<div class="spoiler-overlay"></div>"#,
            r#"</figure>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"<div class="eg-gallery" style="grid-template-columns:repeat(2,1fr)">"#,
            r#"<figure class="eg-media"><img class="eg-gallery-item" src="https://cdn.example.test/a.png" alt=""></figure>"#,
            r#"<figure class="eg-media"><img class="eg-gallery-item" src="https://cdn.example.test/b.png" alt=""></figure>"#,
            r#"</div>"#,
            r##"<div class="eg-file-wrap"><a class="eg-file-card" href="#"><span class="eg-file-name">notes.pdf</span><span class="eg-file-download"></span></a></div>"##,
            r#"<div class="eg-container">"#,
            r#"<div class="eg-container-bar" style="background:#880000"></div>"#,
            r#"<div class="eg-container-inner">"#,
            r#"<div class="eg-text-display"><p>inside <strong>container</strong></p></div>"#,
            r#"<div class="eg-action-row">"#,
            r#"<button type="button" class="eg-btn eg-btn-success"><span>OK</span></button>"#,
            r#"</div>"#,
            r#"<div class="eg-sep-spacer-sm"></div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#
        )
    );
}

#[test]
fn v2_flag_suppresses_content_and_embed_chrome_even_when_present() {
    let raw = r#"{
        "flags": 32768,
        "content": "should vanish",
        "embeds": [{ "title": "vanish too" }],
        "components": [ { "type": 10, "content": "kept" } ]
    }"#;
    let msg: Message = serde_json::from_str(raw).unwrap();
    let html = render_message_html(&msg, NOW);
    assert_eq!(
        html,
        concat!(
            r#"<div id="wrap">"#,
            r#"<div class="eg-body">"#,
            r#"<div class="eg-components">"#,
            r#"<div class="eg-text-display"><p>kept</p></div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"</div>"#
        )
    );
}

#[test]
fn full_document_has_doctype_and_stylesheet_once() {
    let msg: Message = serde_json::from_str(SIMPLE_JSON).unwrap();
    let html = render_html(&msg, NOW);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert_eq!(html.matches("<style>").count(), 1);
    assert!(html.contains("</body></html>"));
}
