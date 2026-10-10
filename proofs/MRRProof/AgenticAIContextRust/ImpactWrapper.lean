import ReverseIndex

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

theorem native_changed_clone_exact (changed : List NativeFactId) :
    alloc.collections.btree.set.BTreeSet.Insts.CoreCloneClone.clone
      mrr_identity.api.FactId.Insts.CoreCloneClone
      alloc.alloc.Global.Insts.CoreAllocAllocatorClone changed = .ok changed := by
  induction changed with
  | nil => rfl
  | cons id rest induction =>
    change cloneValues mrr_identity.api.FactId.Insts.CoreCloneClone rest = .ok rest at induction
    change cloneValues mrr_identity.api.FactId.Insts.CoreCloneClone (id :: rest) = .ok (id :: rest)
    simp only [cloneValues, identity_clone_exact, bind_ok, induction]

theorem native_changed_len_exact (changed : List NativeFactId)
    (capacity : changed.length <= Usize.max) :
    exists size, alloc.collections.btree.set.BTreeSet.len
      alloc.alloc.Global.Insts.CoreAllocAllocatorClone changed = .ok size /\ size.val = changed.length := by
  have within : UScalar.check_bounds .Usize changed.length := by
    rw [UScalar.check_bounds_eq_inBounds]
    unfold UScalar.inBounds
    scalar_tac
  refine Exists.intro (UScalar.ofNatCore changed.length
    (UScalar.check_bounds_imp_inBounds within)) (And.intro ?_ ?_)
  next => simp only [alloc.collections.btree.set.BTreeSet.len,
    UScalar.tryMk, UScalar.tryMkOpt, dif_pos within, Result.ofOption]
  next => exact UScalar.ofNatCore_val_eq _

/-- The actual set iterator seeds pending in its precise value-model order. -/
theorem native_impact_pending_loop_exact (remaining : List NativeFactId)
    (pending : alloc.vec.Vec NativeFactId)
    (capacity : pending.val.length + remaining.length <= Usize.max) :
    exists seeded, state.declared_dependency_impact_loop remaining pending = .ok seeded /\
      seeded.val = pending.val ++ remaining := by
  let expected := pending.val ++ remaining
  let invariant := fun input : Prod (List NativeFactId) (alloc.vec.Vec NativeFactId) =>
    input.2.val ++ input.1 = expected
  have expectedCapacity : expected.length <= Usize.max := by
    simpa only [expected, List.length_append] using capacity
  have total : WP.spec (loop
      (fun (iter, output) => state.declared_dependency_impact_loop.body iter output)
      (remaining, pending)) (fun final : alloc.vec.Vec NativeFactId => final.val = expected) := by
    apply loop.spec_decr_nat (fun input => input.1.length) invariant (fun final : alloc.vec.Vec NativeFactId => final.val = expected)
    next =>
      intro input valid
      cases input
      rename_i current output
      cases current with
      | nil =>
        simp only [state.declared_dependency_impact_loop.body,
          alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next,
          bind_ok, uncurry]
        exact (WP.spec_ok _).mpr (by simpa only [invariant, List.append_nil] using valid)
      | cons id rest =>
        have lengthBound := congrArg List.length valid
        have pushCapacity : output.val.length < Usize.max := by
          simp only [List.length_append, List.length_cons] at lengthBound
          omega
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp (alloc.vec.Vec.push_spec output id pushCapacity))
        intro next nextSpec
        simp only [state.declared_dependency_impact_loop.body,
          alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next,
          bind_ok, uncurry, nextSpec.1]
        apply (WP.spec_ok _).mpr
        refine And.intro ?_ (by simp only [List.length_cons]; omega)
        change next.val ++ rest = expected
        rw [nextSpec.2]
        simpa only [List.append_assoc, List.singleton_append] using valid
    next => rfl
  exact (WP.spec_equiv_exists _ _).mp total

/-- Actual construction, seed iteration, clone, run and return projection.
No pending contents, graph correspondence or adapter-law premise is supplied. -/
theorem native_declared_impact_wrapper_exact (old new : NativeElements)
    (changed : List NativeFactId) (unique : changed.Nodup)
    (capacity : (changed ++ (old ++ new).map Prod.fst).length +
      (changed ++ (old ++ new).map Prod.fst).length <= Usize.max) :
    exists invalidated, state.declared_dependency_impact old new changed = .ok invalidated /\
      forall id : NativeFactId, id inList invalidated <->
        Required (changed.map factModelId)
          (modelReverseGraph (indexDeclared new (indexDeclared old []))) (factModelId id) := by
  have changedCapacity : changed.length <= Usize.max := by
    simp only [List.length_append, List.length_map] at capacity
    omega
  apply Exists.elim (native_changed_len_exact changed changedCapacity)
  intro size sizeSpec
  have seedCapacity : (alloc.vec.Vec.with_capacity NativeFactId size).val.length + changed.length <= Usize.max := by
    simpa only [alloc.vec.Vec.with_capacity, alloc.vec.Vec.new, alloc.vec.Vec.from_val,
      List.length_nil, Nat.zero_add] using changedCapacity
  apply Exists.elim (native_impact_pending_loop_exact changed
    (alloc.vec.Vec.with_capacity NativeFactId size) seedCapacity)
  intro seeded seedSpec
  have contents : seeded.val = changed := by
    simpa only [alloc.vec.Vec.with_capacity, alloc.vec.Vec.new, alloc.vec.Vec.from_val,
      List.nil_append] using seedSpec.2
  apply Exists.elim (native_declared_impact_run_exact old new changed seeded contents unique capacity)
  intro entries entriesSpec
  apply Exists.elim entriesSpec
  intro final finalSpec
  have indexEqual : entries = indexDeclared new (indexDeclared old []) :=
    Result.ok_injective (finalSpec.1.symm.trans (native_reverse_index_exact old new))
  refine Exists.intro final.invalidated (And.intro ?_ ?_)
  next =>
    simp only [state.declared_dependency_impact, finalSpec.1, bind_ok,
      sizeSpec.1, SharedABTreeSet.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter,
      seedSpec.1, native_changed_clone_exact, finalSpec.2.1]
  next => simpa only [indexEqual] using finalSpec.2.2.2

end MRR.ContextRustProofs
