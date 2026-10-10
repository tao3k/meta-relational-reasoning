import ExecutionModel
import Lean.Data.Json.Parser

namespace MRR.SearchComposition.ExecutionChecks

private def result {A : Type} (value : Except String A) : IO A :=
  match value with
  | .ok value => pure value
  | .error error => throw (IO.userError error)

private def field (object : Lean.Json) (name : String) : IO Lean.Json := result (object.getObjVal? name)
private def text (object : Lean.Json) (name : String) : IO String := do result (<- field object name).getStr?
private def flag (object : Lean.Json) (name : String) : IO Bool := do result (<- field object name).getBool?
private def number (object : Lean.Json) (name : String) : IO Nat := do result (<- field object name).getNat?
private def rows (object : Lean.Json) (name : String) : IO (List Lean.Json) := do
  return (<- result (<- field object name).getArr?).toList
private def strings (object : Lean.Json) (name : String) : IO (List String) := do
  (<- rows object name).mapM (fun value => result value.getStr?)
private def require (name : String) (passed : Bool) : IO Unit :=
  if passed then pure () else throw (IO.userError s!"SEARCH-EXECUTION-FAILED: {name}")

private def unique {A : Type} [BEq A] (values : List A) : Bool :=
  values.eraseDups.length == values.length
private def sameSet {A : Type} [BEq A] (left right : List A) : Bool :=
  left.length == right.length && unique left && unique right && left.all right.contains

private def distances (edges : List (Prod String String)) (target : String) :
    Nat -> List String -> List String -> Nat -> Option Nat
  | 0, _, _, _ => none
  | fuel + 1, frontier, visited, depth =>
    if frontier.contains target then some depth else
      let next := (edges.filterMap (fun edge =>
        if frontier.contains edge.1 && !visited.contains edge.2 then some edge.2 else none)).eraseDups
      distances edges target fuel next (visited ++ next) (depth + 1)

private def pathEdges (edges : List (Prod String String)) : List String -> Bool
  | [] | [_] => true
  | first :: second :: rest => edges.contains (first, second) && pathEdges edges (second :: rest)

structure Observed where
  id : String
  candidate : String
  factor : String
  generation : String
  position : Nat
  parents : List String

