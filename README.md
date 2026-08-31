# cl8y-research

Dedicated **Rust + Postgres** worker for CL8Y ecosystem research.

This repo exists so [`CL8Y-web`](https://gitlab.com/PlasticDigits/CL8Y-web) can stay a **static Vite SPA**. Telegram sessions, indexer tokens, GitHub/GitLab tokens, and Replicate spend live here. Draft MDX is copied into `CL8Y-web` by a **human-reviewed** MR. Nothing in this worker pushes `CL8Y-web` `main` or auto-merges.

Implements [cl8y-research#1](https://gitlab.com/PlasticDigits/cl8y-research/-/issues/1) (moved from [CL8Y-web#4](https://gitlab.com/PlasticDigits/CL8Y-web/-/issues/4)).

## Why a new repo

`CL8Y-web` prerenders `/`, `/blog`, and `/blog/:slug`. It has no worker process and must not hold long-lived secrets. A sibling package inside that SPA would still tempt agents to import it at build time. Isolation is the default.

Canonical **voice and hero-image contract** remains:

- `CL8Y-web/blog_gen/SKILL.md`
- `CL8Y-web/blog_gen/generate_image.py` (one `predictions.create` per command, poll by id)

This worker **consumes** that contract. It does not rewrite homepage IA, `CL8Y_WHITEPAPER.md`, or `RETIRED_HOMEPAGE_MODULES`.

## One week

Fixture path (no live tokens):

```bash
cargo test
cargo run -- week --fixtures fixtures/happy-week --out runs/latest --dry-run
cargo run -- week --fixtures fixtures/happy-week --out runs/latest
```

`--dry-run` is collect → plan only (scheduler / CI schedule). A full run writes:

| Artifact | Role |
| --- | --- |
| `sources.json` | Collected records + gaps + timestamped numeric citations |
| `plan.json` | Topic, style, slug, rejected clones, cited recent posts |
| `outline.md` | Outline |
| `passes/*.md` | Named editor-pass artifacts (one Replicate create for the bundle) |
| `post.mdx` | Blog contract MDX (no authored `wordCount`) |
| `report.md` | Gaps, create count, prediction ids |
| `publish-mr.md` | Copy instructions; **no auto-merge** |

Pipeline order is enforced: collect → plan → outline → draft → editor → emit. Skipping a step fails.

## Vector search

Agents query ingested sources without treating Telegram as onchain fact:

```bash
cargo run -- search "dex volume" --fixtures fixtures/happy-week
```

Production persistence is Postgres + pgvector (`docker compose up -d`, `DATABASE_URL`, `cargo run --features postgres -- migrate`). Tests use an in-memory store and a hashing embedder so CI needs no model API.

## Live Replicate

Export `REPLICATE_API_TOKEN` and pass `--live`. **One `predictions.create` per step.** Never retry create. If the proxy drops after accept, recover with `cl8y-research fetch PREDICTION_ID`. Weekly create budget defaults to 6 (`REPLICATE_WEEKLY_CREATE_BUDGET`).

## Telegram

Official Bot API as a **member** of `t.me/plasticann` and `t.me/ceramicliberty` only. Do not scrape public web previews. Session files are `0600`. CI uses fixtures. Retention is 14 days. Non-team identities are redacted. Chat is theme/sentiment, not an onchain citation.

## Indexers

- DEX: allowlisted `https://indexer.dex.cl8y.com/api/v1/overview` (documented).
- Bridge: **no documented public HTTP indexer** in `cl8y-bridge-monorepo` at the time of this worker. The collector records a gap and skips the stat rather than inventing an API. See [`docs/bridge-indexer-gap.md`](docs/bridge-indexer-gap.md).

No HTML scrape of `dex.cl8y.com` / `bridge.cl8y.com`. Host allowlist only (SSRF-closed).

## Docs

- [`docs/invariants.md`](docs/invariants.md) — typed rules (source: `src/invariants.rs`)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/blog-contract.md`](docs/blog-contract.md) — MDX drop-in for CL8Y-web
- [`AGENTS.md`](AGENTS.md)
- Skills: [`skills/cl8y-research-worker/SKILL.md`](skills/cl8y-research-worker/SKILL.md), [`skills/cl8y-research-search/SKILL.md`](skills/cl8y-research-search/SKILL.md)

License: AGPL-3.0-only (same as CL8Y-web).
