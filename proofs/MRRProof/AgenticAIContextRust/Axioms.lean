import Evidence
import Driver
import NativeFacts
import Lean.Util.CollectAxioms
import Lean.Elab.Command

open Lean Elab Command

run_cmd do
  let environment <- getEnv
  let allowed := #[`propext, `Classical.choice, `Quot.sound]
  let required := #[
    `MRR.ContextRustProofs.admission_accepted_iff,
    `MRR.ContextRustProofs.invalid_evidence_rejected,
    `MRR.ContextRustProofs.required_incomplete_rejected,
    `MRR.ContextRustProofs.merge_refines_max,
    `MRR.ContextRustProofs.merge_commutative,
    `MRR.ContextRustProofs.merge_associative,
    `MRR.ContextRustProofs.complete_is_identity,
    `MRR.ContextRustProofs.extracted_driver_total_correctness,
    `MRR.ContextRustProofs.extracted_driver_preserves_invariant,
    `MRR.ContextRustProofs.extracted_driver_graph_invariant_complete,
    `MRR.ContextRustProofs.extracted_driver_refines_graph,
    `MRR.ContextRustProofs.extracted_model_driver_total_correctness,
    `MRR.ContextRustProofs.number_bytes_injective,
    `MRR.ContextRustProofs.fact_model_id_injective,
    `MRR.ContextRustProofs.fact_model_id_equal_iff,
    `MRR.ContextRustProofs.native_fact_id_has_32_bytes,
    `MRR.ContextRustProofs.native_fact_id_equality_exact,
    `MRR.ContextRustProofs.native_fact_id_equality_refines_model,
    `MRR.ContextRustProofs.native_fact_context_exact,
    `MRR.ContextRustProofs.native_completeness_exact,
    `MRR.ContextRustProofs.native_validity_exact,
    `MRR.ContextRustProofs.native_validity_equality_exact,
    `MRR.ContextRustProofs.native_validity_admission_exact,
    `MRR.ContextRustProofs.native_fact_admission_exact,
    `MRR.ContextRustProofs.native_fact_accepted_iff,
    `MRR.ContextRustProofs.native_invalid_fact_rejected,
    `MRR.ContextRustProofs.projected_dependencies_exact,
    `MRR.ContextRustProofs.native_required_projection]
  required.forM fun declarationName => do
    unless environment.contains declarationName do
      throwError "Missing source theorem: {declarationName}"
    let axioms <- collectAxioms declarationName
    axioms.forM fun axiomName => do
      unless allowed.contains axiomName do
        throwError "{declarationName} depends on disallowed axiom {axiomName}"
  logInfo m!"SOURCE-AXIOM-AUDIT-OK: {required.size} source-refinement declarations"
