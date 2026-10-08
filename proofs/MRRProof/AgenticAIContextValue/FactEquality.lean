import ValueRecursive

open Aeneas Aeneas.Std
open MRR.ContextValue

namespace MRR.ContextValueProofs

deriving instance DecidableEq for mrr_relation.api.RelationAuthority
deriving instance DecidableEq for mrr_relation.api.FactProvenance
deriving instance DecidableEq for mrr_relation.api.EvidenceCompleteness
deriving instance DecidableEq for mrr_relation.api.FactValidity
deriving instance DecidableEq for mrr_relation.api.RelationContext
noncomputable instance : DecidableEq mrr_relation.api.Fact := Classical.decEq _
noncomputable instance : DecidableEq state.AgenticAiContextElement := Classical.decEq _

@[simp] theorem native_fact_id_equality_exact (left right : mrr_identity.api.FactId) :
    mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId.eq left right =
      .ok (decide (left = right)) := native_entity_id_equality_exact _ _

@[simp] theorem native_relation_id_equality_exact (left right : mrr_identity.api.RelationId) :
    mrr_identity.api.RelationId.Insts.CoreCmpPartialEqRelationId.eq left right =
      .ok (decide (left = right)) := native_entity_id_equality_exact _ _

@[simp] theorem native_generation_equality_exact (left right : mrr_identity.api.GenerationId) :
    mrr_identity.api.GenerationId.Insts.CoreCmpPartialEqGenerationId.eq left right =
      .ok (decide (left = right)) := native_entity_id_equality_exact _ _

@[simp] theorem native_rule_id_equality_exact (left right : mrr_identity.api.RuleId) :
    mrr_identity.api.RuleId.Insts.CoreCmpPartialEqRuleId.eq left right =
      .ok (decide (left = right)) := native_entity_id_equality_exact _ _

@[simp] theorem native_rule_pack_id_equality_exact (left right : mrr_identity.api.RulePackId) :
    mrr_identity.api.RulePackId.Insts.CoreCmpPartialEqRulePackId.eq left right =
      .ok (decide (left = right)) := native_entity_id_equality_exact _ _

@[simp] theorem native_derivation_id_equality_exact (left right : mrr_identity.api.DerivationId) :
    mrr_identity.api.DerivationId.Insts.CoreCmpPartialEqDerivationId.eq left right =
      .ok (decide (left = right)) := native_entity_id_equality_exact _ _

@[simp] theorem native_authority_equality_exact (left right : mrr_relation.api.RelationAuthority) :
    mrr_relation.api.RelationAuthority.Insts.CoreCmpPartialEqRelationAuthority.eq left right =
      .ok (decide (left = right)) := by
  cases left <;> cases right <;>
    simp [mrr_relation.api.RelationAuthority.Insts.CoreCmpPartialEqRelationAuthority.eq,
      mrr_relation.api.RelationAuthority.read_discriminant, native_entity_id_equality_exact]

@[simp] theorem native_provenance_equality_exact (left right : mrr_relation.api.FactProvenance) :
    mrr_relation.api.FactProvenance.Insts.CoreCmpPartialEqFactProvenance.eq left right =
      .ok (decide (left = right)) := by
  cases left <;> cases right <;>
    simp [mrr_relation.api.FactProvenance.Insts.CoreCmpPartialEqFactProvenance.eq,
      mrr_relation.api.FactProvenance.read_discriminant, native_entity_id_equality_exact]

@[simp] theorem native_completeness_equality_exact (left right : mrr_relation.api.EvidenceCompleteness) :
    mrr_relation.api.EvidenceCompleteness.Insts.CoreCmpPartialEqEvidenceCompleteness.eq left right =
      .ok (decide (left = right)) := by
  cases left <;> cases right <;>
    simp [mrr_relation.api.EvidenceCompleteness.Insts.CoreCmpPartialEqEvidenceCompleteness.eq,
      mrr_relation.api.EvidenceCompleteness.read_discriminant]

@[simp] theorem native_validity_equality_exact (left right : mrr_relation.api.FactValidity) :
    mrr_relation.api.FactValidity.Insts.CoreCmpPartialEqFactValidity.eq left right =
      .ok (decide (left = right)) := by
  cases left <;> cases right <;>
    simp [mrr_relation.api.FactValidity.Insts.CoreCmpPartialEqFactValidity.eq,
      mrr_relation.api.FactValidity.read_discriminant]

@[simp] theorem native_relation_context_equality_exact (left right : mrr_relation.api.RelationContext) :
    mrr_relation.api.RelationContext.Insts.CoreCmpPartialEqRelationContext.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [mrr_relation.api.RelationContext.Insts.CoreCmpPartialEqRelationContext.eq, bind_ok]
  repeat first | (split <;> simp_all) | simp_all

@[simp] theorem native_value_vec_equality_exact (left right : alloc.vec.Vec Value) :
    alloc.vec.partial_eq.PartialEqVec.eq mrr_relation.api.Value.Insts.CoreCmpPartialEqValue left right =
      .ok (decide (left = right)) :=
  native_vec_equality_exact _ native_value_equality_total _ _

/-- Actual derived Fact equality, including all recursive values and context fields. -/
@[simp] theorem native_fact_equality_exact (left right : mrr_relation.api.Fact) :
    mrr_relation.api.Fact.Insts.CoreCmpPartialEqFact.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [mrr_relation.api.Fact.Insts.CoreCmpPartialEqFact.eq, bind_ok]
  repeat first | (split <;> simp_all) | simp_all

@[simp] theorem native_fact_id_vec_equality_exact (left right : alloc.vec.Vec mrr_identity.api.FactId) :
    alloc.vec.partial_eq.PartialEqVec.eq mrr_identity.api.FactId.Insts.CoreCmpPartialEqFactId left right =
      .ok (decide (left = right)) :=
  native_vec_equality_exact _ native_fact_id_equality_exact _ _

/-- Actual derived element equality: every Fact field and ordered dependency vector. -/
theorem native_element_equality_exact (left right : state.AgenticAiContextElement) :
    state.AgenticAiContextElement.Insts.CoreCmpPartialEqAgenticAiContextElement.eq left right =
      .ok (decide (left = right)) := by
  cases left
  cases right
  simp [state.AgenticAiContextElement.Insts.CoreCmpPartialEqAgenticAiContextElement.eq, bind_ok]
  split <;> simp_all

/-- The actual imported Option equality calls the proved borrowed element comparator. -/
theorem native_element_option_equality_exact (old new : Option state.AgenticAiContextElement) :
    core.option.Option.Insts.CoreCmpPartialEqOption.eq
      (core.cmp.PartialEqShared state.AgenticAiContextElement.Insts.CoreCmpPartialEqAgenticAiContextElement)
      old new = .ok (decide (old = new)) := by
  cases old <;> cases new <;>
    simp [core.option.Option.Insts.CoreCmpPartialEqOption.eq,
      native_element_equality_exact, read_discriminant, optionDiscriminant]

/-- Actual production change predicate, with no element equality premise. -/
theorem native_revision_elements_differ_exact (old new : Option state.AgenticAiContextElement) :
    state.revision_elements_differ old new = .ok (decide (Not (old = new))) := by
  simp [state.revision_elements_differ, core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
    native_element_option_equality_exact]

theorem native_revision_elements_changed_iff (old new : Option state.AgenticAiContextElement) :
    state.revision_elements_differ old new = .ok true <-> Not (old = new) := by
  rw [native_revision_elements_differ_exact]
  simp

end MRR.ContextValueProofs
