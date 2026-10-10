import MRR.AgenticAIContext.Worklist
import Lean.Elab.Tactic.Omega

namespace MRR.AgenticAIContext

/-- Every source vertex can enqueue its dependency list only once. This weight
accounts for all future enqueues, including repeated targets and cycles. -/
def remainingEdges (deps : Nat -> List Nat) (visited : List Nat) : List Nat -> Nat
  | [] => 0
  | id :: source =>
    if id inList visited then remainingEdges deps visited source
    else (deps id).length + remainingEdges deps visited source

def worklistCapacity (source : List Nat) (deps : Nat -> List Nat) (state : ContextWorklist) : Nat :=
  state.pending.length + remainingEdges deps state.visited source

theorem remaining_edges_insert_le (deps : Nat -> List Nat) (visited source : List Nat) (head : Nat) :
    remainingEdges deps (head :: visited) source <= remainingEdges deps visited source := by
  induction source with
  | nil => simp [remainingEdges]
  | cons id rest induction =>
    by_cases found : id inList visited
    case pos => simpa [remainingEdges, found, List.mem_cons] using induction
    case neg =>
      by_cases equal : id = head
      case pos =>
        subst id
        simp only [remainingEdges, List.mem_cons, found, true_or, ite_true, ite_false]
        omega
      case neg => simpa [remainingEdges, found, equal, List.mem_cons] using Nat.add_le_add_left induction (deps id).length

/-- A fresh known vertex releases at least its entire enqueue budget. -/
theorem remaining_edges_fresh (deps : Nat -> List Nat) (visited source : List Nat) (head : Nat)
    (known : head inList source) (fresh : Not (head inList visited)) :
    remainingEdges deps (head :: visited) source + (deps head).length <= remainingEdges deps visited source := by
  induction source with
  | nil => cases known
  | cons id rest induction =>
    cases List.mem_cons.mp known with
    | inl equal =>
      subst id
      have smaller := remaining_edges_insert_le deps visited rest head
      simp only [remainingEdges, List.mem_cons, fresh, true_or, ite_true, ite_false]
      omega
    | inr member =>
      have smaller := induction member
      by_cases found : id inList visited
      case pos => simpa [remainingEdges, found, List.mem_cons] using smaller
      case neg =>
        by_cases equal : id = head
        case pos =>
          subst id
          have monotone := remaining_edges_insert_le deps visited rest head
          simp only [remainingEdges, List.mem_cons, fresh, true_or, ite_true, ite_false]
          omega
        case neg =>
          simp only [remainingEdges, List.mem_cons, found, equal, or_false, ite_false]
          omega

theorem worklist_capacity_decreases (source : List Nat) (deps : Nat -> List Nat)
    (state : ContextWorklist) (pendingSource : forall id, id inList state.pending -> id inList source)
    (nonempty : Not (state.pending = [])) :
    worklistCapacity source deps (worklistStep deps state) < worklistCapacity source deps state := by
  cases state with
  | mk visited pending =>
    cases pending with
    | nil => exact False.elim (nonempty rfl)
    | cons head rest =>
      have known := pendingSource head (by simp)
      by_cases found : head inList visited
      case pos => simp [worklistCapacity, worklistStep, found]
      case neg =>
        have released := remaining_edges_fresh deps visited source head known found
        simp only [worklistCapacity, worklistStep, found, ite_false,
          List.length_append, List.length_reverse, List.length_cons]
        omega

private theorem worklist_empty_stable (deps : Nat -> List Nat) (fuel : Nat) (state : ContextWorklist)
    (empty : state.pending = []) : (worklistRun deps fuel state).pending = [] := by
  induction fuel with
  | zero => exact empty
  | succ fuel induction =>
    have step : worklistStep deps state = state := by simp [worklistStep, empty]
    simpa only [worklistRun, step] using induction

/-- The concrete queue/set model terminates for a finite source with complete,
known dependency declarations. No acyclicity premise is required. -/
theorem worklist_run_finishes (source roots : List Nat) (deps : Nat -> List Nat)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (fuel : Nat) (state : ContextWorklist) (valid : WorklistInvariant roots deps state)
    (enough : worklistCapacity source deps state < fuel) :
    (worklistRun deps fuel state).pending = [] := by
  induction fuel generalizing state with
  | zero => omega
  | succ fuel induction =>
    by_cases empty : state.pending = []
    case pos => exact worklist_empty_stable deps (fuel + 1) state empty
    case neg =>
      have pendingSource : forall id, id inList state.pending -> id inList source := by
        intro id member
        exact closed_contains_required roots source deps rootsSource sourceClosed id (valid.pendingSound id member)
      have decreased := worklist_capacity_decreases source deps state pendingSource empty
      have enoughNext : worklistCapacity source deps (worklistStep deps state) < fuel := by omega
      exact induction (worklistStep deps state) (worklist_step_invariant roots deps state valid) enoughNext

/-- The total fuel bound is initial stack length plus all declared edges plus
one. Together with refinement, this gives a terminating complete algorithm. -/
theorem finite_worklist_total_correctness (source roots : List Nat) (deps : Nat -> List Nat)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source -> forall id, id inList deps parent -> id inList source)
    (id : Nat) :
    let initial : ContextWorklist := { visited := [], pending := roots.reverse }
    let fuel := worklistCapacity source deps initial + 1
    (worklistRun deps fuel initial).pending = [] /\
      (id inList (worklistRun deps fuel initial).visited <-> Required roots deps id) := by
  dsimp only
  have finished := worklist_run_finishes source roots deps rootsSource sourceClosed
    (worklistCapacity source deps { visited := [], pending := roots.reverse } + 1)
    _ (worklist_initial_invariant roots deps) (Nat.lt_succ_self _)
  exact And.intro finished (worklist_refines_required roots deps _ finished id)

end MRR.AgenticAIContext
