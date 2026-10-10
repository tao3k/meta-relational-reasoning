"""Regression checks for qualification output under CI backpressure."""

import os
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

from mrr_proof_validation import qualification


class OutputRegression(unittest.TestCase):
    def test_delivered_output_starts_silence_window_after_backpressure(self):
        forward = qualification.forward_output

        def delayed_forward(chunk, deadline):
            # Model a blocked CI sink that completes within the original
            # five-second forwarding deadline. Shutdown then remains silent
            # for less than five seconds after actual log delivery.
            time.sleep(2)
            return forward(chunk, deadline)

        producer = "import time; print('completed batch', flush=True); time.sleep(6)"
        with patch.object(qualification, "forward_output", side_effect=delayed_forward):
            self.assertEqual(qualification.qualify([sys.executable, "-c", producer]), 0)

    def test_batch_cannot_consume_callers_stdin(self):
        runner = (
            "from mrr_proof_validation.qualification import qualify; "
            "import sys; "
            "raise SystemExit(qualify([sys.executable, '-c', "
            "\"import sys; raise SystemExit(0 if sys.stdin.read() == '' else 1)\"]))"
        )
        result = subprocess.run(
            [sys.executable, "-c", runner],
            input=b"caller input must remain private",
            capture_output=True,
            timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    @unittest.skipUnless(hasattr(os, "fork"), "requires POSIX process groups")
    def test_exited_parent_does_not_admit_or_leak_pipe_holding_descendant(self):
        with tempfile.TemporaryDirectory() as directory:
            pid_path = os.path.join(directory, "descendant.pid")
            producer = (
                "import os, signal, time; "
                "signal.signal(signal.SIGTERM, signal.SIG_IGN); "
                "pid = os.fork(); "
                f"open({pid_path!r}, 'w').write(str(pid)) if pid else None; "
                "print('MODULE x\\nCASE-OK y\\nMODULE-OK x\\nOK', flush=True) "
                "if pid else None; "
                "os._exit(0) if pid else time.sleep(30)"
            )
            descendant = None
            try:
                status = qualification.qualify(
                    [sys.executable, "-c", producer], qualification.SchemeReceipt(["x"])
                )
                with open(pid_path) as stream:
                    descendant = int(stream.read())
                self.assertEqual(status, 124)
                deadline = time.monotonic() + 1
                while time.monotonic() < deadline:
                    try:
                        os.kill(descendant, 0)
                    except ProcessLookupError:
                        break
                    if sys.platform == "linux":
                        # An orphan zombie awaits init's reap but has no live
                        # execution or pipe; container init may reap later.
                        try:
                            with open(f"/proc/{descendant}/stat") as stream:
                                state = stream.read().rsplit(") ", 1)[1].split()[0]
                            if state == "Z":
                                break
                        except FileNotFoundError:
                            break
                    time.sleep(0.01)
                else:
                    self.fail("owned descendant survived timeout cleanup")
            finally:
                if descendant is None and os.path.exists(pid_path):
                    with open(pid_path) as stream:
                        descendant = int(stream.read())
                if descendant is not None:
                    try:
                        os.kill(descendant, qualification.signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    def test_exit_race_reaps_before_retrying_group_signal(self):
        child = Mock(pid=123)
        child.poll.side_effect = [None, 0]
        with patch.object(
            qualification.os, "killpg", side_effect=[PermissionError(), ProcessLookupError()]
        ) as kill:
            qualification.signal_owned_group(child, qualification.signal.SIGTERM)
        child.wait.assert_called_once_with()
        self.assertEqual(kill.call_count, 2)

    def test_live_group_permission_refusal_is_retained(self):
        child = Mock(pid=123)
        child.poll.return_value = None
        with patch.object(qualification.os, "killpg", side_effect=PermissionError()):
            with self.assertRaises(PermissionError):
                qualification.signal_owned_group(child, qualification.signal.SIGTERM)
        child.wait.assert_not_called()

    def test_short_writes_and_backpressure_preserve_exact_bytes(self):
        payload = b"MODULE x\nCASE-OK y\n\xff\nOK\n"
        forwarded = bytearray()
        calls = 0

        def write(_fd, remaining):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise BlockingIOError()
            count = min(3, len(remaining))
            forwarded.extend(remaining[:count])
            return count

        with patch.object(qualification.os, "write", side_effect=write):
            with patch.object(qualification.select, "select", return_value=([], [], [])):
                self.assertTrue(qualification.forward_output(payload, time.monotonic() + 2))
        self.assertEqual(bytes(forwarded), payload)

    def test_expired_budget_refuses_output(self):
        with patch.object(qualification.os, "write") as write:
            self.assertFalse(qualification.forward_output(b"OK\n", time.monotonic() - 1))
        write.assert_not_called()

    def test_qualify_nonblocking_stdout_preserves_failure_status(self):
        read_fd, write_fd = os.pipe()
        os.set_blocking(write_fd, False)
        payload_size = 256 * 1024
        producer = (
            f"import os; os.write(1, b'x' * {payload_size}); raise SystemExit(42)"
        )
        runner = (
            "import sys; import mrr_proof_validation.qualification as m; "
            f"raise SystemExit(m.qualify([sys.executable, '-c', {producer!r}]))"
        )
        child = subprocess.Popen([sys.executable, "-c", runner], stdout=write_fd)
        os.close(write_fd)
        received = bytearray()
        try:
            with os.fdopen(read_fd, "rb", buffering=0) as stream:
                while chunk := stream.read(4096):
                    received.extend(chunk)
                    time.sleep(0.001)
            self.assertEqual(child.wait(timeout=5), 42)
            self.assertEqual(received, b"x" * payload_size)
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()


if __name__ == "__main__":
    unittest.main()
