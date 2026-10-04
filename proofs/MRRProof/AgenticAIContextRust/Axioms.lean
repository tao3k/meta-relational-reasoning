import Evidence
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
    `MRR.ContextRustProofs.complete_is_identity]
  required.forM fun declarationName => do
    unless environment.contains declarationName do
      throwError "Missing source theorem: {declarationName}"
    let axioms <- collectAxioms declarationName
    axioms.forM fun axiomName => do
      unless allowed.contains axiomName do
        throwError "{declarationName} depends on disallowed axiom {axiomName}"
  logInfo m!"SOURCE-AXIOM-AUDIT-OK: {required.size} production-function theorems"
