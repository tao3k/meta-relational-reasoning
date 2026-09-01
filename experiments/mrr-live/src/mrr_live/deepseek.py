"""DeepSeek Responses API adapter; never an MRR truth authority."""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from collections.abc import Mapping
from typing import Any

OFFICIAL_RESPONSES_ENDPOINT = "https://api.deepseek.com/responses"


class DeepSeekResponseError(RuntimeError):
    pass


def canonical_json(value: object) -> str:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)


class DeepSeekResponsesAdapter:
    provider = "deepseek"

    def __init__(self, api_key: str, *, timeout_seconds: float = 180.0) -> None:
        if not api_key:
            raise DeepSeekResponseError("DEEPSEEK_API_KEY is required")
        self._api_key = api_key
        self._timeout_seconds = timeout_seconds

    def respond(self, request: Mapping[str, Any]) -> dict[str, Any]:
        http_request = urllib.request.Request(
            OFFICIAL_RESPONSES_ENDPOINT,
            data=canonical_json(request).encode("utf-8"),
            headers={
                "Authorization": f"Bearer {self._api_key}",
                "Content-Type": "application/json",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(
                http_request, timeout=self._timeout_seconds
            ) as response:
                body = response.read()
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", errors="replace")[:1024]
            raise DeepSeekResponseError(
                f"DeepSeek returned HTTP {error.code}: {detail}"
            ) from error
        except urllib.error.URLError as error:
            raise DeepSeekResponseError(
                f"DeepSeek request failed: {error.reason}"
            ) from error
        try:
            decoded = json.loads(body)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise DeepSeekResponseError("DeepSeek returned non-JSON data") from error
        if not isinstance(decoded, dict):
            raise DeepSeekResponseError("DeepSeek response must be a JSON object")
        return decoded
