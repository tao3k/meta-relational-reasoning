import ForwardWrapper
import ImpactWrapper

open Aeneas Aeneas.Std
open MRR.ContextRust

namespace MRR.ContextRustProofs

def insertNative (selected : List NativeFactId) (id : NativeFactId) : List NativeFactId :=
  if id inList selected then selected else id :: selected

def insertAll : List NativeFactId -> List NativeFactId -> List NativeFactId
  | [], selected => selected
  | id :: rest, selected => insertAll rest (insertNative selected id)

theorem insert_all_members (ids selected : List NativeFactId) (id : NativeFactId) :
    id inList insertAll ids selected <-> id inList ids \/ id inList selected := by
  induction ids generalizing selected with
  | nil => simp [insertAll]
  | cons head rest induction =>
    rw [insertAll, induction]
    by_cases known : head inList selected
    <;> simp [insertNative, known, List.mem_cons, or_assoc, or_left_comm]
    intro same
    subst id
    exact Or.inr known

theorem insert_all_unique (ids selected : List NativeFactId) (unique : selected.Nodup) :
    (insertAll ids selected).Nodup := by
  induction ids generalizing selected with
  | nil => exact unique
  | cons head rest induction =>
    apply induction
    by_cases known : head inList selected
    <;> simp [insertNative, known, unique]

theorem insert_all_length (ids selected : List NativeFactId) :
    (insertAll ids selected).length <= ids.length + selected.length := by
  induction ids generalizing selected with
  | nil => simp [insertAll]
  | cons head rest induction =>
    have bound := induction (insertNative selected head)
    by_cases known : head inList selected
    <;> simp [insertAll, insertNative, known] at *
    <;> omega

theorem native_borrowed_contains_exact (selected : List NativeFactId) (id : NativeFactId) :
    alloc.collections.btree.set.BTreeSet.contains
      alloc.alloc.Global.Insts.CoreAllocAllocatorClone
      (core.borrow.Borrow.Blanket NativeFactId) mrr_identity.api.FactId.Insts.CoreCmpOrd
      mrr_identity.api.FactId.Insts.CoreCmpOrd selected id = .ok (decide (id inList selected)) := by
  change borrowedContains core.borrow.Borrow.Blanket.borrow
    mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq id selected = _
  induction selected with
  | nil => rfl
  | cons key rest induction =>
    simp only [borrowedContains, core.borrow.Borrow.Blanket.borrow,
      native_fact_id_equality_exact, bind_ok, induction]
    by_cases same : key = id <;> simp [same, Ne.symm]

theorem native_revision_source_loop_exact (elements : NativeElements) (selected : List NativeFactId) :
    state.revision_source_ids_loop0 elements selected = .ok (insertAll (elements.map Prod.fst) selected) := by
  let expected := insertAll (elements.map Prod.fst) selected
  let invariant := fun input : Prod NativeElements (List NativeFactId) =>
    insertAll (input.1.map Prod.fst) input.2 = expected
  have total : WP.spec (loop
      (fun (iter, all) => state.revision_source_ids_loop0.body iter all)
      (elements, selected)) (fun final => final = expected) := by
    apply loop.spec_decr_nat (fun input => input.1.length) invariant (fun final => final = expected)
    next =>
      intro input valid
      cases input
      rename_i current all
      cases current with
      | nil =>
        simp only [state.revision_source_ids_loop0.body,
          alloc.collections.btree.map.Iter.Insts.CoreIterTraitsIteratorIteratorPairSharedAKSharedAV.next,
          bind_ok, uncurry]
        exact (WP.spec_ok _).mpr valid
      | cons pair rest =>
        cases pair with
        | mk id value =>
          simp only [state.revision_source_ids_loop0.body,
            alloc.collections.btree.map.Iter.Insts.CoreIterTraitsIteratorIteratorPairSharedAKSharedAV.next,
            bind_ok, uncurry, native_set_insert_exact]
          by_cases known : id inList all
          <;> simp [known]
          <;> simpa [invariant, insertAll, insertNative, known] using valid
    next => rfl
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro final spec
  exact spec.1.trans (congrArg Result.ok spec.2)

