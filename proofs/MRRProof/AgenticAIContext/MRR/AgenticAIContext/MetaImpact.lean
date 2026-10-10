import MRR.AgenticAIContext.Revision

namespace MRR.AgenticAIContext

/-- One admitted positive branch. The support list is conjunctive; branches
with the same checked output key are alternatives. Empty support is rejected. -/
structure ImpactDerivation where
  outputKey : Nat
  support : List Nat
  admitted : Bool
  deriving DecidableEq

def BranchEligible (present : List Nat) (branch : ImpactDerivation) : Prop :=
  And (branch.admitted = true)
    (And (Not (branch.support = []))
      (forall input, Membership.mem branch.support input -> Membership.mem present input))

def OutputEligible (branches : List ImpactDerivation) (present : List Nat)
    (outputKey : Nat) : Prop :=
  exists branch, And (Membership.mem branches branch)
    (And (branch.outputKey = outputKey) (BranchEligible present branch))

/-- Losing one branch cannot retract an output while a complete alternative
branch for the same checked output key remains admitted and current. -/
theorem alternative_support_preserves (branches : List ImpactDerivation)
    (present : List Nat) (outputKey : Nat) (alternative : ImpactDerivation)
    (member : Membership.mem branches alternative)
    (same : alternative.outputKey = outputKey)
    (live : BranchEligible present alternative) :
    OutputEligible branches present outputKey :=
  Exists.intro alternative (And.intro member (And.intro same live))

/-- Retraction is a stronger statement than reachability: it needs a complete
support inventory and absence of every current admitted branch. -/
def OutputRetracted (oldBranches newBranches : List ImpactDerivation)
    (oldPresent newPresent : List Nat) (outputKey : Nat)
    (supportComplete : Bool) : Prop :=
  And (OutputEligible oldBranches oldPresent outputKey)
    (And (supportComplete = true)
      (Not (OutputEligible newBranches newPresent outputKey)))

theorem retracted_has_no_current_branch (oldBranches newBranches : List ImpactDerivation)
    (oldPresent newPresent : List Nat) (outputKey : Nat) (supportComplete : Bool)
    (retracted : OutputRetracted oldBranches newBranches oldPresent newPresent
      outputKey supportComplete) :
    Not (OutputEligible newBranches newPresent outputKey) := retracted.2.2

theorem incomplete_support_cannot_retract (oldBranches newBranches : List ImpactDerivation)
    (oldPresent newPresent : List Nat) (outputKey : Nat) :
    Not (OutputRetracted oldBranches newBranches oldPresent newPresent outputKey false) := by
  intro retracted
  exact Bool.false_ne_true retracted.2.1

/-- Direct reevaluation seeds inspect both old and new declared supports.
Transitive closure is the separately proved revision worklist contract. -/
def DirectImpact (oldBranches newBranches : List ImpactDerivation)
    (changed : List Nat) (outputKey : Nat) : Prop :=
  exists branch, And (Membership.mem (oldBranches ++ newBranches) branch)
    (And (branch.outputKey = outputKey)
      (exists input, And (Membership.mem branch.support input)
        (Membership.mem changed input)))

theorem new_support_enters_frontier (oldBranches newBranches : List ImpactDerivation)
    (changed : List Nat) (branch : ImpactDerivation) (input : Nat)
    (member : Membership.mem newBranches branch)
    (supported : Membership.mem branch.support input)
    (changedInput : Membership.mem changed input) :
    DirectImpact oldBranches newBranches changed branch.outputKey := by
  exact Exists.intro branch
    (And.intro (List.mem_append_right oldBranches member)
      (And.intro rfl (Exists.intro input (And.intro supported changedInput))))

theorem old_support_enters_frontier (oldBranches newBranches : List ImpactDerivation)
    (changed : List Nat) (branch : ImpactDerivation) (input : Nat)
    (member : Membership.mem oldBranches branch)
    (supported : Membership.mem branch.support input)
    (changedInput : Membership.mem changed input) :
    DirectImpact oldBranches newBranches changed branch.outputKey := by
  exact Exists.intro branch
    (And.intro (List.mem_append_left newBranches member)
      (And.intro rfl (Exists.intro input (And.intro supported changedInput))))

/-- A negative premise is usable only with a complete finite-domain receipt.
The proposition makes no claim about domains omitted from that receipt. -/
def CertifiedAbsence (coverageComplete : Bool) (observed : List Nat)
    (target : Nat) : Prop :=
  And (coverageComplete = true) (Not (Membership.mem observed target))

theorem partial_coverage_cannot_certify_absence (observed : List Nat)
    (target : Nat) : Not (CertifiedAbsence false observed target) := by
  intro certificate
  exact Bool.false_ne_true certificate.1

theorem addition_defeats_certified_absence (observed : List Nat) (target : Nat) :
    Not (CertifiedAbsence true (target :: observed) target) := by
  intro certificate
  exact certificate.2 (by simp)

/-- Importing a verified POO Flow receipt preserves its owner's claim level.
An unverified receipt has no admitted claim level. -/
inductive TemporalClaim where
  | structural | temporalPossibility | trajectoryContrast | causalEffect
  deriving DecidableEq

def importTemporalClaim (verified : Bool) (owner : TemporalClaim) :
    Option TemporalClaim :=
  if verified then some owner else none

theorem imported_claim_not_promoted (verified : Bool) (owner : TemporalClaim)
    (notEffect : Not (owner = .causalEffect)) :
    Not (importTemporalClaim verified owner = some .causalEffect) := by
  cases verified <;> simp [importTemporalClaim, notEffect]

end MRR.AgenticAIContext
