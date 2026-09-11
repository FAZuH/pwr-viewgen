## 0.1.0 (2026-09-11)


### ⚠ BREAKING CHANGES

* model::parse_message returns ParsedMessage instead of
Message; webhook::prepare/send signatures changed; /api/send returns
HTTP 200 {ok:false} for validation failures instead of 400; no automatic
retry on 429.

### Features

* add large-payload smoke suite with golden HTML assertions ([a2af9cd](https://github.com/FAZuH/pwr-viewgen/commit/a2af9cd9a9bf4f4ec4bc1aafc30dd9d4af433c37))
* add send-gate checks for components v2 text and gallery limits ([315beb9](https://github.com/FAZuH/pwr-viewgen/commit/315beb927a4ba8a35980bfe9224853dd159880ea))
* parse Discord payloads through pwr-ext ([38d6c85](https://github.com/FAZuH/pwr-viewgen/commit/38d6c85d3d3467b0706a51600217fc8f132fa78a))
* **pwr-ext:** add pwr-ext crate with Deserialize support for serenity-next builders ([818caf6](https://github.com/FAZuH/pwr-viewgen/commit/818caf6155e0b16bb3677e3a75fb8a5a5f40d3ab))
* single-request sends and ParsedMessage-based pipeline ([13f051c](https://github.com/FAZuH/pwr-viewgen/commit/13f051cb49e92a37beb5ba2365ebe43a186a5ad5))


### Bug Fixes

* fetch pwr-ext from git so cargo install --git works ([ccb4cf4](https://github.com/FAZuH/pwr-viewgen/commit/ccb4cf4e579cf9c2b8a7ec78c16fe286c93b109c))

