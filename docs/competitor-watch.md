# Competitor fee / liquidity watch (cl8y-research#14)

Scheduled **inbox** collector: allowlisted public HTTPS pages → `SourceKind::CompetitorWatch` notes in the store. Not a `week` pipeline stage; does not emit MDX.

## Page table

Committed in [`src/invariants.rs`](../src/invariants.rs) as `COMPETITOR_WATCH_PAGES` (exact URL match, ≤ `COMPETITOR_WATCH_MAX_PAGES`). Initial live URLs are DeFiLlama protocol JSON objects for Terra Classic venues (Terraswap, Terraport, Astroport Classic). Adding a page is a code change + fixture test; there is no env URL override.

| id | URL | format | extractor |
| --- | --- | --- | --- |
| `defillama-terraswap` | `https://api.llama.fi/protocol/terraswap` | JSON | `defillama_protocol` |
| `defillama-terraport` | `https://api.llama.fi/protocol/terraport` | JSON | `defillama_protocol` |
| `defillama-astroport-classic` | `https://api.llama.fi/protocol/astroport-classic` | JSON | `defillama_protocol` |

Gaps (venues without a stable public JSON endpoint) are listed here over time; do not scrape random marketing HTML.

## CLI

```bash
# Fixture cassette (CI / default)
cargo run -- watch --fixtures fixtures/competitor-watch --out runs/watch/test

# Operator live fetch (not CI)
cargo run -- watch --live-watch --out runs/watch/live

cargo run -- search "liquidity" --fixtures fixtures/competitor-watch
```

Artifacts: `runs/watch/<id>/{notes.json,gaps.md,sources.json}`.

## Extractors

- **`defillama_protocol`**: `fee_bps` (optional), `currentChainTvls` / `tvl[]` for liquidity, optional `volume_24h`. Missing fields → gap lines, not invented zeros.
- **`html_fee_bps`**: `#fee-bps` or `data-fee-bps` only; strips `<script>` / `<style>` from excerpts. Used in unit tests; add a committed HTML URL only after a documented public page exists.

## Policy

- Separate allowlist from `INDEXER_HOSTS`. Forbidden: our dapp / indexer hosts (`COMPETITOR_WATCH_FORBIDDEN_HOSTS`).
- No redirects, no second-hop fetches inside JSON/HTML, body cap `COMPETITOR_WATCH_BODY_CAP`.
- Competitor numbers use `source_id` prefix `competitor-watch:` and are excluded from week `unsourced()` via `claims_for_week_emit`.
- `onchain_authoritative()` is **false** for `competitor_watch`.
