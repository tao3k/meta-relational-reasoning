import Protocol

namespace MRR.SearchComposition

/-- Causal edges are supplied by the POO projection, never invented from C4 order. -/
inductive Path {Node : Type} (edge : Node -> Node -> Prop) : Node -> Node -> Nat -> Prop where
  | self (node : Node) : Path edge node node 0
  | extend {source previous target : Node} {distance : Nat}
      (earlier : Path edge source previous distance) (next : edge previous target) :
      Path edge source target (distance + 1)

structure Observation (Node Candidate : Type) where
  binding : Binding
  factor : Node
  candidate : Candidate

/-- Influence requires the original admitted observation and an actual bounded
causal path. This is the specification of MRR's factor influence, not a proof
that its Rust/Ascent evaluator implements the specification. -/
def influence {Node Candidate : Type} (edge : Node -> Node -> Prop)
    (expected : Binding) (observation : Observation Node Candidate)
    (target : Node) (distance maxDepth : Nat) : Prop :=
  observation.binding = expected /\ Path edge observation.factor target distance /\
    distance <= maxDepth

theorem influence_is_source_bound {Node Candidate : Type} {edge : Node -> Node -> Prop}
    {expected : Binding} {observation : Observation Node Candidate}
    {target : Node} {distance maxDepth : Nat}
    (accepted : influence edge expected observation target distance maxDepth) :
    observation.binding = expected := accepted.1

theorem influence_has_causal_path {Node Candidate : Type} {edge : Node -> Node -> Prop}
    {expected : Binding} {observation : Observation Node Candidate}
    {target : Node} {distance maxDepth : Nat}
    (accepted : influence edge expected observation target distance maxDepth) :
    Path edge observation.factor target distance := accepted.2.1

theorem influence_is_bounded {Node Candidate : Type} {edge : Node -> Node -> Prop}
    {expected : Binding} {observation : Observation Node Candidate}
    {target : Node} {distance maxDepth : Nat}
    (accepted : influence edge expected observation target distance maxDepth) :
    distance <= maxDepth := accepted.2.2

theorem path_stays_in_component {Node : Type} {edge : Node -> Node -> Prop}
    (component : Node -> Nat)
    (edgesStay : forall source target, edge source target -> component source = component target)
    {source target : Node} {distance : Nat} (path : Path edge source target distance) :
    component source = component target := by
  induction path with
  | self => rfl
  | extend earlier next ih => exact ih.trans (edgesStay _ _ next)

/-- Parallel branches without an explicit connecting edge cannot influence one another. -/
theorem parallel_no_cross_influence {Node Candidate : Type} {edge : Node -> Node -> Prop}
    (component : Node -> Nat)
    (edgesStay : forall source target, edge source target -> component source = component target)
    {expected : Binding} {observation : Observation Node Candidate}
    {target : Node} {distance maxDepth : Nat}
    (separate : Not (component observation.factor = component target)) :
    Not (influence edge expected observation target distance maxDepth) := by
  intro accepted
  exact separate (path_stays_in_component component edgesStay accepted.2.1)

end MRR.SearchComposition
