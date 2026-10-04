import ForwardWrapper

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

/-- Graph correspondence only. Missing, invalid and incomplete facts are allowed. -/
def SourceMatches (source : List Nat) (deps : Nat -> List Nat) (elements : NativeElements) : Prop :=
  forall id : NativeFactId, factModelId id inList source -> forall element,
    nativeLookup elements id = some element -> deps (factModelId id) = element.dependencies.val.map factModelId

def RejectionReason (elements : NativeElements) (requireComplete : Bool) (failure : NativeContextError) : Prop :=
  (exists id, failure = .UnknownElement id /\ nativeLookup elements id = none) \/
  (exists id element cause, failure = .InvalidatedElement id /\
    nativeLookup elements id = some element /\ element.fact.context.validity = .InvalidatedBy cause) \/
  (exists id element, failure = .IncompleteEvidence id /\ nativeLookup elements id = some element /\
    element.fact.context.validity = .Valid /\ requireComplete = true /\
    Not (element.fact.context.completeness = .Complete))

def GeneralOutcome (roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool) (final : NativeTraversal) : Prop :=
  (exists failure, final.error = some failure /\ RejectionReason elements requireComplete failure) \/
  (final.error = none /\ (forwardProject final).pending = [] /\
    (forwardProject final).visited.Nodup /\ forall id : NativeFactId, id inList final.selected <-> Required roots deps (factModelId id))

theorem native_rejected_step_exact (traversal : NativeTraversal)
    (id : NativeFactId) (tail : List NativeFactId)
    (pending : traversal.pending.val.reverse = id :: tail)
    (fresh : Not (id inList traversal.selected)) (value : Option NativeElement)
    (lookup : nativeLookup traversal.elements id = value) (failure : NativeContextError)
    (rejected : forall remaining, state.expand_identity id value traversal.require_complete
      remaining traversal.coverage traversal.error = .ok (false, remaining, traversal.coverage, some failure)) :
    exists next, state.advance_closure traversal = .ok (false, next) /\ next.error = some failure := by
  apply Exists.elim (pop_identity_projects_pending traversal.pending id tail pending)
  intro remaining remainingSpec
  let next : NativeTraversal := { traversal with pending := remaining, selected := id :: traversal.selected, error := some failure }
  refine Exists.intro next (And.intro ?_ rfl)
  simp only [state.advance_closure, remainingSpec.1, bind_ok, uncurry,
    native_set_insert_exact, fresh, if_false, if_true, native_map_lookup_exact, lookup, rejected, next]

