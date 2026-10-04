import MRR.AgenticAIContext.Closure

namespace MRR.AgenticAIContext

/-- The pending list is the reverse of Rust's Vec stack. Set insertion is modeled
by membership plus duplicate-free insertion; output set order is irrelevant. -/
structure ContextWorklist where
  visited : List Nat
  pending : List Nat
  deriving DecidableEq

def worklistStep (deps : Nat -> List Nat) (state : ContextWorklist) : ContextWorklist :=
  match state.pending with
  | [] => state
  | id :: remaining =>
    if id inList state.visited then { state with pending := remaining }
    else { visited := id :: state.visited, pending := (deps id).reverse ++ remaining }

def worklistRun (deps : Nat -> List Nat) : Nat -> ContextWorklist -> ContextWorklist
  | 0, state => state
  | fuel + 1, state => worklistRun deps fuel (worklistStep deps state)

def worklistCovered (state : ContextWorklist) (id : Nat) : Prop :=
  id inList state.visited \/ id inList state.pending

structure WorklistInvariant (roots : List Nat) (deps : Nat -> List Nat)
    (state : ContextWorklist) : Prop where
  nodup : state.visited.Nodup
  visitedSound : forall id, id inList state.visited -> Required roots deps id
  pendingSound : forall id, id inList state.pending -> Required roots deps id
  rootsCovered : forall id, id inList roots -> worklistCovered state id
  edgesCovered : forall parent, parent inList state.visited ->
    forall id, id inList deps parent -> worklistCovered state id

/-- Every pop either skips an already visited identity or moves it to visited. -/
theorem worklist_step_coverage (deps : Nat -> List Nat) (state : ContextWorklist)
    (id : Nat) (covered : worklistCovered state id) :
    worklistCovered (worklistStep deps state) id := by
  cases state with
  | mk visited pending =>
    cases pending with
    | nil => exact covered
    | cons head remaining =>
      by_cases found : head inList visited
      case pos =>
        simp only [worklistCovered, List.mem_cons] at covered
        simp only [worklistStep, ite_eq_left found, worklistCovered]
        cases covered with
        | inl member => exact Or.inl member
        | inr member =>
          cases member with
          | inl equal => exact Or.inl (by simpa [equal] using found)
          | inr tail => exact Or.inr tail
      case neg =>
        simp only [worklistCovered, List.mem_cons] at covered
        simp only [worklistStep, ite_eq_right found, worklistCovered, List.mem_cons, List.mem_append, List.mem_reverse]
        cases covered with
        | inl member => exact Or.inl (Or.inr member)
        | inr member =>
          cases member with
          | inl equal => exact Or.inl (Or.inl equal)
          | inr tail => exact Or.inr (Or.inr tail)

/-- This is the loop invariant for the production stack/set algorithm. -/
theorem worklist_step_invariant (roots : List Nat) (deps : Nat -> List Nat)
    (state : ContextWorklist) (valid : WorklistInvariant roots deps state) :
    WorklistInvariant roots deps (worklistStep deps state) := by
  cases state with
  | mk visited pending =>
    cases pending with
    | nil => exact valid
    | cons head remaining =>
      by_cases found : head inList visited
      case pos =>
        simp only [worklistStep, ite_eq_left found]
        refine WorklistInvariant.mk valid.nodup valid.visitedSound ?_ ?_ ?_
        case refine_1 =>
          intro id member
          exact valid.pendingSound id (List.mem_cons_of_mem head member)
        case refine_2 =>
          intro id member
          simpa [worklistStep, found] using worklist_step_coverage deps _ id (valid.rootsCovered id member)
        case refine_3 =>
          intro parent member id edge
          simpa [worklistStep, found] using worklist_step_coverage deps _ id (valid.edgesCovered parent member id edge)
      case neg =>
        simp only [worklistStep, ite_eq_right found]
        refine WorklistInvariant.mk (List.nodup_cons.mpr (And.intro found valid.nodup)) ?_ ?_ ?_ ?_
        case refine_1 =>
          intro id member
          cases List.mem_cons.mp member with
          | inl equal => simpa [equal] using valid.pendingSound head (by simp)
          | inr old => exact valid.visitedSound id old
        case refine_2 =>
          intro id member
          cases List.mem_append.mp member with
          | inl dependency =>
            exact Required.dependency (valid.pendingSound head (by simp))
              (List.mem_reverse.mp dependency)
          | inr old => exact valid.pendingSound id (List.mem_cons_of_mem head old)
        case refine_3 =>
          intro id member
          simpa [worklistStep, found] using worklist_step_coverage deps _ id (valid.rootsCovered id member)
        case refine_4 =>
          intro parent member id edge
          cases List.mem_cons.mp member with
          | inl equal =>
            simp only [worklistCovered, List.mem_cons, List.mem_append, List.mem_reverse]
            exact Or.inr (Or.inl (by simpa [equal] using edge))
          | inr old => simpa [worklistStep, found] using worklist_step_coverage deps _ id (valid.edgesCovered parent old id edge)

