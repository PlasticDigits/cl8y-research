# Bridge indexer gap

Issue #1 requires calling a **documented** bridge-indexer HTTP API on an allowlisted host. HTML scraping of `bridge.cl8y.com` is forbidden.

As of the initial worker (2026-08-31), `cl8y-bridge-monorepo` does not publish a stable public HTTP indexer comparable to `https://indexer.dex.cl8y.com/api/v1/overview`. Canceler `/stats` on localhost is not that API.

**Invariant:** skip the bridge volume stat and record the gap in `sources.json` / `report.md`. Do not invent paths, do not scrape the dapp, do not fill numbers from Telegram or repo names.

When an official host exists, add it to `INDEXER_HOSTS` in `src/invariants.rs`, set `BRIDGE_INDEXER_BASE`, and document the path here. Collectors already no-op unless that base is configured and allowlisted.