private def verify (entry : Lean.Json) : IO Unit := do
  require "witness schema" ((<- text entry "schemaId") == "agent.semantic-protocols.search-execution-witness" &&
    (<- text entry "schemaVersion") == "1" && (<- flag entry "complete"))
  let receipt <- field entry "compositionReceipt"
  require "composition receipt schema" ((<- text receipt "schemaId") == "agent.semantic-protocols.search-data-composition-receipt" &&
    (<- text receipt "schemaVersion") == "1" && !(<- flag receipt "mergedOwnerIdsTruncated"))
  let modeName <- text receipt "mode"
  let mode <- match modeName with
    | "single" => pure Mode.single
    | "rankJoin" => pure Mode.rankJoin
    | "intersect" => pure Mode.intersect
    | _ => throw (IO.userError "unknown composition mode")
  let witness <- field entry "witness"
  let branches <- rows witness "branches"
  require "one- or two-branch qualification scope" (branches.length == (<- number receipt "branchCount") &&
    (if mode == .single then branches.length == 1 else branches.length == 2))
  let generation <- text receipt "generationIdentity"
  let factors <- strings receipt "pooFactorIds"
  require "factor inventory" (unique factors && factors.length == branches.length + 1)
  let mut sets : List (List String) := []
  let mut leafFactors : List String := []
  for branch in branches do
    require "branch binding" ((<- text branch "generation") == generation &&
      (<- text branch "scope") == (<- text receipt "scope") &&
      (<- text branch "source") == (<- text receipt "sourceDigest") &&
      (<- text branch "resident") == (<- text receipt "residentViewDigest") &&
      (<- text branch "abi") == (<- text receipt "compositionAbiDigest"))
    let owners <- strings branch "owners"
    require "branch candidate uniqueness" (unique owners)
    if mode == .intersect || (mode == .rankJoin && sets.isEmpty) then
      require "branch truth completeness" ((<- flag branch "complete") && !(<- flag branch "truncated"))
    let factor <- text branch "factor"
    require "branch factor admission" (factors.contains factor)
    leafFactors := leafFactors ++ [factor]
    sets := sets ++ [owners]
  require "distinct branch factors" (unique leafFactors)
  let merged <- strings receipt "mergedOwnerIds"
  require "Data candidate composition matches Lean semantics"
    (sameSet merged (composeOwners mode (sets[0]?.getD []) (sets[1]?.getD [])) &&
      merged.length == (<- number receipt "mergedOwnerCount"))
  let edges <- (<- rows receipt "pooEdges").mapM fun value => do
    let pair <- result value.getArr?
    require "edge width" (pair.size == 2)
    return (<- result pair[0]!.getStr?, <- result pair[1]!.getStr?)
  let mergeFactors := factors.filter (fun factor => !leafFactors.contains factor)
  require "original POO fan-in graph" (mergeFactors.length == 1 &&
    sameSet edges (leafFactors.map (fun factor => (factor, mergeFactors[0]?.getD ""))))
  let mut actualPaths : List (Prod String (Prod String Nat)) := []
  for value in (<- rows witness "paths") do
    let row <- result value.getArr?
    require "path width" (row.size == 3)
    let source <- result row[0]!.getStr?
    let target <- result row[1]!.getStr?
    let distance <- result row[2]!.getNat?
    require "POO path agrees with independent graph traversal"
      (factors.contains source && factors.contains target &&
        distances edges target (factors.length + 1) [source] [source] 0 == some distance)
    if source != target then actualPaths := actualPaths ++ [(source, target, distance)]
  let mut expectedPaths : List (Prod String (Prod String Nat)) := []
  for source in factors do
    for target in factors do
      if source != target then
        if let some distance := distances edges target (factors.length + 1) [source] [source] 0 then
          expectedPaths := expectedPaths ++ [(source, target, distance)]
  require "complete POO path inventory" (sameSet actualPaths expectedPaths)
  let observations <- (<- rows witness "observations").mapM fun value => do
    return ({
      id := <- text value "id", candidate := <- text value "candidate",
      factor := <- text value "factor", generation := <- text value "generation",
      position := <- number value "position", parents := <- strings value "parents"} : Observed)
  require "observation inventory" (unique (observations.map (fun item => item.id)) &&
    observations.length == (<- number receipt "observationCount"))
  for observation in observations do
    require "MRR observation binding" (observation.generation == generation && factors.contains observation.factor)
    require "causal parent uniqueness" (unique observation.parents)
    if observation.parents.isEmpty then require "acquisition root" (leafFactors.contains observation.factor)
    for parentId in observation.parents do
      let some parent := observations.find? (fun row => row.id == parentId)
        | throw (IO.userError "missing causal parent")
      require "causal observation order" (parent.position <= observation.position &&
        (parent.factor == observation.factor || edges.contains (parent.factor, observation.factor)))
  let mut actualInfluences : List (Prod String (Prod String String)) := []
  for value in (<- rows witness "influences") do
    let candidate <- text value "candidate"
    let target <- text value "factor"
    let support <- text value "support"
    let path <- strings value "path"
    let some observation := observations.find? (fun row => row.id == support)
      | throw (IO.userError "unknown influence support")
    require "MRR influence source and causal path" (candidate == observation.candidate &&
      path.head? == some observation.factor && path.getLast? == some target && unique path && pathEdges edges path &&
      distances edges target (factors.length + 1) [observation.factor] [observation.factor] 0 == some (path.length - 1))
    actualInfluences := actualInfluences ++ [(candidate, target, support)]
  let mut expectedInfluences : List (Prod String (Prod String String)) := []
  for observation in observations do
    for target in factors do
      if (distances edges target (factors.length + 1) [observation.factor] [observation.factor] 0).isSome then
        expectedInfluences := expectedInfluences ++ [(observation.candidate, target, observation.id)]
  require "complete MRR influence inventory" (sameSet actualInfluences expectedInfluences)
  IO.println s!"SEARCH-EXECUTION-LEAN-OK: {modeName} owners={merged.length} observations={observations.length} influences={actualInfluences.length}"

def main (args : List String) : IO Unit := do
  let [path] := args | throw (IO.userError "one actual ASP witness array path required")
  let entries <- result (Lean.Json.parse (<- IO.FS.readFile path))
  let entries <- result entries.getArr?
  require "nonempty execution witness inventory" (!entries.isEmpty)
  for entry in entries do verify entry

end MRR.SearchComposition.ExecutionChecks

def main := MRR.SearchComposition.ExecutionChecks.main
