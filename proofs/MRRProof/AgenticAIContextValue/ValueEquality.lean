import ValueGenerated.Funs

open Aeneas Aeneas.Std
open MRR.ContextValue

namespace MRR.ContextValueProofs

theorem native_list_equality_exact {T : Type} [DecidableEq T]
    (compare : T -> T -> Result Bool)
    (law : forall left right, compare left right = .ok (decide (left = right)))
    (left right : List T) (sizes : left.length = right.length) :
    List.allM (fun (x, y) => compare x y) (left.zip right) = .ok (decide (left = right)) := by
  induction left generalizing right with
  | nil =>
    cases right with
    | nil => rfl
    | cons head rest => simp at sizes
  | cons head rest induction =>
    cases right with
    | nil => simp at sizes
    | cons other tail =>
      have tailSizes : rest.length = tail.length := by simpa using sizes
      have tailExact := induction tail tailSizes
      have headExact := law head other
      simp only [List.zip_cons_cons, List.allM_cons, headExact, bind_tc_ok]
      by_cases same : head = other
      case pos => simpa [same] using tailExact
      case neg => simp [same]; rfl

theorem native_vec_equality_exact {T : Type} [DecidableEq T]
    (compare : core.cmp.PartialEq T T)
    (law : forall left right, compare.eq left right = .ok (decide (left = right)))
    (left right : alloc.vec.Vec T) :
    alloc.vec.partial_eq.PartialEqVec.eq compare left right = .ok (decide (left = right)) := by
  unfold alloc.vec.partial_eq.PartialEqVec.eq
  by_cases sizes : left.val.length = right.val.length
  case pos =>
    simp only [alloc.vec.Vec.length, if_pos sizes, native_list_equality_exact _ law _ _ sizes,
      alloc.vec.Vec.eq_iff]
  case neg =>
    have different : Not (left.val = right.val) := fun equal => sizes (congrArg List.length equal)
    simp only [alloc.vec.Vec.length, if_neg sizes, alloc.vec.Vec.eq_iff, decide_eq_false different]

theorem native_entity_id_equality_exact (left right : mrr_identity.api.EntityId) :
    mrr_identity.api.EntityId.Insts.CoreCmpPartialEqEntityId.eq left right =
      .ok (decide (left = right)) := by
  have elementSpec : forall x y : U8,
      WP.spec (core.cmp.PartialEqU8.ne x y) (fun b => b = true <-> Not (x = y)) := by
    intro x y
    simp [core.cmp.impls.PartialEqU8.ne, liftFun2, WP.spec_ok]
  have sliceSpec := core.slice.cmp.PartialEqSlice.eq_homo_spec
    core.cmp.PartialEqU8 left.to_slice right.to_slice elementSpec
  have equalFunctions :
      mrr_identity.api.EntityId.Insts.CoreCmpPartialEqEntityId.eq left right =
        core.slice.cmp.PartialEqSlice.eq core.cmp.PartialEqU8 left.to_slice right.to_slice := by
    simp only [mrr_identity.api.EntityId.Insts.CoreCmpPartialEqEntityId.eq,
      core.array.equality.PartialEqArray.eq, core.slice.cmp.PartialEqSlice.eq,
      Aeneas.Std.Array.val_to_slice, Aeneas.Std.Array.length, Slice.length]
  apply Exists.elim ((WP.spec_equiv_exists _ _).mp sliceSpec)
  intro result resultSpec
  have output := resultSpec.1
  have correct := resultSpec.2
  have same : left.to_slice = right.to_slice <-> left = right := by
    rw [Slice.eq_iff, Aeneas.Std.Array.val_to_slice, Aeneas.Std.Array.val_to_slice,
      <- Aeneas.Std.Array.eq_iff]
  rw [equalFunctions, output]
  congr 1
  rw [same] at correct
  cases result <;> simp_all


abbrev Value := api.Value
abbrev valueEq := api.Value.Insts.CoreCmpPartialEqValue.eq

theorem native_null_value_equality_exact : valueEq .Null .Null = .ok true := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]

