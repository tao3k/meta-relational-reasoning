import NativeFacts

open Aeneas Aeneas.Std
open MRR.ContextRust

namespace MRR.ContextRustProofs

deriving instance DecidableEq for mrr_revision.api.ExternalRevisionIdentity
deriving instance DecidableEq for mrr_revision.api.RevisionBinding
deriving instance DecidableEq for mrr_revision.snapshot.SemanticSnapshot
deriving instance DecidableEq for state.AgenticAiContextQuery
deriving instance DecidableEq for state.AgenticAiContextContract

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

@[simp] theorem native_generation_equality_exact (left right : mrr_identity.api.GenerationId) :
    mrr_identity.api.GenerationId.Insts.CoreCmpPartialEqGenerationId.eq left right =
      .ok (decide (left = right)) := native_fact_id_equality_exact _ _

@[simp] theorem native_entity_equality_exact (left right : mrr_identity.api.EntityId) :
    mrr_identity.api.EntityId.Insts.CoreCmpPartialEqEntityId.eq left right =
      .ok (decide (left = right)) := native_fact_id_equality_exact _ _

@[simp] theorem native_query_id_equality_exact (left right : mrr_identity.api.QueryId) :
    mrr_identity.api.QueryId.Insts.CoreCmpPartialEqQueryId.eq left right =
      .ok (decide (left = right)) := native_fact_id_equality_exact _ _

@[simp] theorem native_state_id_equality_exact (left right : mrr_identity.api.StateId) :
    mrr_identity.api.StateId.Insts.CoreCmpPartialEqStateId.eq left right =
      .ok (decide (left = right)) := native_fact_id_equality_exact _ _

@[simp] theorem native_revision_id_equality_exact (left right : mrr_identity.api.RevisionId) :
    mrr_identity.api.RevisionId.Insts.CoreCmpPartialEqRevisionId.eq left right =
      .ok (decide (left = right)) := native_fact_id_equality_exact _ _

@[simp] theorem native_byte_array_equality_exact (left right : NativeFactId) :
    core.array.equality.PartialEqArray.eq core.cmp.PartialEqU8 left right =
      .ok (decide (left = right)) := native_fact_id_equality_exact _ _

@[simp] theorem native_fact_vec_equality_exact (left right : alloc.vec.Vec NativeFactId) :
    alloc.vec.partial_eq.PartialEqVec.eq mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId left right =
      .ok (decide (left = right)) :=
  native_vec_equality_exact _ native_fact_id_equality_exact _ _

@[simp] theorem native_external_revision_equality_exact
    (left right : mrr_revision.api.ExternalRevisionIdentity) :
    mrr_revision.api.ExternalRevisionIdentity.Insts.CoreCmpPartialEqExternalRevisionIdentity.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [mrr_revision.api.ExternalRevisionIdentity.Insts.CoreCmpPartialEqExternalRevisionIdentity.eq,
    alloc.string.String.Insts.CoreCmpPartialEqString.eq, bind_ok]
  split <;> simp_all
  split <;> simp_all

@[simp] theorem native_revision_binding_equality_exact
    (left right : mrr_revision.api.RevisionBinding) :
    mrr_revision.api.RevisionBinding.Insts.CoreCmpPartialEqRevisionBinding.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [mrr_revision.api.RevisionBinding.Insts.CoreCmpPartialEqRevisionBinding.eq, bind_ok]
  split <;> simp_all
  split <;> simp_all

@[simp] theorem native_revision_vec_equality_exact
    (left right : alloc.vec.Vec mrr_revision.api.RevisionBinding) :
    alloc.vec.partial_eq.PartialEqVec.eq
      mrr_revision.api.RevisionBinding.Insts.CoreCmpPartialEqRevisionBinding left right =
      .ok (decide (left = right)) :=
  native_vec_equality_exact _ native_revision_binding_equality_exact _ _

@[simp] theorem native_snapshot_equality_exact (left right : mrr_revision.snapshot.SemanticSnapshot) :
    mrr_revision.snapshot.SemanticSnapshot.Insts.CoreCmpPartialEqSemanticSnapshot.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [mrr_revision.snapshot.SemanticSnapshot.Insts.CoreCmpPartialEqSemanticSnapshot.eq, bind_ok]
  split <;> simp_all
  split <;> simp_all

@[simp] theorem native_query_equality_exact (left right : state.AgenticAiContextQuery) :
    state.AgenticAiContextQuery.Insts.CoreCmpPartialEqAgenticAiContextQuery.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [state.AgenticAiContextQuery.Insts.CoreCmpPartialEqAgenticAiContextQuery.eq, bind_ok]
  split <;> simp_all

@[simp] theorem native_contract_equality_exact (left right : state.AgenticAiContextContract) :
    state.AgenticAiContextContract.Insts.CoreCmpPartialEqAgenticAiContextContract.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [state.AgenticAiContextContract.Insts.CoreCmpPartialEqAgenticAiContextContract.eq,
    bind_ok]
  repeat first | (split <;> simp_all) | simp_all

theorem native_revision_bindings_changed_exact
    (oldSnapshot newSnapshot : mrr_revision.snapshot.SemanticSnapshot)
    (oldQuery newQuery : state.AgenticAiContextQuery)
    (oldContract newContract : state.AgenticAiContextContract) (force : Bool) :
    state.revision_bindings_changed oldSnapshot newSnapshot oldQuery newQuery oldContract newContract force =
      .ok (decide (force = true \/ Not (oldSnapshot = newSnapshot) \/ Not (oldQuery = newQuery) \/
        Not (oldContract = newContract))) := by
  simp only [state.revision_bindings_changed, core.cmp.impls.PartialEqShared.ne,
    core.cmp.PartialEq.ne.default, native_snapshot_equality_exact,
    native_query_equality_exact, native_contract_equality_exact, bind_tc_ok, bind_ok]
  cases force <;> by_cases snapshotSame : oldSnapshot = newSnapshot
    <;> by_cases querySame : oldQuery = newQuery
    <;> by_cases contractSame : oldContract = newContract
    <;> simp [snapshotSame, querySame, contractSame]

end MRR.ContextRustProofs
