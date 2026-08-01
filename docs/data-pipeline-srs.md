---

**PROPRIETARY — NOT FOR USE WITHOUT PERMISSION**

Copyright (c) 2026 TAAD (The Abuja Algorithmic Defenders). All rights
reserved.

This document — and the specific software requirements, interfaces,
and design decisions it describes — may not be copied, distributed,
implemented, or otherwise used, in whole or in part, without prior
written permission from TAAD (The Abuja Algorithmic Defenders).

This notice applies **in addition to**, and is even more restrictive
than, the [TAAD D3R·AC Proprietary License](../LICENSE) that already
governs the entire repository (see also
[`CONTRIBUTING.md`](../CONTRIBUTING.md)). Where the two overlap, the
more restrictive term controls.

To request permission to use this document or implement the
requirements it describes, contact TAAD (The Abuja Algorithmic
Defenders), Abuja, Nigeria.

---

# Data Pipeline — Software Requirements Specification

**Status:** Draft — no implementation exists yet.
**Component:** `data-pipeline/` (declared in the top-level
[`README.md`](../README.md)'s repository structure, not yet created).

## 1. Purpose

D3R·AC's data layer is the one piece of the architecture described in
the top-level README that has never been built. Both consumers it
needs to feed already exist and are already tested:

- **On-chain**: [`RiskRegistry.sol`](../contracts/tron/contracts/RiskRegistry.sol)
  computes `R(c,t) = H(t)·E(c)·V(c)` deterministically in 1e18
  fixed-point and fires `ThresholdCrossed` once a community meets θ —
  but nothing has ever called its `updateRisk` function with real
  data. `D3RACHub.sol`'s `updateRisk`/`registerCommunity`/
  `setRiskDataFeeder` functions (see
  [`contracts/tron/README.md`](../contracts/tron/README.md)) are the
  intended entry point once wired.
- **Frontend**: [`frontend/src/lib/riskModel.ts`](../frontend/src/lib/riskModel.ts)
  implements the same formula against a hardcoded array of six
  illustrative communities, explicitly labeled as "standing in for the
  data-pipeline layer described in the README."

This document specifies what has to exist in between: a service that
turns real hazard/exposure/vulnerability signals into the exact
`H`/`E`/`V` inputs both of the above already expect, on a cadence that
makes the on-chain threshold-crossing mechanism meaningful.

## 2. Scope

**In scope:** ingesting hazard/exposure/vulnerability source data,
computing `H(c,t)`/`E(c)`/`V(c)` per monitored community, pushing the
result on-chain via the Hub, and making the same data available for
the frontend to read instead of its current mock array.

**Out of scope:** the hazard/exposure/vulnerability data sources
themselves (which specific APIs, datasets, or reporting channels to
use is a domain decision — see §8, Open Decisions, carried over from
[`docs/risk-model.md`](risk-model.md#open-questions-for-a-production-deployment));
the Casper-chain equivalent (tracked separately, Casper contracts
don't exist yet either); any change to `RiskRegistry.sol` or
`D3RACHub.sol` themselves, both of which are considered stable
interfaces this pipeline must conform to, not extend.

## 3. Definitions

| Term | Meaning |
|---|---|
| Community | A monitored population/region, identified on-chain by a `bytes32 communityId` (`RiskRegistry.registerCommunity`'s first argument) |
| `H(t)` / `E(c)` / `V(c)` | Hazard/exposure/vulnerability inputs, each `[0,1]`, scaled to `[0, 1e18]` on-chain — see [`docs/risk-model.md`](risk-model.md) |
| Data feeder | The role (`RiskRegistry.dataFeeders`) authorized to call `updateRisk`; this pipeline's on-chain identity must hold it |
| Refresh cycle | One complete pass: fetch source data → compute H/E/V → push on-chain for every monitored community |

## 4. Functional requirements

**FR-1 — Community registry sync.** Before the pipeline can push a
risk update for a community, that community must be registered
on-chain (`RiskRegistry.registerCommunity` / `Hub.registerCommunity`).
The pipeline must maintain its own list of monitored communities
(name, region, a stable off-chain identifier it maps to `communityId`)
and register any new one before its first `updateRisk` call, not fail
silently if registration was skipped.

**FR-2 — H/E/V computation per community per cycle.** For every
monitored community, on every refresh cycle, compute `H(t)`, `E(c)`,
and `V(c)` as independent `[0,1]` values from whatever source data
FR-8 designates. `E(c)` and `V(c)` change slowly (geography,
infrastructure, socioeconomics) and may not need recomputation every
cycle; `H(t)` is time-varying by definition and should be recomputed
every cycle. The pipeline must track which of the three actually
changed since the last push and avoid a redundant on-chain write when
all three are unchanged.

**FR-3 — Fixed-point conversion matching the contract exactly.** Convert
each `[0,1]` float to the `uint256` `[0, 1e18]` range
`RiskRegistry.updateRisk` expects, using the same rounding convention
throughout the pipeline (specify and document it — e.g. round-half-up)
so repeated runs against unchanged input produce an identical on-chain
value, not a value that drifts by a wei-equivalent each cycle. Values
must be clamped to `[0, 1e18]` before submission —
`RiskRegistry.updateRisk` reverts on out-of-range input
(`"RiskRegistry: value out of [0,1] range"`), and a reverted pipeline
transaction must be treated as a failed cycle for that community (see
FR-6), not silently skipped.

**FR-4 — On-chain submission via the Hub.** Risk updates are submitted
by calling `Hub.updateRisk(communityId, hazard, exposure,
vulnerability)`, not `RiskRegistry.updateRisk` directly — consistent
with the Hub being the intended single operational surface (see
[`contracts/tron/README.md`](../contracts/tron/README.md)'s "The Hub"
section). The pipeline's on-chain identity must be granted data-feeder
status on `RiskRegistry` (directly or via `Hub.setRiskDataFeeder`,
which itself requires the Hub to hold `RiskRegistry`'s owner role —
see "Wiring the Hub" in the same document) before its first submission
attempt; a submission that reverts with
`"RiskRegistry: caller is not a data feeder"` indicates a deployment/
wiring defect, not a data problem, and must be surfaced distinctly
from a data-validation failure.

**FR-5 — Threshold-crossing awareness, not re-implementation.** The
pipeline does not decide whether a community has crossed θ — that
logic already lives on-chain in `RiskRegistry.updateRisk`, which fires
`ThresholdCrossed` itself. The pipeline must listen for its own
`ThresholdCrossed` events (or poll `RiskRegistry.isAboveThreshold`)
purely for its own logging/alerting, and must never gate whether it
*submits* an update on its own re-derivation of the threshold
comparison — submitting the true computed value and letting the
contract decide is the whole point of keeping this deterministic
on-chain.

**FR-6 — Per-community failure isolation.** One community's data
source being unavailable, invalid, or its on-chain submission
reverting must not block submission for any other community in the
same refresh cycle. Each community's fetch → compute → submit is an
independent unit of work; failures are logged per-community with
enough detail (which stage failed, on-chain revert reason if
applicable) to diagnose without re-running the whole cycle.

**FR-7 — Stale-data policy.** If a community's hazard source hasn't
returned fresh data within a configurable staleness window, the
pipeline must not silently keep resubmitting the last-known value as
if it were current — this is a policy decision the risk-model doc
explicitly leaves open (§8), so the requirement here is that the
pipeline makes the choice **visible and configurable** per deployment
(e.g. a `staleAfter` duration triggering either "hold last known value
and flag it stale" or "stop submitting for this community until fresh
data arrives"), not that it makes the choice for every deployment.

**FR-8 — Source-agnostic ingestion interface.** Hazard/exposure/
vulnerability source integrations (which specific feeds, APIs, or
reporting channels) are out of scope for this document (§2) and are
expected to vary per deployment. The pipeline must define a stable
internal interface — one function/adapter per data category, each
returning a `[0,1]` value (or "no fresh data") per community — so a
specific source integration is swappable without touching FR-1
through FR-7's logic.

**FR-9 — Frontend read-path.** `frontend/src/lib/riskModel.ts`'s
`COMMUNITIES` array must eventually be replaceable by a read of actual
on-chain state (`RiskRegistry.getCommunity` /
`Hub.systemStatus`'s community count, or an indexed/cached read of the
same data for latency reasons) without changing `riskScore`/
`riskTier`'s signatures or the `Community` interface's shape — those
are the contract this pipeline and the existing frontend code both
already conform to (per `docs/risk-model.md`'s own note). This document
does not require the frontend migration itself, only that the pipeline
not produce data in a shape that would force a breaking change to it.

## 5. Non-functional requirements

**NFR-1 — Feeder key custody.** The private key authorizing on-chain
submissions is a real operational secret — anyone holding it can push
arbitrary `H`/`E`/`V` values for any registered community, which
directly gates fund pre-positioning. It must not be stored in plaintext
in application config, source control, or logs; use a secrets manager
or HSM appropriate to the deployment environment.

**NFR-2 — Idempotency.** Re-running a refresh cycle after a partial
failure (some communities submitted, some not) must not double-submit
for communities that already succeeded, and must correctly retry only
the ones that failed.

**NFR-3 — Auditability.** Every on-chain submission this pipeline makes
is already publicly auditable via `RiskUpdated` events (emitted by
`RiskRegistry.updateRisk`) — the pipeline itself must additionally log,
off-chain, which source data produced each submitted value, so a
questioned figure can be traced back to its input, not just to "the
pipeline said so."

**NFR-4 — Observability.** Per-cycle summary (communities processed,
succeeded, failed, skipped-as-unchanged) must be emitted somewhere an
operator can actually see it — this is the mechanism that would surface
a stuck or misconfigured pipeline before it silently stops updating
risk scores for weeks.

## 6. Interfaces this pipeline depends on

All already implemented and tested (115/115 passing, see
[`contracts/tron/README.md`](../contracts/tron/README.md)) — this
pipeline is a consumer of these, not a modifier:

```solidity
// D3RACHub.sol
function registerCommunity(bytes32 communityId, string calldata name_, string calldata region) external;
function updateRisk(bytes32 communityId, uint256 hazard, uint256 exposure, uint256 vulnerability) external;
function setRiskDataFeeder(address feeder, bool isFeeder) external; // deployment-time wiring, not per-cycle

// RiskRegistry.sol (read-only, for verification / frontend read-path)
function riskScore(bytes32 communityId) external view returns (uint256);
function isAboveThreshold(bytes32 communityId) external view returns (bool);
function getCommunity(bytes32 communityId) external view returns (...);
```

## 7. Acceptance criteria

- [ ] A registered community's `H`/`E`/`V` values on-chain, read back
      via `RiskRegistry.getCommunity`, match the pipeline's computed
      values exactly (no rounding drift) for a full refresh cycle.
- [ ] A community crossing θ during a cycle results in an observable
      `ThresholdCrossed` event with the correct `riskScore`.
- [ ] Killing/corrupting one community's data source during a cycle
      does not prevent other communities from being updated (FR-6).
- [ ] Re-running a cycle after a mid-cycle crash does not double-submit
      for already-succeeded communities (NFR-2).
- [ ] The feeder key is never present in plaintext in logs, config
      files committed to source control, or error messages (NFR-1).
- [ ] A stale-data scenario (source returns no fresh data past the
      configured window) produces the deployment-configured behavior
      (hold-and-flag or stop-submitting), not a silent resubmission of
      old data as if fresh.

## 8. Open decisions

Carried over from [`docs/risk-model.md`](risk-model.md#open-questions-for-a-production-deployment)
— this document specifies pipeline *behavior*, not these
domain/policy calls, which need people with disaster-response
expertise, not engineering defaults:

- Which specific hazard-monitoring source(s) feed `H(t)`, and their
  refresh cadence.
- How `E(c)` and `V(c)` are actually derived (geospatial data,
  community-reported indicators, or a mix) and the data-collection/
  privacy questions that come with the latter.
- The staleness window for FR-7, and which of the two documented
  policies (hold-and-flag vs. stop-submitting) a given deployment
  should default to.
- Who is authorized to hold the data-feeder key operationally, and
  what happens if it needs to be rotated (the Hub's
  `setRiskDataFeeder` supports adding a new feeder and removing the
  old one, but the operational runbook for doing so under this
  pipeline's automation doesn't exist yet).
