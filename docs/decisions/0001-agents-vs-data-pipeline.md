# Decision needed: `agents/` vs `data-pipeline/`

**Status:** open — needs a call from TAAD, not decided by this document.

## The overlap

Two things in this repo currently implement (or plan to implement) the same
job — turning hazard/exposure/vulnerability signals into `R(c,t) =
H(t)·E(c)·V(c)` and pushing risk data on-chain:

| | `data-pipeline/` | `agents/python/` |
|---|---|---|
| Status | Implemented, 32 passing tests | Data agents (`hazard_agent.py`, `exposure_agent.py`, `vulnerability_agent.py`, `risk_model.py`) exist; verified end-to-end in a no-network sandbox with sample data |
| Hazard sources | 4 real adapters wired: NASA FIRMS, USGS, NASA EONET, GDACS | Not specified — agents/README.md doesn't describe real source wiring |
| On-chain submission | `chain_client.py` (tronpy) calls `Hub.updateRisk` directly | `contractTriggerAgent.ts` is explicitly stubbed, waiting on deployed addresses/ABIs |
| Extra capability | None beyond the risk pipeline itself | `brainbox` — an LLM-backed (Claude) directive layer that reads `risk.score` events and produces priority/action/rationale per community, with a deterministic fallback if the API call fails or the key is absent |
| Own README's stated intent | Describes itself as the data layer, no mention of being superseded | States explicitly: **"designed to eventually replace the placeholder `data-pipeline/`"** |

Right now both exist in the repo simultaneously, and nothing has decided
which one is canonical going forward.

## Why this matters before more work goes into either

- Real engineering time (adapters, tests, on-chain wiring) shouldn't be
  duplicated in two places that will only converge later.
- `agents/`'s own README calls `data-pipeline/` a "placeholder," but
  `data-pipeline/` is the more complete implementation today (real,
  tested hazard adapters vs. an unspecified data-sourcing story on the
  agents side). That description may already be stale.
- `brainbox` is a genuinely new capability neither the original
  architecture nor `data-pipeline/` has: an LLM producing a
  human-readable rationale and priority call per community, on top of
  the deterministic `R(c,t)` score. That's worth a deliberate decision
  about whether/how it fits, not an accidental byproduct of picking a
  data layer.

## Options, not a recommendation

This is TAAD's call, since it's a product/architecture direction
question, not a bug:

1. **`agents/python`'s data agents replace `data-pipeline/` entirely.**
   `data-pipeline/`'s tested adapters and on-chain submission logic would
   need to be ported over (or the agents rewritten to match) before
   retiring it, so nothing regresses.
2. **`data-pipeline/` stays canonical for hazard→on-chain**, and
   `agents/` is scoped down to what it uniquely adds:
   `brainbox`, `response-coordinator`, `community-alerter`,
   `contract-trigger` — i.e., everything downstream of a risk score,
   not the scoring pipeline itself. `agents/python`'s
   `hazard_agent.py`/`exposure_agent.py`/`vulnerability_agent.py`/
   `risk_model.py` would then be redundant with `data-pipeline/` and
   likely removed or reduced to a thin adapter that reads
   `data-pipeline/`'s output.
3. **Something in between** — e.g., `data-pipeline/` remains the source
   of truth for `H(t)`/`E(c)`/`V(c)` and on-chain submission, while
   `agents/`'s risk-model agent is dropped in favor of subscribing to
   the same on-chain `RiskRegistry` events `data-pipeline/` already
   writes.

## What happens next

Whichever option TAAD picks, update both `README.md`s to stop describing
each other ambiguously, and this file should be updated with the decision
and date once made.
