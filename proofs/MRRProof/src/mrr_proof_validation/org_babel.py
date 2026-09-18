"""Validate executable Mermaid and Typst blocks owned by Org architecture docs."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

LANGUAGES = {"mermaid", "typst"}
SOURCE_BEGIN = re.compile(r"^\s*#\+begin_src\s+(\S+)(.*)$", re.IGNORECASE)
SOURCE_END = re.compile(r"^\s*#\+end_src\s*$", re.IGNORECASE)
MERMAID_CHECK_SCRIPT = r"""
const [flowDiagramModule] = process.argv.slice(1);
const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
const sources = JSON.parse(Buffer.concat(chunks).toString("utf8"));
const { diagram } = await import(flowDiagramModule);
const noop = () => false;
const sink = {};
for (const key of Object.keys(diagram.db)) sink[key] = noop;
sink.lex = { firstGraph: () => true };
diagram.parser.parser.yy = sink;
for (let index = 0; index < sources.length; index += 1) {
  try {
    diagram.parser.parser.parse(sources[index]);
  } catch (error) {
    console.error(JSON.stringify({ index, message: String(error) }));
    process.exitCode = 2;
    break;
  }
}
"""


@dataclass(frozen=True)
class OrgBabelBlock:
    """One executable architecture block with exact source provenance."""

    path: Path
    line: int
    language: str
    source: str


class OrgBabelValidationError(RuntimeError):
    """A fail-closed Org extraction or native syntax-check failure."""


def extract_org_babel_blocks(path: Path, source: str) -> list[OrgBabelBlock]:
    """Extract supported Org Babel blocks without interpreting their languages."""
    blocks: list[OrgBabelBlock] = []
    active_language: str | None = None
    active_line = 0
    active_source: list[str] = []
    for line_number, line in enumerate(source.splitlines(keepends=True), start=1):
        normalized = line.rstrip("\r\n")
        if active_language is not None:
            if SOURCE_END.match(normalized):
                blocks.append(
                    OrgBabelBlock(
                        path=path,
                        line=active_line,
                        language=active_language,
                        source="".join(active_source),
                    )
                )
                active_language = None
                active_source = []
                continue
            if SOURCE_BEGIN.match(normalized):
                raise OrgBabelValidationError(
                    f"{path}:{line_number}: nested source block before #+end_src"
                )
            active_source.append(line)
            continue

        begin = SOURCE_BEGIN.match(normalized)
        if begin is None:
            continue
        language = begin.group(1).lower()
        if language not in LANGUAGES:
            continue
        if re.search(r"(?:^|\s):file(?:\s|$)", begin.group(2), re.IGNORECASE):
            raise OrgBabelValidationError(
                f"{path}:{line_number}: forbidden :file output header"
            )
        active_language = language
        active_line = line_number

    if active_language is not None:
        raise OrgBabelValidationError(
            f"{path}:{active_line}: unclosed {active_language} source block"
        )
    return blocks


def architecture_org_blocks(root: Path) -> list[OrgBabelBlock]:
    """Load executable architecture blocks from every canonical Org document."""
    paths = [root / "ARCHITECTURE.org"]
    paths.extend(sorted((root / "docs/architecture").glob("*.org")))
    blocks: list[OrgBabelBlock] = []
    for path in paths:
        blocks.extend(extract_org_babel_blocks(path, path.read_text()))
    return blocks


def _required_executable(name: str) -> Path:
    executable = shutil.which(name)
    if executable is None:
        raise OrgBabelValidationError(f"required Babel validator is missing: {name}")
    return Path(executable)


def _failure_message(result: subprocess.CompletedProcess, block: OrgBabelBlock) -> str:
    stderr = result.stderr.decode(errors="replace").strip()
    detail = stderr[-2_000:] if stderr else f"exit status {result.returncode}"
    return f"{block.path}:{block.line}: {block.language} validation failed: {detail}"


def _mermaid_cli_package_root() -> Path:
    """Resolve the installed mermaid-cli package without pinning a store path."""
    executable = _required_executable("mmdc")
    resolved = executable.resolve()
    candidates: list[Path] = []
    if resolved.name in {"cli.js", "cli.mjs"}:
        candidates.append(resolved.parent.parent)
    wrapper = executable.read_text(errors="ignore")
    pattern = re.compile(r"(/[^\s\"']*?/src/cli\.(?:js|mjs))")
    candidates.extend(
        Path(match.group(1)).resolve().parent.parent
        for match in pattern.finditer(wrapper)
    )
    package_root = next(
        (
            candidate
            for candidate in candidates
            if (candidate / "package.json").is_file()
        ),
        None,
    )
    if package_root is None:
        raise OrgBabelValidationError(
            f"cannot resolve mermaid-cli package root from {executable}"
        )
    return package_root


def _mermaid_flowchart_module() -> Path:
    """Resolve mermaid-cli's native flowchart grammar module."""
    modules = sorted(
        (
            _mermaid_cli_package_root()
            / "node_modules/mermaid/dist/chunks/mermaid.core"
        ).glob("flowDiagram-*.mjs")
    )
    if len(modules) != 1:
        raise OrgBabelValidationError(
            f"expected one Mermaid flowchart grammar module, found {len(modules)}"
        )
    return modules[0]


