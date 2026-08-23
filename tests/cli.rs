use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use pwr_viewgen::model::parse_message;
use pwr_viewgen::render::render_html_with_width;

const NOW: &str = "1755878400";
const NOW_UNIX: i64 = 1_755_878_400;
const DEFAULT_WIDTH: u32 = 600;
const BAD_CHROME: &str = "/nonexistent/pwr_viewgen-chrome-probe";

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
        .args(args)
        .output()
        .expect("pwr_viewgen binary should run")
}

fn run_cli_with_chrome_override(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
        .args(args)
        .env("PWR_VIEWGEN_CHROME", BAD_CHROME)
        .output()
        .expect("pwr_viewgen binary should run")
}

#[test]
fn render_fixture_matches_library_golden() {
    let output = run_cli(&["render", "-i", "tests/fixtures/full.json", "--now", NOW]);
    assert!(
        output.status.success(),
        "exit code {:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let json = std::fs::read_to_string("tests/fixtures/full.json").unwrap();
    let message = parse_message(&json).unwrap();
    assert_eq!(
        stdout,
        render_html_with_width(&message, NOW_UNIX, DEFAULT_WIDTH)
    );

    assert!(
        stdout.contains(r#"<div id="wrap" style="--eg-width:600px">"#),
        "content width must arrive as a CSS var"
    );
    assert!(stdout.starts_with("<!DOCTYPE html>"));
    assert!(stdout.contains("<span class=\"eg-username\">Notifier</span>"));
    assert!(stdout.contains("style=\"background:#2ecc71\""));
}

#[test]
fn stdin_input_with_custom_width() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
        .args(["render", "-i", "-", "--now", NOW, "--width", "800"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(br#"{"content": "hi"}"#)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"--eg-width:800px"#));
    assert!(stdout.contains("<p>hi</p>"));
}

#[test]
fn invalid_json_exits_nonzero_with_readable_message() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
        .args(["render", "-i", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(br#"{"content": oops}"#)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("invalid message JSON") && stderr.contains("line 1"),
        "readable parse error expected, got: {stderr}"
    );
}

#[test]
fn validation_failure_reports_field_path_and_nonzero_exit() {
    let oversized = format!(r#"{{"content": "{}"}}"#, "x".repeat(2001));
    let mut child = Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
        .args(["render", "-i", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(oversized.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("failed validation")
            && stderr.contains("content must be at most 2000 characters"),
        "field-path error expected, got: {stderr}"
    );
}

#[test]
fn output_file_flag_writes_html() {
    let out_path =
        std::env::temp_dir().join(format!("pwr_viewgen-cli-test-{}.html", std::process::id()));
    let path_str = out_path.to_str().expect("temp path utf8");
    let output = run_cli(&[
        "render",
        "-i",
        "tests/fixtures/simple.json",
        "--now",
        NOW,
        "-o",
        path_str,
    ]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "-o should not print to stdout");
    let written = std::fs::read_to_string(&out_path).expect("file written");
    let _ = std::fs::remove_file(&out_path);
    assert!(written.starts_with("<!DOCTYPE html>"));
}

#[test]
fn png_output_with_failing_chrome_reports_wrapped_error() {
    let fake_chrome =
        std::env::temp_dir().join(format!("eg-cli-fake-chrome-{}.sh", std::process::id()));
    std::fs::write(&fake_chrome, "#!/bin/sh\nexit 3\n").expect("write fake chrome");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_chrome, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake chrome");
    }

    let png_a = std::env::temp_dir().join(format!("eg-cli-a-{}.png", std::process::id()));
    let png_b = std::env::temp_dir().join(format!("eg-cli-b-{}.png", std::process::id()));
    let cases: [(&[&str], &str); 2] = [
        (
            &[
                "render",
                "-i",
                "tests/fixtures/simple.json",
                "--now",
                NOW,
                "-o",
                png_a.to_str().unwrap(),
            ],
            "-o .png inference",
        ),
        (
            &[
                "render",
                "-i",
                "tests/fixtures/simple.json",
                "--now",
                NOW,
                "--png",
                png_b.to_str().unwrap(),
            ],
            "explicit --png flag",
        ),
    ];
    for (args, label) in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_pwr-viewgen"))
            .args(args)
            .env("PWR_VIEWGEN_CHROME", fake_chrome.to_str().unwrap())
            .output()
            .expect("pwr_viewgen binary should run");
        assert!(
            !output.status.success(),
            "{label}: expected nonzero exit when chrome fails"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("PNG export"),
            "{label}: capture error must be wrapped with context, got: {stderr}"
        );
    }
    let _ = std::fs::remove_file(&fake_chrome);
}

#[test]
fn html_output_extension_does_not_require_chrome() {
    let out_path = std::env::temp_dir().join(format!("eg-cli-html-{}.html", std::process::id()));
    let path_str = out_path.to_str().expect("temp path utf8");
    let output = run_cli_with_chrome_override(&[
        "render",
        "-i",
        "tests/fixtures/simple.json",
        "--now",
        NOW,
        "-o",
        path_str,
    ]);
    assert!(
        output.status.success(),
        ".html output must not invoke the capture path"
    );
    let written = std::fs::read_to_string(&out_path).expect("file written");
    let _ = std::fs::remove_file(&out_path);
    assert!(written.starts_with("<!DOCTYPE html>"));
}

#[test]
#[ignore = "requires a local Chrome/Chromium binary; run with: cargo test --test cli -- --ignored"]
fn png_capture_writes_magic_bytes_and_expected_width() {
    let out_path =
        std::env::temp_dir().join(format!("pwr_viewgen-cli-png-{}.png", std::process::id()));
    let path_str = out_path.to_str().expect("temp path utf8");
    let output = run_cli(&[
        "render",
        "-i",
        "tests/fixtures/full.json",
        "--now",
        NOW,
        "--scale",
        "2",
        "--png",
        path_str,
    ]);
    assert!(
        output.status.success(),
        "exit code {:?}",
        output.status.code()
    );
    let bytes = std::fs::read(&out_path).expect("PNG file written");
    let _ = std::fs::remove_file(&out_path);
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "PNG magic bytes");
    let ihdr_width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    assert_eq!(ihdr_width, 1264, "(600 content + 32 padding) * scale 2");
}
