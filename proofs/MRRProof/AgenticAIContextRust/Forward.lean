import Expansion
import Driver

open Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

abbrev NativeTraversal := state.ClosureTraversal
abbrev NativeElements := List (Prod NativeFactId NativeElement)

def nativeLookup (elements : NativeElements) (id : NativeFactId) : Option NativeElement :=
  match elements with
  | [] => none
  | (key, value) :: rest => if key = id then some value else nativeLookup rest id

/-- Extensional container model law, not a stdlib implementation theorem. -/
theorem native_contains_exact (selected : List NativeFactId) (id : NativeFactId) :
    containsValue mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq id selected =
      .ok (decide (id inList selected)) := by
  induction selected with
  | nil => rfl
  | cons key rest induction =>
    simp only [containsValue, native_fact_id_equality_exact, bind_ok, induction]
    by_cases same : key = id
    case pos => simp [same]
    case neg => simp [same, Ne.symm same]

theorem native_set_insert_exact (selected : List NativeFactId) (id : NativeFactId) :
    alloc.collections.btree.set.BTreeSet.insert
      alloc.alloc.Global.Insts.CoreAllocAllocatorClone mrr_identity.api.FactId.Insts.CoreCmpOrd
      selected id = .ok (if id inList selected then (false, selected) else (true, id :: selected)) := by
  simp only [alloc.collections.btree.set.BTreeSet.insert,
    native_contains_exact, bind_ok]
  by_cases member : id inList selected <;> simp [member]

theorem native_map_lookup_exact (elements : NativeElements) (id : NativeFactId) :
    alloc.collections.btree.map.BTreeMap.get alloc.alloc.Global.Insts.CoreAllocAllocatorClone
      (core.borrow.Borrow.Blanket NativeFactId) mrr_identity.api.FactId.Insts.CoreCmpOrd
      mrr_identity.api.FactId.Insts.CoreCmpOrd elements id = .ok (nativeLookup elements id) := by
  change borrowedLookup core.borrow.Borrow.Blanket.borrow
    mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq id elements = _
  induction elements with
  | nil => rfl
  | cons pair rest induction =>
    cases pair with
    | mk key value =>
      simp only [borrowedLookup, core.borrow.Borrow.Blanket.borrow,
        native_fact_id_equality_exact, bind_ok, nativeLookup, induction]
      by_cases same : key = id <;> simp [same]

def forwardProject (traversal : NativeTraversal) : ContextWorklist :=
  { visited := traversal.selected.map factModelId, pending := pendingModel traversal.pending }

theorem selected_model_member (selected : List NativeFactId) (id : NativeFactId) :
    factModelId id inList selected.map factModelId <-> id inList selected := by
  constructor
  next =>
    intro member
    apply Exists.elim (List.mem_map.mp member)
    intro other otherSpec
    have same := fact_model_id_injective otherSpec.2
    simpa only [same] using otherSpec.1
  next => intro member; exact List.mem_map.mpr (Exists.intro id (And.intro member rfl))

theorem native_forward_empty (traversal : NativeTraversal) (empty : traversal.pending.val = []) :
    state.advance_closure traversal = .ok (false, traversal) := by
  simp only [state.advance_closure, pop_identity_empty traversal.pending empty, bind_ok]
  rfl

theorem native_forward_duplicate (traversal : NativeTraversal) (id : NativeFactId)
    (tail : List NativeFactId) (deps : Nat -> List Nat) (pending : traversal.pending.val.reverse = id :: tail)
    (known : id inList traversal.selected) :
    exists next, state.advance_closure traversal = .ok (true, next) /\
      next.elements = traversal.elements /\ next.require_complete = traversal.require_complete /\
      next.error = traversal.error /\ next.coverage = traversal.coverage /\
      forwardProject next = worklistStep deps (forwardProject traversal) := by
  apply Exists.elim (pop_identity_projects_pending traversal.pending id tail pending)
  intro remaining remainingSpec
  refine Exists.intro { traversal with pending := remaining } (And.intro ?_ ?_)
  next =>
    simp only [state.advance_closure, remainingSpec.1, bind_ok, uncurry]
    simp only [native_set_insert_exact, known, if_true, bind_ok, uncurry]
    try rfl
  next =>
    refine And.intro rfl (And.intro rfl (And.intro rfl (And.intro rfl ?_)))
    have modelKnown := (selected_model_member traversal.selected id).mpr known
    simp only [forwardProject, worklistStep, pendingModel, pending, List.map_cons,
      modelKnown, if_true]
    exact congrArg (fun rest => ContextWorklist.mk (traversal.selected.map factModelId) rest)
      remainingSpec.2


