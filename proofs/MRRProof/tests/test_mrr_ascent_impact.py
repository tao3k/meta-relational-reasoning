# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
#
# SPDX-License-Identifier: AGPL-3.0-only

from mrr_proof_validation import cli


def obligations_for(changed: list[str]) -> tuple[set[str], set[str], set[str]]:
    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)
    return seeds, impacted, obligations


def test_mrr_ascent_changes_seed_the_proof_impact_graph() -> None:
    seeds, impacted, obligations = obligations_for(["crates/mrr-ascent/src/api.rs"])

    assert seeds == {"mrr-ascent"}
    assert "meta-relational-reasoning" in impacted
    assert "admitted_closure_binding_is_exact" in obligations
    assert "closure_admission_rejects_any_failed_owner" in obligations


def test_facade_changes_select_atomic_closure_admission_obligations() -> None:
    seeds, _, obligations = obligations_for(
        ["crates/meta-relational-reasoning/src/admission.rs"]
    )

    assert seeds == {"meta-relational-reasoning"}
    assert "admitted_closure_binding_is_exact" in obligations
    assert "closure_admission_rejects_any_failed_owner" in obligations


def test_parser_owned_frontend_changes_select_fail_closed_obligations() -> None:
    seeds, impacted, obligations = obligations_for(
        ["crates/mrr-frontends/src/parser_owned.rs"]
    )

    assert seeds == {"mrr-frontends"}
    assert "mrr-frontends" in impacted
    assert "frontend_common_surface_normalizes_across_languages" in obligations
    assert "frontend_receipt_retains_selected_language" in obligations
    assert "graph_element_predicate_rejection_emits_no_ir" in obligations
    assert "primitive_result_admission_is_typed" in obligations