theorem worklist_run_invariant (roots : List Nat) (deps : Nat -> List Nat)
    (fuel : Nat) (state : ContextWorklist) (valid : WorklistInvariant roots deps state) :
    WorklistInvariant roots deps (worklistRun deps fuel state) := by
  induction fuel generalizing state with
  | zero => exact valid
  | succ fuel induction =>
    exact induction (worklistStep deps state) (worklist_step_invariant roots deps state valid)

theorem worklist_initial_invariant (roots : List Nat) (deps : Nat -> List Nat) :
    WorklistInvariant roots deps { visited := [], pending := roots.reverse } := by
  refine WorklistInvariant.mk (by simp) ?_ ?_ ?_ ?_
  case refine_1 => intro id member; cases member
  case refine_2 => intro id member; exact Required.root (List.mem_reverse.mp member)
  case refine_3 => intro id member; exact Or.inr (List.mem_reverse.mpr member)
  case refine_4 => intro parent member; cases member

/-- Any successful stack/set run computes exactly the least required closure,
without relying on the separately checked certificate definition. -/
theorem worklist_refines_required (roots : List Nat) (deps : Nat -> List Nat) (fuel : Nat)
    (finished : (worklistRun deps fuel { visited := [], pending := roots.reverse }).pending = [])
    (id : Nat) :
    id inList (worklistRun deps fuel { visited := [], pending := roots.reverse }).visited <->
      Required roots deps id := by
  have valid := worklist_run_invariant roots deps fuel _ (worklist_initial_invariant roots deps)
  constructor
  case mp => exact valid.visitedSound id
  case mpr =>
    apply closed_contains_required roots _ deps
    case hasRoots =>
      intro root member
      have covered := valid.rootsCovered root member
      simpa [worklistCovered, finished] using covered
    case closed =>
      intro parent member dependency edge
      have covered := valid.edgesCovered parent member dependency edge
      simpa [worklistCovered, finished] using covered

/-- A finished production-shaped worklist and any accepted closure certificate
agree on every identity, irrespective of traversal or output ordering. -/
theorem worklist_refines_certificate (source roots : List Nat) (deps : Nat -> List Nat)
    (runFuel certificateFuel limit : Nat) (candidate : List Nat)
    (finished : (worklistRun deps runFuel { visited := [], pending := roots.reverse }).pending = [])
    (checked : checkRequiredClosure source roots deps certificateFuel limit candidate = true)
    (id : Nat) :
    id inList (worklistRun deps runFuel { visited := [], pending := roots.reverse }).visited <->
      id inList candidate := by
  rw [worklist_refines_required roots deps runFuel finished id,
    checked_closure_exact source roots deps certificateFuel limit candidate checked id]

end MRR.AgenticAIContext