theorem native_forward_accepted (traversal : NativeTraversal) (id : NativeFactId)
    (tail : List NativeFactId) (deps : Nat -> List Nat)
    (pending : traversal.pending.val.reverse = id :: tail)
    (fresh : Not (id inList traversal.selected)) (element : NativeElement)
    (lookup : nativeLookup traversal.elements id = some element)
    (valid : element.fact.context.validity = .Valid)
    (complete : traversal.require_complete = false \/ element.fact.context.completeness = .Complete)
    (dependencies : deps (factModelId id) = element.dependencies.val.map factModelId)
    (capacity : tail.length + element.dependencies.val.length <= Usize.max) :
    exists next, state.advance_closure traversal = .ok (true, next) /\
      next.elements = traversal.elements /\ next.require_complete = traversal.require_complete /\
      next.error = traversal.error /\
      rank next.coverage = max (rank traversal.coverage) (rank element.fact.context.completeness) /\
      forwardProject next = worklistStep deps (forwardProject traversal) := by
  apply Exists.elim (pop_identity_projects_pending traversal.pending id tail pending)
  intro remaining remainingSpec
  have remainingLength : remaining.val.length = tail.length := by
    simpa only [pendingModel, List.length_map, List.length_reverse] using
      congrArg List.length remainingSpec.2
  have appendCapacity : remaining.val.length + element.dependencies.val.length <= Usize.max := by
    simpa only [remainingLength] using capacity
  apply Exists.elim (expansion_accepted_step id element traversal.require_complete valid complete
    remaining traversal.coverage traversal.error appendCapacity)
  intro nextPending pendingSpec
  apply Exists.elim pendingSpec
  intro merged mergeSpec
  let next : NativeTraversal := { traversal with pending := nextPending, selected := id :: traversal.selected, coverage := merged }
  refine Exists.intro next (And.intro ?_ ?_)
  next =>
    simp only [state.advance_closure, remainingSpec.1, bind_ok, uncurry]
    simp only [native_set_insert_exact, fresh, if_false, bind_ok, uncurry]
    simp only [native_map_lookup_exact, lookup, mergeSpec.1, bind_ok, uncurry]
    try rfl
  next =>
    refine And.intro rfl (And.intro rfl (And.intro rfl (And.intro mergeSpec.2.2 ?_)))
    have modelFresh : Not (factModelId id inList traversal.selected.map factModelId) := by
      intro member; exact fresh ((selected_model_member traversal.selected id).mp member)
    have remainingModel : remaining.val.reverse.map factModelId = tail.map factModelId := remainingSpec.2
    simp only [next, forwardProject, worklistStep, pendingModel, pending, List.map_cons,
      modelFresh, if_false, mergeSpec.2.1, List.reverse_append, List.map_append, remainingModel,
      dependencies, List.map_reverse]


/-- This precondition describes the admitted source and its declared graph. It
is not an assumed adapter simulation or stopping law. -/
def SourceAdmits (source : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool) : Prop :=
  forall id : NativeFactId, factModelId id inList source -> exists element,
    nativeLookup elements id = some element /\
    element.fact.context.validity = .Valid /\
    (requireComplete = false \/ element.fact.context.completeness = .Complete) /\
    deps (factModelId id) = element.dependencies.val.map factModelId

