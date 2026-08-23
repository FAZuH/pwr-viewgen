#![cfg(feature = "png")]

use pwr_viewgen::model::parse_message;
use pwr_viewgen::render::render_html_with_width;
use pwr_viewgen::shot::capture_html;

const NOW: i64 = 1_755_878_400;

#[test]
#[ignore = "requires a local Chrome/Chromium binary; run with: cargo test --test png_spike -- --ignored"]
fn spike_captures_png_of_full_fixture() {
    let json =
        std::fs::read_to_string("tests/fixtures/png_spike.json").expect("spike fixture exists");
    let message = parse_message(&json).expect("fixture parses");

    let html = render_html_with_width(&message, NOW, 600);
    let png = capture_html(&html, 600, 2.0).expect("headless capture succeeds");

    assert_eq!(
        &png[..8],
        b"\x89PNG\r\n\x1a\n",
        "output must start with PNG magic bytes"
    );

    let ihdr_width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let ihdr_height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert!(
        (1150..=1400).contains(&ihdr_width),
        "width {ihdr_width} should be ~632 CSS px * scale 2"
    );
    assert!(
        ihdr_height > 200,
        "height {ihdr_height} should carry content"
    );
    assert!(
        png.len() > 10_000,
        "expected non-trivial capture, got {} bytes",
        png.len()
    );

    std::fs::write("/tmp/opencode/spike.png", &png).expect("sample written for visual review");
    println!(
        "spike PNG: {}x{}, {} bytes",
        ihdr_width,
        ihdr_height,
        png.len()
    );
}
