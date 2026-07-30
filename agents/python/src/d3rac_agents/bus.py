"""
Pluggable pub/sub bus. Default transport is a file-based JSONL append log (zero
dependencies, good for local dev and CI). Same wire format (topic, payload dict) as
node/src/bus/messageBus.ts, so a Python agent's publish is directly readable by a Node
agent's subscribe and vice versa.

Swap to Redis for production by setting `bus.transport: redis` in agents.config.yaml
and re-running `make build` -- RedisBus below implements the same interface.
"""
from __future__ import annotations

import json
import time
from collections.abc import Iterator
from pathlib import Path

from . import _generated_manifest as manifest

# agents/python/src/d3rac_agents/bus.py -> parents[3] == agents/ (the package root
# shared with node/). Used to anchor relative bus paths so they don't depend on the
# process's current working directory, which differs between `python3 -m d3rac_agents.cli`
# (run from agents/) and `npm run start` (run from agents/node/) in the Makefile's `make dev`.
_AGENTS_ROOT = Path(__file__).resolve().parents[3]


class FileBus:
    def __init__(self, base_dir: str | None = None):
        raw = Path(base_dir or manifest.BUS_FILE_DIR)
        self.dir = raw if raw.is_absolute() else _AGENTS_ROOT / raw
        self.dir.mkdir(parents=True, exist_ok=True)

    def _topic_path(self, topic: str) -> Path:
        safe = topic.replace("/", "_")
        return self.dir / f"{safe}.jsonl"

    def publish(self, topic: str, payload: dict) -> None:
        event = {"topic": topic, "ts": time.time(), "payload": payload}
        with self._topic_path(topic).open("a") as f:
            f.write(json.dumps(event) + "\n")

    def read_all(self, topic: str) -> Iterator[dict]:
        path = self._topic_path(topic)
        if not path.exists():
            return
        with path.open() as f:
            for line in f:
                if line.strip():
                    yield json.loads(line)

    def tail_new(self, topic: str, since_index: int = 0) -> list[dict]:
        """Return events published after `since_index` for simple polling loops."""
        events = list(self.read_all(topic))
        return events[since_index:]


class RedisBus:
    """Drop-in replacement for FileBus. Requires `redis` package + running Redis."""

    def __init__(self, url: str | None = None):
        import redis  # local import so file-bus users don't need the dependency

        self.client = redis.Redis.from_url(url or manifest.BUS_REDIS_URL)

    def publish(self, topic: str, payload: dict) -> None:
        self.client.rpush(topic, json.dumps({"topic": topic, "ts": time.time(), "payload": payload}))

    def read_all(self, topic: str) -> list[dict]:
        return [json.loads(x) for x in self.client.lrange(topic, 0, -1)]

    def tail_new(self, topic: str, since_index: int = 0) -> list[dict]:
        return [json.loads(x) for x in self.client.lrange(topic, since_index, -1)]


def get_bus():
    if manifest.BUS_TRANSPORT == "redis":
        return RedisBus()
    return FileBus()
