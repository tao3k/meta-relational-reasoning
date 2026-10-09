import Protocol

namespace MRR.SearchComposition.Reflection

/-- Reflection reads a published snapshot. Coverage of the whole query is a
consumer-owned admission, stronger than Single's permissive truthReady. -/
structure Evidence (Candidate : Type) where
  snapshot : State Candidate
  completeCoverage : Bool
  queryIdentity : String

def bound {Candidate : Type} (current : Binding) (evidence : Evidence Candidate) : Prop :=
  evidence.snapshot.phase = .published ∧ evidence.snapshot.current = current ∧
    evidence.snapshot.observed = current ∧ evidence.snapshot.inferred = true

instance {Candidate : Type} (current : Binding) (evidence : Evidence Candidate) :
    Decidable (bound current evidence) := by
  unfold bound
  infer_instance

def absenceAdmitted {Candidate : Type} (current : Binding) (query : String)
    (evidence : Evidence Candidate) (candidate : Candidate) : Prop :=
  bound current evidence ∧ evidence.queryIdentity = query ∧
    evidence.completeCoverage = true ∧ ¬ evidence.snapshot.output candidate

instance {Candidate : Type} (current : Binding) (query : String)
    (evidence : Evidence Candidate) (candidate : Candidate)
    [Decidable (evidence.snapshot.output candidate)] :
    Decidable (absenceAdmitted current query evidence candidate) := by
  unfold absenceAdmitted
  infer_instance

theorem partial_cannot_certify_absence {Candidate : Type} (current : Binding)
    (query : String) (evidence : Evidence Candidate) (candidate : Candidate)
    (partialCoverage : evidence.completeCoverage = false) :
    ¬ absenceAdmitted current query evidence candidate := by
  intro admitted
  have coverage := admitted.2.2.1
  rw [partialCoverage] at coverage
  exact Bool.noConfusion coverage

theorem stale_cannot_reflect {Candidate : Type} (current : Binding)
    (evidence : Evidence Candidate)
    (stale : evidence.snapshot.observed.generation ≠ current.generation) :
    ¬ bound current evidence := by
  intro admitted
  exact stale (congrArg Binding.generation admitted.2.2.1)

theorem foreign_query_cannot_certify_absence {Candidate : Type} (current : Binding)
    (query : String) (evidence : Evidence Candidate) (candidate : Candidate)
    (foreign : evidence.queryIdentity ≠ query) :
    ¬ absenceAdmitted current query evidence candidate := by
  intro admitted
  exact foreign admitted.2.1

theorem certified_absence_matches_search {Candidate : Type} {search : Search Candidate}
    {initialBinding current : Binding} {query : String} {evidence : Evidence Candidate}
    {candidate : Candidate} (reachable : Reachable search initialBinding evidence.snapshot)
    (admitted : absenceAdmitted current query evidence candidate) :
    ¬ search.truth candidate := by
  intro hit
  exact admitted.2.2.2 ((published_exact reachable admitted.1.1 candidate).2 hit)

/-- Proposals carry their premise binding/query and a decreasing budget. They
are data, not Step constructors or authority to mutate the published snapshot. -/
structure Proposal where
  premise : Binding
  queryIdentity : String
  remaining : Nat

def propose (current : Binding) (query : String) : Nat → Option Proposal
  | 0 => none
  | remaining + 1 => some ⟨current, query, remaining⟩

theorem exhausted_cannot_propose (current : Binding) (query : String) :
    propose current query 0 = none := rfl

theorem proposal_decreases_budget {current : Binding} {query : String}
    {budget : Nat} {proposal : Proposal} (accepted : propose current query budget = some proposal) :
    proposal.remaining < budget := by
  cases budget with
  | zero => simp [propose] at accepted
  | succ remaining =>
    simp [propose] at accepted
    subst proposal
    exact Nat.lt_succ_self remaining

theorem proposal_retains_premise {current : Binding} {query : String}
    {budget : Nat} {proposal : Proposal} (accepted : propose current query budget = some proposal) :
    proposal.premise = current ∧ proposal.queryIdentity = query := by
  cases budget with
  | zero => simp [propose] at accepted
  | succ remaining =>
    simp [propose] at accepted
    subst proposal
    exact ⟨rfl, rfl⟩

end MRR.SearchComposition.Reflection
