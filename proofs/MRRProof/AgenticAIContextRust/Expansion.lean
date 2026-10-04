import Stack

open Aeneas.Std
open MRR.ContextRust

namespace MRR.ContextRustProofs

abbrev NativeElement := state.AgenticAiContextElement
abbrev NativeContextError := state.AgenticAiContextError

theorem expansion_missing_stops (id : NativeFactId) (requireComplete : Bool)
    (pending : alloc.vec.Vec NativeFactId) (coverage : Coverage)
    (error : Option NativeContextError) :
    state.expand_identity id none requireComplete pending coverage error =
      .ok (false, pending, coverage, some (.UnknownElement id)) := by rfl

theorem expansion_invalid_stops (id cause : NativeFactId) (element : NativeElement)
    (invalid : element.fact.context.validity = .InvalidatedBy cause)
    (requireComplete : Bool) (pending : alloc.vec.Vec NativeFactId)
    (coverage : Coverage) (error : Option NativeContextError) :
    state.expand_identity id (some element) requireComplete pending coverage error =
      .ok (false, pending, coverage, some (.InvalidatedElement id)) := by
  simp only [state.expand_identity,
    native_invalid_fact_rejected element.fact cause invalid requireComplete, bind_ok]

theorem expansion_incomplete_stops (id : NativeFactId) (element : NativeElement)
    (valid : element.fact.context.validity = .Valid)
    (incomplete : Not (element.fact.context.completeness = .Complete))
    (pending : alloc.vec.Vec NativeFactId) (coverage : Coverage)
    (error : Option NativeContextError) :
    state.expand_identity id (some element) true pending coverage error =
      .ok (false, pending, coverage, some (.IncompleteEvidence id)) := by
  have rejected : evidence.admit_fact_evidence element.fact true = .ok .Incomplete := by
    rw [native_fact_admission_exact, valid]
    exact required_incomplete_rejected _ incomplete
  simp only [state.expand_identity, rejected, bind_ok]

/-- The actual native expansion continues only after admitting the actual Fact.
It preserves the existing error slot, appends the exact declared dependencies and
aggregates coverage by the independently specified maximum weakness rank. -/
theorem expansion_accepted_step (id : NativeFactId) (element : NativeElement)
    (requireComplete : Bool) (valid : element.fact.context.validity = .Valid)
    (complete : requireComplete = false \/ element.fact.context.completeness = .Complete)
    (pending : alloc.vec.Vec NativeFactId) (coverage : Coverage)
    (error : Option NativeContextError)
    (capacity : pending.val.length + element.dependencies.val.length <= Usize.max) :
    exists next merged,
      state.expand_identity id (some element) requireComplete pending coverage error =
        .ok (true, next, merged, error) /\
      next.val = pending.val ++ element.dependencies.val /\
      rank merged = max (rank coverage) (rank element.fact.context.completeness) := by
  have admitted := (native_fact_accepted_iff element.fact requireComplete).mpr
    (And.intro valid complete)
  apply Exists.elim (merge_refines_max coverage element.fact.context.completeness)
  intro merged mergedSpec
  have dependenciesExact : (alloc.vec.Vec.deref element.dependencies).val =
      element.dependencies.val := by simp only [alloc.vec.Vec.deref, Slice.from_val]
  have sliceCapacity : pending.val.length + (alloc.vec.Vec.deref element.dependencies).val.length <=
      Usize.max := by simpa only [dependenciesExact] using capacity
  apply Exists.elim (append_identities_exact pending
    (alloc.vec.Vec.deref element.dependencies) sliceCapacity)
  intro next nextSpec
  refine Exists.intro next (Exists.intro merged (And.intro ?_ (And.intro (nextSpec.2.trans (congrArg (fun ids => pending.val ++ ids) dependenciesExact)) mergedSpec.2)))
  simp only [state.expand_identity, admitted, bind_ok, native_fact_context_exact,
    native_completeness_exact, mergedSpec.1, nextSpec.1]

end MRR.ContextRustProofs
