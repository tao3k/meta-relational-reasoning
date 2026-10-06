"""Prepare a source-bound native executable of the unchanged upstream gxtest."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from .native_tests import forward_output


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sdk_home() -> Path:
    return Path(os.environ["GERBIL_HOME"]).resolve()


def snapshot(root: Path) -> dict:
    files = [root / "gerbil.pkg", root / "build.ss"]
    for directory in ("scheme", "t", "proofs/MRRProof/fixtures/native-qualification"):
        files.extend(sorted((root / directory).rglob("*.ss")))
    files.extend(
        root / path
        for path in (
            "proofs/MRRProof/fixtures/native-test-progress.ss",
            "proofs/MRRProof/src/mrr_proof_validation/native_prepare.py",
            "proofs/MRRProof/src/mrr_proof_validation/native_tests.py",
        )
    )
    sdk = sdk_home()
    return {
        "sources": {str(path.relative_to(root)): digest(path) for path in files},
        "sdk": {
            name: digest(sdk / name)
            for name in (
                "bin/gxc",
                "bin/gsc",
                "include/gambit.h",
                "lib/static/gerbil__tools__env.scm",
                "lib/static/gerbil__tools__gxtest.scm",
            )
        },
    }


def private_environment(root: Path) -> dict:
    sdk = root / ".gerbil/native-qualification/sdk"
    env = os.environ.copy()
    # BUILD_PREFIX enables SDK-bootstrap mode, which omits package libraries
    # from the runtime load path. HOME alone selects this private installed SDK.
    env.pop("GERBIL_BUILD_PREFIX", None)
    env.update(
        GERBIL_HOME=str(sdk),
        GERBIL_GSC=str(sdk / "bin/gsc"),
        PATH=str(sdk / "bin") + os.pathsep + env["PATH"],
    )
    options = [
        value
        for value in env.get("GAMBOPT", "").split(",")
        if value and not value.startswith(("~~=", "~~bin=", "~~lib="))
    ]
    env["GAMBOPT"] = ",".join(
        options + [f"~~={sdk}", f"~~bin={sdk}/bin", f"~~lib={sdk}/lib"]
    )
    return env


def run(command: list[str], env: dict):
    print("NATIVE-PREPARE: " + " ".join(command), flush=True)
    with subprocess.Popen(
        command,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=0,
    ) as child:
        assert child.stdout is not None
        while chunk := os.read(child.stdout.fileno(), 65536):
            if not forward_output(chunk, time.monotonic() + 5):
                child.kill()
                raise RuntimeError("native compiler output could not be delivered")
        if child.wait() != 0:
            raise RuntimeError(f"native preparation failed: {command[0]}")
    print("NATIVE-PREPARE-OK: " + command[0], flush=True)


def prepare(root: Path):
    state = snapshot(root)
    directory = root / ".gerbil/native-qualification"
    if not directory.resolve().is_relative_to(root.resolve()):
        raise RuntimeError("native preparation requires a private workspace directory")
    directory.mkdir(parents=True, exist_ok=True)
    sdk = directory / "sdk"
    sdk_stamp = directory / "sdk.json"
    if not sdk_stamp.exists() or json.loads(sdk_stamp.read_text()) != state["sdk"]:
        if sdk.exists():
            shutil.rmtree(sdk)
        print("NATIVE-PREPARE: private copy of selected SDK", flush=True)
        if sys.platform == "darwin":
            subprocess.run(["/bin/cp", "-cR", str(sdk_home()), str(sdk)], check=True)
        else:
            shutil.copytree(sdk_home(), sdk, symlinks=True)
        sdk_stamp.write_text(json.dumps(state["sdk"], sort_keys=True))
    env = private_environment(root)
    # SDK distributions retain canonical generated SCM for these tools. Keep
    # C metadata as well as objects: upstream's linker reads both. All derived
    # files stay in this private copy; the installed SDK remains unchanged.
    for module in ("env", "gxtest"):
        base = sdk / f"lib/static/gerbil__tools__{module}"
        if not base.parent.resolve().is_relative_to(sdk.resolve()):
            raise RuntimeError("SDK static output directory escapes its private copy")
        base.with_suffix(".c").unlink(missing_ok=True)
        base.with_suffix(".o").unlink(missing_ok=True)
        run(
            [
                env["GERBIL_GSC"],
                "-c",
                "-o",
                str(base.with_suffix(".c")),
                str(base.with_suffix(".scm")),
            ],
            env,
        )
        run(
            [
                env["GERBIL_GSC"],
                "-obj",
                "-o",
                str(base.with_suffix(".o")),
                str(base.with_suffix(".c")),
            ],
            env,
        )
    paths = sorted(root.glob("t/*-test.ss"))
    paths += [
        root / f"proofs/MRRProof/fixtures/native-qualification/{name}-test.ss"
        for name in ("pass", "assertion", "empty")
    ]
    run([str(sdk / "bin/gxc"), "-V", *map(str, paths)], env)
    driver = directory / "harness.ss"
    imports = "\n".join(
        f"        (prefix-in {json.dumps(str(path.with_suffix('')))} test{index}#)"
        for index, path in enumerate(paths)
    )
    driver.write_text(
        "(import :gerbil/expander\n"
        "        (prefix-in :gerbil/expander gx#)\n"
        "        (rename-in :gerbil/tools/gxtest (main run-gxtest))\n" + imports + ")\n"
        "(export main)\n"
        "(def (install-test-progress!)\n"
        '  (include "../../proofs/MRRProof/fixtures/native-test-progress.ss"))\n'
        "(def (main . args)\n"
        "  (current-expander-module-prelude (make-prelude-context (import-module ':gerbil/core)))\n"
        "  (install-test-progress!)\n"
        '  (let ((status (apply run-gxtest "-v" "5" args)))\n'
        '    (displayln "mrr-test: harness returned " status) (force-output)\n'
        '    (displayln "mrr-test: exit cleanup started") (force-output)\n'
        "    (##exit-cleanup)\n"
        '    (displayln "mrr-test: exit cleanup returned") (force-output)\n'
        "    (exit status)))\n"
    )
    program = directory / "harness"
    program.unlink(missing_ok=True)
    run([str(sdk / "bin/gxc"), "-V", "-exe", "-o", str(program), str(driver)], env)
    if snapshot(root) != state:
        raise RuntimeError(
            "native preparation source or SDK changed during compilation"
        )
    (directory / "receipt.json").write_text(
        json.dumps({"snapshot": state, "program": digest(program)}, sort_keys=True)
    )


def prepared_command(paths: list[str]) -> tuple[list[str], dict]:
    root = Path.cwd()
    directory = root / ".gerbil/native-qualification"
    receipt = json.loads((directory / "receipt.json").read_text())
    program = directory / "harness"
    if receipt != {"snapshot": snapshot(root), "program": digest(program)}:
        raise RuntimeError("native executable is stale; run native_prepare again")
    return [str(program), *paths], private_environment(root)


if __name__ == "__main__":
    prepare(Path.cwd())
