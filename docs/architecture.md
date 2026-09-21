# Architecture

Worker pipeline (product) is below. Merge and review gate:
[ADR 0001](adr/0001-remove-catchall-codeowners.md).

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

## Merge plane

Protected `main` is the only release branch. The merge contract is:

| Gate | Contract |
| --- | --- |
| Direct push | Off (`enable_push: false`) |
| Status check | `ci/woodpecker/pr/woodpecker` required (root `.woodpecker.yml`: `cargo`) |
| Official CODEOWNERS review | Not a merge gate. No file at `CODEOWNERS`, `docs/CODEOWNERS`, or `.forgejo/CODEOWNERS`. |
| `force_merge` | Forbidden |
| Approvals | `required_approvals: 0`; rejected reviews still block |

Catch-all CODEOWNERS removal: [ADR 0001](adr/0001-remove-catchall-codeowners.md)
([#17](https://git.cl8y.com/code/cl8y-research/issues/17)). Forge policy that
keeps push/status protection and drops official-review block is
[cl8y-forgejo#48](https://git.cl8y.com/PlasticDigits/cl8y-forgejo/issues/48)
and
[cl8y-forgejo `docs/INVARIANTS.md`](https://git.cl8y.com/PlasticDigits/cl8y-forgejo/src/branch/main/docs/INVARIANTS.md).
This product tree does not PATCH Forgejo protection and does not edit CAC.

Worker product invariants stay in [`invariants.md`](invariants.md). Branch
protection is operator-owned. Product PRs must not reintroduce
`CODEOWNERS`, `docs/CODEOWNERS`, or `.forgejo/CODEOWNERS` (Forgejo lookup
paths; Go-regexp, not GitHub globs). Land vehicle for ADR 0001 is occupying
pull `#17`; `cac-design-issue-17` is design transport only (do not merge it
to `main`). The named branch is empty vs `main` until implement restores
delete commit `89bedca`.

Spend, Coolify, custody, and CL8Y-web publish remain out of this merge-plane
section
([agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)).
