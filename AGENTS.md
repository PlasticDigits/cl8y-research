# Agent notes (cl8y-research)

Rust + Postgres research worker. Not the marketing site.

1. [`skills/cl8y-research-worker/SKILL.md`](skills/cl8y-research-worker/SKILL.md) — weekly draft pipeline
2. [`skills/cl8y-research-search/SKILL.md`](skills/cl8y-research-search/SKILL.md) — vector search
3. [`docs/invariants.md`](docs/invariants.md) and [`src/invariants.rs`](src/invariants.rs)
4. Forgejo merge plane (no catch-all CODEOWNERS): [`docs/architecture.md`](docs/architecture.md), [ADR 0001](docs/adr/0001-remove-catchall-codeowners.md)
5. Voice remains canonical in **CL8Y-web** [`blog_gen/SKILL.md`](https://gitlab.com/PlasticDigits/CL8Y-web/-/blob/main/blog_gen/SKILL.md)
6. Addresses remain canonical in **CL8Y-web** GitLab #2 / `src/data/tokenDirectory.ts`

Do not auto-publish to `CL8Y-web` `main`. Do not remount `RETIRED_HOMEPAGE_MODULES`. Do not rewrite `CL8Y_WHITEPAPER.md`. Do not retry `predictions.create`.

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo test --all-targets
```
