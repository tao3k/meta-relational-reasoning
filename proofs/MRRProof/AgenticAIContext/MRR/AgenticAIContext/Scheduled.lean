import MRR.AgenticAIContext.Termination

namespace MRR.AgenticAIContext

/-- Reverse impact marks identities at enqueue time. Processed is proof state;
it must not be confused with the native scheduled (invalidated) set. -/
structure ScheduledWorklist where
  processed : List Nat
  scheduled : List Nat
  pending : List Nat
  deriving DecidableEq

def ScheduledWorklist.project (state : ScheduledWorklist) : ContextWorklist :=
  { visited := state.processed, pending := state.pending }

def scheduledStep (deps : Nat -> List Nat) (state : ScheduledWorklist) : ScheduledWorklist :=
  match state.pending with
  | [] => state
  | head :: remaining =>
    let fresh := (deps head).filter (fun id => decide (Not (id inList state.scheduled)))
    { processed := head :: state.processed,
      scheduled := fresh ++ state.scheduled, pending := fresh.reverse ++ remaining }

structure ScheduledInvariant (roots : List Nat) (deps : Nat -> List Nat)
    (state : ScheduledWorklist) : Prop where
  graph : WorklistInvariant roots deps state.project
  scheduledCovered : forall id, id inList state.scheduled <-> worklistCovered state.project id
  pendingUnique : state.pending.Nodup
  pendingFresh : forall id, id inList state.pending -> Not (id inList state.processed)

theorem scheduled_step_coverage (deps : Nat -> List Nat) (state : ScheduledWorklist)
    (id : Nat) (covered : worklistCovered state.project id) :
    worklistCovered (scheduledStep deps state).project id := by
  cases state with
  | mk processed scheduled pending =>
    cases pending with
    | nil => exact covered
    | cons head remaining =>
      simp only [worklistCovered, ScheduledWorklist.project, List.mem_cons] at covered
      simp only [scheduledStep, ScheduledWorklist.project, worklistCovered,
        List.mem_cons, List.mem_append, List.mem_reverse]
      cases covered with
      | inl old => exact Or.inl (Or.inr old)
      | inr queued =>
        cases queued with
        | inl equal => exact Or.inl (Or.inl equal)
        | inr old => exact Or.inr (Or.inr old)

