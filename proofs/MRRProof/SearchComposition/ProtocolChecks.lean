import Graph

open MRR.SearchComposition

def search (mode : Mode) : Search Nat :=
  Search.mk mode (fun candidate => candidate = 0 \/ candidate = 1) (fun candidate => candidate = 1)

instance (mode : Mode) (candidate : Nat) : Decidable ((search mode).truth candidate) := by
  unfold search Search.truth
  cases mode <;> dsimp [intersection] <;> infer_instance

def binding : Binding := Binding.mk "workspace" "source" "resident" "abi" "generation"

def main : IO Unit := do
  if !(decide ((search .single).truth 0)) then
    throw (IO.userError "Single lost the primary candidate")
  if !(decide ((search .rankJoin).truth 0)) then
    throw (IO.userError "RankJoin filtered the primary candidate")
  if decide ((search .intersect).truth 0) then
    throw (IO.userError "Intersect admitted a candidate outside its secondary set")
  if !(decide ((search .intersect).truth 1)) then
    throw (IO.userError "Intersect omitted a candidate in both complete sets")
  let partialResult := readStep (initial (Candidate := Nat) binding) false true false true
  if !(decide (truthReady (search .single) partialResult)) then
    throw (IO.userError "Single must preserve its partial result status")
  let ranked := readStep (initial (Candidate := Nat) binding) true false false true
  if !(decide (truthReady (search .rankJoin) ranked)) then
    throw (IO.userError "RankJoin rejected an auxiliary partial ranking")
  if decide (truthReady (search .intersect) ranked) then
    throw (IO.userError "Intersect accepted an incomplete secondary truth branch")
  let ready := mergeStep (search .intersect)
    (readStep (initial binding) true false true false)
  let revised := reviseStep (publishStep ready) {binding with generation := "next"}
  if revised.phase == .published || revised.inferred then
    throw (IO.userError "Revision retained published inference")
  if revised.observed == revised.current then
    throw (IO.userError "Revision silently rebound old observations")
  IO.println "SEARCH-PROTOCOL-CHECKS-OK: three modes, partial controls, publication retirement"
