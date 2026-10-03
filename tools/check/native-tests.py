#!/usr/bin/env python3
"""Bound native qualification to real output and propagate upstream test status."""

import argparse
import os
import selectors
import signal
import subprocess
import sys
import time


def qualify(command: list[str]) -> int:
    child = subprocess.Popen(
        command,
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
            while selector.get_map():
                ready = selector.select(timeout=0.1)
                if ready:
                    chunk = os.read(child.stdout.fileno(), 65536)
                    if not chunk:
                        selector.unregister(child.stdout)
                        continue
                    sys.stdout.buffer.write(chunk)
                    sys.stdout.buffer.flush()
                    last_output = time.monotonic()
                if child.poll() is None:
                    now = time.monotonic()
                    if now - last_output > 5 or now - started > 45:
                        print(
                            "NATIVE-FAIL: five seconds without output or 45s batch limit",
                            flush=True,
                        )
                        return 124
        return child.wait()
    finally:
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=1)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
        child.stdout.close()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("suite", choices=("scheme", "rust"))
    args = parser.parse_args()
    if args.suite == "rust":
        os.environ["MRR_NATIVE_PROGRESS"] = "1"
        command = [
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
    else:
        command = [
            "gxi",
            "-e",
            '(load "tools/check/native-test-progress.ss")',
            "-e",
            '(import :gerbil/tools/gxtest) (exit (main "-v" "5" "t"))',
        ]
    return qualify(command)


if __name__ == "__main__":
    raise SystemExit(main())
