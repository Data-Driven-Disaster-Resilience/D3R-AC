"""
Usage: python -m d3rac_agents.cli run <agent-id>
       python -m d3rac_agents.cli run-all       # runs all python agents in dependency order
"""
from __future__ import annotations

import importlib
import sys

from dotenv import load_dotenv

from . import _generated_manifest as manifest

# Data agents first, then the risk model that joins their output.
RUN_ORDER = ["hazard-ingestor", "exposure-scorer", "vulnerability-scorer", "risk-model", "brainbox"]


def _load_agent(agent_id: str):
    entry = manifest.AGENTS[agent_id]["entrypoint"]
    module_path, class_name = entry.split(":")
    module = importlib.import_module(module_path)
    return getattr(module, class_name)()


def run(agent_id: str) -> None:
    if agent_id not in manifest.PYTHON_AGENT_IDS:
        print(f"'{agent_id}' is not a python agent. Known python agents: {manifest.PYTHON_AGENT_IDS}")
        sys.exit(1)
    agent = _load_agent(agent_id)
    results = agent.run_once()
    print(f"[{agent_id}] published {len(results)} event(s)")


def run_all() -> None:
    for agent_id in RUN_ORDER:
        run(agent_id)


def main() -> None:
    load_dotenv()  # picks up .env in the current working directory, if present
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    command = sys.argv[1]
    if command == "run-all":
        run_all()
    elif command == "run" and len(sys.argv) == 3:
        run(sys.argv[2])
    else:
        print(__doc__)
        sys.exit(1)


if __name__ == "__main__":
    main()
