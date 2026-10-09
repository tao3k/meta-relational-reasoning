import Graph

namespace MRR.SearchComposition.ExecutionChecks

def composeOwners (mode : Mode) (primary secondary : List String) : List String :=
  match mode with
  | .single | .rankJoin => primary
  | .intersect => primary.filter secondary.contains

theorem composed_membership (mode : Mode) (primary secondary : List String) (owner : String) :
    Membership.mem (composeOwners mode primary secondary) owner <->
      (Search.mk mode (fun item => Membership.mem primary item) (fun item => Membership.mem secondary item)).truth owner := by
  cases mode <;> simp [composeOwners, Search.truth, intersection]

end MRR.SearchComposition.ExecutionChecks
