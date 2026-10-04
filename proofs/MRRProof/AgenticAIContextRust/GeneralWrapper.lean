import GeneralForward

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

def ClosureResultSound (roots : List Nat) (deps : Nat -> List Nat)
    (elements : NativeElements) (requireComplete : Bool)
    (result : core.result.Result state.AgenticAiContextClosure NativeContextError) : Prop :=
  match result with
  | .Err failure => RejectionReason elements requireComplete failure
  | .Ok closure => forall id : NativeFactId, id inList closure.elements.val <-> Required roots deps (factModelId id)

/-- Actual outer Result is total even when source evidence is rejected, after
any number of accepted or duplicate pops. No source-admission assumption. -/
theorem native_required_closure_wrapper_total (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (elements : NativeElements)
    (source : List Nat) (deps : Nat -> List Nat)
    (rootsSource : forall id, id inList (closureRoots query contract).map factModelId -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (sourceMatch : SourceMatches source deps elements)
    (budget : (closureRoots query contract).length + remainingEdges deps [] source <= Usize.max)
    (outputCapacity : source.length <= Usize.max) :
    exists result, state.compute_required_closure query contract elements = .ok result /\
      ClosureResultSound ((closureRoots query contract).map factModelId) deps elements contract.require_complete result := by
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
  apply Exists.elim (native_general_forward_run_total source ((closureRoots query contract).map factModelId)
    deps elements contract.require_complete rootsSource sourceClosed sourceMatch initial valid)
  intro final finalSpec
  cases finalSpec.2 with
  | inl rejected =>
    apply Exists.elim rejected
    intro failure failureSpec
    exact Exists.intro (.Err failure) (And.intro
      (native_required_closure_error_projection query contract elements pending final failure
        pendingSpec.1 finalSpec.1 failureSpec.1) failureSpec.2)
  | inr accepted =>
    have selectedSubset : forall id, id inList (forwardProject final).visited -> id inList source := by
      intro id member
      apply Exists.elim (List.mem_map.mp member)
      intro native nativeSpec
      have required := (accepted.2.2.2 native).mp nativeSpec.1
      have found := closed_contains_required _ source deps rootsSource sourceClosed _ required
      simpa only [nativeSpec.2] using found
    have selectedBound := model_nodup_subset_length (forwardProject final).visited source accepted.2.2.1 selectedSubset
    have collectCapacity : final.selected.length <= Usize.max := by
      simp only [forwardProject, List.length_map] at selectedBound
      omega
    apply Exists.elim (native_owned_collect_exact final.selected collectCapacity)
    intro output outputSpec
    let closure : state.AgenticAiContextClosure := { elements := output, coverage := final.coverage }
    refine Exists.intro (.Ok closure) (And.intro ?_ ?_)
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
        alloc.collections.btree.set.BTreeSetTGlobal.new, bind_ok, runInitialized, accepted.1,
        alloc.collections.btree.set.BTreeSet.Insts.CoreIterTraitsCollectIntoIteratorTIntoIter.into_iter,
        collected, closure]
    next => simpa only [ClosureResultSound, closure, outputSpec.2] using accepted.2.2.2

end MRR.ContextRustProofs
