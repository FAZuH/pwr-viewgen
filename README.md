# pwr-viewgen

**A lightweight Discord embed generator in Rust.**

<hr>

<div align="center">
● <a href="#installation">Installation</a> ﻿ ● <a href="#usage">Usage</a> ﻿ ● <a href="#docs">Docs</a> ﻿ ● <a href="#license">License</a>
</div>

## Installation

Download a prebuilt binary from the
[latest release](https://github.com/FAZuH/pwr-viewgen/releases/latest). Put it
on your `PATH`. Release binaries include every feature.

Or install with [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```bash
cargo install --git https://github.com/FAZuH/pwr-viewgen
```

The default build has the `render` and `send` commands, and PNG export works.
To get the `serve` command, add the `serve` feature:

```bash
cargo install --git https://github.com/FAZuH/pwr-viewgen --features serve
```

PNG export needs Google Chrome or Chromium on your machine. The tool starts
that browser in headless mode.

## Usage

`pwr-viewgen` turns webhook message JSON into a Discord-looking HTML page or a
PNG image. The same JSON can go to a real Discord webhook.

### Render

Write the HTML to stdout, or read the JSON from stdin:

```bash
pwr-viewgen render -i msg.json
pwr-viewgen render -i - < msg.json
```

Write files instead. `-o` picks the format from the extension:

```bash
pwr-viewgen render -i msg.json --html preview.html
pwr-viewgen render -i msg.json --png preview.png   # needs Chrome or Chromium
pwr-viewgen render -i msg.json -o preview.png
```

Other flags: `--width` (content column, in px), `--scale` (PNG scale factor),
and `--now` (Unix timestamp, for repeatable output).

### Serve (web UI)

Start a local web page for live editing. This needs the `serve` feature:

```bash
pwr-viewgen serve            # http://127.0.0.1:8080
pwr-viewgen serve --port 9000
```

The server binds to 127.0.0.1 only. On the page you edit the message JSON and
see a live preview. You can set width and scale, download a PNG, copy the
HTML, or send the message to a webhook URL. The server stores nothing.

### Send to a webhook

Check the payload, then POST it to a Discord webhook:

```bash
pwr-viewgen send -i msg.json --webhook "https://discord.com/api/webhooks/ID/TOKEN"
```

Add `--wait` to append `?wait=true`. The tool then prints the id of the
created message.

`send` uses the same parser as `render` (see
[Accepted inputs](#accepted-inputs)). It sends the canonical JSON, not your
raw input bytes. Button `custom_id` values, select-menu kinds (types 5–8), and
string-select option values pass through unchanged.

Live sends are a manual step. No test talks to the network; CI runs offline
tests only. To check against Discord, create a webhook in a channel
(Channel settings → Integrations → Webhooks), then run the command above.

Bad payloads fail before the tool sends a request
(`message failed validation: …`). A non-2xx response prints the Discord error
JSON, and the tool exits with a non-zero status. The command makes one
request. It does not retry.

### Accepted inputs

The parser uses the `pwr-ext` wrappers. These mirror the serenity-next
builders, so your JSON must match what those builders emit. The parser
rejects unknown component types, select menus outside an action row, and
string-select options with no `value`.

Within that shape, the tool accepts: components v2 trees, select-menu kinds
user/role/mentionable/channel (types 5–8, rendered as closed pills), and
premium buttons (style 6, rendered like a link button with no URL).

Canonicalization keeps all render data. The webhook identity fields
(`username`, `avatar_url`) are kept too.

Note: `send` rejects premium buttons. This is deliberate. A premium (SKU)
button belongs to an application, and a plain webhook cannot create one, so
Discord would reject the request. The renderer still shows them, so previews
of bot-made payloads stay faithful.

## Docs

- [Commit and Changelog Conventions](docs/dev/commit-changelog.md) — commit format and how the changelog is generated
- [Commit Scopes](docs/dev/commit-scopes.md) — the approved `type(scope)` list for this repository

## License

`pwr-viewgen` is distributed under the [MIT](https://spdx.org/licenses/MIT.html) license.
