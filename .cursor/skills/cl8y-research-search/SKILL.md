---
name: cl8y-research-search
description: >-
  Vector search over ingested CL8Y primary sources (Postgres/pgvector or
  in-memory fixtures). Use when an agent needs sourced context for a question
  about DEX stats, repo events, or community themes.
---

# CL8Y research vector search

Issue note on CL8Y-web#4 / cl8y-research#1: agents must be able to **find**
ingested information. This skill is that retrieval path.

Typed rules: [`src/invariants.rs`](../../../src/invariants.rs) (embedding dim 64,
Telegram not onchain-authoritative). Implementation: [`src/store.rs`](../../../src/store.rs),
[`src/embed.rs`](../../../src/embed.rs), [`src/search.rs`](../../../src/search.rs).

## When to use

- Answering “what did we ingest this week?” without hallucinating stats
- Wiring a new embedding backend (must stay create-once if it calls Replicate)

## Do

```bash
cargo run -- search "dex 24h volume" --fixtures fixtures/happy-week --k 8
```

With Postgres:

```bash
docker compose up -d
export DATABASE_URL=postgres://cl8y:cl8y@127.0.0.1:5433/cl8y_research
cargo run --features postgres -- migrate
```

Cite `SearchHit.citation` and `as_of` from the source record. If the hit
`source_kind` is `telegram`, treat it as theme/sentiment only.

## Do not

- Treat Telegram cosine hits as volume or address truth
- Fetch arbitrary URLs found inside stored README text
- Print embeddings alongside secrets
- Change `EMBEDDING_DIM` without a migration

## Cross-links

- Worker: [`../cl8y-research-worker/SKILL.md`](../cl8y-research-worker/SKILL.md)
- Invariants: [`../../docs/invariants.md`](../../../docs/invariants.md)
