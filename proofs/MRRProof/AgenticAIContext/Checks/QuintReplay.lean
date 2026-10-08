import MRR
import Lean.Data.Json.Parser

open MRR.AgenticAIContext

private def result {T : Type} (value : Except String T) : IO T :=
  match value with
  | .ok value => pure value
  | .error error => throw (IO.userError error)

private def numbers (entry : Lean.Json) (key : String) : IO (List Nat) := do
  let items <- result (<- result (entry.getObjVal? key)).getArr?
  items.toList.mapM (fun value => result value.getNat?)

private def edgeList (entry : Lean.Json) (key : String) : IO (List (Prod Nat Nat)) := do
  let items <- result (<- result (entry.getObjVal? key)).getArr?
  items.toList.mapM fun pair => do
    let values <- result pair.getArr?
    if values.size != 2 then throw (IO.userError "invalid Quint edge")
    return (<- result values[0]!.getNat?, <- result values[1]!.getNat?)

private def canonicalSet (items : List Nat) : List Nat :=
  items.eraseDups.mergeSort (fun left right => left <= right)

private def check (name : String) (condition : Bool) : IO Unit := do
  if !condition then throw (IO.userError s!"FAIL: {name}")

def main (args : List String) : IO UInt32 := do
  let path <- match args with
    | [path] => pure path
    | _ => throw (IO.userError "expected covered Quint state JSON")
  let document <- result (Lean.Json.parse (<- IO.FS.readFile path))
  let states <- result document.getArr?
  if states.isEmpty then throw (IO.userError "empty Quint state projection")
  for index in [:states.size] do
    let entry := states[index]!
    let source <- numbers entry "source"
    let changed <- numbers entry "changed"
    let invalidated <- numbers entry "invalidated"
    let reusable <- numbers entry "reusable"
    let todo <- numbers entry "todo"
    let oldSelected <- numbers entry "oldSelected"
    let newSelected <- numbers entry "newSelected"
    let unequal <- numbers entry "unequal"
    let oldEdges <- edgeList entry "oldEdges"
    let newEdges <- edgeList entry "newEdges"
    let global <- result (<- result (entry.getObjVal? "global")).getBool?
    let phase <- result (<- result (entry.getObjVal? "phase")).getStr?
    let expected <- result (<- result (entry.getObjVal? "expected")).getStr?
    let deps := fun id => ((oldEdges ++ newEdges).filter (fun edge => edge.1 == id)).map Prod.snd
    let oldValue := fun (_ : Nat) => 0
    let newValue := fun id => if unequal.contains id then 1 else 0
    let exactChanges := if global then source else unequal
    let expectedChanges := if phase == "classify" then exactChanges.filter (fun id => !todo.contains id)
      else exactChanges
    let reverse := reverseDependencies source deps
    let impact := expandRequired exactChanges reverse source.length
    let complete := checkRequiredClosure source changed reverse source.length source.length invalidated
    if expected == "reject" then
      check "Quint counterexample rejected by Lean publication guard" (!complete)
    else
      check "Quint change classification equals Lean" (canonicalSet changed == canonicalSet expectedChanges)
      check "Quint impact sound in Lean" (invalidated.all (fun id => impact.contains id))
      check "Quint graph admission in Lean"
        (decide source.Nodup && source.all (fun id => (deps id).all source.contains) &&
          oldSelected.all source.contains && newSelected.all source.contains)
      if phase == "ready" || phase == "published" then
        check "Quint seal establishes complete Lean impact" complete
      if phase == "published" then
        let published := publishRevision source changed oldSelected newSelected invalidated
          oldValue newValue global deps source.length source.length
        check "Quint publication equals Lean gate"
          (published.map canonicalSet == some (canonicalSet reusable))
    IO.println s!"PASS: Quint/Lean state {index} ({phase}, {expected})"
  IO.println s!"CONTEXT-QUINT-LEAN-OK: {states.size} covered Quint states"
  return 0
