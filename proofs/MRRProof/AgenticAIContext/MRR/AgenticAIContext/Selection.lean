import MRR.AgenticAIContext.Reuse

namespace MRR.AgenticAIContext

/-- Abstract labels stand for exact facade query/generation/catalog/snapshot identities. -/
structure SelectionBinding where
  query : Nat
  generation : Nat
  catalog : Nat
  snapshot : Nat
  deriving DecidableEq, BEq

structure SelectionRow where
  fact : Nat
  relation : Nat
  deriving DecidableEq, BEq, ReflBEq, LawfulBEq

inductive SelectionError where
  | binding
  | budget
  | fact
  deriving BEq

/-- One named, relation-valued column. Row/cell counts coincide in this model.
The source facts are supplied by source admission; execution correctness is external. -/
def admitSelection (current claimed : SelectionBinding) (facts rows : List SelectionRow)
    (maxRows maxFacts : Nat) : Except SelectionError (List Nat) :=
  if claimed = current then
    if rows.length <= maxRows && facts.length <= maxFacts then
      if rows.all (fun row => facts.contains row && row.relation == 7) then
        .ok ((rows.map SelectionRow.fact).eraseDups)
      else .error .fact
    else .error .budget
  else .error .binding

/-- Deduplication preserves precisely the projected root set. -/
theorem selection_dedup_membership (rows : List SelectionRow) (id : Nat) :
    Membership.mem ((rows.map SelectionRow.fact).eraseDups) id <->
      exists row, Membership.mem rows row /\ row.fact = id := by
  simp only [List.mem_eraseDups, List.mem_map]

theorem selection_binding (current claimed : SelectionBinding) (facts rows : List SelectionRow)
    (maxRows maxFacts : Nat) (roots : List Nat)
    (accepted : admitSelection current claimed facts rows maxRows maxFacts = .ok roots) :
    claimed = current := by
  unfold admitSelection at accepted
  split at accepted
  case isTrue same => exact same
  case isFalse => contradiction

/-- Successful selection contains every and only projected returned identity. -/
theorem selection_exact_roots (current claimed : SelectionBinding) (facts rows : List SelectionRow)
    (maxRows maxFacts : Nat) (roots : List Nat)
    (accepted : admitSelection current claimed facts rows maxRows maxFacts = .ok roots)
    (id : Nat) : Membership.mem roots id <->
      exists row, Membership.mem rows row /\ row.fact = id := by
  unfold admitSelection at accepted
  split at accepted
  case isFalse => contradiction
  case isTrue =>
    split at accepted
    case isFalse => contradiction
    case isTrue =>
      split at accepted
      case isFalse => contradiction
      case isTrue =>
        cases Except.ok.inj accepted
        exact selection_dedup_membership rows id

/-- Each admitted root retains a matching source FactId and relation type. -/
theorem selection_source_membership (current claimed : SelectionBinding) (facts rows : List SelectionRow)
    (maxRows maxFacts : Nat) (roots : List Nat)
    (accepted : admitSelection current claimed facts rows maxRows maxFacts = .ok roots)
    (id : Nat) (member : Membership.mem roots id) :
    exists row, Membership.mem facts row /\ row.fact = id /\ row.relation = 7 := by
  have projected := (selection_exact_roots current claimed facts rows maxRows maxFacts roots accepted id).mp member
  cases projected with
  | intro row evidence =>
    unfold admitSelection at accepted
    split at accepted
    case isFalse => contradiction
    case isTrue =>
      split at accepted
      case isFalse => contradiction
      case isTrue =>
        split at accepted
        case isFalse => contradiction
        case isTrue valid =>
          have checked := List.all_eq_true.mp valid row evidence.1
          have parts : Membership.mem facts row /\ row.relation = 7 := by simpa using checked
          exact Exists.intro row (And.intro parts.1 (And.intro evidence.2 parts.2))

/-- Query, catalog, source generation or snapshot drift refuses admission. -/
theorem selection_drift_refused (current claimed : SelectionBinding) (facts rows : List SelectionRow)
    (maxRows maxFacts : Nat) (drift : Not (claimed = current)) :
    admitSelection current claimed facts rows maxRows maxFacts = .error .binding := by
  simp [admitSelection, drift]

/-- A drifted binding cannot grant a semantic reuse basis. This abstract gate is
independent of actual-token prefix eligibility in the existing computational model. -/
def selectionReusable (oldBinding newBinding : SelectionBinding)
    (oldRoots newRoots unchanged : List Nat) : List Nat :=
  if oldBinding = newBinding /\ oldRoots = newRoots then unchanged else []

theorem selection_drift_no_reuse (oldBinding newBinding : SelectionBinding)
    (oldRoots newRoots unchanged : List Nat) (drift : Not (oldBinding = newBinding)) :
    selectionReusable oldBinding newBinding oldRoots newRoots unchanged = [] := by
  simp [selectionReusable, drift]

theorem selection_root_change_no_reuse (binding : SelectionBinding)
    (oldRoots newRoots unchanged : List Nat) (drift : Not (oldRoots = newRoots)) :
    selectionReusable binding binding oldRoots newRoots unchanged = [] := by
  simp [selectionReusable, drift]

theorem selection_budget (current claimed : SelectionBinding) (facts rows : List SelectionRow)
    (maxRows maxFacts : Nat) (roots : List Nat)
    (accepted : admitSelection current claimed facts rows maxRows maxFacts = .ok roots) :
    rows.length <= maxRows /\ facts.length <= maxFacts := by
  unfold admitSelection at accepted
  split at accepted
  case isFalse => contradiction
  case isTrue =>
    split at accepted
    case isFalse => contradiction
    case isTrue bounded => simpa using bounded

end MRR.AgenticAIContext
