# D3R·AC Data Pipeline

Implements [`docs/data-pipeline-srs.md`](../docs/data-pipeline-srs.md): the
one layer of D3R·AC's architecture that turns real-world hazard signals
into the `H(t)` / `E(c)` / `V(c)` inputs `RiskRegistry.sol` and
`D3RACHub.sol` already expect, and pushes them on-chain.

**Status:** implemented and unit-tested (24 passing tests), **not yet run
against a deployed Hub/RiskRegistry** — do that only after
`contracts/tron` is deployed to a testnet per
[`docs/deployment-guide.md`](../docs/deployment-guide.md), and after this
pipeline's on-chain address has been granted data-feeder status
(`Hub.setRiskDataFeeder`).

## What this actually does — and doesn't

**Does:** pulls real, live hazard data from four satellite/sensor sources,
combines it per community into `H(t)`, converts to the contract's fixed-point
format, and submits via `Hub.updateRisk` — with per-community failure
isolation, idempotency, stale-data handling, and an audit trail, per the
SRS's functional/non-functional requirements (FR-1 through FR-9, NFR-1
through NFR-4).

**Doesn't:** predict a disaster from raw satellite imagery before any
sensor has detected anything. "Before it happens" here means *early
warning from the fastest available real signal* (a hot pixel from a
polar-orbiting satellite, a seismic swarm's first tremors, a storm's
tracked path) — reacting to genuine present-tense signal faster than a
human news cycle, not forecasting from first principles. A true
imagery-based predictive model (e.g. training on historical satellite
scenes to forecast flood risk weeks out) is a much larger ML project and
isn't what's built here; treat `docs/risk-model.md`'s open questions as
the place to scope that if it's wanted later.

## Hazard sources (satellite/sensor adapters)

| Adapter | Source | Coverage | Key required? |
|---|---|---|---|
| `satellite_fire` | NASA FIRMS (VIIRS/MODIS active-fire detection) | Global | Yes — free, see below |
| `seismic_usgs` | USGS Earthquake Catalog | Global | No |
| `eonet_events` | NASA EONET (floods, storms, volcanoes, ...) | Global | No |
| `gdacs_alerts` | GDACS (UN OCHA/JRC severity-scored alerts) | Global | No |

All four run for every monitored community every cycle; results are
combined into a single `H(t)` per `hazard_combine_strategy` in
`config/settings.yaml` (default: `max` — the single most severe active
signal drives the score, matching how response prioritization actually
works). A source with nothing to report for a bbox returns `0.0`, not an
error; a source that's unreachable is logged and skipped without
blocking the other three (FR-6).

Get a free FIRMS key at https://firms.modaps.eosdis.nasa.gov/api/map_key/
and put it in `.env` as `NASA_FIRMS_MAP_KEY`. Without it, `satellite_fire`
is skipped gracefully (not an error) and the other three sources still run.

`E(c)` and `V(c)` are currently static, curated placeholders read from
`config/communities.yaml` — see the module docstrings in
`src/d3rac_pipeline/adapters/static_exposure.py` and
`static_vulnerability.py` for why, and `docs/data-pipeline-srs.md §8` for
the open decision on how to compute these for real.

## Africa prioritization

`config/communities.yaml` tags each community `priority_region: africa` or
`global`. `config.load_communities()` sorts Africa-tagged communities
first; every cycle processes them before any global community, and
Africa communities are never dropped if a run needs to shed load. This
does not mean non-African communities are unmonitored — it's a
processing-order guarantee, not a filter. All six communities shipped by
default are in Nigeria (matching the illustrative dataset the frontend
already uses); add more anywhere in `config/communities.yaml`.

## Setup

```bash
cd data-pipeline
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
cp .env.example .env
# edit .env: NASA_FIRMS_MAP_KEY, and (only once you're ready to submit
# on-chain) D3RAC_FEEDER_PRIVATE_KEY + D3RAC_HUB_ADDRESS
```

## Running

```bash
# Dry-run: fetches real hazard data, computes H/E/V, logs what *would* be
# submitted. No secrets required, no on-chain transaction.
python scripts/run_cycle.py

# Live: actually calls Hub.updateRisk. Requires .env configured with a
# funded, data-feeder-authorized TRON account.
python scripts/run_cycle.py --submit
```

Run it on a schedule (cron, a scheduled CI workflow, etc.) — see
`docs/data-pipeline-srs.md`'s "Refresh cycle" definition. There's no
built-in scheduler; this is intentionally a single-shot script so any
scheduler can own retry/backoff policy.

## Tests

```bash
pytest tests/
```

24 tests covering fixed-point conversion accuracy (FR-3, including the
exact `0.35 → 350000000000000000` example from `RiskRegistry.sol`'s own
constructor comment), community-id derivation, hazard combination
strategies, and full pipeline behavior (unchanged-value skipping,
per-community failure isolation, stale-data policies, Africa-first
ordering) using fake adapters and a dry-run (no-chain) pipeline — no
network access or deployed contract required to run the suite. CI runs
this automatically (`.github/workflows/d3rac-ci.yml`'s `data-pipeline`
job) on any push touching `data-pipeline/**`.

Live adapter calls (actual NASA/USGS/GDACS endpoints) are not exercised
by the test suite — they're integration points, mocked out in tests via
fake adapters implementing the same `HazardAdapter` interface. Verify
against the real endpoints manually (`python scripts/run_cycle.py`, no
`--submit`) before a first live deployment.

## Architecture

```
config/
  communities.yaml   # monitored communities, bboxes, Africa priority, static E/V
  settings.yaml       # staleness policy, combine strategy, chain config (non-secret)
src/d3rac_pipeline/
  adapters/           # FR-8: one file per source, all implementing HazardAdapter
  config.py           # loads the two YAML files above
  community_id.py     # off-chain slug -> on-chain bytes32 communityId (keccak256)
  fixed_point.py       # FR-3: [0,1] float -> [0,1e18] uint256, round-half-up, clamped
  hazard_aggregator.py # combines multiple HazardReadings into one H(t)
  state_store.py       # SQLite: last-submitted values (FR-2) + cycle progress (NFR-2)
  chain_client.py       # tronpy wrapper around Hub.registerCommunity / updateRisk
  audit_log.py          # NFR-3/NFR-4: append-only JSONL trace + per-cycle summary
  pipeline.py            # orchestrates one full refresh cycle (FR-1 through FR-7)
scripts/run_cycle.py     # CLI entrypoint
tests/                    # 24 tests, no network/chain dependency
```

## Security

- The feeder private key (`D3RAC_FEEDER_PRIVATE_KEY`) is never read from a
  committed file — only from the environment (see NFR-1 in the SRS).
  `.env` is git-ignored; use a real secrets manager in production.
- Fixed-point conversion clamps rather than silently overflowing;
  out-of-range submissions still ultimately revert on-chain
  (`RiskRegistry: value out of [0,1] range`) as the final backstop.
- This pipeline has **not** been security-reviewed for production use with
  real funds behind it — same status as the rest of the repo (see the
  top-level README's Security section).
