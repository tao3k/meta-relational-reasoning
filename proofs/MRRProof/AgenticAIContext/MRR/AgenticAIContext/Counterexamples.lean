import MRR.AgenticAIContext.Reuse

namespace MRR.AgenticAIContext.Counterexamples

def oldGraph : LeanPoo.C4.Graph := { nodes := [
  { name := "root" },
  { name := "a", parentOrders := [["root"]] }
] }

def leafGraph : LeanPoo.C4.Graph := { nodes := oldGraph.nodes ++ [
  { name := "child", parentOrders := [["a"]] }
] }

def recomposedGraph : LeanPoo.C4.Graph := { nodes := oldGraph.nodes ++ [
  { name := "b", parentOrders := [["root"]] },
  { name := "child", parentOrders := [["a", "b"]] }
] }

def render : SegmentRenderer := fun name =>
  if name = "root" then [1] else if name = "a" then [2]
  else if name = "b" then [3] else [4]

/-- The order-level statement is kernel checked. Actual C4 fixture outputs are
separately executed in Checks.Main; they are not proved with native_decide. -/
theorem leaf_materialization_appends :
    And (materializeOrder render ["a", "root"] = [1, 2])
      (materializeOrder render ["child", "a", "root"] = [1, 2, 4]) := by
  decide

/-- Existing a/root relative precedence is preserved, but new b interrupts the
physical prefix. C4 monotonicity therefore does not imply a byte prefix. -/
theorem recomposition_breaks_prefix :
    And (materializeOrder render ["child", "a", "b", "root"] = [1, 3, 2, 4])
      (Not (List.IsPrefix ([1, 2] : Bytes) [1, 3, 2, 4])) := by
  decide

/-- Deterministic toy merge encoding: extending a byte string can retokenize
its end. This is a countermodel, not a claim about any particular tokenizer. -/
def mergeTokenizer : Tokenizer := fun bytes =>
  if bytes = [97, 98] then [256] else byteTokens bytes

theorem bytePrefix_doesNotImply_tokenPrefix :
    And (List.IsPrefix ([97] : Bytes) [97, 98])
      (Not (List.IsPrefix (mergeTokenizer [97]) (mergeTokenizer [97, 98]))) := by
  decide

/-- A changing document count also breaks prefix stability despite appending
the body. Global metadata must be outside the certified immutable prefix. -/
theorem changingHeader_breaks_prefix :
    Not (List.IsPrefix ([2, 1, 2] : Bytes) [3, 1, 2, 4]) := by
  decide

/-- Causal dependency witness: a position's cached state depends on its whole
token prefix. This abstracts dependence only; it is not a Transformer model. -/
def causalState (tokens : TokenSequence) (position : Nat) : TokenSequence :=
  tokens.take (position + 1)

theorem unchangedSuffix_doesNotImply_causalStateReuse :
    And (([1, 2, 3] : TokenSequence).drop 2 = [1, 9, 3].drop 2)
      (And (Not (causalState [1, 2, 3] 2 = causalState [1, 9, 3] 2))
        (stableTokenPrefix [1, 2, 3] [1, 9, 3] = [1])) := by
  decide

theorem fullBlocks_roundDown :
    And (eligibleFullBlockTokens 2 [1, 2, 3, 4] [1, 2, 3, 9] = some 2)
      (eligibleFullBlockTokens 0 [1, 2] [1, 2] = none) := by
  decide

end MRR.AgenticAIContext.Counterexamples
