namespace MRRProof

structure Digest256 where
  value : Nat
  is256Bit : value < 2 ^ 256
  deriving DecidableEq

def Digest256.ofFixtureAtom (value : Nat) : Digest256 where
  value := value % (2 ^ 256)
  is256Bit := Nat.mod_lt value (by decide)

structure EntityId where
  digest : Digest256
  deriving DecidableEq

structure RelationId where
  digest : Digest256
  deriving DecidableEq

structure FactId where
  digest : Digest256
  deriving DecidableEq

structure GenerationId where
  digest : Digest256
  deriving DecidableEq

structure QueryId where
  digest : Digest256
  deriving DecidableEq

structure QueryOperatorId where
  digest : Digest256
  deriving DecidableEq

structure RuleId where
  digest : Digest256
  deriving DecidableEq

structure RulePackId where
  digest : Digest256
  deriving DecidableEq

structure DerivationId where
  digest : Digest256
  deriving DecidableEq

structure StateId where
  digest : Digest256
  deriving DecidableEq

structure TransitionId where
  digest : Digest256
  deriving DecidableEq

structure ActionId where
  digest : Digest256
  deriving DecidableEq

structure LineageNodeId where
  digest : Digest256
  deriving DecidableEq

structure LineageEdgeId where
  digest : Digest256
  deriving DecidableEq

structure ReasoningBundleId where
  digest : Digest256
  deriving DecidableEq

structure RevisionId where
  digest : Digest256
  deriving DecidableEq

inductive StableIdentity where
  | entity : EntityId -> StableIdentity
  | relation : RelationId -> StableIdentity
  | fact : FactId -> StableIdentity
  | generation : GenerationId -> StableIdentity
  | query : QueryId -> StableIdentity
  | queryOperator : QueryOperatorId -> StableIdentity
  | rule : RuleId -> StableIdentity
  | rulePack : RulePackId -> StableIdentity
  | derivation : DerivationId -> StableIdentity
  | transition : TransitionId -> StableIdentity
  | action : ActionId -> StableIdentity
  | lineageNode : LineageNodeId -> StableIdentity
  | lineageEdge : LineageEdgeId -> StableIdentity
  | reasoningBundle : ReasoningBundleId -> StableIdentity
  | revision : RevisionId -> StableIdentity
  | state : StateId -> StableIdentity
  deriving DecidableEq

structure RelationSchema where
  id : RelationId
  arity : Nat
  deriving DecidableEq

structure Fact where
  id : FactId
  relation : RelationId
  valueCount : Nat
  deriving DecidableEq

structure Atom where
  relation : RelationId
  deriving DecidableEq

structure Rule where
  head : Atom
  body : List Atom
  deriving DecidableEq

structure Transition where
  insertions : List Fact
  deriving DecidableEq

structure Bundle where
  relations : List RelationSchema
  facts : List Fact
  rules : List Rule
  transitions : List Transition
  deriving DecidableEq

inductive ClosureStatus where
  | complete
  | truncated
  deriving DecidableEq

structure ClosureCandidate where
  generation : GenerationId
  deriving DecidableEq

structure CandidateIdentity where
  fact : FactId
  derivation : DerivationId
  deriving DecidableEq

structure DerivationWitness where
  id : DerivationId
  rule : RuleId
  generation : GenerationId
  output : FactId
  premises : List FactId
  deriving DecidableEq

structure DerivationLineageValid (witness : DerivationWitness) : Prop where
  premisesPresent : Not (witness.premises = [])
  noSelfSupport : Not (List.Mem witness.output witness.premises)

structure CounterexampleStep where
  action : ActionId
  fromState : StateId
  toState : StateId
  legal : Bool
  deriving DecidableEq

structure CounterexampleReceipt where
  initialState : StateId
  steps : List CounterexampleStep
  terminalState : StateId
  initialValid : Bool
  terminalViolates : Bool
  deriving DecidableEq

def CounterexampleValid (receipt : CounterexampleReceipt) : Prop :=
  receipt.initialValid = true /\
    receipt.steps.all (fun step => step.legal) = true /\
    receipt.terminalViolates = true

structure ClosureReceipt where
  status : ClosureStatus
  candidates : List ClosureCandidate
  deriving DecidableEq

structure ClosureBindingValid
    (receipt : ClosureReceipt)
    (target : GenerationId)
    (identities : List CandidateIdentity) : Prop where
  complete : receipt.status = ClosureStatus.complete
  cardinality : receipt.candidates.length = identities.length
  factIdsUnique : (identities.map CandidateIdentity.fact).Nodup
  derivationIdsUnique : (identities.map CandidateIdentity.derivation).Nodup
  targetExact :
    forall candidate,
      List.Mem candidate receipt.candidates -> candidate.generation = target

def ClosureMaterializable
    (receipt : ClosureReceipt)
    (target : GenerationId)
    (identities : List CandidateIdentity)
    (lineageAccepted transitionAccepted : Prop) : Prop :=
  ClosureBindingValid receipt target identities /\
    lineageAccepted /\ transitionAccepted

