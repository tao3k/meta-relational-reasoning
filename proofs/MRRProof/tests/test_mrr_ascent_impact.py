from mrr_proof_validation import cli


def test_mrr_ascent_changes_seed_the_proof_impact_graph() -> None:
    changed = ["crates/mrr-ascent/src/api.rs"]

    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"mrr-ascent"}
    assert "mrr-ascent" in impacted
    assert "meta-relational-reasoning" in impacted
    assert "admitted_closure_binding_is_exact" in obligations
    assert "closure_admission_rejects_any_failed_owner" in obligations


def test_facade_changes_select_atomic_closure_admission_obligations() -> None:
    changed = ["crates/meta-relational-reasoning/src/admission.rs"]
    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()

    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"meta-relational-reasoning"}
    assert "admitted_closure_binding_is_exact" in obligations
    assert "closure_admission_rejects_any_failed_owner" in obligations
