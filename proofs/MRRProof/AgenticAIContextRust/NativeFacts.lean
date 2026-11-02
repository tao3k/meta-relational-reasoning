import Evidence
import MRR.AgenticAIContext.Closure
import Mathlib.Data.Nat.Pairing

open Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

abbrev NativeFactId := mrr_identity.api.FactId
abbrev NativeFact := mrr_relation.api.Fact
abbrev NativeValidity := mrr_relation.api.FactValidity

/-- Lossless proof-only numbering of bytes. This is not a digest and makes no
claim that canonical inputs have distinct SHA-256 digests. -/
def numberBytes : List U8 -> Nat
  | [] => 0
  | byte :: rest => Nat.pair byte.val (numberBytes rest) + 1

theorem number_bytes_injective : Function.Injective numberBytes := by
  intro left
  induction left with
  | nil =>
    intro right equal
    cases right with
    | nil => rfl
    | cons byte rest => simp [numberBytes] at equal
  | cons byte rest induction =>
    intro right equal
    cases right with
    | nil => simp [numberBytes] at equal
    | cons other tail =>
      have pairEqual := Nat.add_right_cancel equal
      have components := Nat.pair_eq_pair.mp pairEqual
      have byteEqual := UScalar.val_eq_imp byte other components.1
      have tailEqual := induction components.2
      rw [byteEqual, tailEqual]

def factModelId (id : NativeFactId) : Nat := numberBytes id.val

/-- Identity-to-Nat projection for the actual extracted 32-byte Rust newtype. -/
theorem fact_model_id_injective : Function.Injective factModelId := by
  intro left right equal
  exact Aeneas.Std.Array.ext left right (number_bytes_injective equal)

theorem fact_model_id_equal_iff (left right : NativeFactId) :
    factModelId left = factModelId right <-> left = right :=
  fact_model_id_injective.eq_iff

theorem native_fact_id_has_32_bytes (id : NativeFactId) : id.val.length = 32 := by
  exact Aeneas.Std.Array.property id

