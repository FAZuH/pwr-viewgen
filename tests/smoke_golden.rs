//! Golden smoke assertions over the builder-generated fixtures: every
//! fixture renders through `parse_message` → `render_html_with_width` with a
//! pinned clock, structural counts are asserted from fixture metadata, and
//! selected markup is pinned exactly. CLI-level runs cover the two largest
//! payloads. No wall clock, no network, no fs writes (spec D2/D4).

#[path = "smoke_fixtures.rs"]
mod fixtures;

use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use fixtures::Fixture;
use pwr_viewgen::model::parse_message;
use pwr_viewgen::model::ParsedMessage;
use pwr_viewgen::render::render_html_with_width;
use pwr_viewgen::render::DEFAULT_CONTENT_WIDTH;

const NOW: i64 = 1_755_878_400;
const NOW_ARG: &str = "1755878400";

fn render(fixture: &Fixture) -> String {
    let message = parse_message(&fixture.json).expect("fixture parses");
    render_html_with_width(&message, NOW, DEFAULT_CONTENT_WIDTH)
}

fn body_of(full_document: &str) -> &str {
    full_document
        .split_once("<body>")
        .expect("full document carries a body")
        .1
}

fn assert_fixture_counts(fixture: &Fixture, html: &str) {
    for (marker, expected) in fixture.expected_counts {
        let actual = html.matches(marker).count();
        assert_eq!(
            actual,
            *expected,
            "{}",
            fixture.count_message(marker, *expected, actual)
        );
    }
}

// ---- Max-limit stress ----

#[test]
fn max_limit_embed_stress_renders_every_embed_field_and_footer() {
    let fixture = fixtures::max_limit_embed_stress();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert!(
        html.contains(r#"<div class="eg-embed-bar" style="background:#ffffff"></div>"#),
        "every stress embed carries the max colour bar"
    );
}

#[test]
fn deep_v2_nesting_stress_renders_every_region_inside_one_container() {
    let fixture = fixtures::deep_v2_nesting_stress();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert!(
        html.contains(r#"<hr class="eg-separator eg-sep-lg">"#),
        "large separator pinned exactly"
    );
    assert!(
        html.contains(r#"<span class="eg-file-name">deep-nesting.pdf</span>"#),
        "file card keeps the attachment basename"
    );
    assert!(
        html.contains(r#"style="background:#5865f2""#),
        "container accent colour renders as the bar"
    );
}

// ---- Realistic large ----

#[test]
fn release_notes_post_renders_embeds_buttons_and_version_select() {
    let fixture = fixtures::release_notes_post();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert_eq!(
        html.matches(r#"<div class="eg-select"><span>Pick a version…</span><span class="eg-select-chevron"></span></div>"#).count(),
        1,
        "closed select pill shows the placeholder exactly once"
    );
    assert!(
        html.contains(r#"<a class="eg-btn eg-btn-link" href="https://example.test/docs""#),
        "link buttons keep their target"
    );
}

#[test]
fn bot_status_panel_renders_every_v2_region_with_pinned_timestamp() {
    let fixture = fixtures::bot_status_panel();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert!(
        html.contains(r##"<div class="eg-file-wrap"><a class="eg-file-card" href="#"><span class="eg-file-name">postmortem.md</span><span class="eg-file-download"></span></a></div>"##),
        "unspoiled file card has no overlay"
    );
    assert!(
        body_of(&html).contains(r#"<span class="timestamp">now</span>"#),
        "relative timestamp resolves against the pinned now"
    );
}

#[test]
fn poll_message_keeps_the_poll_through_parse_and_renders_content() {
    let fixture = fixtures::poll_announcement();

    let parsed: ParsedMessage = fixture.json.parse().expect("fixture parses");
    assert_eq!(
        parsed
            .canonical
            .pointer("/poll/question/text")
            .and_then(serde_json::Value::as_str),
        Some("Best deploy window?"),
        "the poll survives strict canonicalization"
    );
    let html = render(&fixture);
    assert!(
        html.contains("<p>Please vote below!</p>"),
        "content renders normally alongside the poll"
    );
}

// ---- Edge/adversarial ----

#[test]
fn zwj_emoji_storm_preserves_graphemes_and_emoji_chrome() {
    let fixture = fixtures::zwj_emoji_storm();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert!(
        html.contains("👨‍👩‍👧‍👦"),
        "ZWJ sequence must pass through untouched"
    );
    assert!(
        html.contains("\u{202E}gnitirw-rtl\u{202C}"),
        "RTL overrides are preserved text"
    );
}

#[test]
fn spoiler_everything_overlays_each_spoilerable_media_region() {
    let fixture = fixtures::spoiler_everything();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert!(
        body_of(&html).matches("eg-has-spoiler").count() == 6,
        "container, thumbnail, two gallery items and two files carry the class"
    );
}

#[test]
fn legal_minimum_empties_render_skeleton_regions_without_panic() {
    let fixture = fixtures::legal_minimum_empties();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
    assert!(
        html.contains(r#"<div class="eg-text-display"></div>"#),
        "an empty text display still renders its region shell"
    );
    assert!(
        html.contains(r#"<div class="eg-sep-spacer-sm"></div>"#),
        "a divider-less separator defaults to a small spacer"
    );
    assert!(
        html.contains(r#"<div class="eg-container-inner"></div>"#),
        "an empty container renders an empty inner column"
    );
}

#[test]
fn mixed_v1_rows_render_inside_the_components_v2_tree() {
    let fixture = fixtures::mixed_v1_rows_in_v2_tree();
    let html = render(&fixture);

    assert_fixture_counts(&fixture, &html);
}

#[test]
fn unknown_flag_bits_do_not_change_the_rendered_output() {
    let spliced =
        parse_message(&fixtures::unknown_flag_bits().json).expect("spliced payload parses");
    let clean =
        parse_message(&fixtures::unknown_flag_bits_before_splice()).expect("clean payload parses");

    assert_eq!(
        render_html_with_width(&spliced, NOW, DEFAULT_CONTENT_WIDTH),
        render_html_with_width(&clean, NOW, DEFAULT_CONTENT_WIDTH),
        "bits undefined on MessageFlags must be invisible to the renderer"
    );
}

// ---- Determinism ----

#[test]
fn every_fixture_renders_identically_across_two_runs() {
    for fixture in fixtures::all_fixtures() {
        assert_eq!(
            render(&fixture),
            render(&fixture),
            "fixture `{}` is deterministic",
            fixture.name
        );
    }
}

// ---- CLI level ----

fn cli_render_from_stdin(json: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
        .args(["render", "-i", "-", "--now", NOW_ARG])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("pwr_viewgen binary should run");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(json.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "CLI exit code {:?}",
        output.status.code()
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn assert_cli_matches_library(fixture_name: &'static str, json: String) {
    let cli_html = cli_render_from_stdin(&json);
    let library_html = render_html_with_width(
        &parse_message(&json).expect("fixture parses"),
        NOW,
        DEFAULT_CONTENT_WIDTH,
    );

    assert!(cli_html.starts_with("<!DOCTYPE html>"));
    assert_eq!(
        cli_html, library_html,
        "`{fixture_name}` renders byte-identical via CLI and library"
    );
}

#[test]
fn cli_renders_the_max_limit_embed_stream_byte_identically_to_the_library() {
    let fixture = fixtures::max_limit_embed_stress();
    assert_cli_matches_library(fixture.name, fixture.json);
}

#[test]
fn cli_renders_the_bot_status_panel_byte_identically_to_the_library() {
    let fixture = fixtures::bot_status_panel();
    assert_cli_matches_library(fixture.name, fixture.json);
}
