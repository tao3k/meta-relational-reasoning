import Generated.Funs

open Aeneas.Std
open MRR.ContextRust

namespace MRR.ContextRustProofs

abbrev Coverage := MRR.ContextRust.mrr_relation.api.EvidenceCompleteness
abbrev Admission := MRR.ContextRust.evidence.EvidenceAdmission

/-- Independent policy order: larger values indicate weaker evidence. -/
def rank : Coverage -> Nat
  | .Complete => 0
  | .Partial => 1
  | .Unknown => 2

/-- This theorem is about the automatically extracted production function. -/
theorem admission_accepted_iff (valid requireComplete : Bool) (coverage : Coverage) :
    evidence.admit_evidence valid coverage requireComplete = .ok .Accepted <->
      valid = true /\ (requireComplete = false \/ coverage = .Complete) := by
  cases valid <;> cases requireComplete <;> cases coverage <;>
    simp [evidence.admit_evidence]

theorem invalid_evidence_rejected (requireComplete : Bool) (coverage : Coverage) :
    evidence.admit_evidence false coverage requireComplete = .ok .Invalid := by
  rfl

theorem required_incomplete_rejected (coverage : Coverage) (incomplete : Not (coverage = .Complete)) :
    evidence.admit_evidence true coverage true = .ok .Incomplete := by
  cases coverage <;> simp_all [evidence.admit_evidence]

/-- Universal refinement to the independent worst-evidence specification. -/
theorem merge_refines_max (left right : Coverage) :
    exists result, evidence.merge_completeness left right = .ok result /\
      rank result = max (rank left) (rank right) := by
  cases left <;> cases right <;> simp [evidence.merge_completeness, rank]

theorem merge_commutative (left right : Coverage) :
    evidence.merge_completeness left right = evidence.merge_completeness right left := by
  cases left <;> cases right <;> rfl

theorem merge_associative (left middle right : Coverage) :
    (do let merged <- evidence.merge_completeness left middle
        evidence.merge_completeness merged right) =
    (do let merged <- evidence.merge_completeness middle right
        evidence.merge_completeness left merged) := by
  cases left <;> cases middle <;> cases right <;> simp [evidence.merge_completeness]

theorem complete_is_identity (coverage : Coverage) :
    evidence.merge_completeness .Complete coverage = .ok coverage := by
  cases coverage <;> rfl

end MRR.ContextRustProofs
