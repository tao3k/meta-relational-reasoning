#!/usr/bin/env python3
"""Bound native qualification to real output and propagate upstream test status."""

import argparse
import os
import selectors
import select
import signal
import subprocess
import sys
import time
from pathlib import Path


class SchemeReceipt:
    """Validate upstream reporting; gxtest still owns discovery and execution."""

    def __init__(self, expected: list[str]):
        self.expected = set(expected)
        self.completed: set[str] = set()
        self.cases: dict[str, set[str]] = {}
        self.current = ""
        self.final_ok = False
        self.failure = False
        self.pending = b""

    def observe(self, chunk: bytes) -> None:
        self.pending += chunk
        while b"\n" in self.pending:
            line, self.pending = self.pending.split(b"\n", 1)
            value = line.decode("utf-8", errors="replace").strip()
            if value.startswith("MODULE "):
                self.current = value.removeprefix("MODULE ")
            elif value.startswith("CASE-OK "):
                self.cases.setdefault(self.current, set()).add(
                    value.removeprefix("CASE-OK ")
                )
            elif value.startswith("MODULE-OK "):
                self.completed.add(value.removeprefix("MODULE-OK "))
            elif value == "OK":
                self.final_ok = True
            elif value.startswith(
                ("ERROR MODULE", "ERROR HARNESS", "CASE-FAIL", "SUITE-FAIL")
            ):
                self.failure = True

    def valid(self) -> bool:
        return (
            self.final_ok
            and not self.failure
            and self.completed == self.expected
            and all(self.cases.get(module) for module in self.expected)
        )


def forward_output(chunk: bytes, deadline: float) -> bool:
    """Forward exactly once, including short writes to CI's nonblocking stdout.

    Backpressure consumes the existing output/batch budget; it cannot create a
    heartbeat or extend qualification. Do not change shared descriptor flags.
    """
    remaining = memoryview(chunk)
    while remaining:
        if time.monotonic() >= deadline:
            return False
        try:
            written = os.write(sys.stdout.fileno(), remaining)
        except BlockingIOError:
            select.select(
                [],
                [sys.stdout.fileno()],
                [],
                min(0.1, max(0.0, deadline - time.monotonic())),
            )
            continue
        if written == 0:
            return False
        remaining = remaining[written:]
    return True


def signal_owned_group(child, signum):
    """Reap an exited direct child before signalling its owned descendants."""
    child.poll()
    try:
        os.killpg(child.pid, signum)
    except ProcessLookupError:
        pass
    except PermissionError:
        # Darwin can refuse signalling a zombie-only group. An exit may race
        # the first poll; retry after reaping, retaining any live refusal.
        if child.poll() is None:
            raise
        child.wait()
        try:
            os.killpg(child.pid, signum)
        except ProcessLookupError:
            pass


def report_owned_processes(child):
    """Capture the owned process tree once, after qualification has failed."""
    try:
        snapshot = subprocess.check_output(
            ["/bin/ps", "-axo", "pid,ppid,pgid,uid,stat,etime,command"],
            stderr=subprocess.STDOUT,
            text=True,
            timeout=1,
        )
    except (OSError, subprocess.SubprocessError) as error:
        print(f"NATIVE-DIAGNOSTIC: process snapshot unavailable: {error}", flush=True)
        return
    lines = snapshot.splitlines()
    rows = [line.split(None, 6) for line in lines[1:]]
    owned = {child.pid}
    while True:
        descendants = {
            int(row[0])
            for row in rows
            if len(row) == 7 and (int(row[1]) in owned or int(row[2]) == child.pid)
        }
        if descendants <= owned:
            break
        owned.update(descendants)
    print("NATIVE-DIAGNOSTIC: " + lines[0], flush=True)
    for line, row in zip(lines[1:], rows):
        if len(row) == 7 and int(row[0]) in owned:
            print("NATIVE-DIAGNOSTIC: " + line, flush=True)


