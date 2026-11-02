import MRR.AgenticAIContext.Tokenization

namespace MRR.AgenticAIContext

/-- Compare actual token IDs after the complete rendering/tokenization pipeline. -/
def stableTokenPrefix : TokenSequence -> TokenSequence -> TokenSequence
  | x :: xs, y :: ys => if x = y then x :: stableTokenPrefix xs ys else []
  | _, _ => []

theorem stableTokenPrefix_prefixLeft (oldTokens newTokens : TokenSequence) :
    List.IsPrefix (stableTokenPrefix oldTokens newTokens) oldTokens := by
  induction oldTokens generalizing newTokens with
  | nil => simp [stableTokenPrefix]
  | cons x xs ih =>
    cases newTokens with
    | nil => simp [stableTokenPrefix]
    | cons y ys =>
      simp only [stableTokenPrefix]
      split
      case _ =>
        exact List.cons_prefix_cons.mpr (And.intro rfl (ih ys))
      case _ =>
        exact List.nil_prefix

theorem stableTokenPrefix_prefixRight (oldTokens newTokens : TokenSequence) :
    List.IsPrefix (stableTokenPrefix oldTokens newTokens) newTokens := by
  induction oldTokens generalizing newTokens with
  | nil => simp [stableTokenPrefix]
  | cons x xs ih =>
    cases newTokens with
    | nil => simp [stableTokenPrefix]
    | cons y ys =>
      simp only [stableTokenPrefix]
      split
      case _ =>
        rename_i equal
        exact List.cons_prefix_cons.mpr (And.intro equal (ih ys))
      case _ =>
        exact List.nil_prefix

theorem commonPrefix_le_stableTokenPrefix (initial oldTokens newTokens : TokenSequence)
    (left : List.IsPrefix initial oldTokens) (right : List.IsPrefix initial newTokens) :
    initial.length <= (stableTokenPrefix oldTokens newTokens).length := by
  induction initial generalizing oldTokens newTokens with
  | nil => simp
  | cons x xs ih =>
    cases oldTokens with
    | nil => simp at left
    | cons a as =>
      cases newTokens with
      | nil => simp at right
      | cons b bs =>
        have partsOld := List.cons_prefix_cons.mp left
        have equalOld := partsOld.1
        have tailOld := partsOld.2
        have partsNew := List.cons_prefix_cons.mp right
        have equalNew := partsNew.1
        have tailNew := partsNew.2
        subst a
        subst b
        simpa [stableTokenPrefix] using Nat.succ_le_succ (ih as bs tailOld tailNew)

/-- Maximum complete-block token count eligible from the common prefix.
Zero block size is deliberately rejected rather than silently accepted. -/
def eligibleFullBlockTokens (blockSize : Nat) (oldTokens newTokens : TokenSequence) : Option Nat :=
  if blockSize = 0 then none
  else some ((stableTokenPrefix oldTokens newTokens).length / blockSize * blockSize)

theorem eligibleFullBlockTokens_le (blockSize : Nat) (oldTokens newTokens : TokenSequence)
    (count : Nat) (accepted : eligibleFullBlockTokens blockSize oldTokens newTokens = some count) :
    count <= (stableTokenPrefix oldTokens newTokens).length := by
  unfold eligibleFullBlockTokens at accepted
  split at accepted
  case _ =>
    contradiction
  case _ =>
    cases Option.some.inj accepted
    exact Nat.div_mul_le_self _ _

end MRR.AgenticAIContext