/-- Fresh neighbor membership uses the pre-step scheduled set. The native
neighbor container is a BTreeSet, so its list projection has no duplicates. -/
theorem scheduled_step_invariant (roots : List Nat) (deps : Nat -> List Nat)
    (neighborsUnique : forall id, (deps id).Nodup)
    (state : ScheduledWorklist) (valid : ScheduledInvariant roots deps state) :
    ScheduledInvariant roots deps (scheduledStep deps state) := by
  cases state with
  | mk processed scheduled pending =>
    cases pending with
    | nil => exact valid
    | cons head remaining =>
      let fresh := (deps head).filter (fun id => decide (Not (id inList scheduled)))
      have freshMember : forall id, id inList fresh <->
          id inList deps head /\ Not (id inList scheduled) := by
        intro id
        simp [fresh]
      have freshNotProcessed : forall id, id inList fresh -> Not (id inList processed) := by
        intro id member old
        exact (freshMember id).mp member |>.2
          ((valid.scheduledCovered id).mpr (Or.inl old))
      have freshNotHead : forall id, id inList fresh -> Not (id = head) := by
        intro id member equal
        subst id
        exact (freshMember head).mp member |>.2
          ((valid.scheduledCovered head).mpr (Or.inr (by simp [ScheduledWorklist.project])))
      have freshUnique := (neighborsUnique head).filter
        (fun id => decide (Not (id inList scheduled)))
      have freshNotRemaining : forall id, id inList fresh -> Not (id inList remaining) := by
        intro id member old
        exact (freshMember id).mp member |>.2
          ((valid.scheduledCovered id).mpr (Or.inr (by simp [ScheduledWorklist.project, old])))
      have headFresh : Not (head inList processed) := valid.pendingFresh head (by simp)
      have preserved := scheduled_step_coverage deps
        { processed := processed, scheduled := scheduled, pending := head :: remaining }
      simp only [scheduledStep]
      change ScheduledInvariant roots deps
        { processed := head :: processed, scheduled := fresh ++ scheduled,
          pending := fresh.reverse ++ remaining }
      refine ScheduledInvariant.mk ?_ ?_ ?_ ?_
      next =>
        refine WorklistInvariant.mk (List.nodup_cons.mpr (And.intro headFresh valid.graph.nodup)) ?_ ?_ ?_ ?_
        next =>
          intro id member
          cases List.mem_cons.mp member with
          | inl equal => simpa [equal] using valid.graph.pendingSound head (by simp [ScheduledWorklist.project])
          | inr old => exact valid.graph.visitedSound id old
        next =>
          intro id member
          cases List.mem_append.mp member with
          | inl added =>
            exact Required.dependency (valid.graph.pendingSound head (by simp [ScheduledWorklist.project]))
              ((freshMember id).mp (List.mem_reverse.mp added)).1
          | inr old => exact valid.graph.pendingSound id (by simp [ScheduledWorklist.project, old])
        next =>
          intro id member
          exact preserved id (valid.graph.rootsCovered id member)
        next =>
          intro parent member id edge
          cases List.mem_cons.mp member with
          | inl equal =>
            have dependency : id inList deps head := by simpa [equal] using edge
            by_cases known : id inList scheduled
            case pos => exact preserved id ((valid.scheduledCovered id).mp known)
            case neg =>
              exact Or.inr (List.mem_append.mpr (Or.inl
                (List.mem_reverse.mpr ((freshMember id).mpr (And.intro dependency known)))))
          | inr old => exact preserved id (valid.graph.edgesCovered parent old id edge)
      next =>
        intro id
        change id inList (fresh ++ scheduled) <->
          id inList (head :: processed) \/ id inList (fresh.reverse ++ remaining)
        simp only [List.mem_append, List.mem_cons, List.mem_reverse]
        constructor
        next =>
          intro member
          cases member with
          | inl added => exact Or.inr (Or.inl added)
          | inr old =>
            have covered := (valid.scheduledCovered id).mp old
            simpa only [scheduledStep, ScheduledWorklist.project, worklistCovered,
              List.mem_cons, List.mem_append, List.mem_reverse] using preserved id covered
        next =>
          intro covered
          cases covered with
          | inl done =>
            cases done with
            | inl equal =>
              exact Or.inr ((valid.scheduledCovered id).mpr (Or.inr (by simp [ScheduledWorklist.project, equal])))
            | inr old => exact Or.inr ((valid.scheduledCovered id).mpr (Or.inl old))
          | inr queued =>
            cases queued with
            | inl added => exact Or.inl added
            | inr old => exact Or.inr ((valid.scheduledCovered id).mpr
                (Or.inr (by simp [ScheduledWorklist.project, old])))
      next =>
        change (fresh.reverse ++ remaining).Nodup
        apply List.nodup_append.mpr
        refine And.intro ?_ (And.intro valid.pendingUnique.tail ?_)
        next => simpa only [List.Nodup, List.pairwise_reverse, ne_comm] using freshUnique
        next =>
          intro id added other old equal
          subst other
          exact freshNotRemaining id (List.mem_reverse.mp added) old
      next =>
        intro id member processedNow
        change id inList (fresh.reverse ++ remaining) at member
        change id inList (head :: processed) at processedNow
        cases List.mem_append.mp member with
        | inl added =>
          have freshId := List.mem_reverse.mp added
          cases List.mem_cons.mp processedNow with
          | inl equal => exact freshNotHead id freshId equal
          | inr old => exact freshNotProcessed id freshId old
        | inr old =>
          cases List.mem_cons.mp processedNow with
          | inl equal =>
            subst id
            exact (List.nodup_cons.mp valid.pendingUnique).1 old
          | inr done => exact valid.pendingFresh id (by simp [old]) done


theorem scheduled_initial_invariant (roots : List Nat) (deps : Nat -> List Nat)
    (rootsUnique : roots.Nodup) :
    ScheduledInvariant roots deps
      { processed := [], scheduled := roots, pending := roots.reverse } := by
  refine ScheduledInvariant.mk (worklist_initial_invariant roots deps) ?_ ?_ ?_
  next => intro id; simp [ScheduledWorklist.project, worklistCovered]
  next => simpa only [List.Nodup, List.pairwise_reverse, ne_comm] using rootsUnique
  next => intro id _ impossible; cases impossible

theorem scheduled_capacity_decreases (source roots : List Nat) (deps : Nat -> List Nat)
    (state : ScheduledWorklist) (valid : ScheduledInvariant roots deps state)
    (pendingSource : forall id, id inList state.pending -> id inList source)
    (nonempty : Not (state.pending = [])) :
    worklistCapacity source deps (scheduledStep deps state).project <
      worklistCapacity source deps state.project := by
  cases state with
  | mk processed scheduled pending =>
    cases pending with
    | nil => exact False.elim (nonempty rfl)
    | cons head remaining =>
      have known := pendingSource head (by simp)
      have fresh := valid.pendingFresh head (by simp)
      have released := remaining_edges_fresh deps processed source head known fresh
      have shorter := List.length_filter_le
        (fun id => decide (Not (id inList scheduled))) (deps head)
      simp only [worklistCapacity, scheduledStep, ScheduledWorklist.project,
        List.length_append, List.length_reverse, List.length_cons]
      omega

/-- At termination every scheduled identity has actually been processed; only
then can the native invalidated set serve as the exact output closure. -/
theorem scheduled_finished_exact (roots : List Nat) (deps : Nat -> List Nat)
    (state : ScheduledWorklist) (valid : ScheduledInvariant roots deps state)
    (finished : state.pending = []) (id : Nat) :
    id inList state.scheduled <-> Required roots deps id := by
  rw [valid.scheduledCovered id]
  simpa only [worklistCovered, ScheduledWorklist.project, finished, List.not_mem_nil, or_false]
    using worklist_finished_invariant_exact roots deps state.project valid.graph finished id

end MRR.AgenticAIContext