structure ForwardInvariant (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool) (traversal : NativeTraversal) : Prop where
  graph : WorklistInvariant roots deps (forwardProject traversal)
  elements : traversal.elements = elements
  policy : traversal.require_complete = requireComplete
  error : traversal.error = none
  budget : worklistCapacity source deps (forwardProject traversal) <= Usize.max

/-- Universal actual-source one-pop refinement under the named library models.
All BTree calls are evaluated by their explicit definitions, not step-law premises. -/
theorem native_forward_step_refines_graph (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source ->
      forall id, id inList deps parent -> id inList source)
    (admitted : SourceAdmits source deps elements requireComplete)
    (traversal : NativeTraversal)
    (invariant : ForwardInvariant source roots deps elements requireComplete traversal) :
    exists flag next, state.advance_closure traversal = .ok (flag, next) /\
      next.elements = traversal.elements /\ next.require_complete = traversal.require_complete /\
      next.error = traversal.error /\
      (if flag then Not ((forwardProject traversal).pending = []) /\
        forwardProject next = worklistStep deps (forwardProject traversal)
       else (forwardProject traversal).pending = [] /\ next = traversal) := by
  cases pendingEq : traversal.pending.val.reverse with
  | nil =>
    have empty : traversal.pending.val = [] := by
      simpa only [List.reverse_reverse, List.reverse_nil] using congrArg List.reverse pendingEq
    refine Exists.intro false (Exists.intro traversal
      (And.intro (native_forward_empty traversal empty)
        (And.intro rfl (And.intro rfl (And.intro rfl ?_)))))
    simp only [Bool.false_eq_true, if_false, forwardProject, pendingModel, pendingEq,
      List.map_nil, and_self]
  | cons id tail =>
    have nonempty : Not ((forwardProject traversal).pending = []) := by
      simp only [forwardProject, pendingModel, pendingEq, List.map_cons, List.cons_ne_nil, not_false_eq_true]
    by_cases known : id inList traversal.selected
    case pos =>
      apply Exists.elim (native_forward_duplicate traversal id tail deps pendingEq known)
      intro next nextSpec
      exact Exists.intro true (Exists.intro next (And.intro nextSpec.1
        (And.intro nextSpec.2.1 (And.intro nextSpec.2.2.1
          (And.intro nextSpec.2.2.2.1 (And.intro nonempty nextSpec.2.2.2.2.2))))))
    case neg =>
      have headRequired : Required roots deps (factModelId id) :=
        invariant.graph.pendingSound _ (by simp only [forwardProject, pendingModel, pendingEq, List.map_cons, List.mem_cons]; exact Or.inl True.intro)
      have headSource := closed_contains_required roots source deps rootsSource sourceClosed _ headRequired
      apply Exists.elim (admitted id headSource)
      intro element elementSpec
      have lookup : nativeLookup traversal.elements id = some element := by
        simpa only [invariant.elements] using elementSpec.1
      have complete : traversal.require_complete = false \/ element.fact.context.completeness = .Complete := by
        simpa only [invariant.policy] using elementSpec.2.2.1
      have modelFresh : Not (factModelId id inList traversal.selected.map factModelId) := by
        intro member; exact known ((selected_model_member traversal.selected id).mp member)
      have released := remaining_edges_fresh deps (traversal.selected.map factModelId) source
        (factModelId id) headSource modelFresh
      have pendingLength : traversal.pending.val.length = tail.length + 1 := by
        simpa only [List.length_reverse, List.length_cons] using congrArg List.length pendingEq
      have dependenciesLength : (deps (factModelId id)).length = element.dependencies.val.length := by
        simpa only [List.length_map] using congrArg List.length elementSpec.2.2.2
      have budget := invariant.budget
      simp only [worklistCapacity, forwardProject, pendingModel, List.length_map,
        List.length_reverse] at budget
      have capacity : tail.length + element.dependencies.val.length <= Usize.max := by omega
      apply Exists.elim (native_forward_accepted traversal id tail deps pendingEq known element
        lookup elementSpec.2.1 complete elementSpec.2.2.2 capacity)
      intro next nextSpec
      exact Exists.intro true (Exists.intro next (And.intro nextSpec.1
        (And.intro nextSpec.2.1 (And.intro nextSpec.2.2.1
          (And.intro nextSpec.2.2.2.1 (And.intro nonempty nextSpec.2.2.2.2.2))))))

