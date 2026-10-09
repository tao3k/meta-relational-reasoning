import Graph

namespace MRR.SearchComposition.ExecutionChecks

def composeOwners (mode : Mode) (primary secondary : List String) : List String :=
  match mode with
  | .single | .rankJoin => primary
  | .intersect => primary.filter secondary.contains

theorem composed_membership (mode : Mode) (primary secondary : List String) (owner : String) :
    owner ∈ composeOwners mode primary secondary ↔
      (Search.mk mode (fun item => item ∈ primary) (fun item => item ∈ secondary)).truth owner := by
  cases mode <;> simp [composeOwners, Search.truth, intersection]

end MRR.SearchComposition.ExecutionChecks
