import PooFlowComposition

namespace MRR.SearchComposition

/-- Data's explicit intersection cannot introduce or omit a complete-set match. -/
def intersection {Candidate : Type} (left right : Candidate -> Prop) : Candidate -> Prop :=
  fun candidate => left candidate /\ right candidate

theorem intersection_exact {Candidate : Type} (left right : Candidate -> Prop)
    (candidate : Candidate) : intersection left right candidate <->
      left candidate /\ right candidate := Iff.rfl

/-- RankJoin may reorder the primary truth set but cannot filter it. -/
def rankJoinTruth {Candidate : Type} (primary : Candidate -> Prop) : Candidate -> Prop := primary

theorem rank_join_preserves_truth {Candidate : Type} (primary : Candidate -> Prop)
    (candidate : Candidate) : rankJoinTruth primary candidate <-> primary candidate := Iff.rfl

/-- Equality admission represents exact identities, not digest authentication. -/
structure Binding where
  scope : String
  source : String
  resident : String
  abi : String
  generation : String
  deriving DecidableEq, BEq

structure Branch where
  binding : Binding
  complete : Bool
  truncated : Bool

def intersectionAdmitted (expected : Binding) (branch : Branch) : Prop :=
  branch.binding = expected /\ branch.complete = true /\ branch.truncated = false

theorem stale_generation_rejected (expected : Binding) (branch : Branch)
    (stale : Not (branch.binding.generation = expected.generation)) :
    Not (intersectionAdmitted expected branch) := by
  intro admitted
  exact stale (congrArg Binding.generation admitted.1)

theorem incomplete_intersection_rejected (expected : Binding) (branch : Branch)
    (incomplete : branch.complete = false) : Not (intersectionAdmitted expected branch) := by
  simp [intersectionAdmitted, incomplete]

theorem truncated_intersection_rejected (expected : Binding) (branch : Branch)
    (truncated : branch.truncated = true) : Not (intersectionAdmitted expected branch) := by
  simp [intersectionAdmitted, truncated]

end MRR.SearchComposition
