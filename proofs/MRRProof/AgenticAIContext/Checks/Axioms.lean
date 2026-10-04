import MRR
import Lean.Util.CollectAxioms
import Lean.Elab.Command

open Lean Elab Command

/- Audit every declaration in this domain, including imported transitive
dependencies. Imported C4 certificates and MRR mapping laws share the same axiom gate. -/
run_cmd do
  let environment <- getEnv
  let allowed := #[`propext, `Classical.choice, `Quot.sound]
  for required in #[
      `MRR.AgenticAIContext.worklist_refines_certificate,
      `MRR.AgenticAIContext.worklist_finished_invariant_exact,
      `MRR.AgenticAIContext.finite_worklist_total_correctness,
      `MRR.AgenticAIContext.exact_restore_identity,
      `MRR.AgenticAIContext.exact_restore_collision_refused,
      `MRR.AgenticAIContext.cbor_decoder_consumes_input,
      `MRR.AgenticAIContext.sha256_digest_length,
      `MRR.AgenticAIContext.checked_closure_exact,
      `MRR.AgenticAIContext.checked_closure_unique,
      `MRR.AgenticAIContext.context_system_sound,
      `MRR.AgenticAIContext.context_restore_sound,
      `MRR.AgenticAIContext.context_restore_identity_refused,
      `MRR.AgenticAIContext.revision_reuse_dependency_safe,
      `MRR.AgenticAIContext.revision_global_change_no_reuse,
      `MRR.AgenticAIContext.context_system_revision_render_tokens] do
    unless environment.contains required do
      throwError "Required Context system theorem was not loaded: {required}"
  let mut count : Nat := 0
  let mut c4Count : Nat := 0
  for (name, _) in environment.constants.toList do
    if (`MRR.AgenticAIContext).isPrefixOf name || (`LeanPoo.C4).isPrefixOf name then
      if (`LeanPoo.C4).isPrefixOf name then c4Count := c4Count + 1
      count := count + 1
      for axiomName in (<- collectAxioms name) do
        unless allowed.contains axiomName do
          throwError "{name} depends on disallowed axiom {axiomName}"
  if c4Count = 0 then throwError "Upstream C4 certificates were not loaded"
  if count = 0 then throwError "Agentic AI Context namespace was not loaded"
  logInfo m!"AXIOM-AUDIT-OK: {count} declarations ({c4Count} upstream C4); standard Lean axioms only"
