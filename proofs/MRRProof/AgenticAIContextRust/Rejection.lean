import Forward

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

/-- Compose a typed expansion rejection with the actual pop, set insert and
production driver's stopping behavior. Concrete corollaries discharge expansion. -/
theorem native_forward_rejection_run_stop (traversal : NativeTraversal)
    (id : NativeFactId) (tail : List NativeFactId)
    (pending : traversal.pending.val.reverse = id :: tail)
    (fresh : Not (id inList traversal.selected)) (value : Option NativeElement)
    (lookup : nativeLookup traversal.elements id = value) (failure : NativeContextError)
    (rejected : forall remaining, state.expand_identity id value traversal.require_complete
      remaining traversal.coverage traversal.error = .ok (false, remaining, traversal.coverage, some failure)) :
    exists final, state.run_closure traversal = .ok final /\ final.error = some failure /\
      final.coverage = traversal.coverage /\ final.selected = id :: traversal.selected /\
      pendingModel final.pending = tail.map factModelId := by
  apply Exists.elim (pop_identity_projects_pending traversal.pending id tail pending)
  intro remaining remainingSpec
  let final : NativeTraversal := { traversal with pending := remaining, selected := id :: traversal.selected, error := some failure }
  have stopped : state.advance_closure traversal = .ok (false, final) := by
    simp only [state.advance_closure, remainingSpec.1, bind_ok, uncurry,
      native_set_insert_exact, fresh, if_false, if_true, native_map_lookup_exact, lookup,
      rejected, final]
  have runExact : state.run_closure traversal = .ok final :=
    extracted_driver_stop_exact nativeForwardAdapter traversal final stopped
  exact Exists.intro final (And.intro runExact (And.intro rfl
    (And.intro rfl (And.intro rfl remainingSpec.2))))

theorem native_forward_missing_run_stop (traversal : NativeTraversal)
    (id : NativeFactId) (tail : List NativeFactId)
    (pending : traversal.pending.val.reverse = id :: tail)
    (fresh : Not (id inList traversal.selected))
    (missing : nativeLookup traversal.elements id = none) :
    exists final, state.run_closure traversal = .ok final /\ final.error = some (.UnknownElement id) /\
      final.coverage = traversal.coverage /\ final.selected = id :: traversal.selected /\
      pendingModel final.pending = tail.map factModelId := by
  exact native_forward_rejection_run_stop traversal id tail pending fresh none missing
    (.UnknownElement id) (fun remaining => expansion_missing_stops id traversal.require_complete
      remaining traversal.coverage traversal.error)

theorem native_forward_invalid_run_stop (traversal : NativeTraversal)
    (id cause : NativeFactId) (tail : List NativeFactId) (element : NativeElement)
    (pending : traversal.pending.val.reverse = id :: tail)
    (fresh : Not (id inList traversal.selected))
    (lookup : nativeLookup traversal.elements id = some element)
    (invalid : element.fact.context.validity = .InvalidatedBy cause) :
    exists final, state.run_closure traversal = .ok final /\ final.error = some (.InvalidatedElement id) /\
      final.coverage = traversal.coverage /\ final.selected = id :: traversal.selected /\
      pendingModel final.pending = tail.map factModelId := by
  exact native_forward_rejection_run_stop traversal id tail pending fresh (some element) lookup
    (.InvalidatedElement id) (fun remaining => expansion_invalid_stops id cause element invalid
      traversal.require_complete remaining traversal.coverage traversal.error)

theorem native_forward_incomplete_run_stop (traversal : NativeTraversal)
    (id : NativeFactId) (tail : List NativeFactId) (element : NativeElement)
    (pending : traversal.pending.val.reverse = id :: tail)
    (fresh : Not (id inList traversal.selected))
    (lookup : nativeLookup traversal.elements id = some element)
    (valid : element.fact.context.validity = .Valid)
    (incomplete : Not (element.fact.context.completeness = .Complete))
    (required : traversal.require_complete = true) :
    exists final, state.run_closure traversal = .ok final /\ final.error = some (.IncompleteEvidence id) /\
      final.coverage = traversal.coverage /\ final.selected = id :: traversal.selected /\
      pendingModel final.pending = tail.map factModelId := by
  apply native_forward_rejection_run_stop traversal id tail pending fresh (some element) lookup
    (.IncompleteEvidence id)
  intro remaining
  rw [required]
  exact expansion_incomplete_stops id element valid incomplete remaining traversal.coverage traversal.error

end MRR.ContextRustProofs
