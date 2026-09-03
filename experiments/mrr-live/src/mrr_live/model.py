"""Provider-neutral model boundary for MRR live experiments."""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Protocol


class AgentModel(Protocol):
    """One stateless model turn; the harness owns the full trajectory."""

    provider: str

    def respond(self, request: Mapping[str, Any]) -> dict[str, Any]: ...