theorem native_revision_source_union (old new : NativeElements) :
    exists selected, state.revision_source_ids old new = .ok selected /\ selected.Nodup /\
      forall id, id inList selected <->
        id inList old.map Prod.fst \/ id inList new.map Prod.fst := by
  let selected := insertAll (new.map Prod.fst) (insertAll (old.map Prod.fst) [])
  have second : state.revision_source_ids_loop1 new (insertAll (old.map Prod.fst) []) = .ok selected :=
    native_revision_source_loop_exact _ _
  refine Exists.intro selected (And.intro ?_ (And.intro
    (insert_all_unique _ _ (insert_all_unique _ _ (by simp))) ?_))
  next => simp only [state.revision_source_ids,
    alloc.collections.btree.set.BTreeSetTGlobal.new,
    SharedABTreeMap.Insts.CoreIterTraitsCollectIntoIteratorPairSharedAKSharedAVIter.into_iter,
    bind_ok, native_revision_source_loop_exact, second]
  next => intro id; simp [selected, insert_all_members, or_comm]

theorem native_revision_selected_loop_exact (iter : core.slice.iter.Iter NativeFactId)
    (selected : List NativeFactId) :
    state.revision_reusable_ids_loop0 iter selected =
      .ok (insertAll (iter.slice.val.drop iter.i) selected) := by
  let expected := insertAll (iter.slice.val.drop iter.i) selected
  let invariant := fun input : Prod (core.slice.iter.Iter NativeFactId) (List NativeFactId) =>
    insertAll (input.1.slice.val.drop input.1.i) input.2 = expected
  have total : WP.spec (loop
      (fun (current, all) => state.revision_reusable_ids_loop0.body current all)
      (iter, selected)) (fun final => final = expected) := by
    apply loop.spec_decr_nat (fun input => input.1.slice.val.length - input.1.i)
      invariant (fun final => final = expected)
    next =>
      intro input valid
      cases input
      rename_i current all
      simp only [state.revision_reusable_ids_loop0.body, core.slice.iter.IteratorSliceIter.next]
      by_cases active : current.i < current.slice.len
      case pos =>
        simp only [dif_pos active, bind_ok, uncurry, native_set_insert_exact]
        have indexBound : current.i < current.slice.val.length := by
          simpa only [Slice.len_val] using active
        change insertAll (current.slice.val.drop current.i) all = expected at valid
        rw [List.drop_eq_getElem_cons indexBound] at valid
        change WP.spec
          (Result.ok (ControlFlow.cont
            (({ slice := current.slice, i := current.i + 1 } : core.slice.iter.Iter NativeFactId),
              (if current.slice.val[current.i] inList all then (false, all)
               else (true, current.slice.val[current.i] :: all)).2)) :
            Result (ControlFlow (Prod (core.slice.iter.Iter NativeFactId) (List NativeFactId))
              (List NativeFactId))) _
        by_cases known : current.slice.val[current.i] inList all
        <;> simp [known]
        <;> refine And.intro ?_ (by omega)
        <;> simpa [invariant, insertAll, insertNative, known] using valid
      case neg =>
        simp only [dif_neg active, bind_ok, uncurry]
        apply (WP.spec_ok _).mpr
        have beyond : current.slice.val.length <= current.i := by
          have inactive : Not (current.i < current.slice.val.length) := by
            simpa only [Slice.len_val] using active
          omega
        simpa only [invariant, List.drop_eq_nil_iff.mpr beyond, insertAll] using valid
    next => rfl
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro final spec
  exact spec.1.trans (congrArg Result.ok spec.2)

def reuseFilter (new invalidated : List NativeFactId) (id : NativeFactId) : Bool :=
  decide (id inList new /\ Not (id inList invalidated))

