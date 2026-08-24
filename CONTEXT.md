# pwr-viewgen

Turns Discord webhook message payloads into static HTML/PNG previews and forwards them to webhooks. Built for inspecting what a bot would produce without running the bot.

## Language

**Payload**:
The Discord webhook-message JSON a user feeds in — the thing being previewed.
_Avoid_: input JSON, request body

**Parse**:
The strict front door: a payload either matches what serenity-next builders emit or it is rejected before anything else happens. Parsing runs through the `pwr-ext` wrappers.
_Avoid_: load, deserialize (when speaking about the pipeline stage)

**Canonical body**:
The normalized JSON produced by parsing — exactly what `send` posts to the webhook and what round-trip tests pin. Lossless for sending, even where the view is lossy.
_Avoid_: normalized payload, clean JSON

**View**:
The render-side projection of a parsed payload (`model::Message`). Deliberately lossier than the canonical body: it keeps what previews need and drops what only sending needs.
_Avoid_: model, AST, intermediate representation

**Render**:
Producing the HTML preview from a view. Pure and deterministic — same view bytes in, same HTML bytes out.
_Avoid_: draw, generate

**Validate gate**:
The pre-send checks that reject payloads Discord itself would reject (forbidden component mixes, limit overruns, premium buttons). Rendering stays permissive; only sending passes the gate.
_Avoid_: lint, sanitizer

**Send**:
Validating, then posting the canonical body verbatim to a webhook URL. A manual, network-touching step — no test performs it.
_Avoid_: forward, execute

**Chrome**:
The preview-page shell around the rendered messages (background, fonts), selectable via `PWR_VIEWGEN_CHROME`. Distinct from the message content itself.
_Avoid_: theme, skin

**Golden**:
A pinned, byte-exact expected output (usually HTML) asserted in tests. Goldens are executable specs of render behavior, not snapshots to mindlessly refresh.
_Avoid_: snapshot, baseline
