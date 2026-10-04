import Forward

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

def modelEvidenceRank : NativeElements -> Nat -> Nat
  | [], _ => 0
  | (key, element) :: rest, id =>
    if factModelId key = id then rank element.fact.context.completeness else modelEvidenceRank rest id

theorem native_evidence_rank_projection (elements : NativeElements) (id : NativeFactId) :
    modelEvidenceRank elements (factModelId id) =
      ((nativeLookup elements id).map (fun element => rank element.fact.context.completeness)).getD 0 := by
  induction elements with
  | nil => rfl
  | cons pair rest induction =>
    cases pair with
    | mk key element =>
      by_cases same : key = id
      case pos => simp [modelEvidenceRank, nativeLookup, same]
      case neg =>
        have different : Not (factModelId key = factModelId id) := fun equal => same (fact_model_id_injective equal)
        simp [modelEvidenceRank, nativeLookup, same, different, induction]

def selectedEvidenceRank (elements : NativeElements) (visited : List Nat) : Nat :=
  visited.foldr (fun id total => max (modelEvidenceRank elements id) total) 0

def CoverageInvariant (elements : NativeElements) (traversal : NativeTraversal) : Prop :=
  rank traversal.coverage = selectedEvidenceRank elements (forwardProject traversal).visited

/-- Universal aggregate law for an actual source step, including duplicate pops. -/
theorem native_forward_step_preserves_coverage (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (admitted : SourceAdmits source deps elements requireComplete)
    (traversal : NativeTraversal)
    (valid : ForwardInvariant source roots deps elements requireComplete traversal)
    (coverage : CoverageInvariant elements traversal)
    (flag : Bool) (next : NativeTraversal)
    (advanced : state.advance_closure traversal = .ok (flag, next)) : CoverageInvariant elements next := by
  cases pendingEq : traversal.pending.val.reverse with
  | nil =>
    have empty : traversal.pending.val = [] := by
      simpa only [List.reverse_reverse, List.reverse_nil] using congrArg List.reverse pendingEq
    have same : next = traversal := congrArg Prod.snd (Result.ok_injective
      (advanced.symm.trans (native_forward_empty traversal empty)))
    simpa only [same] using coverage
  | cons id tail =>
    by_cases known : id inList traversal.selected
    case pos =>
      apply Exists.elim (native_forward_duplicate traversal id tail deps pendingEq known)
      intro duplicate duplicateSpec
      have same : next = duplicate := congrArg Prod.snd (Result.ok_injective (advanced.symm.trans duplicateSpec.1))
      have modelKnown := (selected_model_member traversal.selected id).mpr known
      unfold CoverageInvariant at *
      rw [same, duplicateSpec.2.2.2.2.1, duplicateSpec.2.2.2.2.2]
      simpa only [worklistStep, forwardProject, pendingModel, pendingEq, List.map_cons,
        modelKnown, if_true] using coverage
    case neg =>
      have headRequired : Required roots deps (factModelId id) :=
        valid.graph.pendingSound _ (by simp only [forwardProject, pendingModel, pendingEq, List.map_cons, List.mem_cons]; exact Or.inl True.intro)
      have headSource := closed_contains_required roots source deps rootsSource sourceClosed _ headRequired
      apply Exists.elim (admitted id headSource)
      intro element elementSpec
      have lookup : nativeLookup traversal.elements id = some element := by
        simpa only [valid.elements] using elementSpec.1
      have complete : traversal.require_complete = false \/ element.fact.context.completeness = .Complete := by
        simpa only [valid.policy] using elementSpec.2.2.1
      have modelFresh : Not (factModelId id inList traversal.selected.map factModelId) := by
        intro member; exact known ((selected_model_member traversal.selected id).mp member)
      have released := remaining_edges_fresh deps (traversal.selected.map factModelId) source
        (factModelId id) headSource modelFresh
      have pendingLength : traversal.pending.val.length = tail.length + 1 := by
        simpa only [List.length_reverse, List.length_cons] using congrArg List.length pendingEq
      have dependenciesLength : (deps (factModelId id)).length = element.dependencies.val.length := by
        simpa only [List.length_map] using congrArg List.length elementSpec.2.2.2
      have budget := valid.budget
      simp only [worklistCapacity, forwardProject, pendingModel, List.length_map, List.length_reverse] at budget
      have capacity : tail.length + element.dependencies.val.length <= Usize.max := by omega
      apply Exists.elim (native_forward_accepted traversal id tail deps pendingEq known element
        lookup elementSpec.2.1 complete elementSpec.2.2.2 capacity)
      intro accepted acceptedSpec
      have same : next = accepted := congrArg Prod.snd (Result.ok_injective (advanced.symm.trans acceptedSpec.1))
      have rankExact : modelEvidenceRank elements (factModelId id) = rank element.fact.context.completeness := by
        simp only [native_evidence_rank_projection, elementSpec.1, Option.map_some, Option.getD_some]
      unfold CoverageInvariant at *
      rw [same, acceptedSpec.2.2.2.2.1, acceptedSpec.2.2.2.2.2, coverage]
      simp only [worklistStep, forwardProject, pendingModel, pendingEq, List.map_cons,
        modelFresh, if_false, selectedEvidenceRank, List.foldr_cons, rankExact]
      exact Nat.max_comm _ _

/-- Exact selected closure together with the actual aggregate coverage rank. -/
theorem native_forward_run_coverage_exact (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (admitted : SourceAdmits source deps elements requireComplete)
    (initial : NativeTraversal)
    (valid : ForwardInvariant source roots deps elements requireComplete initial)
    (coverage : CoverageInvariant elements initial) :
    exists final, state.run_closure initial = .ok final /\ final.error = none /\
      (forwardProject final).pending = [] /\ CoverageInvariant elements final /\
      forall id : NativeFactId, id inList final.selected <-> Required roots deps (factModelId id) := by
  let invariant := fun traversal => ForwardInvariant source roots deps elements requireComplete traversal /\
    CoverageInvariant elements traversal
  have contract : AdvanceContract nativeForwardAdapter invariant
      (fun traversal => (forwardProject traversal).pending = [])
      (fun traversal => worklistCapacity source deps (forwardProject traversal)) := by
    intro traversal current
    apply Exists.elim (native_forward_advance_contract source roots deps elements requireComplete
      rootsSource sourceClosed admitted traversal current.1)
    intro flag flagSpec
    apply Exists.elim flagSpec
    intro next spec
    refine Exists.intro flag (Exists.intro next (And.intro spec.1 (And.intro ?_ spec.2.2)))
    exact And.intro spec.2.1 (native_forward_step_preserves_coverage source roots deps elements requireComplete
      rootsSource sourceClosed admitted traversal current.1 current.2 flag next spec.1)
  apply Exists.elim (extracted_driver_preserves_invariant nativeForwardAdapter invariant
    (fun traversal => (forwardProject traversal).pending = [])
    (fun traversal => worklistCapacity source deps (forwardProject traversal)) contract initial (And.intro valid coverage))
  intro final spec
  refine Exists.intro final (And.intro spec.1 (And.intro spec.2.1.1.error
    (And.intro spec.2.2 (And.intro spec.2.1.2 ?_))))
  intro id
  have exactSet := worklist_finished_invariant_exact roots deps (forwardProject final)
    spec.2.1.1.graph spec.2.2 (factModelId id)
  exact (selected_model_member final.selected id).symm.trans exactSet

theorem native_empty_coverage_invariant (elements : NativeElements) (traversal : NativeTraversal)
    (empty : traversal.selected = []) (complete : traversal.coverage = .Complete) :
    CoverageInvariant elements traversal := by
  simp only [CoverageInvariant, forwardProject, empty, List.map_nil,
    selectedEvidenceRank, List.foldr_nil, complete, rank]

end MRR.ContextRustProofs
