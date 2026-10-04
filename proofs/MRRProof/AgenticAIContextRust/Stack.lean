import NativeFacts
import MRR.AgenticAIContext.Worklist

open Aeneas Aeneas.Std
open MRR.ContextRust

namespace MRR.ContextRustProofs

theorem identity_clone_exact (id : NativeFactId) :
    mrr_identity.api.FactId.Insts.CoreCloneClone.clone id = .ok id := by rfl

theorem pop_identity_empty (pending : alloc.vec.Vec NativeFactId)
    (empty : pending.val = []) : worklist.pop_identity pending = .ok (none, pending) := by
  have lengthZero : alloc.vec.Vec.len pending = 0#usize := by scalar_tac
  simp [worklist.pop_identity, lengthZero]

theorem pop_identity_nonempty (pending : alloc.vec.Vec NativeFactId)
    (nonempty : 0 < pending.val.length) :
    exists id next, worklist.pop_identity pending = .ok (some id, next) /\
      id = pending.val[pending.val.length - 1] /\
      next.val = pending.val.resize (pending.val.length - 1) id := by
  have lengthNonzero : Not (alloc.vec.Vec.len pending = 0#usize) := by scalar_tac
  have total : WP.spec (worklist.pop_identity pending) (fun output =>
      exists id, output.1 = some id /\ id = pending.val[pending.val.length - 1] /\
        output.2.val = pending.val.resize (pending.val.length - 1) id) := by
    simp only [worklist.pop_identity, lengthNonzero, ite_false]
    step
    step
    step
    case hClone => rfl
    case a =>
      refine Exists.intro id (And.intro rfl (And.intro ?_ ?_))
      next => simpa only [i_post, alloc.vec.Vec.len_val] using id_post
      next => simpa only [i_post, alloc.vec.Vec.len_val] using pending1_post
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro output outputSpec
  apply Exists.elim outputSpec.2
  intro id idSpec
  have pair : output = (some id, output.2) := by
    calc
      output = (output.1, output.2) := rfl
      _ = (some id, output.2) := by rw [idSpec.1]
  exact Exists.intro id (Exists.intro output.2 (And.intro
    (outputSpec.1.trans (congrArg Result.ok pair)) idSpec.2))

theorem pop_identity_lifo (pending : alloc.vec.Vec NativeFactId)
    (remainingIds : List NativeFactId) (id : NativeFactId)
    (contents : pending.val = remainingIds ++ [id]) :
    exists next, worklist.pop_identity pending = .ok (some id, next) /\ next.val = remainingIds := by
  have nonempty : 0 < pending.val.length := by simp [contents]
  apply Exists.elim (pop_identity_nonempty pending nonempty)
  intro picked pickedSpec
  apply Exists.elim pickedSpec
  intro next nextSpec
  have same : picked = id := by simpa [contents] using nextSpec.2.1
  have remaining : next.val = remainingIds := by simpa [List.resize, contents, same] using nextSpec.2.2
  exact Exists.intro next (And.intro (by simpa only [same] using nextSpec.1) remaining)

/-- Rust's Vec is projected in reverse order to the graph proof's pending list. -/
def pendingModel (pending : alloc.vec.Vec NativeFactId) : List Nat :=
  pending.val.reverse.map factModelId

theorem pop_identity_projects_pending (pending : alloc.vec.Vec NativeFactId)
    (id : NativeFactId) (remainingIds : List NativeFactId)
    (contents : pending.val.reverse = id :: remainingIds) :
    exists next, worklist.pop_identity pending = .ok (some id, next) /\
      pendingModel next = remainingIds.map factModelId := by
  have original : pending.val = remainingIds.reverse ++ [id] := by
    simpa only [List.reverse_reverse, List.reverse_cons] using congrArg List.reverse contents
  apply Exists.elim (pop_identity_lifo pending _ id original)
  intro next nextSpec
  exact Exists.intro next (And.intro nextSpec.1
    (by simp only [pendingModel, nextSpec.2, List.reverse_reverse]))

/-- The real source append preserves order and duplicates, subject to the
explicit machine-size bound in Aeneas's Vec model. Allocation failure is outside
that model, as for its other Vec operations. -/
theorem append_identities_exact (pending : alloc.vec.Vec NativeFactId)
    (dependencies : Slice NativeFactId)
    (capacity : pending.val.length + dependencies.val.length <= Usize.max) :
    exists next, worklist.append_identities pending dependencies = .ok next /\
      next.val = pending.val ++ dependencies.val := by
  have cloneSpec := Slice.clone_spec
    (s := dependencies) (clone := mrr_identity.api.FactId.Insts.CoreCloneClone.clone)
    (fun id _ => identity_clone_exact id)
  have cloneExact : Slice.clone mrr_identity.api.FactId.Insts.CoreCloneClone.clone dependencies =
      .ok dependencies := by
    apply Exists.elim ((WP.spec_equiv_exists _ _).mp cloneSpec)
    intro cloned clonedSpec
    simpa only [clonedSpec.2] using clonedSpec.1
  have total : WP.spec (worklist.append_identities pending dependencies)
      (fun next => next.val = pending.val ++ dependencies.val) := by
    unfold worklist.append_identities alloc.vec.Vec.extend_from_slice
    split
    next bound =>
      have matched : (Slice.clone mrr_identity.api.FactId.Insts.CoreCloneClone.clone dependencies).match =
          .ok dependencies := by rw [cloneExact]; exact Result.match.ok
      split
      next copied copiedMatch =>
        have same : dependencies = copied := MatchResult.ok.inj (matched.symm.trans copiedMatch)
        subst copied
        exact (WP.spec_ok _).mpr (alloc.vec.Vec.from_val _ _)
      next effect continuation impossible =>
        rw [matched] at impossible
        cases impossible
      next impossible =>
        rw [matched] at impossible
        cases impossible
    next insufficient => exact False.elim (insufficient capacity)
  exact (WP.spec_equiv_exists _ _).mp total

theorem append_identities_projects_pending (pending : alloc.vec.Vec NativeFactId)
    (dependencies : Slice NativeFactId)
    (capacity : pending.val.length + dependencies.val.length <= Usize.max) :
    exists next, worklist.append_identities pending dependencies = .ok next /\
      pendingModel next = dependencies.val.reverse.map factModelId ++ pendingModel pending := by
  apply Exists.elim (append_identities_exact pending dependencies capacity)
  intro next nextSpec
  exact Exists.intro next (And.intro nextSpec.1
    (by simp only [pendingModel, nextSpec.2, List.reverse_append, List.map_append]))


/-- The production seed keeps roots, mandatory dependencies and temporal
receipts, including duplicates, in their exact declared order. -/
theorem initial_pending_exact (roots required temporal : Slice NativeFactId)
    (capacity : roots.val.length + required.val.length + temporal.val.length <= Usize.max) :
    exists pending, worklist.initial_pending roots required temporal = .ok pending /\
      pending.val = roots.val ++ required.val ++ temporal.val := by
  have firstCapacity : (alloc.vec.Vec.new NativeFactId).val.length + roots.val.length <= Usize.max := by
    simp only [alloc.vec.Vec.new, alloc.vec.Vec.from_val, List.length_nil, Nat.zero_add]
    omega
  apply Exists.elim (append_identities_exact (alloc.vec.Vec.new NativeFactId) roots firstCapacity)
  intro first firstSpec
  have firstContents : first.val = roots.val := by simpa only [alloc.vec.Vec.new, alloc.vec.Vec.from_val, List.nil_append] using firstSpec.2
  have secondCapacity : first.val.length + required.val.length <= Usize.max := by
    rw [firstContents]
    omega
  apply Exists.elim (append_identities_exact first required secondCapacity)
  intro second secondSpec
  have secondContents : second.val = roots.val ++ required.val := by simpa only [firstContents] using secondSpec.2
  have thirdCapacity : second.val.length + temporal.val.length <= Usize.max := by
    simpa only [secondContents, List.length_append] using capacity
  apply Exists.elim (append_identities_exact second temporal thirdCapacity)
  intro pending pendingSpec
  refine Exists.intro pending (And.intro ?_ ?_)
  next => simp only [worklist.initial_pending, firstSpec.1, secondSpec.1, pendingSpec.1, bind_ok]
  next => simpa only [secondContents] using pendingSpec.2

theorem initial_pending_projects_roots (roots required temporal : Slice NativeFactId)
    (capacity : roots.val.length + required.val.length + temporal.val.length <= Usize.max) :
    exists pending, worklist.initial_pending roots required temporal = .ok pending /\
      pendingModel pending = ((roots.val ++ required.val ++ temporal.val).map factModelId).reverse := by
  apply Exists.elim (initial_pending_exact roots required temporal capacity)
  intro pending pendingSpec
  exact Exists.intro pending (And.intro pendingSpec.1
    (by simp only [pendingModel, pendingSpec.2, List.map_reverse]))


theorem initial_pending_graph_invariant (roots required temporal : Slice NativeFactId)
    (deps : Nat -> List Nat)
    (capacity : roots.val.length + required.val.length + temporal.val.length <= Usize.max) :
    exists pending, worklist.initial_pending roots required temporal = .ok pending /\
      MRR.AgenticAIContext.WorklistInvariant
        ((roots.val ++ required.val ++ temporal.val).map factModelId) deps
        { visited := [], pending := pendingModel pending } := by
  apply Exists.elim (initial_pending_projects_roots roots required temporal capacity)
  intro pending pendingSpec
  refine Exists.intro pending (And.intro pendingSpec.1 ?_)
  rw [pendingSpec.2]
  exact MRR.AgenticAIContext.worklist_initial_invariant _ deps

end MRR.ContextRustProofs