theorem native_boolean_value_equality_exact (left right : Bool) :
    valueEq (.Boolean left) (.Boolean right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_integer_value_equality_exact (left right : I64) :
    valueEq (.Integer left) (.Integer right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_decimal_value_equality_exact (left right : String) :
    valueEq (.Decimal left) (.Decimal right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_float_value_equality_exact (left right : String) :
    valueEq (.Float left) (.Float right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_string_value_equality_exact (left right : String) :
    valueEq (.String left) (.String right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_date_value_equality_exact (left right : String) :
    valueEq (.Date left) (.Date right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_time_value_equality_exact (left right : String) :
    valueEq (.Time left) (.Time right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_timestamp_value_equality_exact (left right : String) :
    valueEq (.Timestamp left) (.Timestamp right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_duration_value_equality_exact (left right : String) :
    valueEq (.Duration left) (.Duration right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  rfl

theorem native_entity_value_equality_exact (left right : mrr_identity.api.EntityId) :
    valueEq (.Entity left) (.Entity right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  exact native_entity_id_equality_exact left right

theorem native_bytes_value_equality_exact (left right : alloc.vec.Vec U8) :
    valueEq (.ByteString left) (.ByteString right) = .ok (decide (left = right)) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]
  apply native_vec_equality_exact
  intro x y
  rfl

/-- Constructor identity is part of Rust equality, including Decimal versus Float. -/
def valueTag : Value -> Nat
  | .Entity _ => 0
  | .Null => 1
  | .Boolean _ => 2
  | .Integer _ => 3
  | .Decimal _ => 4
  | .Float _ => 5
  | .String _ => 6
  | .ByteString _ => 7
  | .Date _ => 8
  | .Time _ => 9
  | .Timestamp _ => 10
  | .Duration _ => 11
  | .List _ => 12
  | .Record _ => 13

theorem native_value_variant_mismatch (left right : Value)
    (different : Not (valueTag left = valueTag right)) :
    valueEq left right = .ok false := by
  cases left <;> cases right <;> simp [valueTag] at different
  all_goals unfold valueEq
  all_goals rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]

theorem native_value_list_loop_short_circuit
    (left right : Slice (Value)) (index : Usize) :
    api.equal_value_lists_loop left right index false = .ok false := by
  rw [api.equal_value_lists_loop.eq_def]
  simp only [Bool.false_eq_true, if_false, ite_self]

theorem native_value_list_loop_exhausted
    (left right : Slice (Value)) (index : Usize) (equal : Bool)
    (exhausted : Not (index < Slice.len left)) :
    api.equal_value_lists_loop left right index equal = .ok equal := by
  rw [api.equal_value_lists_loop.eq_def]
  simp only [if_neg exhausted]

theorem native_value_list_length_mismatch
    (left right : Slice (Value)) (different : Not (Slice.len left = Slice.len right)) :
    api.equal_value_lists left right = .ok false := by
  rw [api.equal_value_lists.eq_def]
  simp [different]

theorem native_value_list_empty_exact
    (left right : Slice (Value)) (leftEmpty : left.val = []) (rightEmpty : right.val = []) :
    api.equal_value_lists left right = .ok true := by
  have leftZero : Slice.len left = 0#usize := by scalar_tac
  have rightZero : Slice.len right = 0#usize := by scalar_tac
  rw [api.equal_value_lists.eq_def]
  simp only [leftZero, rightZero, bne_self_eq_false, Bool.false_eq_true, if_false]
  apply native_value_list_loop_exhausted
  simp [leftZero]

theorem native_value_record_loop_short_circuit
    (left right : Slice (Prod String Value)) (index : Usize) :
    api.equal_value_records_loop left right index false = .ok false := by
  rw [api.equal_value_records_loop.eq_def]
  simp only [Bool.false_eq_true, if_false, ite_self]

theorem native_value_record_loop_exhausted
    (left right : Slice (Prod String Value)) (index : Usize) (equal : Bool)
    (exhausted : Not (index < Slice.len left)) :
    api.equal_value_records_loop left right index equal = .ok equal := by
  rw [api.equal_value_records_loop.eq_def]
  simp only [if_neg exhausted]

theorem native_value_record_length_mismatch
    (left right : Slice (Prod String Value)) (different : Not (Slice.len left = Slice.len right)) :
    api.equal_value_records left right = .ok false := by
  rw [api.equal_value_records.eq_def]
  simp [different]

theorem native_value_record_empty_exact
    (left right : Slice (Prod String Value)) (leftEmpty : left.val = []) (rightEmpty : right.val = []) :
    api.equal_value_records left right = .ok true := by
  have leftZero : Slice.len left = 0#usize := by scalar_tac
  have rightZero : Slice.len right = 0#usize := by scalar_tac
  rw [api.equal_value_records.eq_def]
  simp only [leftZero, rightZero, bne_self_eq_false, Bool.false_eq_true, if_false]
  apply native_value_record_loop_exhausted
  simp [leftZero]

theorem native_record_key_mismatch (leftKey rightKey : String) (left right : Value)
    (different : Not (leftKey = rightKey)) :
    api.equal_record_fields leftKey rightKey left right = .ok false := by
  rw [api.equal_record_fields.eq_def]
  simp [alloc.string.String.Insts.CoreCmpPartialEqString.eq, different, bind_ok]

theorem native_record_key_match (key : String) (left right : Value) :
    api.equal_record_fields key key left right = valueEq left right := by
  rw [api.equal_record_fields.eq_def]
  simp [alloc.string.String.Insts.CoreCmpPartialEqString.eq, bind_ok]

theorem native_list_value_delegates (left right : alloc.vec.Vec Value) :
    valueEq (.List left) (.List right) =
      api.equal_value_lists (alloc.vec.Vec.deref left) (alloc.vec.Vec.deref right) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]

theorem native_record_value_delegates (left right : alloc.vec.Vec (Prod String Value)) :
    valueEq (.Record left) (.Record right) =
      api.equal_value_records (alloc.vec.Vec.deref left) (alloc.vec.Vec.deref right) := by
  unfold valueEq
  rw [api.Value.Insts.CoreCmpPartialEqValue.eq.eq_def]


-- Proof-only logical equality; it does not implement or assume Rust Value::eq.
noncomputable instance : DecidableEq Value := Classical.decEq _

/-- Atomic on the left suffices, even when the right is a nested value. -/
def atomicValue : Value -> Prop
  | .List _ | .Record _ => False
  | _ => True

theorem native_atomic_value_equality_exact (left right : Value)
    (atomic : atomicValue left) :
    valueEq left right = .ok (decide (left = right)) := by
  classical
  cases left <;> simp only [atomicValue] at atomic <;> try contradiction
  all_goals cases right
  all_goals simp [native_null_value_equality_exact,
    native_boolean_value_equality_exact, native_integer_value_equality_exact,
    native_decimal_value_equality_exact, native_float_value_equality_exact,
    native_string_value_equality_exact, native_date_value_equality_exact,
    native_time_value_equality_exact, native_timestamp_value_equality_exact,
    native_duration_value_equality_exact, native_entity_value_equality_exact,
    native_bytes_value_equality_exact]
  all_goals apply native_value_variant_mismatch
  all_goals simp [valueTag]

end MRR.ContextValueProofs