def RelationAdmitted (relations : List RelationSchema) (relation : RelationId) : Prop :=
  Exists fun schema => List.Mem schema relations /\ schema.id = relation

def FactValid (relations : List RelationSchema) (fact : Fact) : Prop :=
  Exists fun schema =>
    List.Mem schema relations /\ schema.id = fact.relation /\ schema.arity = fact.valueCount

def RuleValid (relations : List RelationSchema) (rule : Rule) : Prop :=
  RelationAdmitted relations rule.head.relation /\
    forall atom, List.Mem atom rule.body -> RelationAdmitted relations atom.relation

def insertedFacts (transitions : List Transition) : List Fact :=
  transitions.flatMap Transition.insertions

structure BundleValid (bundle : Bundle) : Prop where
  relationIdsUnique : (bundle.relations.map RelationSchema.id).Nodup
  factIdsUnique : ((bundle.facts ++ insertedFacts bundle.transitions).map Fact.id).Nodup
  baseFactsValid : forall fact, List.Mem fact bundle.facts -> FactValid bundle.relations fact
  insertionFactsValid :
    forall transition,
      List.Mem transition bundle.transitions ->
        forall fact, List.Mem fact transition.insertions -> FactValid bundle.relations fact
  rulesValid : forall rule, List.Mem rule bundle.rules -> RuleValid bundle.relations rule

theorem inserted_fact_has_admitted_schema
    {bundle : Bundle}
    (valid : BundleValid bundle)
    {transition : Transition}
    (transitionMember : List.Mem transition bundle.transitions)
    {fact : Fact}
    (factMember : List.Mem fact transition.insertions) :
    Exists fun schema =>
      List.Mem schema bundle.relations /\
        schema.id = fact.relation /\ schema.arity = fact.valueCount := by
  exact valid.insertionFactsValid transition transitionMember fact factMember

theorem identity_domain_separation
    (fixtureAtom : Nat) :
    Not
        (StableIdentity.fact { digest := Digest256.ofFixtureAtom fixtureAtom } =
          StableIdentity.state { digest := Digest256.ofFixtureAtom fixtureAtom }) /\
      Not
        (StableIdentity.generation { digest := Digest256.ofFixtureAtom fixtureAtom } =
          StableIdentity.revision { digest := Digest256.ofFixtureAtom fixtureAtom }) := by
  constructor <;> intro equality <;> cases equality

theorem admitted_derivation_has_rule_and_premises
    {witness : DerivationWitness}
    (valid : DerivationLineageValid witness) :
    Not (witness.premises = []) /\
      Not (List.Mem witness.output witness.premises) := by
  exact And.intro valid.premisesPresent valid.noSelfSupport

theorem returned_counterexample_is_valid
    {receipt : CounterexampleReceipt}
    (valid : CounterexampleValid receipt) :
    receipt.initialValid = true /\
      (forall step, List.Mem step receipt.steps -> step.legal = true) /\
      receipt.terminalViolates = true := by
  exact And.intro valid.left
    (And.intro (List.all_eq_true.mp valid.right.left) valid.right.right)

theorem admitted_rule_uses_only_admitted_relations
    {bundle : Bundle}
    (valid : BundleValid bundle)
    {rule : Rule}
    (ruleMember : List.Mem rule bundle.rules) :
    RelationAdmitted bundle.relations rule.head.relation /\
      forall atom,
        List.Mem atom rule.body -> RelationAdmitted bundle.relations atom.relation := by
  exact valid.rulesValid rule ruleMember

theorem admitted_fact_ids_are_unique
    {bundle : Bundle}
    (valid : BundleValid bundle) :
    ((bundle.facts ++ insertedFacts bundle.transitions).map Fact.id).Nodup := by
  exact valid.factIdsUnique

theorem admitted_closure_binding_is_exact
    {receipt : ClosureReceipt}
    {target : GenerationId}
    {identities : List CandidateIdentity}
    (valid : ClosureBindingValid receipt target identities) :
    receipt.status = ClosureStatus.complete /\
      receipt.candidates.length = identities.length /\
      (identities.map CandidateIdentity.fact).Nodup /\
      (identities.map CandidateIdentity.derivation).Nodup /\
      forall candidate,
        List.Mem candidate receipt.candidates -> candidate.generation = target := by
  exact And.intro valid.complete
    (And.intro valid.cardinality
      (And.intro valid.factIdsUnique
        (And.intro valid.derivationIdsUnique valid.targetExact)))

theorem closure_admission_rejects_any_failed_owner
    {receipt : ClosureReceipt}
    {target : GenerationId}
    {identities : List CandidateIdentity}
    {lineageAccepted transitionAccepted : Prop}
    (failed :
      (Not (ClosureBindingValid receipt target identities)) \/
        (Not lineageAccepted) \/ (Not transitionAccepted)) :
    Not
      (ClosureMaterializable
        receipt target identities lineageAccepted transitionAccepted) := by
  intro materializable
  exact failed.elim
    (fun bindingFailed => bindingFailed materializable.left)
    (fun remaining => remaining.elim
      (fun lineageFailed => lineageFailed materializable.right.left)
      (fun transitionFailed => transitionFailed materializable.right.right))

end MRRProof
