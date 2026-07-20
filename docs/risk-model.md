# Risk Model

D3R·AC decides *when* and *where* to pre-position disaster-relief funding
using a resilience-funding priority score, `R(c, t)`, computed per
community `c` at time `t`.

## The formula

```
R(c, t) = H(t) · E(c) · V(c)
```

| Symbol | Name | Range | What it captures |
|---|---|---|---|
| `H(t)` | Hazard probability | `[0, 1]` | Likelihood of a disaster event occurring at time `t` (flood, drought, conflict displacement, etc.), from forecast/monitoring data. |
| `E(c)` | Exposure factor | `[0, 1]` | How exposed community `c` is to that hazard — population density, geography (floodplain, fault line, coastline), critical infrastructure in the hazard's path. |
| `V(c)` | Vulnerability index | `[0, 1]` | How much harm the community would suffer if the hazard hits — infrastructure resilience, socioeconomic factors, access to alternative resources, existing displacement. |

All three factors are normalized to `[0, 1]`, so `R(c, t)` is also bounded
`[0, 1]`. The multiplicative form is deliberate: a community with near-zero
exposure or near-zero vulnerability should score near zero even under high
hazard, since the point of the model is to prioritize *funding*, not just
flag risk in the abstract.

## Threshold and triggering

A threshold `θ` (theta) defines the point at which a community becomes
eligible for milestone-based fund pre-positioning:

```
R(c, t) ≥ θ  →  eligible for pre-positioning
```

The current reference implementation (`frontend/src/lib/riskModel.ts`) uses
`θ = 0.35` as an illustrative default, with two informal bands above it for
UI purposes:

- **Watch** — `R(c, t) < θ`
- **Elevated** — `θ ≤ R(c, t) < 1.8θ`
- **Critical** — `R(c, t) ≥ 1.8θ`

These bands are a display convenience, not part of the core formula. The
threshold(s) an actual deployment uses should be set by people with
disaster-response domain expertise, not hardcoded by engineering — `θ` is
a policy decision with real consequences for which communities get funded
first.

## From score to fund release

Crossing `θ` doesn't release funds by itself. The intended flow is:

1. The **data layer** computes `R(c, t)` for monitored communities from
   hazard, exposure, and vulnerability inputs.
2. When a community crosses `θ`, the **smart contract layer** is notified
   (mechanism is deployment-specific — an oracle, an authorized
   off-chain reporter, or a manual attestation, depending on what data
   feeds are actually available and trustworthy for a given deployment).
3. Funds are released **by milestone**, not as a lump sum — see the
   contract interface in [`contracts/tron/README.md`](../contracts/tron/README.md)
   for how milestones and disbursement are expected to work on-chain.
4. Every release is on-chain and auditable — the whole point of the
   architecture is that funding decisions aren't opaque.

## Current implementation status

As of this writing, `R(c, t)` is implemented in two places:

1. **The frontend dashboard** (`frontend/src/lib/riskModel.ts`), against
   **illustrative mock data** — six example communities with hand-set
   `H`, `E`, `V` values, not live hazard feeds.
2. **On-chain**, in
   [`contracts/tron/contracts/RiskRegistry.sol`](../contracts/tron/contracts/RiskRegistry.sol) —
   the same formula, computed with 1e18 fixed-point arithmetic, per
   community, with a restricted `dataFeeders` role that pushes fresh
   `H`/`E`/`V` values and a `ThresholdCrossed` event once `θ` is met or
   exceeded. Like the frontend version, this has no data source of its
   own — it's the deterministic scoring/threshold layer, waiting on real
   input.

There is no data-pipeline implementation in this repository yet (see the
top-level README's Status section), and nothing currently calls
`RiskRegistry.updateRisk` with real data — both implementations above are
ready to receive real hazard/exposure/vulnerability input, not connected
to it yet. Anyone building the real data layer should treat the mock
dataset's shape (one `H`/`E`/`V` triple per community, refreshed per `t`)
as the contract both the frontend and `RiskRegistry.sol` already expect,
and wire real sources in behind it.

## Open questions for a production deployment

These aren't answered by the formula alone and need to be decided
per-deployment:

- **Data provenance for `H(t)`**: which hazard-monitoring source(s), how
  often refreshed, and what happens when a feed is stale or unavailable.
- **Setting `E(c)` and `V(c)`**: these likely come from a mix of
  geospatial data and community-reported/socioeconomic indicators, which
  raises data-collection and privacy questions of their own.
- **Threshold governance**: who sets and can change `θ`, and how that
  decision is itself made auditable.
- **False positives/negatives**: pre-positioning funds for a hazard that
  doesn't materialize has a cost; missing a real one has a much larger
  one. The threshold and monitoring cadence should reflect that asymmetry
  deliberately, not by default.
