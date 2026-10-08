"""Run the pinned Quint CLI with real output and measured-process progress."""

from dataclasses import dataclass
import os
from pathlib import Path
import selectors
import signal
import subprocess
import time

from . import native_tests


@dataclass(frozen=True)
class Result:
    status: int
    output: bytes
    log: Path


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
        native_tests.signal_owned_group(child, signal.SIGKILL)
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


def run(command: list[str], *, cwd: Path, log: Path,
        idle_limit: float = 5, total_limit: float = 45) -> Result:
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
                    if not native_tests.forward_output(chunk, started + total_limit):
                        return Result(124, bytes(data), log)
                    data.extend(chunk)
                    with log.open("ab") as stream:
                        stream.write(chunk)
                    last_progress = now
                now = time.monotonic()
                if now - last_progress >= 2:
                    try:
                        cpu = _owned_cpu(child.pid)
                    except (OSError, ValueError, subprocess.SubprocessError):
                        cpu = {}
                    if any(seconds > last_cpu.get(pid, 0) + 0.01
                           for pid, seconds in cpu.items()):
                        message = (f"QUINT-WORK: measured checker CPU "
                                   f"{sum(cpu.values()):.2f}s\n").encode()
                        if not native_tests.forward_output(message, started + total_limit):
                            return Result(124, bytes(data), log)
                        last_progress = time.monotonic()
                    last_cpu = cpu
                if now - last_progress > idle_limit or now - started > total_limit:
                    reason = (f"QUINT-FAIL: no output or measured CPU for {idle_limit}s"
                              if now - last_progress > idle_limit else
                              f"QUINT-FAIL: {total_limit}s total limit")
                    print(reason, flush=True)
                    native_tests.report_owned_processes(child)
                    native_tests.signal_owned_group(child, signal.SIGTERM)
                    return Result(124, bytes(data), log)
        return Result(child.wait(), bytes(data), log)
    finally:
        if child.poll() is None:
            native_tests.signal_owned_group(child, signal.SIGTERM)
            try:
                child.wait(timeout=1)
            except subprocess.TimeoutExpired:
                native_tests.signal_owned_group(child, signal.SIGKILL)
                child.wait()
        _finish_owned_group(child)
        child.stdout.close()
