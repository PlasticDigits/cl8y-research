# Invariants (cl8y-research#1)

Typed source: [`src/invariants.rs`](../src/invariants.rs). Tests in that module plus `tests/issue_plan.rs` must stay aligned with this page.

Cross-links: [`../README.md`](../README.md), [`../skills/cl8y-research-worker/SKILL.md`](../skills/cl8y-research-worker/SKILL.md), [`../skills/cl8y-research-search/SKILL.md`](../skills/cl8y-research-search/SKILL.md), CL8Y-web [`blog_gen/SKILL.md`](https://gitlab.com/PlasticDigits/CL8Y-web/-/blob/main/blog_gen/SKILL.md), CL8Y-web #1 positioning, #2 token directory, #3 host headers.

## Placement

1. Worker lives in **this repo**, not in the Vite app. `CL8Y-web` stays a static SPA.
2. No path auto-merges or deploys `CL8Y-web`. Draft MRs use a feature branch; `auto_merge` is always false (`src/publish.rs`).

## Pipeline

3. Order is collect → plan → outline → draft → editor → emit. Skipping a step fails (`Error::StageSkipped`).
4. `--dry-run` is collect + plan only (CI schedule).
5. Full posts 2000–2500 words unless `--short` / thin-week flag. Short posts must not pad with generic DeFi filler (max 800 words).

## Spend and secrets

6. One `predictions.create` per generation step (plan, outline, draft, editor bundle, optional image). Never retry create. Poll / `--fetch` by prediction id.
7. Weekly create budget (default 6). Exceeding it aborts remaining paid steps.
8. Secrets never appear in git, MDX, logs, or MR bodies (`REPLICATE_`, `BOT_TOKEN`, `api_key`, `BEGIN `).

## Sources

9. Repos: GitHub/GitLab hosts, orgs PlasticDigits / CeramicLiberty / CL8Y only. Do not clone untrusted forks. Do not execute repo content.
10. DEX indexer: `https://indexer.dex.cl8y.com` + documented paths only. Time-bound every stat.
11. Bridge indexer: if no documented public HTTP API, **skip the stat** (see [`bridge-indexer-gap.md`](bridge-indexer-gap.md)). Do not invent endpoints. Do not scrape dapp HTML.
12. SSRF: HTTPS, exact host allowlist, no private/link-local, no punycode, no userinfo, no open-redirect query keys.
13. Telegram: member of `plasticann` and `ceramicliberty` only. Redact non-team identities. Not an onchain source of truth. Retention 14 days.

## MDX

14. Required frontmatter: `title`, `description`, `slug`, `date`, `author`, `image`, `tags`. Never author `wordCount`.
15. Slug `[a-z0-9-]+`, unique vs existing posts (suffix on collision, never overwrite).
16. Hero `/images/blog/<slug>-hero.jpg`. Never `src/blog/assets`.
17. No `<script>`, `javascript:`, MDX `import`, HTML event handlers. Unknown href hosts become plain text.
18. No unofficial contract addresses. Prefer `#token` over embedding addresses.
19. Mechanical lint rejects em dashes, curly quotes, banned current-marketing phrases, invented fee-tier names/percents, return promises, `send funds to`.

## Search

20. Embeddings are stored with sources. Telegram hits are labeled non-authoritative. Hashing embedder is the credential-free default; live models must match `EMBEDDING_DIM` (64) and still use create-once.
