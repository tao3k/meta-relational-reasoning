"""Run an owned process with real output or measured CPU progress."""

import argparse
from dataclasses import dataclass
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time

from . import native_tests


@dataclass(frozen=True)
class Result:
    status: int
    output: bytes
    log: Path | None


def _cpu_seconds(value: str) -> float:
    days = 0
    if "-" in value:
        raw_days, value = value.split("-", 1)
        days = int(raw_days)
    parts = [float(part) for part in value.split(":")]
    if len(parts) == 2:
        hours, minutes, seconds = 0, parts[0], parts[1]
    elif len(parts) == 3:
        hours, minutes, seconds = parts
    else:
        raise ValueError("invalid process CPU time")
    return days * 86400 + hours * 3600 + minutes * 60 + seconds


def _owned_cpu(group: int) -> dict[int, float]:
    output = subprocess.check_output(
        ["ps", "-Ao", "pid=,ppid=,pgid=,time="], text=True, timeout=1
    )
    rows = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) == 4:
            rows.append((int(fields[0]), int(fields[1]), int(fields[2]),
                         _cpu_seconds(fields[3])))
    owned = {group}
    while True:
        previous = len(owned)
        owned.update(pid for pid, parent, pgid, _ in rows
                     if parent in owned or pgid == group)
        if len(owned) == previous:
            break
    return {pid: seconds for pid, _, _, seconds in rows if pid in owned}


def _finish_owned_group(child: subprocess.Popen[bytes]) -> None:
    """Ignore Darwin's refusal to signal an already-dead process group only."""
    try:
        _signal_owned_group(child, signal.SIGKILL)
    except PermissionError:
        snapshot = subprocess.check_output(
            ["ps", "-Ao", "pgid=,stat="], text=True, timeout=1
        )
        if any(
            int(fields[0]) == child.pid and not fields[1].startswith("Z")
            for line in snapshot.splitlines()
            if len(fields := line.split()) == 2
        ):
            raise


def _signal_owned_group(child: subprocess.Popen[bytes], signum: int) -> None:
    try:
        native_tests.signal_owned_group(child, signum)
        return
    except PermissionError:
        # Darwin may reject a negative-PID group signal for a live, owned
        # session. Verify the exact group and UID before signalling each PID.
        snapshot = subprocess.check_output(
            ["ps", "-Ao", "pid=,ppid=,pgid=,uid=,stat="], text=True, timeout=1
        )
        processes = {}
        for line in snapshot.splitlines():
            fields = line.split()
            if len(fields) != 5:
                continue
            pid, ppid, pgid, uid, stat = fields
            processes[int(pid)] = (int(ppid), int(pgid), int(uid), stat)
        owned = {child.pid}
        while True:
            previous = len(owned)
            owned.update(pid for pid, (parent, pgid, _, _) in processes.items()
                         if parent in owned or pgid == child.pid)
            if len(owned) == previous:
                break
        live = [pid for pid in owned if pid in processes
                and not processes[pid][3].startswith("Z")]
        if child.pid not in live and child.poll() is None:
            raise PermissionError("live child missing from owned process snapshot")
        if any(processes[pid][2] != os.getuid() for pid in live):
            raise PermissionError("owned process group contains a different UID")
        for pid in sorted(live, key=lambda value: value == child.pid):
            try:
                os.kill(pid, signum)
            except ProcessLookupError:
                pass


def run(command: list[str], *, cwd: Path, log: Path | None = None,
        idle_limit: float = 5, total_limit: float = 45,
        label: str = "QUINT", capture_output: bool = True) -> Result:
    if log is not None:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.unlink(missing_ok=True)
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
    started = last_progress = time.monotonic()
    last_cpu: dict[int, float] = {}
    data = bytearray()
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(child.stdout, selectors.EVENT_READ)
            while True:
                status = child.poll()
                if not selector.get_map() and status is not None:
                    break
                ready = selector.select(timeout=0.2)
                if ready:
                    chunk = os.read(child.stdout.fileno(), 65536)
                    if not chunk:
                        selector.unregister(child.stdout)
                        continue
                    now = time.monotonic()
                    if not native_tests.forward_output(
                        chunk, min(started + total_limit, now + idle_limit)
                    ):
                        native_tests.forward_output(
                            f"{label}-FAIL: output backpressure exceeded {idle_limit}s\n".encode(),
                            time.monotonic() + 1,
                        )
                        return Result(124, bytes(data), log)
                    if capture_output:
                        data.extend(chunk)
                    if log is not None:
                        with log.open("ab") as stream:
                            stream.write(chunk)
                    last_progress = now
                now = time.monotonic()
                if now - last_progress >= 2:
                    try:
                        cpu = _owned_cpu(child.pid)
                    except (OSError, ValueError, subprocess.SubprocessError):
                        cpu = {}
                    cpu_delta = sum(max(0.0, seconds - last_cpu.get(pid, 0.0))
                                    for pid, seconds in cpu.items())
                    if cpu_delta > 0.01:
                        message = (f"{label}-WORK: measured child CPU "
                                   f"+{cpu_delta:.2f}s processes={len(cpu)}\n").encode()
                        if not native_tests.forward_output(
                            message, min(started + total_limit,
                                         time.monotonic() + idle_limit)
                        ):
                            native_tests.forward_output(
                                f"{label}-FAIL: output backpressure exceeded {idle_limit}s\n".encode(),
                                time.monotonic() + 1,
                            )
                            return Result(124, bytes(data), log)
                        last_progress = time.monotonic()
                    last_cpu = cpu
                if now - last_progress > idle_limit or now - started > total_limit:
                    reason = (f"{label}-FAIL: no output or measured CPU for {idle_limit}s"
                              if now - last_progress > idle_limit else
                              f"{label}-FAIL: {total_limit}s total limit")
                    print(reason, flush=True)
                    native_tests.report_owned_processes(child)
                    _signal_owned_group(child, signal.SIGTERM)
                    return Result(124, bytes(data), log)
        return Result(child.wait(), bytes(data), log)
    finally:
        if child.poll() is None:
            _signal_owned_group(child, signal.SIGTERM)
            try:
                child.wait(timeout=1)
            except subprocess.TimeoutExpired:
                _signal_owned_group(child, signal.SIGKILL)
                child.wait()
        _finish_owned_group(child)
        child.stdout.close()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--idle", type=float, default=5)
    parser.add_argument("--total", type=float, required=True)
    parser.add_argument("--label", default="BUILD")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or args.idle <= 0 or args.total <= 0:
        parser.error("provide a command and positive idle and total limits")
    return run(command, cwd=Path.cwd(), idle_limit=args.idle,
               total_limit=args.total, label=args.label,
               capture_output=False).status


if __name__ == "__main__":
    sys.exit(main())
