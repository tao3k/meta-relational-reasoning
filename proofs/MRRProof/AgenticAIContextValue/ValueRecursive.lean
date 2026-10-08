import ValueLoops

open Aeneas Aeneas.Std
open MRR.ContextValue

namespace MRR.ContextValueProofs

private def Exact (left : Value) : Prop :=
  forall right, valueEq left right = .ok (decide (left = right))

theorem native_record_fields_exact (key otherKey : String) (left right : Value)
    (child : forall other, valueEq left other = .ok (decide (left = other))) :
    mrr_relation.api.equal_record_fields key otherKey left right =
      .ok (decide ((key, left) = (otherKey, right))) := by
  by_cases same : key = otherKey
  case pos =>
    subst otherKey
    simpa only [Prod.mk.injEq, true_and] using
      (native_record_key_match key left right).trans (child right)
  case neg =>
    simpa only [Prod.mk.injEq, same, false_and, decide_false] using
      native_record_key_mismatch key otherKey left right same

/-- Equality of arbitrary recursive payloads, for the extracted Rust function.
All indexed child premises are discharged by the generated nested recursor. -/
theorem native_value_equality_total (left : Value) :
    forall right, valueEq left right = .ok (decide (left = right)) := by
  apply mrr_relation.api.Value.rec
    (motive_1 := Exact)
    (motive_2 := fun vec => forall child, Membership.mem vec.val child -> Exact child)
    (motive_3 := fun vec => forall entry, Membership.mem vec.val entry -> Exact entry.2)
    (motive_4 := fun slice => forall child, Membership.mem slice.val child -> Exact child)
    (motive_5 := fun slice => forall entry, Membership.mem slice.val entry -> Exact entry.2)
    (motive_6 := fun _ list => forall child, Membership.mem list.toList child -> Exact child)
    (motive_7 := fun _ list => forall entry, Membership.mem list.toList entry -> Exact entry.2)
    (motive_8 := fun entry => Exact entry.2)
  all_goals try (intro payload right; exact native_atomic_value_equality_exact _ right (by trivial))
  next =>
    intro right
    exact native_atomic_value_equality_exact _ right (by trivial)
  next =>
    intro vec children right
    cases right <;> try (simpa using native_value_variant_mismatch (.List vec) _ (by simp [valueTag]))
    rename_i other
    rw [native_list_value_delegates]
    have exactLoop := native_value_lists_exact (alloc.vec.Vec.deref vec) (alloc.vec.Vec.deref other)
      (by
        intro i hl hr
        have hlv : i < vec.val.length := by simpa only [alloc.vec.Vec.deref, Slice.from_val] using hl
        have hrv : i < other.val.length := by simpa only [alloc.vec.Vec.deref, Slice.from_val] using hr
        simpa only [alloc.vec.Vec.deref, Slice.from_val] using
          children vec.val[i] (List.getElem_mem hlv) other.val[i])
    simpa only [alloc.vec.Vec.deref, Slice.from_val, mrr_relation.api.Value.List.injEq, alloc.vec.Vec.eq_iff] using exactLoop
  next =>
    intro vec children right
    cases right <;> try (simpa using native_value_variant_mismatch (.Record vec) _ (by simp [valueTag]))
    rename_i other
    rw [native_record_value_delegates]
    have exactLoop := native_value_records_exact (alloc.vec.Vec.deref vec) (alloc.vec.Vec.deref other)
      (by
        intro i hl hr
        have hlv : i < vec.val.length := by simpa only [alloc.vec.Vec.deref, Slice.from_val] using hl
        have hrv : i < other.val.length := by simpa only [alloc.vec.Vec.deref, Slice.from_val] using hr
        simpa only [alloc.vec.Vec.deref, Slice.from_val, Prod.mk.eta] using
          native_record_fields_exact vec.val[i].1 other.val[i].1 vec.val[i].2 other.val[i].2
            (children vec.val[i] (List.getElem_mem hlv)))
    simpa only [alloc.vec.Vec.deref, Slice.from_val, mrr_relation.api.Value.Record.injEq, alloc.vec.Vec.eq_iff] using exactLoop
  next =>
    intro slice children
    exact children
  next =>
    intro slice children
    exact children
  next =>
    intro leng list bound children
    exact children
  next =>
    intro leng list bound children
    exact children
  next =>
    intro child member
    simp [Aeneas.Data.ListN.ListN.toList] at member
  next =>
    intro n head rest headExact restExact child member
    simp only [Aeneas.Data.ListN.ListN.toList, List.mem_cons] at member
    cases member with
    | inl equal => simpa only [equal] using headExact
    | inr inside => exact restExact child inside
  next =>
    intro entry member
    simp [Aeneas.Data.ListN.ListN.toList] at member
  next =>
    intro n head rest headExact restExact entry member
    simp only [Aeneas.Data.ListN.ListN.toList, List.mem_cons] at member
    cases member with
    | inl equal => simpa only [equal] using headExact
    | inr inside => exact restExact entry inside
  next =>
    intro key child childExact
    exact childExact

end MRR.ContextValueProofs
