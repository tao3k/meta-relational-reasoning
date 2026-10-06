
/-! Exact affine size/cost composition and state-indexed authorized effect traces. -/
namespace MRRTransformation

structure AffineBound where
  slope : Nat
  offset : Nat
deriving Repr, DecidableEq

def AffineBound.apply (f : AffineBound) (n : Nat) : Nat := f.slope * n + f.offset

def AffineBound.compose (f g : AffineBound) : AffineBound :=
  { slope := g.slope * f.slope, offset := g.slope * f.offset + g.offset }

theorem affine_compose_apply (f g : AffineBound) (n : Nat) :
    (f.compose g).apply n = g.apply (f.apply n) := by
  simp [AffineBound.compose, AffineBound.apply, Nat.mul_add, Nat.mul_assoc, Nat.add_assoc]

theorem affine_monotone (f : AffineBound) {a b : Nat} (h : a <= b) : f.apply a <= f.apply b :=
  Nat.add_le_add_right (Nat.mul_le_mul_left f.slope h) f.offset

structure ResourceContract where
  size : AffineBound
  cost : AffineBound
deriving Repr, DecidableEq

/-- The second cost is evaluated at the intermediate size, then added to the first. -/
def ResourceContract.compose (f g : ResourceContract) : ResourceContract :=
  { size := f.size.compose g.size
    cost := { slope := f.cost.slope + g.cost.slope * f.size.slope,
      offset := f.cost.offset + g.cost.slope * f.size.offset + g.cost.offset } }

theorem resource_compose_cost (f g : ResourceContract) (n : Nat) :
    (f.compose g).cost.apply n = f.cost.apply n + g.cost.apply (f.size.apply n) := by
  simp [ResourceContract.compose, AffineBound.apply, Nat.add_mul, Nat.mul_add,
    Nat.mul_assoc, Nat.add_assoc, Nat.add_comm, Nat.add_left_comm]

theorem resource_compose_assoc (f g h : ResourceContract) :
    (f.compose g).compose h = f.compose (g.compose h) := by
  cases f; cases g; cases h
  simp [ResourceContract.compose, AffineBound.compose, Nat.mul_add, Nat.add_mul,
    Nat.mul_assoc, Nat.add_assoc, Nat.add_comm, Nat.add_left_comm]

universe u v

/-- A bound describes actual measured input/output and execution cost. -/
structure MeasuredProblem extends Problem.{u, v} where
  size : Input -> Nat

structure CertifiedResourceTransformation (A B : MeasuredProblem.{u, v}) where
  transport : CertifiedTransformation A.toProblem B.toProblem
  cost : A.Input -> Nat
  budget : ResourceContract
  output_bound : forall a, A.Valid a -> B.size (transport.forward a) <= budget.size.apply (A.size a)
  cost_bound : forall a, A.Valid a -> cost a <= budget.cost.apply (A.size a)

def CertifiedResourceTransformation.compose {A B C : MeasuredProblem}
    (f : CertifiedResourceTransformation A B) (g : CertifiedResourceTransformation B C) :
    CertifiedResourceTransformation A C where
  transport := MRRTransformation.compose f.transport g.transport
  cost := fun a => f.cost a + g.cost (f.transport.forward a)
  budget := f.budget.compose g.budget
  output_bound := fun a valid => by
    change C.size (g.transport.forward (f.transport.forward a)) <=
      (f.budget.size.compose g.budget.size).apply (A.size a)
    rw [affine_compose_apply]
    exact Nat.le_trans (g.output_bound _ (f.transport.preserves a valid))
      (affine_monotone g.budget.size (f.output_bound a valid))
  cost_bound := fun a valid => by
    rw [resource_compose_cost]
    exact Nat.add_le_add (f.cost_bound a valid)
      (Nat.le_trans (g.cost_bound _ (f.transport.preserves a valid))
        (affine_monotone g.budget.cost (f.output_bound a valid)))

theorem measured_composition_bound {A B C : MeasuredProblem}
    (f : CertifiedResourceTransformation A B) (g : CertifiedResourceTransformation B C)
    (a : A.Input) (valid : A.Valid a) :
    (f.compose g).cost a <= (f.budget.compose g.budget).cost.apply (A.size a) :=
  (f.compose g).cost_bound a valid


structure EffectSystem where
  State : Type u
  Effect : Type v
  Authorized : State -> Effect -> Prop
  Transition : State -> Effect -> State -> Prop