def qualify(
    command: list[str], receipt: SchemeReceipt | None = None, *, cwd: str | None = None
) -> int:
    child = subprocess.Popen(
        command,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
        bufsize=0,
    )
    assert child.stdout is not None
    last_output = time.monotonic()
    started = last_output
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(child.stdout, selectors.EVENT_READ)
            while True:
                # Reap independently of pipe readiness: descendants can hold
                # stdout open after the direct child has exited.
                status = child.poll()
                if not selector.get_map() and status is not None:
                    break
                ready = selector.select(timeout=0.1)
                if ready:
                    chunk = os.read(child.stdout.fileno(), 65536)
                    if not chunk:
                        selector.unregister(child.stdout)
                        continue
                    last_output = time.monotonic()
                    if not forward_output(chunk, min(last_output + 5, started + 45)):
                        return 124
                    if receipt is not None:
                        receipt.observe(chunk)
                now = time.monotonic()
                if now - last_output > 5 or now - started > 45:
                    print(
                        "NATIVE-FAIL: five seconds without output or 45s batch limit "
                        f"(direct child status={child.poll()}, "
                        f"output pipe open={bool(selector.get_map())})",
                        flush=True,
                    )
                    report_owned_processes(child)
                    # A descendant may retain the pipe after its parent exits.
                    signal_owned_group(child, signal.SIGTERM)
                    return 124
        status = child.wait()
        if status == 0 and receipt is not None and not receipt.valid():
            print(
                "NATIVE-FAIL: missing successful modules, nonempty cases or final OK",
                flush=True,
            )
            return 65
        return status
    finally:
        if child.poll() is None:
            signal_owned_group(child, signal.SIGTERM)
            try:
                child.wait(timeout=1)
            except subprocess.TimeoutExpired:
                signal_owned_group(child, signal.SIGKILL)
                child.wait()
        # A reaped parent does not prove its process group is gone. In
        # particular a descendant retaining stdout may ignore timeout SIGTERM.
        signal_owned_group(child, signal.SIGKILL)
        child.stdout.close()


def scheme_command(paths: list[str]) -> list[str]:
    # String arguments are encoded as Scheme strings; no shell interpolation.
    import json

    arguments = " ".join(json.dumps(path) for path in paths)
    return [
        "gxi",
        "-e",
        '(load "proofs/MRRProof/fixtures/native-test-progress.ss")',
        "-e",
        f"(import :gerbil/tools/gxtest) "
        f'(let ((status (main "-v" "5" {arguments}))) '
        '(displayln "mrr-test: harness returned " status) (force-output) '
        '(displayln "mrr-test: exit cleanup started") (force-output) '
        "(##exit-cleanup) "
        '(displayln "mrr-test: exit cleanup returned") (force-output) '
        "(exit status))",
    ]


def self_test() -> int:
    for name, expected in [
        ("pass", 0),
        ("assertion", 42),
        ("missing", 42),
        ("empty", 65),
    ]:
        path = f"proofs/MRRProof/fixtures/native-qualification/{name}-test.ss"
        print(f"QUALIFICATION-PROBE: {name}", flush=True)
        actual = qualify(scheme_command([path]), SchemeReceipt([path]))
        if actual != expected:
            print(f"PROBE-FAIL: {name}: expected {expected}, got {actual}", flush=True)
            return 1
        print(f"PROBE-OK: {name} rejected/passed as expected ({actual})", flush=True)
    print("QUALIFICATION-PROBE: silent child", flush=True)
    actual = qualify(["gxi", "-e", "(thread-sleep! 6)"])
    if actual != 124:
        return 1
    print("PROBE-OK: silent child cutoff (124)", flush=True)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", choices=("scheme", "rust", "self-test"))
    args = parser.parse_args()
    if args.suite == "self-test":
        return self_test()
    if args.suite == "rust":
        os.environ["MRR_NATIVE_PROGRESS"] = "1"
        return qualify(
            [
                "cargo",
                "test",
                "-p",
                "mrr-gerbil",
                "--tests",
                "--locked",
                "--offline",
                "--",
                "--nocapture",
            ]
        )
    paths = [str(path) for path in sorted(Path("t").glob("*-test.ss"))]
    if not paths:
        print("NATIVE-FAIL: no Scheme test modules", flush=True)
        return 65
    return qualify(scheme_command(paths), SchemeReceipt(paths))


if __name__ == "__main__":
    raise SystemExit(main())
