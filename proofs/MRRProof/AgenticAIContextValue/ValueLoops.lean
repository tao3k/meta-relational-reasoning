import ValueEquality

open Aeneas Aeneas.Std
open MRR.ContextValue

namespace MRR.ContextValueProofs

/-- Totality of the actual index loop, parameterized by its indexed child laws.
The final recursive equality proof must discharge those laws by induction. -/
theorem native_value_list_loop_exact (left right : Slice Value)
    (sizes : left.val.length = right.val.length)
    (children : forall (i : Nat) (hl : i < left.val.length) (hr : i < right.val.length),
      valueEq left.val[i] right.val[i] = .ok (decide (left.val[i] = right.val[i])))
    (index : Usize) (equal : Bool) :
    api.equal_value_lists_loop left right index equal =
      .ok (equal && decide (left.val.drop index.val = right.val.drop index.val)) := by
  generalize distance : left.val.length - index.val = remaining
  induction remaining using Nat.strong_induction_on generalizing index equal with
  | h remaining induction =>
    cases equal with
    | false => simp [native_value_list_loop_short_circuit]
    | true =>
      by_cases active : index < Slice.len left
      case neg =>
        have beyondLeft : left.val.length <= index.val := by scalar_tac
        have beyondRight : right.val.length <= index.val := by omega
        have nilLeft := List.drop_eq_nil_iff.mpr beyondLeft
        have nilRight := List.drop_eq_nil_iff.mpr beyondRight
        simpa [nilLeft, nilRight] using
          native_value_list_loop_exhausted left right index true active
      case pos =>
        have hl : index.val < left.val.length := by scalar_tac
        have hr : index.val < right.val.length := by omega
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp (Slice.index_usize_spec left index hl))
        intro x leftSpec
        have readLeft := leftSpec.1
        have xExact := leftSpec.2
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp (Slice.index_usize_spec right index hr))
        intro y rightSpec
        have readRight := rightSpec.1
        have yExact := rightSpec.2
        have additionBound : index.val + (1#usize).val <= Usize.max := by
          have bound := Slice.length_ineq left
          scalar_tac
        have addition : WP.spec (index + 1#usize)
            (fun next : Usize => next.val = index.val + 1) := by
          simpa using (Usize.add_spec (x := index) (y := 1#usize) additionBound)
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp addition)
        intro next nextSpec
        have addCall := nextSpec.1
        have nextExact := nextSpec.2
        subst x
        subst y
        have compare := children index.val hl hr
        unfold valueEq at compare
        have smaller : left.val.length - next.val < remaining := by omega
        have recurse := induction (left.val.length - next.val) smaller next true rfl
        rw [api.equal_value_lists_loop.eq_def]
        simp only [if_pos active, if_true, readLeft, readRight, compare, bind_ok, addCall]
        by_cases same : left.val[index.val] = right.val[index.val]
        case pos =>
          simp only [decide_eq_true same]
          rw [List.drop_eq_getElem_cons hl, List.drop_eq_getElem_cons hr]
          simp only [List.cons.injEq, same, true_and, Bool.true_and]
          simpa only [nextExact, Bool.true_and] using recurse
        case neg =>
          simp only [decide_eq_false same,
            native_value_list_loop_short_circuit]
          rw [List.drop_eq_getElem_cons hl, List.drop_eq_getElem_cons hr]
          simp only [List.cons.injEq, same, false_and]
          rfl

/-- Totality of the actual index loop, parameterized by its indexed child laws.
The final recursive equality proof must discharge those laws by induction. -/
theorem native_value_record_loop_exact (left right : Slice (Prod String Value))
    (sizes : left.val.length = right.val.length)
    (children : forall (i : Nat) (hl : i < left.val.length) (hr : i < right.val.length),
      api.equal_record_fields left.val[i].1 right.val[i].1 left.val[i].2 right.val[i].2 = .ok (decide (left.val[i] = right.val[i])))
    (index : Usize) (equal : Bool) :
    api.equal_value_records_loop left right index equal =
      .ok (equal && decide (left.val.drop index.val = right.val.drop index.val)) := by
  generalize distance : left.val.length - index.val = remaining
  induction remaining using Nat.strong_induction_on generalizing index equal with
  | h remaining induction =>
    cases equal with
    | false => simp [native_value_record_loop_short_circuit]
    | true =>
      by_cases active : index < Slice.len left
      case neg =>
        have beyondLeft : left.val.length <= index.val := by scalar_tac
        have beyondRight : right.val.length <= index.val := by omega
        have nilLeft := List.drop_eq_nil_iff.mpr beyondLeft
        have nilRight := List.drop_eq_nil_iff.mpr beyondRight
        simpa [nilLeft, nilRight] using
          native_value_record_loop_exhausted left right index true active
      case pos =>
        have hl : index.val < left.val.length := by scalar_tac
        have hr : index.val < right.val.length := by omega
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp (Slice.index_usize_spec left index hl))
        intro x leftSpec
        have readLeft := leftSpec.1
        have xExact := leftSpec.2
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp (Slice.index_usize_spec right index hr))
        intro y rightSpec
        have readRight := rightSpec.1
        have yExact := rightSpec.2
        have additionBound : index.val + (1#usize).val <= Usize.max := by
          have bound := Slice.length_ineq left
          scalar_tac
        have addition : WP.spec (index + 1#usize)
            (fun next : Usize => next.val = index.val + 1) := by
          simpa using (Usize.add_spec (x := index) (y := 1#usize) additionBound)
        apply Exists.elim ((WP.spec_equiv_exists _ _).mp addition)
        intro next nextSpec
        have addCall := nextSpec.1
        have nextExact := nextSpec.2
        subst x
        subst y
        have compare := children index.val hl hr
        have smaller : left.val.length - next.val < remaining := by omega
        have recurse := induction (left.val.length - next.val) smaller next true rfl
        rw [api.equal_value_records_loop.eq_def]
        simp only [if_pos active, if_true, readLeft, readRight, bind_ok, addCall]
        cases leftEntry : left.val[index.val]
        rename_i key child
        cases rightEntry : right.val[index.val]
        rename_i otherKey otherChild
        have compareCall : api.equal_record_fields key otherKey child otherChild =
            .ok (decide (left.val[index.val] = right.val[index.val])) := by
          simpa only [leftEntry, rightEntry] using compare
        simp only [uncurry, compareCall, bind_ok]
        by_cases same : left.val[index.val] = right.val[index.val]
        case pos =>
          simp only [decide_eq_true same]
          rw [List.drop_eq_getElem_cons hl, List.drop_eq_getElem_cons hr]
          simp only [List.cons.injEq, same, true_and, Bool.true_and]
          simpa only [nextExact, Bool.true_and] using recurse
        case neg =>
          simp only [decide_eq_false same,
            native_value_record_loop_short_circuit]
          rw [List.drop_eq_getElem_cons hl, List.drop_eq_getElem_cons hr]
          simp only [List.cons.injEq, same, false_and]
          rfl

theorem native_value_lists_exact (left right : Slice (Value))
    (children : forall (i : Nat) (hl : i < left.val.length) (hr : i < right.val.length),
      valueEq left.val[i] right.val[i] = .ok (decide (left.val[i] = right.val[i]))) :
    api.equal_value_lists left right = .ok (decide (left.val = right.val)) := by
  by_cases sizes : left.val.length = right.val.length
  case pos =>
    have lengths : Slice.len left = Slice.len right := by scalar_tac
    rw [api.equal_value_lists.eq_def]
    simp only [lengths, bne_self_eq_false, Bool.false_eq_true, if_false]
    simpa using native_value_list_loop_exact left right sizes children 0#usize true
  case neg =>
    have different : Not (Slice.len left = Slice.len right) := by
      intro equal
      have values := congrArg UScalar.val equal
      exact sizes (by simpa only [Slice.len_val] using values)
    have unequal : Not (left.val = right.val) := fun equal => sizes (congrArg List.length equal)
    simpa only [decide_eq_false unequal] using
      native_value_list_length_mismatch left right different

theorem native_value_records_exact (left right : Slice (Prod String Value))
    (children : forall (i : Nat) (hl : i < left.val.length) (hr : i < right.val.length),
      api.equal_record_fields left.val[i].1 right.val[i].1 left.val[i].2 right.val[i].2 = .ok (decide (left.val[i] = right.val[i]))) :
    api.equal_value_records left right = .ok (decide (left.val = right.val)) := by
  by_cases sizes : left.val.length = right.val.length
  case pos =>
    have lengths : Slice.len left = Slice.len right := by scalar_tac
    rw [api.equal_value_records.eq_def]
    simp only [lengths, bne_self_eq_false, Bool.false_eq_true, if_false]
    simpa using native_value_record_loop_exact left right sizes children 0#usize true
  case neg =>
    have different : Not (Slice.len left = Slice.len right) := by
      intro equal
      have values := congrArg UScalar.val equal
      exact sizes (by simpa only [Slice.len_val] using values)
    have unequal : Not (left.val = right.val) := fun equal => sizes (congrArg List.length equal)
    simpa only [decide_eq_false unequal] using
      native_value_record_length_mismatch left right different

end MRR.ContextValueProofs
