# Outstanding concerns

As of 2026-08-26. casts @ `5c0c818` ("Poller integration"), commit-store @
`d144c93` ("casts-driven patches"). Deployed: casts task definition
revision 8 (image `casts:5c0c818`), commit-store task definition revision 8.

This is a living list, not a backlog with owners or dates. Update it as
items get resolved or superseded rather than letting it drift out of sync
with reality.

---

## Active, undeployed bug

### The podtrac User-Agent fix has not shipped

`casts/src/archiver.rs` on disk fixes two real defects, but the change is
**uncommitted**, and the running image (`casts:5c0c818`, task definition
revision 8) predates it.

- `reqwest::get` sends no `User-Agent` by default. podtrac.com — which
  fronts audio for at least The History of Rome, and is common across
  podcasting generally — returns HTTP 400 to such a request. Confirmed live
  (`curl -A ""` reproduces it; a normal request succeeds), and confirmed the
  fix the same way.
- The failure logging only printed `{err}`, and `anyhow`'s `Display` for
  that shows just the outermost "fetching {url}" context — the actual HTTP
  status was silently dropped. Fixed to `{err:?}`, which walks the full
  chain.

**Consequence:** every podtrac-fronted entry has failed every 15-minute
archival pass since deployment, with the real cause invisible in CloudWatch
until this is deployed. Not a hypothetical — it is happening right now.

**Action:** commit `casts/src/archiver.rs`, then
`./scripts/build-and-push.sh --deploy`.

---

## Security

### No authorization, only authentication

mTLS establishes *who* may connect; nothing governs *what* a connected
caller may do. Any certificate issued by the casts intermediate (`casts-ca`)
gets full read and write access to the library. `AddFeed` is a server-side
fetch of an arbitrary URL, reachable by any accepted client — a real SSRF
primitive against your VPC, not just a theoretical one. Was flagged before
the ALB/NLB decision and accepted as the deployment's posture; still true.

### commit-store's fail-open default-CA fix is also uncommitted

Separate repo, separate bug, same category: `src/config/client.rs` used to
fall back to a baked-in `DEFAULT_CA_PEM` — actually ISRG Root X1, Let's
Encrypt's public root, despite a comment calling it a dev CA — whenever no
CA was configured. An unconfigured client therefore trusted *any* server
holding a genuine Let's Encrypt certificate, which is backwards for a system
whose entire trust model is a private CA. Fixed on disk, not committed.

### The root CA private key is still on the machine

`pki/private/root-ca/root-ca.key.pem` exists locally, contrary to
`pki/README.md`'s own instruction to move it to offline storage once the
intermediate exists and delete it from disk. Two intermediates now exist
(`intermediate-ca` for commit-store, `casts-ca` for casts); day-to-day
issuance needs neither of them to touch the root key again. This is the
right moment to actually do the offline move.

### No retention or cost cap on the archive bucket

`s3://casts-archive-690591551001/` currently holds 35 objects, ~2.71 GiB,
growing at up to 5 entries per 15-minute pass with no lifecycle policy and
no expiry. It will keep growing indefinitely once redeployed and pointed at
the rest of the back-catalogue. No decision has been made on a bound (age,
total size, per-show cap) or on deletion.

### Bulk-archiving copyright question, unresolved

Noted in `poll_fix.md` when the archiver was designed and never decided:
downloading and durably storing complete podcast back-catalogues is a
different posture than personal time-shifted listening. Fine for personal
use; worth being deliberate before this is ever multi-user or the archive
is exposed to anyone else.

---

## Operational

### Neither service has a working automatic rollback

commit-store's ECS deployment circuit breaker is confirmed **disabled**
(`deploymentCircuitBreaker.enable = false`). casts's is enabled but was
observed **not to trip** during a real crash loop earlier in this project
(`CannotPullContainerError`, retried roughly hourly with `failedTasks`
staying at 0 throughout). Effectively neither service has a safety net that
actually works if a bad deploy goes out.

### The macOS app's hang fix is unverified in practice

