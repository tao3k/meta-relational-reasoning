import Lean
import SearchAttempt

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

-- Scheme transport admits cancellation only for the currently active identity.
-- Its retained owner then delegates to the producer revision transition.
def cancelAttempt (state : POO.Flow.SearchAttempt.State)
    (request : POO.Flow.SearchAttempt.Request) : Option POO.Flow.SearchAttempt.State :=
  if POO.Flow.SearchAttempt.Admits state request then
    POO.Flow.SearchAttempt.revise state state.scope.sourceCut
  else some state

theorem stale_cancel_noop (state : POO.Flow.SearchAttempt.State)
    (request : POO.Flow.SearchAttempt.Request)
    (stale : ¬ POO.Flow.SearchAttempt.Admits state request) :
    cancelAttempt state request = some state := by
  simp [cancelAttempt, stale]

theorem cancelled_attempt_cannot_settle
    {state next : POO.Flow.SearchAttempt.State} {request : POO.Flow.SearchAttempt.Request}
    (current : POO.Flow.SearchAttempt.Admits state request)
    (cancelled : cancelAttempt state request = some next) :
    POO.Flow.SearchAttempt.settle next request = none := by
  have revised : POO.Flow.SearchAttempt.revise state state.scope.sourceCut = some next := by
    unfold cancelAttempt at cancelled
    split at cancelled
    · exact cancelled
    · contradiction
  exact POO.Flow.SearchAttempt.revision_fences_old_attempt revised

end MRR.SearchDispatch
#print axioms MRR.SearchDispatch.step_preserves_bound
#print axioms MRR.SearchDispatch.retirement_sticky
#print axioms MRR.SearchDispatch.retired_no_new_output
