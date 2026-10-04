import Generated.Funs
import MRR.AgenticAIContext.Termination

open Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

/-- A contract for the native adapter, not an axiom about a library container.
The continuation flag requires a strictly smaller rank; stopping establishes the
postcondition. Both cases preserve the invariant and cannot panic or diverge. -/
def AdvanceContract {State : Type} (adapter : worklist.Worklist State)
    (invariant post : State -> Prop) (rank : State -> Nat) : Prop :=
  forall state, invariant state -> exists flag next,
    adapter.advance state = .ok (flag, next) /\ invariant next /\
    (flag = true -> rank next < rank state) /\ (flag = false -> post next)

/-- Universal total correctness of the actual extracted production while loop.
No fuel or finite test bound is present in the statement. -/
theorem extracted_driver_total_correctness {State : Type}
    (adapter : worklist.Worklist State) (invariant post : State -> Prop)
    (rank : State -> Nat) (contract : AdvanceContract adapter invariant post rank)
    (initial : State) (valid : invariant initial) :
    exists final, worklist.run adapter initial = .ok final /\ post final := by
  have total : WP.spec (loop (worklist.run_loop.body adapter) initial) post := by
    apply loop.spec_decr_nat rank invariant post (worklist.run_loop.body adapter)
    next =>
      intro state stateValid
      apply Exists.elim (contract state stateValid)
      intro flag flagSpec
      apply Exists.elim flagSpec
      intro next nextSpec
      have advanced := nextSpec.1
      have nextValid := nextSpec.2.1
      have decreases := nextSpec.2.2.1
      have stopped := nextSpec.2.2.2
      cases flag <;> simp only [worklist.run_loop.body, advanced, bind_ok]
      case false => exact (WP.spec_ok _).mpr (stopped rfl)
      case true => exact (WP.spec_ok _).mpr (And.intro nextValid (decreases rfl))
    next =>
      exact valid
  simpa only [worklist.run, worklist.run_loop, WP.spec_equiv_exists] using total

theorem extracted_driver_preserves_invariant {State : Type}
    (adapter : worklist.Worklist State) (invariant post : State -> Prop)
    (rank : State -> Nat) (contract : AdvanceContract adapter invariant post rank)
    (initial : State) (valid : invariant initial) :
    exists final, worklist.run adapter initial = .ok final /\ invariant final /\ post final := by
  have stronger : AdvanceContract adapter invariant (fun state => invariant state /\ post state) rank := by
    intro state stateValid
    apply Exists.elim (contract state stateValid)
    intro flag flagSpec
    apply Exists.elim flagSpec
    intro next nextSpec
    exact Exists.intro flag (Exists.intro next (And.intro nextSpec.1
      (And.intro nextSpec.2.1 (And.intro nextSpec.2.2.1
        (fun done => And.intro nextSpec.2.1 (nextSpec.2.2.2 done))))))
  exact extracted_driver_total_correctness adapter invariant _ rank stronger initial valid

/-- This form permits duplicate suppression at enqueue time, as used by reverse
impact, or at pop time, as used by forward closure. It requires the adapter's
actual invariant and progress laws, not equality to a particular scheduling order. -/
theorem extracted_driver_graph_invariant_complete {State : Type}
    (adapter : worklist.Worklist State) (project : State -> ContextWorklist)
    (source roots : List Nat) (deps : Nat -> List Nat)
    (contract : AdvanceContract adapter
      (fun state => WorklistInvariant roots deps (project state))
      (fun state => (project state).pending = [])
      (fun state => worklistCapacity source deps (project state)))
    (initial : State) (valid : WorklistInvariant roots deps (project initial)) :
    exists final, worklist.run adapter initial = .ok final /\
      (project final).pending = [] /\
      forall id, id inList (project final).visited <-> Required roots deps id := by
  apply Exists.elim (extracted_driver_preserves_invariant adapter _ _ _ contract initial valid)
  intro final finalSpec
  exact Exists.intro final (And.intro finalSpec.1 (And.intro finalSpec.2.2
    (worklist_finished_invariant_exact roots deps _ finalSpec.2.1 finalSpec.2.2)))

/-- Explicit missing seam: the native adapter must simulate one model pop. This
is a universally quantified premise, rather than a replay-derived assumption. -/
def GraphAdvanceRefinement {State : Type} (adapter : worklist.Worklist State)
    (project : State -> ContextWorklist) (roots : List Nat) (deps : Nat -> List Nat) : Prop :=
  forall state, WorklistInvariant roots deps (project state) -> exists flag next,
    adapter.advance state = .ok (flag, next) /\
    (if flag then
      Not ((project state).pending = []) /\ project next = worklistStep deps (project state)
    else (project state).pending = [] /\ project next = project state)