`CastsConnection.swift` now sets per-RPC deadlines, connection keepalive
(`permitWithoutCalls: true`), a connectivity-state logger, and OSLog
instrumentation, all aimed at the theory that a dead-but-`.ready`-looking
gRPC connection (post-sleep, network switch, VPN reconnect) was the cause of
the reported "sporadic hangs." The code compiles and the app runs with it.
What is **not** verified: whether the OSLog output is actually visible via
Console.app or `log stream` in normal use (the automation sandbox used to
build this could not see the unified log at all, confirmed with an isolated
test) and whether the hang is actually gone, since "sporadic" is hard to
force a repro of.

### The stable code-signing identity's first-run transition is unconfirmed

`casts-local-dev`, a self-signed code-signing certificate, now signs local
builds with a certificate-anchored (not content-hash-anchored) identity,
fixing the "Keychain re-prompts on every rebuild" problem structurally. But
the *existing* Keychain items were granted under the old ad-hoc signature,
so the first launch under the new identity may still need one manual
re-grant. Not yet confirmed clean end to end by actual use.

### No load testing, ever

`stopTimeout` and graceful shutdown were verified once for commit-store's
first real rolling deployment. Never verified for casts, and neither
service has been load-tested at all.

---

## Repository hygiene

### Uncommitted work

**casts** (12 paths):
```
 M .gitignore
 M casts-ios/Shared/CastsConnection.swift
 M casts-ios/Shared/FeedListViewModel.swift
 M casts-ios/Shared/PlayView.swift
 M casts/src/archiver.rs                    <- the live bug above
 M deploy/ecs/task-definition.json
AM scripts/build-app.sh
 M scripts/import-casts-credentials.sh
?? default.profraw                          <- coverage artifact, not gitignored
?? plan.md / poll_fix.md / research.md      <- working notes, not source
```

**commit-store**:
```
D  data/tls/ca.pem                          <- the fail-open fix above
 M src/config/client.rs
 M src/config/mod.rs
?? data/                                    <- now only rds-global-bundle.pem
```

**infra**: PKI script changes (`pki/scripts/init-intermediate-ca.sh`,
`issue-cert.sh`, `README.md`) and the new `casts-ca`/`intermediate-ca` dist
certs are this project's. `bastion.tf`, `lightsail.tf`, `rds.tf`,
`sbcl/*`, `variables.tf`, and whatever the tfvars change referenced earlier
are **not** — untouched and uninspected by this work, flagged here only so
they're not lost sight of when committing the PKI changes.

### `default.profraw` is untracked and not gitignored

A Rust code-coverage artifact (`-C instrument-coverage` / `llvm-cov` output)
sitting in the casts repo root. Should be deleted and added to
`.gitignore`, or the coverage run that produces it should target a
gitignored directory.

### The casts GitHub Actions workflow is dead configuration

`.github/workflows/ci.yml` checks out `duane/commit-store` from GitHub, but
casts has **no GitHub remote at all** — `origin` is SourceHut only. The
workflow has never run and cannot run as configured.

### commit-store's `github` remote can silently lag `srht`

`main` tracks `srht/main`; `github` only advances on an explicit push.
Relevant only if the (currently dead) casts CI workflow above is ever
activated — it would then build against whatever commit-store state
happens to be on GitHub, not the latest.

---

## Smaller, lower priority

- **iOS (the phone app) has no client certificate or provisioning story.**
  `CastsConnection` is shared with macOS, but nobody has issued a
  device cert or worked out how credentials reach a physical iPhone —
  materially different from macOS's `security` CLI convenience.
- **XCUITest targets are still empty templates.** No real Swift test
  coverage exists. Newly unblocked, though: they could not even run
  locally before today (no valid signing identity for the test runner);
  `casts-local-dev` may resolve that, untried.
- **`casts-server` (the GraphQL binary) carries a pinned `hyper-014`
  dependency** as a workaround for the juniper rev in use, serving a binary
  nothing currently deploys. Dead weight unless GraphQL is actually stood
  up.
- **Real, growing AWS costs**: the archive bucket (see above), the NLB, and
  the Fargate tasks are all now billed resources on top of what
  commit-store already had running.
