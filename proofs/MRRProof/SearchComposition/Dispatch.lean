import Lean
import SearchAttempt
import SearchEvidenceHistory

namespace MRR.SearchDispatch
structure State where
  retired : Bool
  active : Nat
  input : Nat
  output : Nat
  deriving DecidableEq, Repr

def Bounded (cap : Nat) (s : State) : Prop :=
  s.input <= cap /\ s.output + s.active <= cap

-- Names match the Rust lease operations; each active lease reserves one output.
inductive Step (cap : Nat) : State -> State -> Prop where
  | reserve (s : State) (live : s.retired = false)
      (inputRoom : s.input + 1 <= cap) (outputRoom : s.output + s.active + 1 <= cap) :
      Step cap s { s with active := s.active + 1, input := s.input + 1 }
  | admit (s : State) (live : s.retired = false) (running : 0 < s.active) :
      Step cap s { s with active := s.active - 1, output := s.output + 1 }
  | drop (s : State) (running : 0 < s.active) :
      Step cap s { s with active := s.active - 1 }
  | retire (s : State) : Step cap s { s with retired := true }

theorem step_preserves_bound {cap : Nat} {a b : State}
    (valid : Bounded cap a) (step : Step cap a b) : Bounded cap b := by
  cases step <;> simp_all [Bounded] <;> omega

theorem retirement_sticky {cap : Nat} {a b : State}
    (retired : a.retired = true) (step : Step cap a b) : b.retired = true := by
  cases step <;> simp_all

theorem retired_no_new_output {cap : Nat} {a b : State}
    (retired : a.retired = true) (step : Step cap a b) : b.output = a.output := by
  cases step <;> simp_all

-- Rust publication is permitted only after the producer accepts the observation.
def publishObservation (state : State) (producerAccepted : Bool) : Option State :=
  if producerAccepted = true ∧ state.retired = false ∧ 0 < state.active then
    some {state with active := state.active - 1, output := state.output + 1}
  else none

theorem rejected_observation_no_publication (state : State) :
    publishObservation state false = none := by
  simp [publishObservation]

theorem historical_observation_rejected
    {node : LeanPoo.C4.Node} {state : POO.Flow.SearchAttempt.State}
    {request : POO.Flow.SearchAttempt.Request}
    {observations : String → Option POO.Flow.SearchEvidence.Event}
    {history : List Nat} {event : POO.Flow.SearchEvidence.Event}
    (used : event.identity ∈ history) :
    ¬ POO.Flow.SearchEvidence.AdmitsEvidence node state request observations history event :=
  POO.Flow.SearchEvidence.historical_identity_rejects used

-- Scheme transport admits cancellation only for the currently active identity.
-- Its retained owner then delegates to the producer revision transition.
abbrev cancelAttempt := POO.Flow.SearchAttempt.cancel

theorem stale_cancel_noop (state : POO.Flow.SearchAttempt.State)
    (request : POO.Flow.SearchAttempt.Request)
    (stale : ¬ POO.Flow.SearchAttempt.Admits state request) :
    cancelAttempt state request = some state := by
  exact POO.Flow.SearchAttempt.stale_cancel_noop state request stale

theorem cancelled_attempt_cannot_settle
    {state next : POO.Flow.SearchAttempt.State} {request : POO.Flow.SearchAttempt.Request}
    (current : POO.Flow.SearchAttempt.Admits state request)
    (cancelled : cancelAttempt state request = some next) :
    POO.Flow.SearchAttempt.settle next request = none := by
  exact POO.Flow.SearchAttempt.cancelled_attempt_cannot_settle current cancelled

end MRR.SearchDispatch
#print axioms MRR.SearchDispatch.step_preserves_bound
#print axioms MRR.SearchDispatch.retirement_sticky
#print axioms MRR.SearchDispatch.retired_no_new_output