/-- Each effect is authorized at the state in which it actually occurs. -/
inductive AuthorizedTrace (S : EffectSystem) : S.State -> List S.Effect -> S.State -> Prop
  | nil (state) : AuthorizedTrace S state [] state
  | step {before middle after effect tail} :
      S.Authorized before effect -> S.Transition before effect middle ->
      AuthorizedTrace S middle tail after -> AuthorizedTrace S before (effect :: tail) after

theorem authorized_trace_append {S : EffectSystem} {a b c : S.State}
    {first second : List S.Effect} (f : AuthorizedTrace S a first b)
    (g : AuthorizedTrace S b second c) : AuthorizedTrace S a (first ++ second) c := by
  induction f with
  | nil => exact g
  | step authorized transition rest ih =>
      exact .step authorized transition (ih g)

/-- Runtime witnesses need both correct output and an authorized actual trace. -/
structure CertifiedEffectExecution (S : EffectSystem) (A : Problem) (input : A.Input)
    (before : S.State) where
  answer : A.Result input
  after : S.State
  trace : List S.Effect
  correct : A.Correct input answer
  authorized : AuthorizedTrace S before trace after

/-- An actual state transition carries the authorization proof for every event. -/
structure EffectResult (S : EffectSystem.{u, v}) (R : Type) (before : S.State) where
  value : R
  after : S.State
  trace : List S.Effect
  authorized : AuthorizedTrace S before trace after

def EffectResult.bind {S : EffectSystem} {A B : Type} {before : S.State}
    (first : EffectResult S A before)
    (next : (value : A) -> (state : S.State) -> EffectResult S B state) :
    EffectResult S B before :=
  let second := next first.value first.after
  { value := second.value, after := second.after, trace := first.trace ++ second.trace,
    authorized := authorized_trace_append first.authorized second.authorized }

/-- Source input and actual forward result are bound before the effectful extractor runs. -/
structure EffectTransportAt (S : EffectSystem) (A B : Problem.{0, 0})
    (a : A.Input) (before : S.State) where
  forward : EffectResult S B.Input before
  preserves : A.Valid a -> B.Valid forward.value
  extract : (state : S.State) -> B.Result forward.value -> EffectResult S (A.Result a) state
  sound : forall state r, A.Valid a -> B.Correct forward.value r ->
    A.Correct a (extract state r).value

structure CertifiedEffectTransformation (S : EffectSystem) (A B : Problem.{0, 0}) where
  runAt : (a : A.Input) -> (before : S.State) -> EffectTransportAt S A B a before

def CertifiedEffectTransformation.compose {S : EffectSystem} {A B C : Problem.{0, 0}}
    (f : CertifiedEffectTransformation S A B) (g : CertifiedEffectTransformation S B C) :
    CertifiedEffectTransformation S A C where
  runAt := fun a before =>
    let first := f.runAt a before
    let second := g.runAt first.forward.value first.forward.after
    { forward := { value := second.forward.value, after := second.forward.after,
        trace := first.forward.trace ++ second.forward.trace,
        authorized := authorized_trace_append first.forward.authorized second.forward.authorized }
      preserves := fun valid => second.preserves (first.preserves valid)
      extract := fun state result =>
        (second.extract state result).bind fun intermediate after => first.extract after intermediate
      sound := fun state result valid correct =>
        first.sound (second.extract state result).after _ valid
          (second.sound state result (first.preserves valid) correct) }

theorem effectful_composition_sound {S : EffectSystem} {A B C : Problem.{0, 0}}
    (f : CertifiedEffectTransformation S A B) (g : CertifiedEffectTransformation S B C)
    (a : A.Input) (before state : S.State) (valid : A.Valid a)
    (r : C.Result ((f.compose g).runAt a before).forward.value)
    (correct : C.Correct ((f.compose g).runAt a before).forward.value r) :
    A.Correct a (((f.compose g).runAt a before).extract state r).value :=
  ((f.compose g).runAt a before).sound state r valid correct

end MRRTransformation

#print axioms MRRTransformation.affine_compose_apply
#print axioms MRRTransformation.affine_monotone
#print axioms MRRTransformation.resource_compose_cost
#print axioms MRRTransformation.resource_compose_assoc
#print axioms MRRTransformation.authorized_trace_append

#print axioms MRRTransformation.measured_composition_bound
#print axioms MRRTransformation.effectful_composition_sound
