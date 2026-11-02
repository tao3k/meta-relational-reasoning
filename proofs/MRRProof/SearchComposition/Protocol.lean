import SearchComposition

namespace MRR.SearchComposition

inductive Mode where
  | single | rankJoin | intersect
  deriving DecidableEq, BEq

/-- Provider sets are abstract predicates; these laws do not assume a finite corpus. -/
structure Search (Candidate : Type) where
  mode : Mode
  primary : Candidate -> Prop
  secondary : Candidate -> Prop

def Search.truth {Candidate : Type} (search : Search Candidate) : Candidate -> Prop :=
  match search.mode with
  | .single | .rankJoin => search.primary
  | .intersect => intersection search.primary search.secondary

theorem intersection_sound {Candidate : Type} (left right : Candidate -> Prop)
    (candidate : Candidate) (hit : intersection left right candidate) :
    left candidate /\ right candidate := hit

theorem intersection_commutes {Candidate : Type} (left right : Candidate -> Prop) :
    intersection left right = intersection right left := by
  funext candidate
  exact propext (and_comm)

theorem intersection_associates {Candidate : Type} (a b c : Candidate -> Prop) :
    intersection (intersection a b) c = intersection a (intersection b c) := by
  funext candidate
  exact propext (and_assoc)

theorem rank_join_ignores_partial_ranking {Candidate : Type}
    (primary rankingA rankingB : Candidate -> Prop) :
    (Search.mk .rankJoin primary rankingA).truth =
      (Search.mk .rankJoin primary rankingB).truth := rfl

inductive Phase where
  | read | merge | ready | published
  deriving DecidableEq, BEq

/-- The snapshot binding is shared by both admitted branches. ASP must establish
this equality for each branch before lowering to this protocol. -/
structure State (Candidate : Type) where
  phase : Phase
  current : Binding
  observed : Binding
  complete : Bool
  truncated : Bool
  secondaryComplete : Bool
  secondaryTruncated : Bool
  inferred : Bool
  output : Candidate -> Prop

def truthReady {Candidate : Type} (search : Search Candidate) (state : State Candidate) : Prop :=
  match search.mode with
  | .single => True
  | .rankJoin => state.complete = true /\ state.truncated = false
  | .intersect => state.complete = true /\ state.truncated = false /\
      state.secondaryComplete = true /\ state.secondaryTruncated = false

instance {Candidate : Type} (search : Search Candidate) (state : State Candidate) :
    Decidable (truthReady search state) := by
  unfold truthReady
  cases search.mode <;> infer_instance

def valid {Candidate : Type} (search : Search Candidate) (state : State Candidate) : Prop :=
  (state.phase = .ready \/ state.phase = .published) ->
    state.observed = state.current /\ truthReady search state /\
      state.inferred = true /\ forall candidate, state.output candidate <-> search.truth candidate

def initial {Candidate : Type} (binding : Binding) : State Candidate :=
  State.mk .read binding binding true false true false false (fun _ => False)

def readStep {Candidate : Type} (state : State Candidate)
    (complete truncated secondaryComplete secondaryTruncated : Bool) : State Candidate :=
  {state with
    phase := .merge, observed := state.current, complete := complete,
    truncated := truncated, secondaryComplete := secondaryComplete,
    secondaryTruncated := secondaryTruncated, inferred := false, output := fun _ => False}

def mergeStep {Candidate : Type} (search : Search Candidate) (state : State Candidate) :
    State Candidate := {state with phase := .ready, inferred := true, output := search.truth}

/-- A revision retires results even when it arrives after publication. -/
def reviseStep {Candidate : Type} (state : State Candidate) (binding : Binding) :
    State Candidate :=
  {state with phase := .merge, current := binding, inferred := false, output := fun _ => False}

def restartStep {Candidate : Type} (state : State Candidate) : State Candidate :=
  {state with phase := .read, inferred := false, output := fun _ => False}

def publishStep {Candidate : Type} (state : State Candidate) : State Candidate :=
  {state with phase := .published}