def nativeForwardAdapter : worklist.Worklist NativeTraversal :=
  state.ClosureTraversal.Insts.Mrr_agentic_ai_contextWorklistWorklist

/-- The actual production adapter discharges the contract under the explicit
library models. No adapter refinement or stopping premise is supplied. -/
theorem native_forward_advance_contract (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source ->
      forall id, id inList deps parent -> id inList source)
    (admitted : SourceAdmits source deps elements requireComplete) :
    AdvanceContract nativeForwardAdapter
      (ForwardInvariant source roots deps elements requireComplete)
      (fun traversal => (forwardProject traversal).pending = [])
      (fun traversal => worklistCapacity source deps (forwardProject traversal)) := by
  intro traversal valid
  apply Exists.elim (native_forward_step_refines_graph source roots deps elements requireComplete
    rootsSource sourceClosed admitted traversal valid)
  intro flag flagSpec
  apply Exists.elim flagSpec
  intro next spec
  refine Exists.intro flag (Exists.intro next (And.intro spec.1 ?_))
  have simulation := spec.2.2.2.2
  cases flag
  case false =>
    simp only [Bool.false_eq_true, if_false] at simulation
    have equal := simulation.2
    subst next
    exact And.intro valid (And.intro (by intro impossible; cases impossible)
      (fun _ => simulation.1))
  case true =>
    simp only [if_true] at simulation
    have pendingSource : forall id, id inList (forwardProject traversal).pending -> id inList source := by
      intro id member
      exact closed_contains_required roots source deps rootsSource sourceClosed id
        (valid.graph.pendingSound id member)
    have decrease : worklistCapacity source deps (forwardProject next) <
        worklistCapacity source deps (forwardProject traversal) := by
      simpa only [simulation.2] using worklist_capacity_decreases source deps
        (forwardProject traversal) pendingSource simulation.1
    have nextValid : ForwardInvariant source roots deps elements requireComplete next :=
      { graph := by simpa only [simulation.2] using worklist_step_invariant roots deps _ valid.graph
        elements := spec.2.1.trans valid.elements
        policy := spec.2.2.1.trans valid.policy
        error := spec.2.2.2.1.trans valid.error
        budget := Nat.le_trans (Nat.le_of_lt decrease) valid.budget }
    exact And.intro nextValid (And.intro (fun _ => decrease)
      (by intro impossible; cases impossible))

/-- Universal total correctness of the extracted native forward traversal,
relative to the documented standard-library value models and admitted graph.
There is no fuel bound and no assumed adapter law in this theorem. -/
theorem native_forward_run_exact (source roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source ->
      forall id, id inList deps parent -> id inList source)
    (admitted : SourceAdmits source deps elements requireComplete)
    (initial : NativeTraversal)
    (valid : ForwardInvariant source roots deps elements requireComplete initial) :
    exists final, state.run_closure initial = .ok final /\ final.error = none /\
      (forwardProject final).pending = [] /\
      forall id : NativeFactId, id inList final.selected <-> Required roots deps (factModelId id) := by
  apply Exists.elim (extracted_driver_preserves_invariant nativeForwardAdapter
    (ForwardInvariant source roots deps elements requireComplete)
    (fun traversal => (forwardProject traversal).pending = [])
    (fun traversal => worklistCapacity source deps (forwardProject traversal))
    (native_forward_advance_contract source roots deps elements requireComplete
      rootsSource sourceClosed admitted) initial valid)
  intro final spec
  refine Exists.intro final (And.intro spec.1
    (And.intro spec.2.1.error (And.intro spec.2.2 ?_)))
  intro id
  have exactSet := worklist_finished_invariant_exact roots deps (forwardProject final)
    spec.2.1.graph spec.2.2 (factModelId id)
  exact (selected_model_member final.selected id).symm.trans exactSet

end MRR.ContextRustProofs
