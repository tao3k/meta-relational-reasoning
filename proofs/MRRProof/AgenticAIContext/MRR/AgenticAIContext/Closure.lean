import MRR.AgenticAIContext.Selection

namespace MRR.AgenticAIContext

/-- ASCII syntax expands to standard list membership, without a new predicate. -/
notation:50 id:51 " inList " items:51 => Membership.mem items id

/-- Dependencies are complete declarations from an admitted source. Mandatory
contract roots join query roots before closure; cycles are permitted. -/
inductive Required (roots : List Nat) (deps : Nat -> List Nat) : Nat -> Prop where
  | root {id : Nat} : id inList roots -> Required roots deps id
  | dependency {parent id : Nat} : Required roots deps parent -> id inList deps parent ->
      Required roots deps id

/-- Finite dependency expansion used to check rooted witnesses. Fuel limits the
certificate search, not the closure theorem: a closed accepted candidate must
contain every reachable element, including paths longer than the chosen fuel. -/
def expandRequired (roots : List Nat) (deps : Nat -> List Nat) : Nat -> List Nat
  | 0 => roots.eraseDups
  | fuel + 1 =>
      let previous := expandRequired roots deps fuel
      (previous ++ previous.flatMap deps).eraseDups

theorem expansion_sound (roots : List Nat) (deps : Nat -> List Nat) (fuel id : Nat)
    (member : id inList expandRequired roots deps fuel) : Required roots deps id := by
  induction fuel generalizing id with
  | zero => exact Required.root (by simpa [expandRequired] using member)
  | succ fuel induction =>
    simp only [expandRequired, List.mem_eraseDups, List.mem_append, List.mem_flatMap] at member
    cases member with
    | inl old => exact induction id old
    | inr witness =>
      cases witness with
      | intro parent evidence =>
        exact Required.dependency (induction parent evidence.1) evidence.2

/-- An executable certificate checker verifies source membership, rooted
witnesses, duplicate freedom, all roots, dependency closure, and a caller ceiling.
It does not trust a stored closure list merely because it was decoded. -/
def checkRequiredClosure (source roots : List Nat) (deps : Nat -> List Nat)
    (fuel limit : Nat) (candidate : List Nat) : Bool :=
  decide (candidate.Nodup /\ candidate.length <= limit /\
    (forall id, id inList candidate -> id inList source /\ id inList expandRequired roots deps fuel) /\
    (forall id, id inList roots -> id inList candidate) /\
    (forall parent, parent inList candidate -> forall id, id inList deps parent -> id inList candidate))

/-- Every reachable dependency belongs to a closed candidate containing roots. -/
theorem closed_contains_required (roots candidate : List Nat) (deps : Nat -> List Nat)
    (hasRoots : forall id, id inList roots -> id inList candidate)
    (closed : forall parent, parent inList candidate -> forall id, id inList deps parent -> id inList candidate)
    (id : Nat) (required : Required roots deps id) : id inList candidate := by
  induction required with
  | root member => exact hasRoots _ member
  | dependency _ edge induction => exact closed _ induction _ edge

/-- Acceptance establishes the least complete rooted dependency closure,
not only a superset or a bounded prefix of the traversal. -/
theorem checked_closure_exact (source roots : List Nat) (deps : Nat -> List Nat)
    (fuel limit : Nat) (candidate : List Nat)
    (accepted : checkRequiredClosure source roots deps fuel limit candidate = true)
    (id : Nat) : id inList candidate <-> Required roots deps id := by
  have checked := of_decide_eq_true accepted
  constructor
  case mp =>
    intro member
    exact expansion_sound roots deps fuel id (checked.2.2.1 id member).2
  case mpr => exact closed_contains_required roots candidate deps checked.2.2.2.1 checked.2.2.2.2 id

theorem checked_closure_source_budget (source roots : List Nat) (deps : Nat -> List Nat)
    (fuel limit : Nat) (candidate : List Nat)
    (accepted : checkRequiredClosure source roots deps fuel limit candidate = true) :
    candidate.Nodup /\ candidate.length <= limit /\ forall id, id inList candidate -> id inList source := by
  have checked := of_decide_eq_true accepted
  exact And.intro checked.1 (And.intro checked.2.1 (fun id member => (checked.2.2.1 id member).1))

/-- Two accepted certificates describe the same set even with different fuel or order. -/
theorem checked_closure_unique (source roots : List Nat) (deps : Nat -> List Nat)
    (fuelA fuelB limitA limitB : Nat) (left right : List Nat)
    (leftOk : checkRequiredClosure source roots deps fuelA limitA left = true)
    (rightOk : checkRequiredClosure source roots deps fuelB limitB right = true)
    (id : Nat) : id inList left <-> id inList right := by
  rw [checked_closure_exact source roots deps fuelA limitA left leftOk id,
    checked_closure_exact source roots deps fuelB limitB right rightOk id]

end MRR.AgenticAIContext
