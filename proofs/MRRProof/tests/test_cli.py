import argparse
import hashlib
import unittest
from pathlib import Path
from unittest import mock

from mrr_proof_validation import cli, org_babel


class ProofImpactTests(unittest.TestCase):
    def test_extract_org_babel_blocks_preserves_language_source_and_line(self) -> None:
        source = """* Contract
#+begin_src mermaid
flowchart LR
  A --> B
#+end_src
#+begin_src typst
$ A => B $
#+end_src
"""

        blocks = org_babel.extract_org_babel_blocks(Path("contract.org"), source)

        self.assertEqual(
            [(block.language, block.line, block.source) for block in blocks],
            [
                ("mermaid", 2, "flowchart LR\n  A --> B\n"),
                ("typst", 6, "$ A => B $\n"),
            ],
        )

    def test_extract_org_babel_blocks_rejects_outputs_and_unclosed_blocks(self) -> None:
        with self.assertRaisesRegex(
            org_babel.OrgBabelValidationError, "forbidden :file"
        ):
            org_babel.extract_org_babel_blocks(
                Path("contract.org"),
                "#+begin_src typst :file contract.pdf\nhello\n#+end_src\n",
            )
        with self.assertRaisesRegex(org_babel.OrgBabelValidationError, "unclosed"):
            org_babel.extract_org_babel_blocks(
                Path("contract.org"), "#+begin_src mermaid\nflowchart LR\n"
            )

    def test_validate_org_babel_sources_uses_parser_only_and_stdout(self) -> None:
        blocks = [
            org_babel.OrgBabelBlock(
                Path("contract.org"), 1, "mermaid", "flowchart LR\nA-->B\n"
            ),
            org_babel.OrgBabelBlock(Path("contract.org"), 4, "typst", "$ A => B $\n"),
        ]
        with (
            mock.patch.object(
                org_babel, "architecture_org_blocks", return_value=blocks
            ),
            mock.patch.object(org_babel, "validate_mermaid_sources") as mermaid,
            mock.patch.object(org_babel, "validate_typst_source") as typst,
        ):
            receipt = org_babel.validate_org_babel_sources(cli.ROOT)

        self.assertEqual(mermaid.call_count, 1)
        self.assertEqual(mermaid.call_args.args, ([blocks[0]], cli.ROOT))
        typst.assert_called_once_with(blocks[1], cli.ROOT)
        self.assertEqual(
            receipt,
            {
                "documents": 1,
                "blocks": {"mermaid": 1, "typst": 1},
                "mermaidParser": "validated",
                "typstCompiler": "validated",
                "outputs": "none",
            },
        )

    def test_typst_validation_skips_system_font_discovery_and_emits_no_file(
        self,
    ) -> None:
        block = org_babel.OrgBabelBlock(
            Path("contract.org"), 4, "typst", "$ A => B $\n"
        )
        completed = mock.Mock(returncode=0, stderr=b"")
        with (
            mock.patch.object(
                org_babel, "_required_executable", return_value=Path("/tool/typst")
            ),
            mock.patch.object(
                org_babel.subprocess, "run", return_value=completed
            ) as run,
        ):
            org_babel.validate_typst_source(block, cli.ROOT)

        self.assertEqual(
            run.call_args.args[0],
            [
                Path("/tool/typst"),
                "compile",
                "--root",
                cli.ROOT,
                "--ignore-system-fonts",
                "-",
                "-",
            ],
        )
        self.assertEqual(run.call_args.kwargs["input"], block.source.encode())
        self.assertEqual(run.call_args.kwargs["cwd"], cli.ROOT)

    def test_typst_validation_compiles_identical_sources_with_exact_provenance(
        self,
    ) -> None:
        blocks = [
            org_babel.OrgBabelBlock(Path("first.org"), 4, "typst", "$ A => B $\n"),
            org_babel.OrgBabelBlock(Path("second.org"), 19, "typst", "$ A => B $\n"),
        ]
        with mock.patch.object(org_babel, "validate_typst_source") as validate:
            org_babel.validate_typst_sources(blocks, cli.ROOT)

        self.assertEqual(validate.call_count, 2)
        validate.assert_has_calls(
            [mock.call(blocks[0], cli.ROOT), mock.call(blocks[1], cli.ROOT)],
            any_order=True,
        )

    def test_mermaid_native_grammar_rejects_invalid_or_unsupported_source(self) -> None:
        invalid = org_babel.OrgBabelBlock(
            Path("contract.org"), 7, "mermaid", "flowchart TD\nA -->\n"
        )
        with self.assertRaisesRegex(
            org_babel.OrgBabelValidationError,
            r"contract\.org:7: mermaid validation failed:.*Parse error",
        ):
            org_babel.validate_mermaid_sources([invalid], cli.ROOT)

        unsupported = org_babel.OrgBabelBlock(
            Path("contract.org"), 11, "mermaid", "sequenceDiagram\nA->>B: hello\n"
        )
        with self.assertRaisesRegex(
            org_babel.OrgBabelValidationError,
            "unsupported Mermaid diagram type: sequencediagram",
        ):
            org_babel.validate_mermaid_sources([unsupported], cli.ROOT)

    def test_downstream_closure_finds_transitive_consumers(self) -> None:
        graph = {
            "identity": set(),
            "relation": {"identity"},
            "bundle": {"relation"},
            "facade": {"bundle"},
            "sibling": set(),
        }
        self.assertEqual(
            cli.downstream_closure(graph, {"relation"}),
            {"relation", "bundle", "facade"},
        )

    def test_downstream_closure_terminates_on_cycles_and_preserves_external_seeds(
        self,
    ) -> None:
        graph = {
            "first": {"third"},
            "second": {"first"},
            "third": {"second"},
            "consumer": {"third"},
        }
        self.assertEqual(
            cli.downstream_closure(graph, {"first", "external"}),
            {"first", "second", "third", "consumer", "external"},
        )

    def test_changed_crates_ignores_non_mrr_paths(self) -> None:
        self.assertEqual(
            cli.changed_crates(
                ["crates/mrr-bundle/src/api.rs", "docs/architecture/example.org"]
            ),
            {"mrr-bundle"},
        )

    def test_current_workspace_and_proof_contract_are_locally_admitted(self) -> None:
        graph = cli.dependency_graph()
        lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
        obligations = cli.validate_proof_obligations(set(cli.CORE_CRATES), lean_source)
        babel = org_babel.validate_org_babel_contract(cli.ROOT)
        self.assertIn("mrr-bundle", graph)
        ascent_blocks = org_babel.extract_org_babel_blocks(
            cli.ROOT / "docs/architecture/0003-mrr-ascent-evaluation.org",
            (cli.ROOT / "docs/architecture/0003-mrr-ascent-evaluation.org").read_text(),
        )
        self.assertEqual(
            [block.language for block in ascent_blocks],
            ["mermaid", "typst", "mermaid", "typst"],
        )
        self.assertEqual(babel["outputs"], "none")
        self.assertEqual(
            obligations,
            [
                "admitted_closure_binding_is_exact",
                "admitted_derivation_has_rule_and_premises",
                "admitted_fact_ids_are_unique",
                "admitted_rule_uses_only_admitted_relations",
                "closure_admission_rejects_any_failed_owner",
                "filter_for_admission_is_typed",
                "filter_for_rejection_emits_no_ir",
                "graph_element_predicate_admission_is_typed",
                "graph_element_predicate_rejection_emits_no_ir",
                "graph_match_path_prefix_admission_is_typed",
                "graph_match_path_prefix_rejection_emits_no_ir",
                "identity_domain_separation",
                "inserted_fact_has_admitted_schema",
                "order_page_admission_is_typed",
                "order_page_rejection_emits_no_ir",
                "primitive_result_admission_is_typed",
                "primitive_result_rejection_emits_no_ir",
                "returned_counterexample_is_valid",
            ],
        )

    def test_counterexample_fixture_is_replayed_as_a_lean_obligation(self) -> None:
        source = cli.counterexample_replay_source()
        self.assertIn("def replayCounterexample", source)
        self.assertIn("action := { digest := Digest256.ofFixtureAtom", source)
        self.assertIn("initialState := { digest := Digest256.ofFixtureAtom", source)
        self.assertIn("theorem replay_counterexample_fixture_valid", source)
        self.assertIn("CounterexampleValid replayCounterexample", source)
        self.assertIn("exact And.intro rfl (And.intro rfl rfl)", source)
        self.assertNotIn("by decide", source)

    def test_local_lean_validation_is_pinned_hashed_and_uses_no_transport(
        self,
    ) -> None:
        source = "namespace LocalProof\nexample : True := by trivial\nend LocalProof\n"
        version = mock.Mock(
            returncode=0,
            stdout="Lean (version 4.31.0, arm64-apple-darwin)\n",
            stderr="",
        )
        checked = mock.Mock(returncode=0, stdout="", stderr="")
        with (
            mock.patch.object(cli.shutil, "which", return_value="/tool/elan"),
            mock.patch.object(
                cli.subprocess, "run", side_effect=[version, checked]
            ) as run,
        ):
            receipt = cli.local_lean_check(source)

        self.assertEqual(run.call_count, 2)
        self.assertEqual(
            run.call_args_list[0].args[0],
            ["/tool/elan", "run", cli.LOCAL_LEAN_TOOLCHAIN, "lean", "--version"],
        )
        check_command = run.call_args_list[1].args[0]
        self.assertEqual(
            check_command[:4],
            ["/tool/elan", "run", cli.LOCAL_LEAN_TOOLCHAIN, "lean"],
        )
        self.assertFalse(Path(check_command[4]).exists())
        self.assertEqual(receipt["transport"], "local-process")
        self.assertEqual(receipt["temporarySource"], "removed")
        self.assertEqual(receipt["sourceSha256"], hashlib.sha256(source.encode()).hexdigest())

    def test_local_lean_receipt_rejects_kernel_failure_with_diagnostics(self) -> None:
        version = mock.Mock(
            returncode=0,
            stdout="Lean (version 4.31.0, arm64-apple-darwin)\n",
            stderr="",
        )
        rejected = mock.Mock(
            returncode=1,
            stdout="",
            stderr="BundleAdmission.generated.lean:7:3: error: unsolved goals",
        )
        with (
            mock.patch.object(cli.shutil, "which", return_value="/tool/elan"),
            mock.patch.object(cli.subprocess, "run", side_effect=[version, rejected]),
            self.assertRaisesRegex(AssertionError, "7:3: error: unsolved goals"),
        ):
            cli.local_lean_check("example : False := by trivial\n")

    def test_local_lean_cli_receipt_contains_impact_and_kernel_evidence(self) -> None:
        arguments = argparse.Namespace(
            paths=["crates/mrr-identity/src/api.rs"],
            toolchain=cli.LOCAL_LEAN_TOOLCHAIN,
        )
        kernel = {
            "schema": "mrr.local-lean-kernel-receipt.v1",
            "toolchain": cli.LOCAL_LEAN_TOOLCHAIN,
            "sourceSha256": "0" * 64,
            "kernelExitStatus": 0,
            "transport": "local-process",
            "temporarySource": "removed",
        }
        with (
            mock.patch.object(cli, "validate_org_babel_contract", return_value={}),
            mock.patch.object(cli, "local_lean_check", return_value=kernel),
        ):
            receipt = cli.run(arguments)

        self.assertEqual(receipt["schema"], "mrr.local-lean-impact-receipt.v1")
        self.assertEqual(receipt["localLean"], kernel)


if __name__ == "__main__":
    unittest.main()
