# ADR 0001: Remove catch-all CODEOWNERS

Status: **Proposed** — [#17](https://git.cl8y.com/code/cl8y-research/issues/17).
Keywords in that issue are not architecture approval. Ordinary design is not
a founder card. This ADR does not authorize deploy, spend, custody rotation,
or Forgejo protection PATCH
([agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)
/ [ADR 0004](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/src/branch/main/docs/adr/0004-autonomy-policy.md)).

Date: 2026-09-21

Overview: [`architecture.md`](../architecture.md) (merge-plane section).
Org policy:
[cl8y-forgejo#48](https://git.cl8y.com/PlasticDigits/cl8y-forgejo/issues/48),
[cl8y-forgejo `docs/INVARIANTS.md`](https://git.cl8y.com/PlasticDigits/cl8y-forgejo/src/branch/main/docs/INVARIANTS.md).
CAC skip class (do not dismiss reviewers, do not delete CODEOWNERS from
drain): [cl8y-agent-control#388](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/388).
This ticket **is** the product-tree delete (forge #48 AC5); drain agents
still must not delete the file as a workaround.

No in-repo issue dependencies. Merged
[#15](https://git.cl8y.com/code/cl8y-research/issues/15) already put
`.woodpecker.yml` on `main` (required context
`ci/woodpecker/pr/woodpecker`). Open
[#16](https://git.cl8y.com/code/cl8y-research/pulls/16)
(`renovate/configure`) is unrelated occupancy.
[#12](https://git.cl8y.com/code/cl8y-research/issues/12) (Gondola / Cursor
CLI) is unrelated.

## Outcome

1. **No plant.** After land, Forgejo does not request official review from
   `@code/maintainers` (or any team/user) on every change. Files
   `CODEOWNERS`, `docs/CODEOWNERS`, and `.forgejo/CODEOWNERS` are absent.
   Outcome 1 is **no file** at those lookup paths, not “no catch-all rule
   inside a remaining CODEOWNERS file.”
2. **Merge gates unchanged.** Direct `main` stays closed. Required status
   context remains `ci/woodpecker/pr/woodpecker`. Merge stays `Do: merge`
   with `head_commit_id`. Never `force_merge`. Existing Woodpecker `cargo`
   step stays (`cargo test --workspace --locked`).
3. **No product change.** Worker pipeline, Postgres/pgvector, collectors,
   Replicate/Gondola spend, skills, fixtures, and CL8Y-web publish are
   untouched.

Author-attested live protection (authenticated GET, 2026-09-21,
design-author session) on `main`: `enable_push=false`,
`enable_status_check=true` with `ci/woodpecker/pr/woodpecker`,
`required_approvals=0`, `block_on_official_review_requests=false`,
`block_on_rejected_reviews=true`, `updated_at=2026-09-21T07:28:47Z`.
Anonymous `GET /repos/code/cl8y-research/branch_protections` returns 401
(`token is required`). Independent review cannot re-verify those flags
without a token. If live flags differ from this attestation, stop and
escalate to forge owners. Do not PATCH protection from this repo. This ADR
does **not** treat the GET as implement write permission.

## Context

`main` currently has a root [`CODEOWNERS`](../../CODEOWNERS) (commit
`291674f`) whose only rule is Forgejo Go-regexp `.* @code/maintainers`.
Forgejo plants official review requests from that file. With a one-person
maintainers team the PR author cannot approve their own pull (405 official
review / 422 self-approve). CAC autoland/drain then skip
(`DrainSkip::OfficialReview`); they must not `force_merge`, dismiss
reviewers, or delete CODEOWNERS ([#388](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/388)).

cl8y-forgejo#48 is the org reversal: stop official-review as a merge gate
on `code/*` and `PlasticDigits/*`, stop migrate/apply from re-planting
templates, keep push + Woodpecker. AC5 of that ticket is per-repo removal
of catch-all CODEOWNERS **via PR**. Protection for this repo already
matches the forge contract (attestation above). Leaving the file still
plants requests on new PRs.

This repository's occupying work is pull
[`#17`](https://git.cl8y.com/code/cl8y-research/pulls/17)
(`chore/remove-catchall-codeowners`). Issue `#17` **is** pull `#17`
(`html_url` → `/pulls/17`). Live named-branch tip is `e683015` (same as
`main`; PR reports `changed_files: 0`). Historical delete commit
`89bedca` (“Remove catch-all CODEOWNERS (not a merge gate).”) is a
fast-forward of that branch (parent `e683015`), still reachable as
`refs/pull/17/head`. Woodpecker `ci/woodpecker/pr/woodpecker` succeeded on
`89bedca` (2026-09-21T07:49:06Z, pipeline 5). Merging the empty
named-branch tip does **not** achieve Outcome 1.

Occupying leftover plant is live on `#17` and on `#16`: Reviews API team
`maintainers`, `official: true`, `REQUEST_REVIEW`, not dismissed. Drain
comments on `#17` are occupying-job / design-author queue, not
`DrainSkip::OfficialReview`. Deleting the file does not dismiss those
leftovers. “No new plant” is proven on a PR **opened after** land, not
on `#17` or `#16`.

Root Woodpecker is [`.woodpecker.yml`](../../.woodpecker.yml) (`cargo`
then, after slice 3, Alpine absence checks). That workflow posts the
required merge context. Worker product invariants stay in
[`docs/invariants.md`](../invariants.md) / `src/invariants.rs` (pipeline,
spend, sources, MDX). They are not the Forgejo merge plane. The
merge-plane table in [`architecture.md`](../architecture.md) is the local
merge contract after this ADR.

## Non-goals

- PATCH Forgejo branch protection, `apply_repo_policy.py`, or migrate
  templates (cl8y-forgejo#48).
- Edit CAC `autoland` / `merge_drain` / `autonomy.rs` / HMAC, dismiss
  reviewers (including leftovers on `#17` and `#16`), or retire
  `DrainSkip::OfficialReview` (optional CAC follow-up).
- Add path-specific CODEOWNERS, required approvals, or a second human
  reviewer.
- Drop, rename, or fake `ci/woodpecker/pr/woodpecker`; weaken the `cargo`
  step; add a required push context on PR tips; POST fake commit statuses.
- `force_merge`, direct push to `main`, or enabling `enable_push`.
- Worker runtime (`src/`), fixtures, skills, Replicate/Gondola (#12),
  Coolify (#9), Postgres store (#7), or CL8Y-web publish / `auto_merge`.
- Remount `RETIRED_HOMEPAGE_MODULES`, rewrite `CL8Y_WHITEPAPER.md`, or
  retry `predictions.create`.
- Edit `.gitlab-ci.yml` (GitLab leftover; not the Forgejo merge gate).
- Land or rewrite occupying `#16` (`renovate/configure`).
- Sibling PR `issue/17` while `#17` is open.
- Open `cac-design-issue-17` as a PR, or merge that transport ref to `main`
  (it still carries root `CODEOWNERS` until implement lands `#17`).
- Founder card for this ordinary chrome change.
- Rewriting [`README.md`](../../README.md) worker narrative, or duplicating
  this decision into [`docs/invariants.md`](../invariants.md) /
  `src/invariants.rs`.
- Add `scripts/` or `apk add` / `rg` in Woodpecker for this assertion.

## Component / state / interface changes

| Surface | Change |
| --- | --- |
| `CODEOWNERS` (root) | Delete. Do not recreate under `docs/` or `.forgejo/`. |
| `docs/architecture.md` | Merge-plane map (this change). Worker diagram stays. |
| `docs/adr/0001-remove-catchall-codeowners.md` | This decision. |
| `.woodpecker.yml` | Keep existing `cargo` step. Add exactly the Alpine step in slice 3. No `scripts/`. No `apk add`. No POST statuses. |
| `.gitignore` | Unchanged (`docs/` is not ignored here). |
| Branch protection JSON | No change from this repo. |
| `src/` / `.gitlab-ci.yml` / skills | No change. |

Forgejo CODEOWNERS lookup is root, `docs/`, or `.forgejo/` (Go-regexp, not
GitHub globs). Outcome 1 is **no file** at those three paths.

## Affected invariants

Worker invariants 1–20 in [`docs/invariants.md`](../invariants.md) (typed
in `src/invariants.rs`) stay: dedicated repo, no CL8Y-web auto-merge,
pipeline order, create-once spend, allowlists, MDX contract, search.
Those rules are not the Forgejo merge plane and must not be rewritten
for this chore.

After this ADR the merge-plane table in
[`architecture.md`](../architecture.md) is the local merge contract:

- No file at `CODEOWNERS`, `docs/CODEOWNERS`, or `.forgejo/CODEOWNERS` on `main`.
- No direct `main`; Woodpecker PR context required; no `force_merge`.
- Rejected reviews still block; official CODEOWNERS review does not.

Do not weaken cl8y-forgejo protection invariants **2–10** and **13** from
product commits. CAC invariants 29 / 67 / #388 stay: skip official-review
deadlock; never `force_merge`; drain does not delete CODEOWNERS. Product
land makes the skip class stop firing **for this repo** once new PRs have
no plant.

## Alternatives

| Option | Why not |
| --- | --- |
| Keep file; operators dismiss self-request | The plant is the bug. Manual dismiss does not scale; drain forbids dismiss. |
| Keep file; CAC dismisses or `force_merge` | Forbidden by #388 / merge policy. |
| Path-specific CODEOWNERS | Out of scope; still plants official review on matching paths; Outcome 1 forbids the file. |
| Wait for forge #48 only | Protection is already off; the file still plants requests and re-infects if protection regresses. AC5 is the product delete. |
| Required approvals = 1 | Same self-approve deadlock with a one-person team. |
| Merge empty `#17` (`e683015`) | Named branch equals `main`; CODEOWNERS stays. |
| Merge `cac-design-issue-17` to `main` | That ref still has root `CODEOWNERS`; it is not a valid Outcome 1 tip. |
| Open `issue/17` beside `#17` | Occupancy violation. |
| Wait on open `#12` or re-do merged `#15` | Unrelated / already on `main` (`e683015` is the #15 merge). |

## Complexity added / removed

**Removed:** official review plant on every change; self-approve 405/422
class for this repo's later PRs; operator dismiss-to-land ritual.

**Added:** two tracked docs files and one Alpine `test ! -f` step in
`.woodpecker.yml`. No `scripts/` helper, no new runtime, no new merge API,
no `.gitignore` carve-out, no `src/invariants.rs` change.

## Migration

Git only. No Coolify, no token move, no Postgres migration, no CL8Y-web
publish.

Land vehicle is occupying pull `#17` (`chore/remove-catchall-codeowners`).
Do **not** merge the current empty named-branch tip. Fast-forward that
branch to historical delete `89bedca` (parent is `e683015`; Woodpecker
already succeeded there), then cherry-pick accepted
`cac-design-issue-17` commit(s) on top (additive). The cherry-pick must
not restore `CODEOWNERS`. After slices 1–3 the occupying tip has: file
absent, these docs, and the Alpine step in slice 3.

`cac-design-issue-17` is design transport only: do not open it as a PR and
do not merge it to `main`.

Open PRs opened while root `CODEOWNERS` existed may still show a leftover
official request (`#17`, `#16`). Implement does **not** dismiss them.
After this lands, new PRs must not receive a CODEOWNERS plant from this
tree.

Title/body of `#17` need no extra `Fixes #17` (the pull **is** iid 17).

## Observability

- Forgejo PR “Reviews”: no official CODEOWNERS request on PRs **opened after
  land**. `#17` and `#16` are not that oracle (leftover plants remain).
- Protection GET (operator, authenticated): still `enable_push=false`,
  status context `ci/woodpecker/pr/woodpecker`,
  `block_on_official_review_requests=false`. Anonymous GET is 401. Not a CI
  job in this repo (no Forgejo admin token in Woodpecker). If flags differ
  from the author attestation above, escalate; do not PATCH from here.
- CAC drain skip comments mentioning official CODEOWNERS should stop for
  **new** occupying PRs on this path. Leftover on `#17`/`#16` is not a
  drain-skip failure of this ticket. No `/health` or `/status` change.
- Woodpecker still posts `ci/woodpecker/pr/woodpecker` on PR tips
  (`cargo` + new assertion).

## Failure modes

| Failure | Behavior |
| --- | --- |
| File deleted; leftover official request on `#17` or `#16` | Do not dismiss; do not `force_merge`. Land `#17` with tip ACCEPT + PR Woodpecker + SHA-pinned `Do: merge`. If CAC later skips `OfficialReview` on that leftover, operators use the same `Do: merge` path. |
| Merge empty `#17` without restoring `89bedca` | CODEOWNERS remains; Outcome 1 fails. Restore the delete on the occupying branch first. |
| File deleted; protection later PATCHed back to official-review block | New PRs still have no plant. Old leftover requests could 405 until they expire or a human dismisses. Re-planting CODEOWNERS is a regression. |
| CODEOWNERS re-added on a later PR | Woodpecker `no-catchall-codeowners` fails; do not merge that tip. |
| Sibling PR `issue/17` | Occupancy violation. Update `#17` only. |
| Merge `cac-design-issue-17` to `main` | Leaves the plant; contradicts Outcome 1. Transport only. |
| Implement PATCHes protection or edits CAC | Out of authority / wrong repo. |
| `force_merge` to land `#17` | Forbidden. Wait for ACCEPT + Woodpecker PR context + SHA-pinned `Do: merge`. |
| Woodpecker `rg` / `scripts/check-no-catchall-codeowners.sh` | Wrong interface. Alpine `3.20` has `test`/`sh` only. Bare `rg force_merge docs/` matches this ADR. |
| Drop or skip `cargo` while adding the assertion | Weakens the merge gate; keep the existing `cargo` step. |
| Treat `cargo test` / `src/invariants.rs` as the CODEOWNERS oracle | Wrong plane. Absence is the Alpine `test ! -f` step. |

## Ordered implementation slices

1. **Delete catch-all file** — restore root `CODEOWNERS` absence on occupying
   `#17`. Fast-forward `chore/remove-catchall-codeowners` to `89bedca`
   (already a delete-only commit on `e683015`) or replay an equivalent
   delete. Confirm `docs/CODEOWNERS` and `.forgejo/CODEOWNERS` do not
   exist. No `src/` edits. Prefer additive commits; do not open
   `issue/17`.
2. **Preserve design** — keep this ADR and `docs/architecture.md` on the
   occupying head via cherry-pick of accepted `cac-design-issue-17`
   commit(s) onto the slice-1 tip. Cherry-pick must not restore
   `CODEOWNERS`.
3. **CI assertion** — add this step to `.woodpecker.yml`; do not change
   the existing `cargo` step; do not add `scripts/`; do not `apk add`;
   do not POST statuses:

```yaml
  - name: no-catchall-codeowners
    image: alpine:3.20
    commands:
      - test ! -f CODEOWNERS
      - test ! -f docs/CODEOWNERS
      - test ! -f .forgejo/CODEOWNERS
```

Slice 1 before 3. Slice 2 is the design transport (`cac-design-issue-17`);
implement cherry-picks it onto `#17`. No CAC or forge-script slice.

## Tests

Woodpecker (only):

| Requirement | Check |
| --- | --- |
| No CODEOWNERS at Forgejo lookup paths | Step `no-catchall-codeowners` as in slice 3 (`alpine:3.20`, `test ! -f` on the three paths). Keep existing `cargo` step. |

Human / review (not Woodpecker; alpine has no `rg`; do not add `scripts/`):

| Requirement | Check |
| --- | --- |
| Catch-all rule gone | Satisfied by file absence; do not grep this ADR for the historical `.* @code/maintainers` string |
| Merge docs do not advise `force_merge: true` | Reviewers confirm product docs do not recommend enabling `force_merge`. If a docs grep is used at all, match `force_merge:\s*true` as **advice**, not any occurrence of `force_merge` (this ADR names the forbidden API). |
| Worker unchanged | No `src/` / `fixtures/` / `skills/` edits on this ticket |
| Not live Forgejo PATCH | No protection API client in this repo |
| Leftover on `#17`/`#16` is not the plant oracle | “No new plant” is a PR opened after land |
| Occupying tip is not empty | `#17` diff deletes `CODEOWNERS`; named branch is not `e683015` |

`cargo test` is not the merge-plane oracle. Do not add a Rust test that
reads `CODEOWNERS`.

## Rollout

1. Design review of this published revision (independent job). Author
   cannot approve. `cac-design-issue-17` stays unpublished as a PR.
2. Implement on occupying `#17` (slices 1–3). Autoland / operator land is
   tip ACCEPT + Woodpecker `ci/woodpecker/pr/woodpecker` + SHA-pinned
   `Do: merge`. Do not dismiss leftover official requests on `#17` or `#16`.
   If CAC later skips `OfficialReview` on that leftover, use the same
   `Do: merge` path. Never `force_merge`.
3. Operator glance (not a merge gate): a **subsequent** PR in this repo
   (opened after land) has no official CODEOWNERS request. Protection GET
   (authenticated) unchanged vs author attestation; if it differs, escalate
   to forge owners, do not PATCH from this repo.

Chat/issue cannot set protection flags.

## Rollback

Restore a CODEOWNERS file **only via PR** (never direct `main`). That
re-plants official review; do it only if a later ADR wants path owners.
Revert the Woodpecker assertion in the same PR so CI matches the file.
Do not PATCH protection to `block_on_official_review_requests=true` as
rollback of this ticket.

## Integration completion criteria

- Root / `docs/` / `.forgejo/` CODEOWNERS files absent on `main`.
- `.woodpecker.yml` contains the slice 3 Alpine step and fails closed if
  those paths return; existing `cargo` step remains.
- [`architecture.md`](../architecture.md) merge table matches Outcome 1
  (no file at the three lookup paths); this ADR remains the decision record.
- No `force_merge: true` advice in product docs; no protection PATCH; no
  CAC source edits; no worker/deploy/spend edits.
- Occupying work is still a single PR (`#17`); no sibling head; design
  transport was not merged to `main`.
- Leftover official request on `#17` (and `#16`) may remain until those PRs
  merge or expire; “no new plant” is a PR opened after land.

Live Grafana/CAC leftover cleanup is not a completion criterion here.

## Requirement-to-test mapping

See **Tests**. Implement keeps the mapping next to the Woodpecker step
(the YAML in slice 3 is the mapping).

## Cross-links

- [#17](https://git.cl8y.com/code/cl8y-research/issues/17)
- [cl8y-forgejo#48](https://git.cl8y.com/PlasticDigits/cl8y-forgejo/issues/48)
- [cl8y-forgejo INVARIANTS](https://git.cl8y.com/PlasticDigits/cl8y-forgejo/src/branch/main/docs/INVARIANTS.md)
- [cl8y-agent-control#388](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/388)
- [cl8y-agent-control#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)
- [`architecture.md`](../architecture.md)
- [code/hello#15](https://git.cl8y.com/code/hello/pulls/15) (canary delete)
