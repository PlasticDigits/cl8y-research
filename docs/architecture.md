# Architecture

```
allowlisted remotes / indexer hosts / Telegram rooms
        │
        ▼
   collect (graceful gaps)
        │
        ▼
  Postgres + pgvector   ◄── agent search
  (or MemoryStore in tests)
        │
        ▼
 collect → plan → outline → draft → editor → emit
        │
        ▼
  runs/<week>/{plan.json,outline.md,passes,post.mdx,report.md}
        │
        ▼
  human copies MDX + hero into CL8Y-web feature branch
```

Modules:

| Module | Responsibility |
| --- | --- |
| `allowlist` | SSRF, href hosts, repo orgs, canonical addresses |
| `collect` | Fixture + live GET of allowlisted JSON only |
| `replicate` | Create-once predictor, budget, untrusted wrap |
| `pipeline` | Stage machine, diversity ranker, emit |
| `lint` / `mdx` | Mechanical editor + blog contract |
| `telegram` | Redaction + theme aggregates |
| `store` | Memory + optional Postgres/pgvector |
| `publish` | Draft MR payload that cannot auto-merge |

Live HTTP never follows redirects and never fetches user-controlled URLs.