/-- This relation has the same read/merge/revise/restart/publish/stay actions as
the Quint model. Inference is an admitted evidence bit, not an Ascent refinement proof. -/
inductive Step {Candidate : Type} (search : Search Candidate) :
    State Candidate -> State Candidate -> Prop where
  | read (state : State Candidate) (complete truncated secondaryComplete secondaryTruncated : Bool)
      (phase : state.phase = .read) :
      Step search state (readStep state complete truncated secondaryComplete secondaryTruncated)
  | merge (state : State Candidate) (phase : state.phase = .merge)
      (binding : state.observed = state.current) (complete : truthReady search state) :
      Step search state (mergeStep search state)
  | revise (state : State Candidate) (binding : Binding) :
      Step search state (reviseStep state binding)
  | restart (state : State Candidate) (phase : state.phase = .merge)
      (stale : Not (state.observed = state.current)) : Step search state (restartStep state)
  | publish (state : State Candidate) (phase : state.phase = .ready)
      (inferred : state.inferred = true) : Step search state (publishStep state)
  | stay (state : State Candidate) : Step search state state

theorem initial_valid {Candidate : Type} (search : Search Candidate) (binding : Binding) :
    valid search (initial binding) := by
  simp [valid, initial]

theorem step_preserves_valid {Candidate : Type} {search : Search Candidate}
    {before after : State Candidate} (previous : valid search before)
    (step : Step search before after) : valid search after := by
  cases step with
  | read complete truncated secondaryComplete secondaryTruncated phase =>
    simp [valid, readStep]
  | merge phase binding complete =>
    intro _
    exact And.intro binding (And.intro complete (And.intro rfl (fun _ => Iff.rfl)))
  | revise binding => simp [valid, reviseStep]
  | restart phase stale => simp [valid, restartStep]
  | publish phase inferred =>
    intro _
    simpa [publishStep, truthReady] using previous (Or.inl phase)
  | stay => exact previous

inductive Reachable {Candidate : Type} (search : Search Candidate) (binding : Binding) :
    State Candidate -> Prop where
  | initial : Reachable search binding (MRR.SearchComposition.initial binding)
  | next {before after : State Candidate} (beforeReachable : Reachable search binding before)
      (step : Step search before after) : Reachable search binding after

theorem reachable_valid {Candidate : Type} {search : Search Candidate} {binding : Binding}
    {state : State Candidate} (reachable : Reachable search binding state) : valid search state := by
  induction reachable with
  | initial => exact initial_valid search binding
  | next beforeReachable step ih => exact step_preserves_valid ih step

theorem published_exact {Candidate : Type} {search : Search Candidate} {binding : Binding}
    {state : State Candidate} (reachable : Reachable search binding state)
    (published : state.phase = .published) (candidate : Candidate) :
    state.output candidate <-> search.truth candidate :=
  (reachable_valid reachable (Or.inr published)).2.2.2 candidate

theorem published_binding {Candidate : Type} {search : Search Candidate} {binding : Binding}
    {state : State Candidate} (reachable : Reachable search binding state)
    (published : state.phase = .published) : state.observed = state.current :=
  (reachable_valid reachable (Or.inr published)).1

theorem stale_cannot_publish {Candidate : Type} {search : Search Candidate} {binding : Binding}
    {state : State Candidate} (reachable : Reachable search binding state)
    (stale : Not (state.observed.generation = state.current.generation)) :
    Not (state.phase = .published) := by
  intro published
  exact stale (congrArg Binding.generation (published_binding reachable published))

theorem foreign_binding_cannot_publish {Candidate : Type} {search : Search Candidate}
    {binding : Binding} {state : State Candidate}
    (reachable : Reachable search binding state) (foreign : Not (state.observed = state.current)) :
    Not (state.phase = .published) := by
  intro published
  exact foreign (published_binding reachable published)

theorem published_intersection_complete {Candidate : Type} {search : Search Candidate}
    {binding : Binding} {state : State Candidate}
    (reachable : Reachable search binding state) (published : state.phase = .published)
    (mode : search.mode = .intersect) :
    state.complete = true /\ state.truncated = false /\
      state.secondaryComplete = true /\ state.secondaryTruncated = false := by
  simpa [truthReady, mode] using (reachable_valid reachable (Or.inr published)).2.1

theorem empty_intersection_certifies_absence {Candidate : Type}
    {search : Search Candidate} {binding : Binding} {state : State Candidate}
    (reachable : Reachable search binding state) (published : state.phase = .published)
    (mode : search.mode = .intersect) (empty : forall candidate, Not (state.output candidate)) :
    forall candidate, Not (search.primary candidate /\ search.secondary candidate) := by
  intro candidate matchBoth
  apply empty candidate
  apply (published_exact reachable published candidate).mpr
  simpa [Search.truth, mode, intersection] using matchBoth

end MRR.SearchComposition