/-- The source-derived driver computes the exact least closure for any finite
closed graph and any adapter satisfying the explicit one-pop refinement law. -/
theorem extracted_driver_refines_graph {State : Type}
    (adapter : worklist.Worklist State) (project : State -> ContextWorklist)
    (source roots : List Nat) (deps : Nat -> List Nat)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source ->
      forall id, id inList deps parent -> id inList source)
    (refinement : GraphAdvanceRefinement adapter project roots deps)
    (initial : State) (valid : WorklistInvariant roots deps (project initial)) :
    exists final, worklist.run adapter initial = .ok final /\
      (project final).pending = [] /\
      forall id, id inList (project final).visited <-> Required roots deps id := by
  let invariant := fun state => WorklistInvariant roots deps (project state)
  let post := fun state => (project state).pending = [] /\
    forall id, id inList (project state).visited <-> Required roots deps id
  let rank := fun state => worklistCapacity source deps (project state)
  have contract : AdvanceContract adapter invariant post rank := by
    intro state stateValid
    apply Exists.elim (refinement state stateValid)
    intro flag flagSpec
    apply Exists.elim flagSpec
    intro next nextSpec
    have simulation := nextSpec.2
    refine Exists.intro flag (Exists.intro next (And.intro nextSpec.1 ?_))
    cases flag
    case false =>
      simp only [Bool.false_eq_true, if_false] at simulation
      have finished := simulation.1
      have equal := simulation.2
      refine And.intro ?_ (And.intro ?_ ?_)
      next =>
        simpa only [invariant, equal] using stateValid
      next =>
        intro impossible; cases impossible
      next =>
        intro _
        simp only [post, equal]
        exact And.intro finished (worklist_finished_invariant_exact roots deps _ stateValid finished)
    case true =>
      simp only [if_true] at simulation
      have nonempty := simulation.1
      have equal := simulation.2
      refine And.intro ?_ (And.intro ?_ ?_)
      next =>
        simpa only [invariant, equal] using worklist_step_invariant roots deps _ stateValid
      next =>
        intro _
        have pendingSource : forall id, id inList (project state).pending -> id inList source := by
          intro id member
          exact closed_contains_required roots source deps rootsSource sourceClosed id
            (stateValid.pendingSound id member)
        simpa only [rank, equal] using
          worklist_capacity_decreases source deps (project state) pendingSource nonempty
      next =>
        intro impossible; cases impossible
  exact extracted_driver_total_correctness adapter invariant post rank contract initial valid

def modelAdapter (deps : Nat -> List Nat) : worklist.Worklist ContextWorklist :=
  { advance := fun state =>
      if state.pending.isEmpty then .ok (false, state)
      else .ok (true, worklistStep deps state) }

/-- The unchanged shared graph model discharges the adapter law universally.
This is an instantiation of the extracted production driver, not its native
BTreeMap/BTreeSet adapter, whose law is still a separate obligation. -/
theorem extracted_model_driver_total_correctness (source roots : List Nat)
    (deps : Nat -> List Nat)
    (rootsSource : forall id, id inList roots -> id inList source)
    (sourceClosed : forall parent, parent inList source ->
      forall id, id inList deps parent -> id inList source) :
    exists final, worklist.run (modelAdapter deps)
      { visited := [], pending := roots.reverse } = .ok final /\
      final.pending = [] /\ forall id, id inList final.visited <-> Required roots deps id := by
  have refinement : GraphAdvanceRefinement (modelAdapter deps) id roots deps := by
    intro state _
    cases pendingEq : state.pending with
    | nil =>
      refine Exists.intro false (Exists.intro state (And.intro ?_ ?_))
      next =>
        simp [modelAdapter, pendingEq]
      next =>
        simp [pendingEq]
    | cons head remaining =>
      refine Exists.intro true (Exists.intro (worklistStep deps state) (And.intro ?_ ?_))
      next =>
        simp [modelAdapter, pendingEq]
      next =>
        simp [pendingEq]
  exact extracted_driver_refines_graph (modelAdapter deps) id source roots deps
    rootsSource sourceClosed refinement _ (worklist_initial_invariant roots deps)

end MRR.ContextRustProofs