theorem native_revision_reuse_loop_exact (remaining new invalidated : List NativeFactId)
    (output : alloc.vec.Vec NativeFactId)
    (capacity : output.val.length + remaining.length <= Usize.max) :
    exists final, state.revision_reusable_ids_loop2 remaining invalidated new output = .ok final /\
      final.val = output.val ++ remaining.filter (reuseFilter new invalidated) := by
  let expected := output.val ++ remaining.filter (reuseFilter new invalidated)
  let invariant := fun input : Prod (List NativeFactId) (alloc.vec.Vec NativeFactId) =>
    input.2.val ++ input.1.filter (reuseFilter new invalidated) = expected /\
      input.2.val.length + input.1.length <= Usize.max
  have total : WP.spec (loop
      (fun (iter, reusable) => state.revision_reusable_ids_loop2.body invalidated new iter reusable)
      (remaining, output)) (fun final : alloc.vec.Vec NativeFactId => final.val = expected) := by
    apply loop.spec_decr_nat (fun input => input.1.length) invariant (fun final : alloc.vec.Vec NativeFactId => final.val = expected)
    next =>
      intro input valid
      cases input
      rename_i current reusable
      cases current with
      | nil =>
        simp only [state.revision_reusable_ids_loop2.body,
          alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next,
          bind_ok, uncurry]
        exact (WP.spec_ok _).mpr (by simpa only [List.filter_nil, List.append_nil] using valid.1)
      | cons id rest =>
        have budget : reusable.val.length + (id :: rest).length <= Usize.max := valid.2
        have equation : reusable.val ++ (id :: rest).filter (reuseFilter new invalidated) = expected := valid.1
        simp only [state.revision_reusable_ids_loop2.body,
          alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next,
          bind_ok, uncurry, native_borrowed_contains_exact]
        by_cases selected : id inList new
        case neg =>
          simp only [decide_eq_false selected, Bool.false_eq_true, if_false]
          apply (WP.spec_ok _).mpr
          refine And.intro (And.intro ?_ ?_) (by change rest.length < (id :: rest).length; simp only [List.length_cons]; omega)
          next => simpa [reuseFilter, selected] using equation
          next =>
            change reusable.val.length + rest.length <= Usize.max
            simp only [List.length_cons] at budget
            omega
        case pos =>
          simp only [decide_eq_true selected, if_true]
          by_cases rejected : id inList invalidated
          case pos =>
            simp only [decide_eq_true rejected, if_true]
            apply (WP.spec_ok _).mpr
            refine And.intro (And.intro ?_ ?_) (by change rest.length < (id :: rest).length; simp only [List.length_cons]; omega)
            next => simpa [reuseFilter, selected, rejected] using equation
            next =>
              change reusable.val.length + rest.length <= Usize.max
              simp only [List.length_cons] at budget
              omega
          case neg =>
            simp only [decide_eq_false rejected, Bool.false_eq_true, if_false]
            have pushCapacity : reusable.val.length < Usize.max := by
              simp only [List.length_cons] at budget; omega
            apply Exists.elim ((WP.spec_equiv_exists _ _).mp
              (alloc.vec.Vec.push_spec reusable id pushCapacity))
            intro pushed pushSpec
            have contents : pushed.val = reusable.val ++ [id] := pushSpec.2
            rw [pushSpec.1]
            simp only [bind_ok]
            apply (WP.spec_ok _).mpr
            refine And.intro (And.intro ?_ ?_) (by change rest.length < (id :: rest).length; simp only [List.length_cons]; omega)
            next => simpa [reuseFilter, selected, rejected, contents, List.append_assoc] using equation
            next =>
              change pushed.val.length + rest.length <= Usize.max
              rw [contents]
              simp only [List.length_append, List.length_cons, List.length_nil] at *
              omega
    next => exact And.intro rfl capacity
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp total)
  intro final spec
  exact Exists.intro final (And.intro spec.1 spec.2)

theorem native_revision_reusable_exact (old new : Slice NativeFactId) (invalidated : List NativeFactId)
    (capacity : old.val.length <= Usize.max) :
    exists output, state.revision_reusable_ids old new invalidated = .ok output /\
      output.val.Nodup /\ forall id, id inList output.val <->
        id inList old.val /\ id inList new.val /\ Not (id inList invalidated) := by
  let oldSet := insertAll old.val []
  let newSet := insertAll new.val []
  have bound : (alloc.vec.Vec.new NativeFactId).val.length + oldSet.length <= Usize.max := by
    have small := insert_all_length old.val []
    simp only [List.length_nil, Nat.add_zero] at small
    simpa only [alloc.vec.Vec.new, alloc.vec.Vec.from_val, List.length_nil, Nat.zero_add] using Nat.le_trans small capacity
  apply Exists.elim (native_revision_reuse_loop_exact oldSet newSet invalidated
    (alloc.vec.Vec.new NativeFactId) bound)
  intro output outputSpec
  have run := outputSpec.1
  have contents := outputSpec.2
  have second : state.revision_reusable_ids_loop1 { slice := new, i := 0 } [] = .ok newSet :=
    native_revision_selected_loop_exact _ _
  refine Exists.intro output (And.intro ?_ (And.intro ?_ ?_))
  next =>
    simp only [state.revision_reusable_ids,
      alloc.collections.btree.set.BTreeSetTGlobal.new,
      SharedSlice.Insts.CoreIterTraitsCollectIntoIteratorSharedIter.into_iter,
      bind_ok, native_revision_selected_loop_exact, List.drop_zero,
      second, SharedABTreeSet.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter]
    exact run
  next =>
    rw [contents]
    simp only [alloc.vec.Vec.new, alloc.vec.Vec.from_val, List.nil_append]
    exact List.Nodup.filter _ (insert_all_unique _ _ (by simp))
  next =>
    intro id
    rw [contents]
    simp [alloc.vec.Vec.new, alloc.vec.Vec.from_val, reuseFilter, oldSet, newSet, insert_all_members]

end MRR.ContextRustProofs