def validate_mermaid_sources(blocks: list[OrgBabelBlock], root: Path) -> None:
    """Parse every flowchart block with Mermaid's native grammar and no outputs."""
    if not blocks:
        return
    for block in blocks:
        declaration = next(
            (
                line.strip().split(maxsplit=1)[0].lower()
                for line in block.source.splitlines()
                if line.strip() and not line.lstrip().startswith("%%")
            ),
            "",
        )
        if declaration not in {"flowchart", "graph"}:
            raise OrgBabelValidationError(
                f"{block.path}:{block.line}: unsupported Mermaid diagram type: {declaration}"
            )
    result = subprocess.run(
        [
            _required_executable("node"),
            "--input-type=module",
            "--eval",
            MERMAID_CHECK_SCRIPT,
            _mermaid_flowchart_module(),
        ],
        cwd=root,
        input=json.dumps([block.source for block in blocks]).encode(),
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode != 0:
        index_match = re.search(rb'"index":(\d+)', result.stderr)
        index = int(index_match.group(1)) if index_match else 0
        raise OrgBabelValidationError(_failure_message(result, blocks[index]))


def validate_typst_source(block: OrgBabelBlock, root: Path) -> None:
    """Compile Typst from stdin to stdout, retaining no PDF artifact."""
    result = subprocess.run(
        [
            _required_executable("typst"),
            "compile",
            "--root",
            root,
            "--ignore-system-fonts",
            "-",
            "-",
        ],
        cwd=root,
        input=block.source.encode(),
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode != 0:
        raise OrgBabelValidationError(_failure_message(result, block))


def validate_typst_sources(blocks: list[OrgBabelBlock], root: Path) -> None:
    """Compile every Typst block concurrently without retaining outputs."""
    if not blocks:
        return
    with ThreadPoolExecutor(max_workers=min(4, len(blocks))) as executor:
        list(executor.map(lambda block: validate_typst_source(block, root), blocks))


def validate_org_babel_sources(root: Path) -> dict:
    """Validate every supported block and return the typed receipt slice."""
    blocks = architecture_org_blocks(root)
    counts = {language: 0 for language in sorted(LANGUAGES)}
    documents: set[Path] = set()
    mermaid_blocks: list[OrgBabelBlock] = []
    typst_blocks: list[OrgBabelBlock] = []
    for block in blocks:
        counts[block.language] += 1
        documents.add(block.path)
        if block.language == "mermaid":
            mermaid_blocks.append(block)
        else:
            typst_blocks.append(block)
    validate_mermaid_sources(mermaid_blocks, root)
    validate_typst_sources(typst_blocks, root)
    return {
        "documents": len(documents),
        "blocks": counts,
        "mermaidParser": "validated",
        "typstCompiler": "validated",
        "outputs": "none",
    }


def validate_org_babel_contract(root: Path) -> dict:
    """Validate Org authority markers and all executable architecture blocks."""
    architecture = (root / "ARCHITECTURE.org").read_text()
    ownership = (
        root / "docs/architecture/0001-mrr-workspace-ownership.org"
    ).read_text()
    for marker in (
        "#+begin_src mermaid",
        "#+begin_src typst",
        '"BundleValid"(R, F_0, P, T)',
    ):
        if marker not in ownership:
            raise OrgBabelValidationError(
                f"Org architecture authority is missing {marker}"
            )
    for marker in ("#+begin_src mermaid", "#+begin_src typst"):
        if marker not in architecture:
            raise OrgBabelValidationError(f"root Org architecture is missing {marker}")
    forbidden = (
        root / "ARCHITECTURE.md",
        root / "docs/architecture/0001-mrr-workspace-ownership.md",
    )
    for path in forbidden:
        if path.exists():
            raise OrgBabelValidationError(
                f"Markdown architecture authority remains: {path}"
            )
    return validate_org_babel_sources(root)
