# pwr-viewgen

A lightweight Discord embed generator in Rust: turn webhook message JSON into
Discord-looking HTML or PNG previews, and optionally POST it to a real Discord
webhook.

## Install

Use a pre-built binary at [release page](https://github.com/FAZuH/pwr-viewgen/releases/latest)

Build & install with [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```bash
cargo install --git https://github.com/FAZuH/pwr-viewgen
```


## Usage

### Render

Render a message JSON to HTML on stdout:

```bash
pwr-viewgen render -i msg.json
pwr-viewgen render -i - < msg.json
```

Write to files instead (`-o` infers format from extension):

```bash
pwr-viewgen render -i msg.json --html preview.html
pwr-viewgen render -i msg.json --png preview.png   # needs headless Chrome
```

Useful flags: `--width` (content column px), `--scale` (PNG device scale),
`--now` (Unix timestamp for deterministic output).

### Serve (web UI)

```bash
pwr-viewgen serve            # http://127.0.0.1:8080
pwr-viewgen serve --port 9000
```

Requires the `serve` cargo feature (on by default). A local single page opens
at `http://127.0.0.1:<port>` (bound to 127.0.0.1 only): edit message JSON in
the textarea for a debounced live preview, set width/scale, Download PNG,
Copy HTML, or Send to a Discord webhook URL. Nothing is persisted.

### Send to a webhook

Validate the payload, then POST it as JSON to a Discord webhook:

```bash
pwr-viewgen send -i msg.json --webhook "https://discord.com/api/webhooks/ID/TOKEN"
```

Add `--wait` to append `?wait=true`; the created message id is printed on
success.

Live send is a manual step — no test talks to the network; CI runs only
offline unit/integration tests. To verify against Discord by hand, create a
webhook in a server channel (Channel settings → Integrations → Webhooks) and
run the `send` command above.

Errors: oversized/invalid payloads are rejected before any request
(`message failed validation: …`). Non-2xx responses print Discord's error
JSON and exit non-zero. A single 429 rate-limit response is retried once,
honoring `retry_after`.
