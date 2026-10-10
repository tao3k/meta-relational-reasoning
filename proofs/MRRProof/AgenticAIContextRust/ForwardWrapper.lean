import Coverage
import Rejection

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

abbrev ownedFactIterator := alloc.collections.btree.set.IntoIter.Insts.CoreIterTraitsIteratorIterator
  NativeFactId alloc.alloc.Global.Insts.CoreAllocAllocatorClone

theorem native_owned_iterator_list_exact (remaining accumulator : List NativeFactId) :
    alloc.vec.FromIteratorVec.iterToList ownedFactIterator remaining accumulator =
      .ok (accumulator.reverse ++ remaining) := by
  induction remaining generalizing accumulator with
  | nil =>
    rw [alloc.vec.FromIteratorVec.iterToList.eq_def]
    simp only [ownedFactIterator,
      alloc.collections.btree.set.IntoIter.Insts.CoreIterTraitsIteratorIterator,
      alloc.collections.btree.set.IntoIter.Insts.CoreIterTraitsIteratorIterator.next,
      bind_tc_ok, List.append_nil]
  | cons id rest induction =>
    rw [alloc.vec.FromIteratorVec.iterToList.eq_def]
    simp only [ownedFactIterator,
      alloc.collections.btree.set.IntoIter.Insts.CoreIterTraitsIteratorIterator,
      alloc.collections.btree.set.IntoIter.Insts.CoreIterTraitsIteratorIterator.next,
      bind_tc_ok]
    rw [induction]
    simp only [List.reverse_cons, List.append_assoc, List.singleton_append]

theorem native_owned_collect_exact (values : List NativeFactId)
    (capacity : values.length <= Usize.max) :
    exists output, core.iter.traits.iterator.Iterator.collect.default ownedFactIterator
      (core.iter.traits.collect.FromIteratorVec NativeFactId) values = .ok output /\
      output.val = values := by
  refine Exists.intro (alloc.vec.Vec.from values capacity) (And.intro ?_ ?_)
  next =>
    change alloc.vec.FromIteratorVec.from_iter
      (core.iter.traits.collect.IntoIterator.Blanket ownedFactIterator) values =
        .ok (alloc.vec.Vec.from values capacity)
    simp only [alloc.vec.FromIteratorVec.from_iter,
      core.iter.traits.collect.IntoIterator.Blanket.into_iter, bind_tc_ok,
      native_owned_iterator_list_exact, List.reverse_nil, List.nil_append, dif_pos capacity]

  next => exact alloc.vec.Vec.from_val values capacity

def closureRoots (query : state.AgenticAiContextQuery) (contract : state.AgenticAiContextContract) :
    List NativeFactId := query.roots.val ++ contract.required.val ++ contract.temporal_receipts.val

def closureSeed (_query : state.AgenticAiContextQuery) (contract : state.AgenticAiContextContract)
    (elements : NativeElements) (pending : alloc.vec.Vec NativeFactId) : NativeTraversal :=
  { elements := elements, require_complete := contract.require_complete, pending := pending,
    selected := [], coverage := .Complete, error := none }

