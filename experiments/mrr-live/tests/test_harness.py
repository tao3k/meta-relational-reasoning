import json
import tempfile
import unittest
from pathlib import Path

from mrr_live.harness import (
    EXPECTED_FIXTURE,
    PROTOCOL_VERSION,
    TOOL_NAME,
    LiveEvaluationError,
    run_p00_smoke,
)


def response(output, *, response_id):
    return {
        "id": response_id,
        "object": "response",
        "status": "completed",
        "model": "deepseek-test-model",
        "output": output,
        "usage": {
            "input_tokens": 10,
            "input_tokens_details": {"cached_tokens": 0},
            "output_tokens": 20,
            "output_tokens_details": {"reasoning_tokens": 12},
            "total_tokens": 30,
        },
    }


class FakeModel:
    provider = "fake"

    def __init__(self, fixture_id=EXPECTED_FIXTURE, mutate_answer=None):
        self.fixture_id = fixture_id
        self.mutate_answer = mutate_answer
        self.requests = []

    def respond(self, request):
        self.requests.append(request)
        if len(self.requests) == 1:
            return response(
                [
                    {"type": "reasoning", "content": []},
                    {
                        "type": "function_call",
                        "call_id": "call-1",
                        "name": TOOL_NAME,
                        "arguments": json.dumps({"fixture_id": self.fixture_id}),
                    },
                ],
                response_id="response-1",
            )
        schema = request["text"]["format"]["schema"]
        answer = {name: rule["const"] for name, rule in schema["properties"].items()}
        if self.mutate_answer is not None:
            self.mutate_answer(answer)
        return response(
            [
                {
                    "type": "message",
                    "content": [{"type": "output_text", "text": json.dumps(answer)}],
                }
            ],
            response_id="response-2",
        )


def kernel_receipt(fixture_id):
    return {
        "schema": "mrr.live-kernel-tool-receipt",
        "schema_version": PROTOCOL_VERSION,
        "fixture_id": fixture_id,
        "rust_test": "knowledge_provenance_uses_the_shared_kernel",
        "exit_code": 0,
        "accepted": True,
        "mrr_execution_time_ms": 1.25,
        "output_sha256": "a" * 64,
    }


class P00HarnessTests(unittest.TestCase):
    def test_provider_neutral_v0_1_smoke_binds_authoritative_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            model = FakeModel()
            receipt = run_p00_smoke(
                model,
                model_name="deepseek-test-model",
                reasoning_effort="low",
                run_dir=Path(directory),
                kernel_runner=kernel_receipt,
            )
            run_dir = Path(receipt["run_dir"])

            self.assertEqual(receipt["protocol_version"], PROTOCOL_VERSION)
            self.assertEqual(receipt["status"], "PASS")
            self.assertFalse(receipt["benchmark_claim"])
            self.assertEqual(receipt["authority"], "mrr-kernel-receipt")
            self.assertEqual(len(model.requests), 2)
            self.assertEqual(model.requests[0]["reasoning"], {"effort": "low"})
            self.assertNotIn("tool_choice", model.requests[0])
            self.assertNotIn("tools", model.requests[1])
            self.assertTrue((run_dir / "trajectory.jsonl").is_file())
            self.assertTrue((run_dir / "verdict.json").is_file())

    def test_wrong_tool_argument_is_a_model_failure_before_kernel(self):
        calls = []
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        directory = temporary.name
        with self.assertRaises(LiveEvaluationError) as raised:
            run_p00_smoke(
                FakeModel("software-lifecycle"),
                model_name="test",
                reasoning_effort="low",
                run_dir=Path(directory),
                kernel_runner=lambda fixture: calls.append(fixture),
            )
        verdicts = list(Path(directory).glob("*/verdict.json"))
        self.assertEqual(len(verdicts), 1)
        failure = json.loads(verdicts[0].read_text(encoding="utf-8"))
        self.assertEqual(failure["status"], "FAIL")
        self.assertEqual(failure["failure_code"], "F02_WRONG_TOOL_ARGUMENTS")
        self.assertEqual(raised.exception.failure_code, "F02_WRONG_TOOL_ARGUMENTS")
        self.assertEqual(calls, [])

    def test_model_cannot_replace_kernel_receipt(self):
        def mutate(answer):
            answer["kernel_receipt_sha256"] = "0" * 64

        with (
            tempfile.TemporaryDirectory() as directory,
            self.assertRaises(LiveEvaluationError) as raised,
        ):
            run_p00_smoke(
                FakeModel(mutate_answer=mutate),
                model_name="test",
                reasoning_effort="low",
                run_dir=Path(directory),
                kernel_runner=kernel_receipt,
            )
        self.assertEqual(
            raised.exception.failure_code,
            "F15_IGNORED_AUTHORITATIVE_RECEIPT",
        )

    def test_invalid_kernel_schema_is_not_reinterpreted_by_model(self):
        def invalid(fixture_id):
            receipt = kernel_receipt(fixture_id)
            receipt["schema"] = "unknown"
            return receipt

        with (
            tempfile.TemporaryDirectory() as directory,
            self.assertRaises(LiveEvaluationError) as raised,
        ):
            run_p00_smoke(
                FakeModel(),
                model_name="test",
                reasoning_effort="low",
                run_dir=Path(directory),
                kernel_runner=invalid,
            )
        self.assertEqual(raised.exception.failure_code, "K02_INVALID_RECEIPT")

    def test_trajectory_is_append_only_and_run_directory_is_unique(self):
        with tempfile.TemporaryDirectory() as directory:
            first = run_p00_smoke(
                FakeModel(),
                model_name="test",
                reasoning_effort="low",
                run_dir=Path(directory),
                kernel_runner=kernel_receipt,
            )
            second = run_p00_smoke(
                FakeModel(),
                model_name="test",
                reasoning_effort="low",
                run_dir=Path(directory),
                kernel_runner=kernel_receipt,
            )
        self.assertNotEqual(first["run_id"], second["run_id"])
