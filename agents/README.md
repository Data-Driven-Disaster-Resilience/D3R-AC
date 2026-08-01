# d3rac-agents

**A manifest-driven AI agent build system for [D3R·AC](https://github.com/Data-Driven-Disaster-Resilience/D3R-AC) — Data-Driven Disaster Resilience for All Communities.**

This is a standalone package you drop into the D3R·AC repo (as a top-level `agents/` folder) that adds a fleet
of AI agents implementing the risk model `R(c,t) = H(t) · E(c) · V(c)` and the response-coordination layer,
without touching the existing `contracts/`, `frontend/`, or `data-pipeline/` code.

## Why this is a "100% build system," not just agent code

Every agent — Python or Node — is declared **once**, in `agents.config.yaml`. Nothing else hand-wires agent
IDs, topics, or runtimes. A single build step:

1. **Validates** `agents.config.yaml` against `schemas/agent-manifest.schema.json`.
2. **Codegens** typed bindings into both stacks (`python/src/d3rac_agents/_generated_manifest.py` and
   `node/src/config/generatedManifest.ts`) so Python and Node agents can never drift out of sync with each
   other's topic names or IDs.
3. **Installs + builds** both the Python package (`pip`) and the Node/TypeScript workspace (`npm`).
4. **Wires the message bus** — a pluggable pub/sub interface (file-based JSONL by default for zero-dependency
   local dev, Redis in one config flag for real deployments) that both languages speak identically.

Run all four steps with:

```bash
make build
```

## What's genuinely "AI" here vs. deterministic automation

Worth being precise about this: `hazard-ingestor`, `exposure-scorer`, `vulnerability-scorer`,
and `risk-model` are deterministic — the risk math is a fixed formula, on purpose, since
predictability and auditability matter when the output can trigger a real fund release.

**`brainbox`** (`python/src/d3rac_agents/brainbox.py`) is the one agent that's genuinely
LLM-backed. It reads every triggered `risk.score` event, sends it to Claude with a system
prompt describing D3R·AC's mission, and gets back a structured directive per community
(priority, recommended action, optional milestone-percentage override, plain-language
rationale, and an alert message). `response-coordinator`, `community-alerter`, and
`contract-trigger` all read `brainbox.directive` first and only fall back to their own
built-in rule if brainbox produced nothing for that community — so a missing/failed API
call degrades gracefully instead of silently dropping a triggered community.

Set `ANTHROPIC_API_KEY` in the environment to get real Claude reasoning. Without it,
`brainbox` still runs and publishes a deterministic fallback directive (clearly tagged
`"source": "fallback-deterministic"` on every event) so CI and offline dev keep working.

## How it maps onto D3R·AC's three layers

| D3R·AC layer | This repo |
|---|---|
| **Data layer** (hazard, exposure, vulnerability signals) | `python/` data agents: `hazard_agent.py`, `exposure_agent.py`, `vulnerability_agent.py` |
| **Risk model** `R(c,t) = H(t)·E(c)·V(c)` | `python/src/d3rac_agents/risk_model.py` — subscribes to the three signals, publishes `risk.score`, fires when it crosses `θ` |
| **Smart contract layer** (TRON / Casper milestone release) | `node/src/agents/contractTriggerAgent.ts` — subscribes to `response.plan`, calls into `contracts/tron` and `contracts/casper` interfaces (stubbed — wire in your deployed contract addresses/ABIs) |
| **Community access layer** (NGOs, coordinators) | `node/src/agents/coordinationAgent.ts` (drafts a response plan) + `alertAgent.ts` (notifies community/NGO channels) |

> **Note on CI:** GitHub only reads workflows from the repo root's `.github/workflows/`.
> When you drop this folder in as `agents/`, move `agents/.github/workflows/agents-ci.yml`
> up to the main repo's `.github/workflows/agents-ci.yml` (it's already scoped to only
> trigger on changes under `agents/**`, so it won't conflict with existing workflows).

## Installing into the D3R·AC repo

```bash
# from inside a clone of Data-Driven-Disaster-Resilience/D3R-AC
git clone https://github.com/<you>/d3rac-agents agents
cd agents
make build
make dev        # runs all agents locally against the file-based bus
```

Nothing here assumes it's the only thing in the repo — it does not modify `data-pipeline/`, `contracts/`, or
`frontend/`. It's designed to eventually **replace** the placeholder `data-pipeline/` once you're happy with
it (the risk model math is the same `R(c,t)` formula from the main README), and to call into `contracts/tron`
and `contracts/casper` once those are deployed.

## Repo layout

```
d3rac-agents/
├── agents.config.yaml              # single source of truth — every agent declared here
├── schemas/agent-manifest.schema.json
├── scripts/build_manifest.py       # validates + codegens from agents.config.yaml
├── Makefile                        # make build / make dev / make test / make lint
├── docker-compose.yml              # optional: redis bus + both runtimes, for real deployments
├── .github/workflows/agents-ci.yml # CI: validates manifest, builds+tests both stacks
├── python/                         # data + risk-model agents
│   ├── pyproject.toml
│   └── src/d3rac_agents/
│       ├── bus.py                  # file/redis pub-sub, same wire format as node/src/bus
│       ├── hazard_agent.py
│       ├── exposure_agent.py
│       ├── vulnerability_agent.py
│       ├── risk_model.py
│       ├── brainbox.py             # Claude-backed central directive controller
│       └── cli.py                  # `python -m d3rac_agents.cli run <agent-id>`
└── node/                           # coordination + contract-trigger agents
    ├── package.json
    └── src/
        ├── bus/messageBus.ts
        ├── agents/coordinationAgent.ts
        ├── agents/alertAgent.ts
        ├── agents/contractTriggerAgent.ts
        └── orchestrator.ts         # `npm run start -- <agent-id>`
```

## Status

Verified in a sandbox without network access: the manifest codegen, the full Python
pipeline (hazard → exposure → vulnerability → risk-model → brainbox), and the brainbox
deterministic fallback path all run end-to-end and produce a triggered `risk.score` →
`brainbox.directive` → (would-be) `response.plan` chain from the bundled sample data.
The Node/TypeScript half (`coordinationAgent`, `alertAgent`, `contractTriggerAgent`) is
standard `tsc`/`ts-node` code but wasn't runnable in that sandbox (`npm install` needs
network access) — it will build via `make build` in any normal dev/CI environment.

What's stubbed and needs your input: real hazard-data feeds, a live `ANTHROPIC_API_KEY`
to exercise brainbox's actual Claude reasoning (vs. its tested fallback path), real
TRON/Casper contract calls in `contractTriggerAgent.ts`, and swapping the file bus for
Redis/Kafka in `docker-compose.yml` for production.
