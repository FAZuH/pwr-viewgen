use std::io::Read;
use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use clap::Subcommand;
use pwr_viewgen::model::parse_message;
use pwr_viewgen::render::{self};
#[cfg(feature = "png")]
use pwr_viewgen::shot;
use pwr_viewgen::webhook;

#[derive(Parser)]
#[command(
    name = "pwr-viewgen",
    version,
    about = "Discord embed generator: webhook message JSON to Discord-looking HTML"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a webhook message JSON to a standalone HTML document and/or PNG screenshot
    Render {
        /// Input JSON file path, or '-' to read from stdin
        #[arg(short, long)]
        input: String,
        /// Output file; format inferred by extension (.png captures via headless Chrome)
        #[arg(short, long)]
        output: Option<String>,
        /// Write the HTML document to this file
        #[arg(long)]
        html: Option<String>,
        /// Capture a PNG screenshot to this file (requires headless Chrome)
        #[arg(long)]
        png: Option<String>,
        /// Content column width in px
        #[arg(long, default_value_t = render::DEFAULT_CONTENT_WIDTH)]
        width: u32,
        /// Device scale factor for PNG export
        #[arg(long, default_value_t = 2)]
        scale: u32,
        /// Unix timestamp overriding the clock, for deterministic output
        #[arg(long)]
        now: Option<i64>,
    },
    /// POST a webhook message JSON to a Discord webhook URL
    Send {
        /// Input JSON file path, or '-' to read from stdin
        #[arg(short, long)]
        input: String,
        /// Discord webhook URL to post the message to
        #[arg(long)]
        webhook: String,
        /// Wait for Discord to create the message and print its id
        #[arg(long)]
        wait: bool,
    },
    /// Serve a local web UI for interactive editing and preview
    #[cfg(feature = "serve")]
    Serve {
        /// TCP port to bind on 127.0.0.1 only
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

enum OutputTarget {
    Html(String),
    Png(String),
}

fn resolve_targets(
    output: Option<String>,
    html: Option<String>,
    png: Option<String>,
) -> Vec<OutputTarget> {
    let mut targets = Vec::new();
    if let Some(path) = html {
        targets.push(OutputTarget::Html(path));
    }
    if let Some(path) = png {
        targets.push(OutputTarget::Png(path));
    }
    if let Some(path) = output {
        if path.to_ascii_lowercase().ends_with(".png") {
            targets.push(OutputTarget::Png(path));
        } else {
            targets.push(OutputTarget::Html(path));
        }
    }
    targets
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render {
            input,
            output,
            html,
            png,
            width,
            scale,
            now,
        } => {
            let json = read_input(&input)?;
            let parsed = parse_message(&json).map_err(|e| format!("invalid message JSON: {e}"))?;
            let now_unix = now.unwrap_or_else(|| chrono::Utc::now().timestamp());
            let document = render::render_html_with_width(&parsed.message, now_unix, width);
            write_targets(&resolve_targets(output, html, png), &document, width, scale)
        }
        Command::Send {
            input,
            webhook: url,
            wait,
        } => {
            let json = read_input(&input)?;
            let parsed = parse_message(&json).map_err(|e| format!("invalid message JSON: {e}"))?;
            match webhook::send(&url, &parsed, wait) {
                Ok(result) => {
                    if let Some(id) = result.message_id {
                        println!("{id}");
                    }
                    Ok(())
                }
                Err(error) => Err(error.to_string()),
            }
        }
        #[cfg(feature = "serve")]
        Command::Serve { port } => pwr_viewgen::server::run_server(port),
    }
}

fn read_input(path: &str) -> Result<String, String> {
    match path {
        "-" => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| format!("failed to read stdin: {e}"))?;
            Ok(buffer)
        }
        path => std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read input file {path:?}: {e}")),
    }
}

fn write_targets(
    targets: &[OutputTarget],
    document: &str,
    width: u32,
    scale: u32,
) -> Result<(), String> {
    if targets.is_empty() {
        return write_stdout(document);
    }
    for target in targets {
        match target {
            OutputTarget::Html(path) => std::fs::write(path, document)
                .map_err(|e| format!("failed to write output file {path:?}: {e}"))?,
            OutputTarget::Png(path) => write_png(path, document, width, scale)?,
        }
    }
    Ok(())
}

fn write_stdout(document: &str) -> Result<(), String> {
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(document.as_bytes())
        .and_then(|_| stdout.flush())
        .map_err(|e| format!("failed to write to stdout: {e}"))
}

#[cfg(feature = "png")]
fn write_png(path: &str, document: &str, width: u32, scale: u32) -> Result<(), String> {
    let bytes = shot::capture_html(document, width, scale as f32)
        .map_err(|e| format!("PNG export to {path:?} failed: {e}"))?;
    std::fs::write(path, bytes).map_err(|e| format!("failed to write output file {path:?}: {e}"))
}

#[cfg(not(feature = "png"))]
fn write_png(_path: &str, _document: &str, _width: u32, _scale: u32) -> Result<(), String> {
    Err("PNG export unavailable: binary was built without the `png` feature".to_owned())
}