/-- The extracted derive(PartialEq) uses Aeneas's explicit byte-array library
model. This proves its equality law, not a source proof of Rust's stdlib. -/
theorem native_fact_id_equality_exact (left right : NativeFactId) :
    mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq left right =
      .ok (decide (left = right)) := by
  have elementSpec : forall x y : U8,
      WP.spec (core.cmp.PartialEqU8.ne x y) (fun b => b = true <-> Not (x = y)) := by
    intro x y
    simp [core.cmp.impls.PartialEqU8.ne, liftFun2, WP.spec_ok]
  have sliceSpec := core.slice.cmp.PartialEqSlice.eq_homo_spec
    core.cmp.PartialEqU8 left.to_slice right.to_slice elementSpec
  have equalFunctions :
      mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq left right =
        core.slice.cmp.PartialEqSlice.eq core.cmp.PartialEqU8 left.to_slice right.to_slice := by
    simp only [mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq,
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

theorem native_fact_id_equality_refines_model (left right : NativeFactId) :
    mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq left right =
      .ok (decide (factModelId left = factModelId right)) := by
  simp only [native_fact_id_equality_exact, fact_model_id_equal_iff]

theorem native_fact_context_exact (fact : NativeFact) :
    mrr_relation.api.Fact.impl.context fact = .ok fact.context := by rfl

theorem native_completeness_exact (context : mrr_relation.api.RelationContext) :
    mrr_relation.api.RelationContext.impl.completeness context = .ok context.completeness := by rfl

theorem native_validity_exact (context : mrr_relation.api.RelationContext) :
    mrr_relation.api.RelationContext.impl.validity context = .ok context.validity := by rfl

theorem native_validity_equality_exact (left right : NativeValidity) :
    exists same,
      mrr_relation.api.FactValidity.Insts.CoreCmpPartialEqFactValidity.eq left right = .ok same /\
        (same = true <-> left = right) := by
  cases left <;> cases right <;>
    simp [mrr_relation.api.FactValidity.Insts.CoreCmpPartialEqFactValidity.eq,
      mrr_relation.api.FactValidity.read_discriminant, native_fact_id_equality_exact]

/-- The production validity comparison against Valid never calls the identity
comparator, even if the invalidation reason carries an arbitrary FactId. -/
theorem native_validity_admission_exact (validity : NativeValidity) :
    mrr_relation.api.FactValidity.Insts.CoreCmpPartialEqFactValidity.eq validity .Valid =
      .ok (match validity with | .Valid => true | .InvalidatedBy _ => false) := by
  cases validity <;> simp [mrr_relation.api.FactValidity.Insts.CoreCmpPartialEqFactValidity.eq,
    mrr_relation.api.FactValidity.read_discriminant]

theorem native_fact_admission_exact (fact : NativeFact) (requireComplete : Bool) :
    evidence.admit_fact_evidence fact requireComplete =
      evidence.admit_evidence
        (match fact.context.validity with | .Valid => true | .InvalidatedBy _ => false)
        fact.context.completeness requireComplete := by
  simp only [evidence.admit_fact_evidence, native_fact_context_exact,
    native_validity_exact, native_completeness_exact, native_validity_admission_exact, bind_ok]
  rfl

/-- Exact acceptance of the actual source Fact; no supplied validity Boolean is
assumed. Authority and generation are separate source-admission obligations. -/
theorem native_fact_accepted_iff (fact : NativeFact) (requireComplete : Bool) :
    evidence.admit_fact_evidence fact requireComplete = .ok .Accepted <->
      fact.context.validity = .Valid /\
        (requireComplete = false \/ fact.context.completeness = .Complete) := by
  rw [native_fact_admission_exact, admission_accepted_iff]
  cases fact.context.validity <;> simp

theorem native_invalid_fact_rejected (fact : NativeFact) (cause : NativeFactId)
    (invalid : fact.context.validity = .InvalidatedBy cause) (requireComplete : Bool) :
    evidence.admit_fact_evidence fact requireComplete = .ok .Invalid := by
  rw [native_fact_admission_exact, invalid]
  exact invalid_evidence_rejected _ _

/-- Least dependency closure over actual extracted fact identities. -/
inductive NativeRequired (roots : List NativeFactId) (deps : NativeFactId -> List NativeFactId) :
    NativeFactId -> Prop where
  | root {id} : id inList roots -> NativeRequired roots deps id
  | dependency {parent id} : NativeRequired roots deps parent ->
      id inList deps parent -> NativeRequired roots deps id

/-- Values outside the image have no edges. Within the image the projection is
lossless, so the witness choice cannot change a dependency list. -/
noncomputable def projectedDependencies (deps : NativeFactId -> List NativeFactId)
    (number : Nat) : List Nat := by
  classical
  exact
  if image : exists id, factModelId id = number then
    (deps (Classical.choose image)).map factModelId
  else []

theorem projected_dependencies_exact (deps : NativeFactId -> List NativeFactId)
    (id : NativeFactId) :
    projectedDependencies deps (factModelId id) = (deps id).map factModelId := by
  unfold projectedDependencies
  split
  next image =>
    have equal := fact_model_id_injective (Classical.choose_spec image)
    rw [equal]
  next outside => exact False.elim (outside (Exists.intro id rfl))

/-- Universal closure transport between actual 32-byte FactIds and the existing
Nat graph proofs, in both directions, including arbitrary cycles. -/
theorem native_required_projection (roots : List NativeFactId)
    (deps : NativeFactId -> List NativeFactId) (id : NativeFactId) :
    NativeRequired roots deps id <->
      Required (roots.map factModelId) (projectedDependencies deps) (factModelId id) := by
  constructor
  next =>
    intro required
    induction required with
    | root member => exact Required.root (List.mem_map.mpr (Exists.intro _ (And.intro member rfl)))
    | dependency _ edge induction =>
      apply Required.dependency induction
      rw [projected_dependencies_exact]
      exact List.mem_map.mpr (Exists.intro _ (And.intro edge rfl))
  next =>
    intro required
    have lift : forall number,
        Required (roots.map factModelId) (projectedDependencies deps) number ->
        exists native, factModelId native = number /\ NativeRequired roots deps native := by
      intro number proof
      induction proof with
      | root member =>
        apply Exists.elim (List.mem_map.mp member)
        intro native nativeSpec
        exact Exists.intro native (And.intro nativeSpec.2 (NativeRequired.root nativeSpec.1))
      | dependency _ edge induction =>
        apply Exists.elim induction
        intro parent parentSpec
        rw [<- parentSpec.1, projected_dependencies_exact] at edge
        apply Exists.elim (List.mem_map.mp edge)
        intro native nativeSpec
        exact Exists.intro native (And.intro nativeSpec.2
          (NativeRequired.dependency parentSpec.2 nativeSpec.1))
    apply Exists.elim (lift _ required)
    intro native nativeSpec
    have same := fact_model_id_injective nativeSpec.1
    simpa only [same] using nativeSpec.2

end MRR.ContextRustProofs
