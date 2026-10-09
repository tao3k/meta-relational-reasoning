import Lean.Data.Json.Parser

namespace MRR.SearchComposition.CandidateChecks

private def result {A : Type} (value : Except String A) : IO A :=
  match value with
  | .ok value => pure value
  | .error error => throw (IO.userError error)
private def field (object : Lean.Json) (name : String) : IO Lean.Json := result (object.getObjVal? name)
private def text (object : Lean.Json) (name : String) : IO String := do result (← field object name).getStr?
private def rows (object : Lean.Json) (name : String) : IO (List Lean.Json) := do
  return (← result (← field object name).getArr?).toList
private def strings (object : Lean.Json) (name : String) : IO (List String) := do
  (← rows object name).mapM (fun value => result value.getStr?)
private def require (name : String) (passed : Bool) : IO Unit :=
  unless passed do throw (IO.userError s!"SEARCH-CANDIDATE-FAILED: {name}")
private def unique {A : Type} [BEq A] (values : List A) : Bool :=
  values.eraseDups.length == values.length
private def sameSet {A : Type} [BEq A] (left right : List A) : Bool :=
  left.length == right.length && unique left && unique right && left.all right.contains

private def verify (execution entry : Lean.Json) : IO Unit := do
  require "retained candidate schema" ((← text entry "schemaId") == "agent.semantic-protocols.search-candidate-witness" &&
    (← text entry "schemaVersion") == "1" && (← result (← field entry "complete").getBool?))
  let receipt ← field entry "compositionReceipt"
  require "exact execution receipt binding" (receipt == (← field execution "compositionReceipt") &&
    (← result (← field execution "complete").getBool?))
  let inventory ← (← rows entry "ownerCandidates").mapM fun row => do
    let pair ← result row.getArr?
    require "owner candidate pair width" (pair.size == 2)
    return (← result pair[0]!.getStr?, ← result pair[1]!.getStr?)
  require "unique owner candidate inventory" (unique (inventory.map (·.1)) && unique (inventory.map (·.2)))
  let witness ← field execution "witness"
  let mut owners : List String := []
  let mut leafFactors : List String := []
  let mut expected : List (String × String) := []
  for branch in (← rows witness "branches") do
    let factor ← text branch "factor"
    leafFactors := leafFactors ++ [factor]
    for owner in (← strings branch "owners") do
      owners := owners ++ [owner]
      let some (_, candidate) := inventory.find? (fun row => row.1 == owner)
        | throw (IO.userError "SEARCH-CANDIDATE-FAILED: missing branch owner")
      expected := expected ++ [(factor, candidate)]
  require "exact physical owner union" (sameSet (inventory.map (·.1)) owners.eraseDups)
  let factors ← strings receipt "pooFactorIds"
  let merge := factors.filter (fun factor => !leafFactors.contains factor)
  require "single merge factor" (merge.length == 1)
  for owner in (← strings receipt "mergedOwnerIds") do
    let some (_, candidate) := inventory.find? (fun row => row.1 == owner)
      | throw (IO.userError "SEARCH-CANDIDATE-FAILED: missing merged owner")
    expected := expected ++ [(merge[0]?.getD "", candidate)]
  let actual ← (← rows witness "observations").mapM fun row => do
    return (← text row "factor", ← text row "candidate")
  require "exact owner candidate observation correspondence" (sameSet expected actual)
  IO.println s!"SEARCH-CANDIDATE-LEAN-OK: {← text receipt "mode"} owners={inventory.length} observations={actual.length}"

def main (args : List String) : IO Unit := do
  let [executionPath, candidatePath] := args
    | throw (IO.userError "execution and candidate witness arrays required")
  let executions := (← result (← result (Lean.Json.parse (← IO.FS.readFile executionPath))).getArr?).toList
  let candidates := (← result (← result (Lean.Json.parse (← IO.FS.readFile candidatePath))).getArr?).toList
  let identities ← executions.mapM fun row => do text (← field row "compositionReceipt") "compositionIdentity"
  let candidateIds ← candidates.mapM fun row => do text (← field row "compositionReceipt") "compositionIdentity"
  require "exact composition inventory" (!candidates.isEmpty && sameSet identities candidateIds)
  for entry in candidates do
    let identity ← text (← field entry "compositionReceipt") "compositionIdentity"
    for execution in executions do
      if (← text (← field execution "compositionReceipt") "compositionIdentity") == identity then
        verify execution entry

end MRR.SearchComposition.CandidateChecks

def main := MRR.SearchComposition.CandidateChecks.main
