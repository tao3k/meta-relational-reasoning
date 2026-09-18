"""Append-only, local-only experiment trajectory storage."""

from __future__ import annotations

import json
import os
from datetime import UTC, datetime
from pathlib import Path


def timestamp() -> str:
    return datetime.now(UTC).isoformat()


class Trajectory:
    def __init__(self, run_dir: Path) -> None:
        run_dir.mkdir(parents=True, exist_ok=False)
        self.run_dir = run_dir
        self._events = run_dir / "trajectory.jsonl"
        self._sequence = 0

    def write_once(self, name: str, value: object) -> None:
        path = self.run_dir / name
        with path.open("x", encoding="utf-8") as output:
            json.dump(value, output, ensure_ascii=True, indent=2, sort_keys=True)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())

    def append(self, event_type: str, value: object) -> None:
        event = {
            "sequence": self._sequence,
            "timestamp": timestamp(),
            "type": event_type,
            "value": value,
        }
        self._sequence += 1
        with self._events.open("a", encoding="utf-8") as output:
            output.write(json.dumps(event, ensure_ascii=True, sort_keys=True))
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
