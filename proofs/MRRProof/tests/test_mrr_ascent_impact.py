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


def test_gql_syntax_changes_reach_typed_predicate_and_fail_closed_obligations() -> None:
    changed = ["crates/gql-syntax/src/parser/core/engine/implementation/predicate_expression.rs"]

    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"gql-syntax"}
    assert {"gql-ast", "gql-sema", "gql-compiler", "gql"} <= impacted
    assert "graph_element_predicate_admission_is_typed" in obligations
    assert "graph_element_predicate_rejection_emits_no_ir" in obligations


def test_path_prefix_changes_reach_typed_admission_and_fail_closed_obligations() -> None:
    changed = [
        "crates/gql-syntax/src/parser/core/engine/implementation/path_prefix.rs"
    ]

    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"gql-syntax"}
    assert {"gql-ast", "gql-sema", "gql-compiler", "gql"} <= impacted
    assert "graph_match_path_prefix_admission_is_typed" in obligations
    assert "graph_match_path_prefix_rejection_emits_no_ir" in obligations


def test_order_page_changes_reach_typed_admission_and_fail_closed_obligations() -> None:
    changed = ["crates/gql-syntax/src/parser/core/engine/query_result.rs"]

    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"gql-syntax"}
    assert {"gql-ast", "gql-sema", "gql-compiler", "gql"} <= impacted
    assert "order_page_admission_is_typed" in obligations
    assert "order_page_rejection_emits_no_ir" in obligations


def test_filter_for_changes_reach_typed_admission_and_fail_closed_obligations() -> None:
    changed = [
        "crates/gql-syntax/src/parser/core/engine/implementation/primitive_query.rs"
    ]

    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"gql-syntax"}
    assert {"gql-ast", "gql-sema", "gql-compiler", "gql"} <= impacted
    assert "filter_for_admission_is_typed" in obligations
    assert "filter_for_rejection_emits_no_ir" in obligations


def test_primitive_result_changes_reach_typed_admission_and_fail_closed_obligations() -> None:
    changed = ["crates/gql-syntax/src/parser/core/engine/query_result.rs"]

    seeds = cli.changed_crates(changed)
    impacted = cli.downstream_closure(cli.dependency_graph(), seeds)
    lean_source = (cli.ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    obligations = cli.validate_proof_obligations(impacted, lean_source)

    assert seeds == {"gql-syntax"}
    assert {"gql-ast", "gql-sema", "gql-compiler", "gql"} <= impacted
    assert "primitive_result_admission_is_typed" in obligations
    assert "primitive_result_rejection_emits_no_ir" in obligations
