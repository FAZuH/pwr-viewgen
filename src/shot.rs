use std::path::PathBuf;

pub const CHROME_ENV_VAR: &str = "PWR_VIEWGEN_CHROME";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_error_names_env_escape_hatch_and_tried_paths() {
        let err = ShotError::NotFound {
            tried: vec!["/usr/bin/chromium".into(), "/usr/bin/google-chrome".into()],
        };
        let message = err.to_string();
        assert!(message.contains("PWR_VIEWGEN_CHROME"), "got: {message}");
        assert!(message.contains("/usr/bin/chromium"));
        assert!(message.contains("/usr/bin/google-chrome"));
    }

    #[test]
    fn executable_check_rejects_missing_or_non_file_paths() {
        assert!(!is_executable(&PathBuf::from(
            "/nonexistent/pwr_viewgen-chrome-probe"
        )));
        assert!(!is_executable(&PathBuf::from("/dev/null")));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShotError {
    #[error(
        "no Chrome/Chromium executable found (set {CHROME_ENV_VAR}=/path/to/chrome to override); tried:\n{}",
        tried.join("\n")
    )]
    NotFound { tried: Vec<String> },
    #[error("{0}")]
    Other(String),
}

fn other<E: std::fmt::Display>(err: E) -> ShotError {
    ShotError::Other(err.to_string())
}

pub fn find_chrome() -> Result<PathBuf, ShotError> {
    const BARE_NAMES: [&str; 4] = [
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
    ];
    const ABSOLUTE_PATHS: [&str; 11] = [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/google-chrome",
        "/usr/local/bin/chromium",
        "/usr/local/bin/google-chrome-stable",
        "/snap/bin/chromium",
        "/opt/google/chrome/chrome",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    ];

    let mut tried = Vec::new();

    if let Ok(env_path) = std::env::var(CHROME_ENV_VAR) {
        let candidate = PathBuf::from(&env_path);
        if is_executable(&candidate) {
            return Ok(candidate);
        }
        tried.push(format!(
            "{CHROME_ENV_VAR}={} (env var set but not executable)",
            candidate.display()
        ));
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for name in BARE_NAMES {
                let candidate = dir.join(name);
                if is_executable(&candidate) {
                    return Ok(candidate);
                }
                tried.push(candidate.display().to_string());
            }
        }
    }

    for path in ABSOLUTE_PATHS {
        let candidate = PathBuf::from(path);
        if is_executable(&candidate) {
            return Ok(candidate);
        }
        tried.push(path.to_owned());
    }

    Err(ShotError::NotFound { tried })
}

fn is_executable(path: &PathBuf) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn capture_html(html: &str, width: u32, scale: f32) -> Result<Vec<u8>, ShotError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(other)?;
    runtime.block_on(capture_html_async(html, width, scale))
}

async fn capture_html_async(html: &str, width: u32, scale: f32) -> Result<Vec<u8>, ShotError> {
    use chromiumoxide::browser::Browser;
    use chromiumoxide::browser::BrowserConfig;
    use futures::StreamExt;

    let chrome = find_chrome()?;
    let temp_path = write_temp_html(html)?;
    let page_url = format!("file://{}", encode_path(&temp_path));

    let mut config_builder = BrowserConfig::builder()
        .chrome_executable(chrome)
        .arg("--disable-gpu")
        .window_size(width + 80, 1200);
    if running_as_root() {
        config_builder = config_builder.no_sandbox();
    }
    let config = config_builder.build().map_err(other)?;

    let (mut browser, mut handler) = Browser::launch(config).await.map_err(other)?;
    let pump = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let result = capture_page(&browser, &page_url, scale).await;

    let _ = browser.close().await;
    let _ = pump.await;
    let _ = std::fs::remove_file(&temp_path);
    result
}

async fn capture_page(
    browser: &chromiumoxide::Browser,
    page_url: &str,
    scale: f32,
) -> Result<Vec<u8>, ShotError> {
    use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
    use chromiumoxide::cdp::browser_protocol::page::Viewport;
    use chromiumoxide::page::ScreenshotParams;

    let page = browser
        .new_page("about:blank")
        .await
        .map_err(|e| other(format!("opening page failed: {e}")))?;
    page.goto(page_url)
        .await
        .map_err(|e| other(format!("navigating failed: {e}")))?;

    page.evaluate(SETTLE_JS)
        .await
        .map_err(|e| other(format!("waiting for fonts and images failed: {e}")))?;

    let rect: Rect = page
        .evaluate(RECT_JS)
        .await
        .map_err(|e| other(format!("measuring #wrap failed: {e}")))?
        .into_value()
        .map_err(|e| other(format!("unexpected rect payload: {e}")))?;
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Err(ShotError::Other(format!(
            "#wrap has degenerate size {}x{}",
            rect.width, rect.height
        )));
    }

    let clip = Viewport::builder()
        .x(rect.x)
        .y(rect.y)
        .width(rect.width)
        .height(rect.height)
        .scale(scale)
        .build()
        .map_err(other)?;
    let png = page
        .screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .clip(clip)
                .capture_beyond_viewport(true)
                .build(),
        )
        .await
        .map_err(|e| other(format!("screenshot failed: {e}")))?;
    Ok(png)
}

const SETTLE_JS: &str = r#"(async () => {
  await document.fonts.ready;
  await Promise.all(Array.from(document.images).map(img =>
    img.complete ? null : img.decode().catch(() => {})
  ));
})()"#;

const RECT_JS: &str = r#"(() => {
  const r = document.querySelector('#wrap').getBoundingClientRect();
  return { x: r.x, y: r.y, width: r.width, height: r.height };
})()"#;

#[derive(Debug, serde::Deserialize)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn write_temp_html(html: &str) -> Result<PathBuf, ShotError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path =
        std::env::temp_dir().join(format!("pwr_viewgen-{}-{nanos}.html", std::process::id()));
    std::fs::write(&path, html).map_err(other)?;
    Ok(path)
}

fn encode_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('%', "%25")
        .replace(' ', "%20")
}

#[cfg(unix)]
fn running_as_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("Uid:")
                    .and_then(|rest| rest.split_whitespace().next())
                    .map(|uid| uid == "0")
            })
        })
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn running_as_root() -> bool {
    false
}