theorem native_closure_seed_exact (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (deps : Nat -> List Nat)
    (capacity : (closureRoots query contract).length <= Usize.max) :
    exists pending, worklist.initial_pending (alloc.vec.Vec.deref query.roots)
      (alloc.vec.Vec.deref contract.required) (alloc.vec.Vec.deref contract.temporal_receipts) = .ok pending /\
      pending.val = closureRoots query contract /\
      WorklistInvariant ((closureRoots query contract).map factModelId) deps
        { visited := [], pending := pendingModel pending } := by
  have sumCapacity : (alloc.vec.Vec.deref query.roots).val.length +
      (alloc.vec.Vec.deref contract.required).val.length +
      (alloc.vec.Vec.deref contract.temporal_receipts).val.length <= Usize.max := by
    simpa only [closureRoots, alloc.vec.Vec.deref, Slice.from_val, List.length_append] using capacity
  apply Exists.elim (initial_pending_exact (alloc.vec.Vec.deref query.roots)
    (alloc.vec.Vec.deref contract.required) (alloc.vec.Vec.deref contract.temporal_receipts) sumCapacity)
  intro pending pendingSpec
  have contents : pending.val = closureRoots query contract := by
    simpa only [closureRoots, alloc.vec.Vec.deref, Slice.from_val] using pendingSpec.2
  refine Exists.intro pending (And.intro pendingSpec.1 (And.intro contents ?_))
  have projected : pendingModel pending = ((closureRoots query contract).map factModelId).reverse := by
    simp only [pendingModel, contents, List.map_reverse]
  rw [projected]
  exact worklist_initial_invariant _ deps

theorem model_nodup_subset_length (ids source : List Nat)
    (unique : ids.Nodup) (subset : forall id, id inList ids -> id inList source) :
    ids.length <= source.length := by
  induction ids generalizing source with
  | nil => simp
  | cons head rest induction =>
    have distinct := List.nodup_cons.mp unique
    have found : head inList source := subset head (by simp)
    have smaller : forall id, id inList rest -> id inList source.erase head := by
      intro id member
      have different : Not (id = head) := by intro equal; subst id; exact distinct.1 member
      exact (List.mem_erase_of_ne different).mpr (subset id (by simp [member]))
    have bound := induction (source.erase head) distinct.2 smaller
    have removed := List.length_erase_of_mem found
    have positive : 0 < source.length := by
      cases source with
      | nil => simp at found
      | cons id remaining => simp
    simp only [List.length_cons]
    omega

/-- The actual outer wrapper: actual seed, actual run, actual collect and typed
Ok projection. No initialized-traversal, collection or adapter law is assumed. -/
theorem native_required_closure_wrapper_exact (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (elements : NativeElements)
    (source : List Nat) (deps : Nat -> List Nat)
    (rootsSource : forall id, id inList (closureRoots query contract).map factModelId -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (admitted : SourceAdmits source deps elements contract.require_complete)
    (budget : (closureRoots query contract).length + remainingEdges deps [] source <= Usize.max)
    (outputCapacity : source.length <= Usize.max) :
    exists closure, state.compute_required_closure query contract elements = .ok (.Ok closure) /\
      (forall id : NativeFactId, id inList closure.elements.val <->
        Required ((closureRoots query contract).map factModelId) deps (factModelId id)) /\
      rank closure.coverage = selectedEvidenceRank elements (closure.elements.val.map factModelId) := by
  have seedCapacity : (closureRoots query contract).length <= Usize.max := by omega
  apply Exists.elim (native_closure_seed_exact query contract deps seedCapacity)
  intro pending pendingSpec
  let initial := closureSeed query contract elements pending
  have valid : ForwardInvariant source ((closureRoots query contract).map factModelId) deps
      elements contract.require_complete initial :=
    { graph := pendingSpec.2.2
      elements := rfl
      policy := rfl
      error := rfl
      budget := by
        simpa only [initial, closureSeed, forwardProject, worklistCapacity, pendingModel,
          List.length_map, List.length_reverse, List.map_nil, pendingSpec.2.1] using budget }
  have coverage := native_empty_coverage_invariant elements initial rfl rfl
  apply Exists.elim (native_forward_run_coverage_exact source ((closureRoots query contract).map factModelId)
    deps elements contract.require_complete rootsSource sourceClosed admitted initial valid coverage)
  intro final finalSpec
  have selectedSubset : forall id, id inList (forwardProject final).visited -> id inList source := by
    intro id member
    exact closed_contains_required _ source deps rootsSource sourceClosed id
      (finalSpec.2.2.2.2.1.graph.visitedSound id member)
  have selectedBound := model_nodup_subset_length (forwardProject final).visited source
    finalSpec.2.2.2.2.1.graph.nodup selectedSubset
  have collectCapacity : final.selected.length <= Usize.max := by
    simp only [forwardProject, List.length_map] at selectedBound
    omega
  apply Exists.elim (native_owned_collect_exact final.selected collectCapacity)
  intro output outputSpec
  let closure : state.AgenticAiContextClosure := { elements := output, coverage := final.coverage }
  refine Exists.intro closure (And.intro ?_ (And.intro ?_ ?_))
  next =>
    have runInitialized : state.run_closure
        { elements := elements, require_complete := contract.require_complete, pending := pending,
          selected := [], coverage := .Complete, error := none } = .ok final := by
      simpa only [initial, closureSeed] using finalSpec.1
    have collected : core.iter.traits.iterator.Iterator.collect.default
        (alloc.collections.btree.set.IntoIter.Insts.CoreIterTraitsIteratorIterator NativeFactId
          alloc.alloc.Global.Insts.CoreAllocAllocatorClone)
        (core.iter.traits.collect.FromIteratorVec NativeFactId) final.selected = .ok output := outputSpec.1
    simp only [state.compute_required_closure, pendingSpec.1,
      alloc.collections.btree.set.BTreeSetTGlobal.new, bind_ok, runInitialized,
      finalSpec.2.1,
      alloc.collections.btree.set.BTreeSet.Insts.CoreIterTraitsCollectIntoIteratorTIntoIter.into_iter,
      collected, closure]
  next => simpa only [closure, outputSpec.2] using finalSpec.2.2.2.2.2
  next => simpa only [CoverageInvariant, forwardProject, closure, outputSpec.2] using finalSpec.2.2.2.1

/-- The actual outer wrapper retains the driver's exact error and returns Err,
without entering owned collection. This law assumes only the actual run receipt. -/
theorem native_required_closure_error_projection (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (elements : NativeElements)
    (pending : alloc.vec.Vec NativeFactId) (final : NativeTraversal) (failure : NativeContextError)
    (seed : worklist.initial_pending (alloc.vec.Vec.deref query.roots)
      (alloc.vec.Vec.deref contract.required) (alloc.vec.Vec.deref contract.temporal_receipts) = .ok pending)
    (runExact : state.run_closure (closureSeed query contract elements pending) = .ok final)
    (error : final.error = some failure) :
    state.compute_required_closure query contract elements = .ok (.Err failure) := by
  have runInitialized : state.run_closure
      { elements := elements, require_complete := contract.require_complete, pending := pending,
        selected := [], coverage := .Complete, error := none } = .ok final := by
    simpa only [closureSeed] using runExact
  simp only [state.compute_required_closure, seed,
    alloc.collections.btree.set.BTreeSetTGlobal.new, bind_ok, runInitialized, error]

end MRR.ContextRustProofs