theorem native_continuation_invariant (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (traversal next : NativeTraversal)
    (valid : ForwardInvariant source roots deps elements requireComplete traversal)
    (nonempty : Not ((forwardProject traversal).pending = []))
    (sameElements : next.elements = traversal.elements)
    (samePolicy : next.require_complete = traversal.require_complete)
    (sameError : next.error = traversal.error)
    (graphStep : forwardProject next = worklistStep deps (forwardProject traversal)) :
    ForwardInvariant source roots deps elements requireComplete next /\
      worklistCapacity source deps (forwardProject next) < worklistCapacity source deps (forwardProject traversal) := by
  have pendingSource : forall id, id inList (forwardProject traversal).pending -> id inList source := by
    intro id member
    exact closed_contains_required roots source deps rootsSource sourceClosed id (valid.graph.pendingSound id member)
  have decrease : worklistCapacity source deps (forwardProject next) < worklistCapacity source deps (forwardProject traversal) := by
    simpa only [graphStep] using worklist_capacity_decreases source deps (forwardProject traversal) pendingSource nonempty
  exact And.intro
    { graph := by simpa only [graphStep] using worklist_step_invariant roots deps _ valid.graph
      elements := sameElements.trans valid.elements
      policy := samePolicy.trans valid.policy
      error := sameError.trans valid.error
      budget := Nat.le_trans (Nat.le_of_lt decrease) valid.budget } decrease

/-- Actual step classification permits typed rejection at any accepted prefix. -/
theorem native_general_forward_step (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (sourceMatch : SourceMatches source deps elements) (traversal : NativeTraversal)
    (valid : ForwardInvariant source roots deps elements requireComplete traversal) :
    exists flag next, state.advance_closure traversal = .ok (flag, next) /\
      (if flag then ForwardInvariant source roots deps elements requireComplete next /\
        worklistCapacity source deps (forwardProject next) < worklistCapacity source deps (forwardProject traversal)
       else GeneralOutcome roots deps elements requireComplete next) := by
  cases pendingEq : traversal.pending.val.reverse with
  | nil =>
    have empty : traversal.pending.val = [] := by
      simpa only [List.reverse_reverse, List.reverse_nil] using congrArg List.reverse pendingEq
    have finished : (forwardProject traversal).pending = [] := by
      simp only [forwardProject, pendingModel, empty, List.reverse_nil, List.map_nil]
    refine Exists.intro false (Exists.intro traversal (And.intro (native_forward_empty traversal empty) ?_))
    simp only [Bool.false_eq_true, if_false]
    apply Or.inr
    refine And.intro valid.error (And.intro finished (And.intro valid.graph.nodup ?_))
    intro id
    exact (selected_model_member traversal.selected id).symm.trans
      (worklist_finished_invariant_exact roots deps _ valid.graph finished (factModelId id))
  | cons id tail =>
    have nonempty : Not ((forwardProject traversal).pending = []) := by
      simp only [forwardProject, pendingModel, pendingEq, List.map_cons, List.cons_ne_nil, not_false_eq_true]
    by_cases known : id inList traversal.selected
    case pos =>
      apply Exists.elim (native_forward_duplicate traversal id tail deps pendingEq known)
      intro next spec
      refine Exists.intro true (Exists.intro next (And.intro spec.1 ?_))
      simp only [if_true]
      exact native_continuation_invariant source roots deps elements requireComplete rootsSource sourceClosed
        traversal next valid nonempty spec.2.1 spec.2.2.1 spec.2.2.2.1 spec.2.2.2.2.2
    case neg =>
      have reject : forall value failure, nativeLookup elements id = value ->
          RejectionReason elements requireComplete failure ->
          (forall remaining, state.expand_identity id value traversal.require_complete remaining
            traversal.coverage traversal.error = .ok (false, remaining, traversal.coverage, some failure)) ->
          exists flag next, state.advance_closure traversal = .ok (flag, next) /\
            (if flag then ForwardInvariant source roots deps elements requireComplete next /\
              worklistCapacity source deps (forwardProject next) < worklistCapacity source deps (forwardProject traversal)
             else GeneralOutcome roots deps elements requireComplete next) := by
        intro value failure lookup reason rejected
        have currentLookup : nativeLookup traversal.elements id = value := by simpa only [valid.elements] using lookup
        apply Exists.elim (native_rejected_step_exact traversal id tail pendingEq known value currentLookup failure rejected)
        intro next spec
        exact Exists.intro false (Exists.intro next (And.intro spec.1
          (Or.inl (Exists.intro failure (And.intro spec.2 reason)))))
      cases lookup : nativeLookup elements id with
      | none =>
        apply reject none (.UnknownElement id) lookup
        next => exact Or.inl (Exists.intro id (And.intro rfl lookup))
        next => exact fun remaining => expansion_missing_stops id traversal.require_complete remaining traversal.coverage traversal.error
      | some element =>
        cases validity : element.fact.context.validity with
        | InvalidatedBy cause =>
          apply reject (some element) (.InvalidatedElement id) lookup
          next => exact Or.inr (Or.inl (Exists.intro id (Exists.intro element (Exists.intro cause
            (And.intro rfl (And.intro lookup validity))))))
          next => exact fun remaining => expansion_invalid_stops id cause element validity traversal.require_complete remaining traversal.coverage traversal.error
        | Valid =>
          by_cases complete : traversal.require_complete = false \/ element.fact.context.completeness = .Complete
          case neg =>
            have required : traversal.require_complete = true := by
              cases policy : traversal.require_complete
              case false => exact False.elim (complete (Or.inl policy))
              case true => rfl
            have incomplete : Not (element.fact.context.completeness = .Complete) := fun equal => complete (Or.inr equal)
            apply reject (some element) (.IncompleteEvidence id) lookup
            next => exact Or.inr (Or.inr (Exists.intro id (Exists.intro element
              (And.intro rfl (And.intro lookup (And.intro validity (And.intro (valid.policy.symm.trans required) incomplete)))))))
            next => intro remaining; rw [required]; exact expansion_incomplete_stops id element validity incomplete remaining traversal.coverage traversal.error
          case pos =>
            have headRequired : Required roots deps (factModelId id) := valid.graph.pendingSound _
              (by simp only [forwardProject, pendingModel, pendingEq, List.map_cons, List.mem_cons]; exact Or.inl True.intro)
            have headSource := closed_contains_required roots source deps rootsSource sourceClosed _ headRequired
            have dependencies := sourceMatch id headSource element lookup
            have currentLookup : nativeLookup traversal.elements id = some element := by simpa only [valid.elements] using lookup
            have modelFresh : Not (factModelId id inList traversal.selected.map factModelId) := by
              intro member; exact known ((selected_model_member traversal.selected id).mp member)
            have released := remaining_edges_fresh deps (traversal.selected.map factModelId) source (factModelId id) headSource modelFresh
            have pendingLength : traversal.pending.val.length = tail.length + 1 := by
              simpa only [List.length_reverse, List.length_cons] using congrArg List.length pendingEq
            have dependenciesLength : (deps (factModelId id)).length = element.dependencies.val.length := by
              simpa only [List.length_map] using congrArg List.length dependencies
            have budget := valid.budget
            simp only [worklistCapacity, forwardProject, pendingModel, List.length_map, List.length_reverse] at budget
            have capacity : tail.length + element.dependencies.val.length <= Usize.max := by omega
            apply Exists.elim (native_forward_accepted traversal id tail deps pendingEq known element currentLookup
              validity complete dependencies capacity)
            intro next spec
            refine Exists.intro true (Exists.intro next (And.intro spec.1 ?_))
            simp only [if_true]
            exact native_continuation_invariant source roots deps elements requireComplete rootsSource sourceClosed
              traversal next valid nonempty spec.2.1 spec.2.2.1 spec.2.2.2.1 spec.2.2.2.2.2

/-- Universal actual-driver termination and sound typed errors. No source
admission or fuel premise: every accepted prefix is covered by the rank proof. -/
theorem native_general_forward_run_total (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (sourceMatch : SourceMatches source deps elements) (initial : NativeTraversal)
    (valid : ForwardInvariant source roots deps elements requireComplete initial) :
    exists final, state.run_closure initial = .ok final /\ GeneralOutcome roots deps elements requireComplete final := by
  have total : WP.spec (loop (worklist.run_loop.body nativeForwardAdapter) initial)
      (GeneralOutcome roots deps elements requireComplete) := by
    apply loop.spec_decr_nat (fun traversal => worklistCapacity source deps (forwardProject traversal))
      (ForwardInvariant source roots deps elements requireComplete) (GeneralOutcome roots deps elements requireComplete)
    next =>
      intro traversal current
      apply Exists.elim (native_general_forward_step source roots deps elements requireComplete rootsSource sourceClosed sourceMatch traversal current)
      intro flag flagSpec
      apply Exists.elim flagSpec
      intro next spec
      cases flag <;> simp only [worklist.run_loop.body, nativeForwardAdapter,
        state.ClosureTraversal.Insts.Mrr_agentic_ai_contextWorklistWorklist,
        state.ClosureTraversal.Insts.Mrr_agentic_ai_contextWorklistWorklist.advance,
        spec.1, bind_ok] <;> apply (WP.spec_ok _).mpr <;> simpa only [Bool.false_eq_true, if_false, if_true] using spec.2
    next => exact valid
  exact (WP.spec_equiv_exists _ _).mp total

end MRR.ContextRustProofs
