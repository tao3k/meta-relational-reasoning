import PlanOrders
import Lean.Data.Json.Parser

namespace MRR.SearchComposition.OrderChecks

private def result {A : Type} (value : Except String A) : IO A :=
  match value with
  | .ok value => pure value
  | .error error => throw (IO.userError error)
private def field (object : Lean.Json) (name : String) : IO Lean.Json := result (object.getObjVal? name)
private def text (object : Lean.Json) (name : String) : IO String := do result (<- field object name).getStr?
private def rows (object : Lean.Json) (name : String) : IO (List Lean.Json) := do
  return (<- result (<- field object name).getArr?).toList
private def strings (value : Lean.Json) : IO (List String) := do
  (<- result value.getArr?).toList.mapM (fun item => result item.getStr?)
private def require (name : String) (passed : Bool) : IO Unit :=
  if passed then pure () else throw (IO.userError s!"SEARCH-ORDER-FAILED: {name}")
private def unique {A : Type} [BEq A] (items : List A) : Bool :=
  items.eraseDups.length == items.length
private def sameSet {A : Type} [BEq A] (left right : List A) : Bool :=
  unique left && unique right && left.length == right.length && left.all right.contains
private def pair (value : Lean.Json) : IO (Prod String Lean.Json) := do
  let items <- result value.getArr?
  require "pair width" (items.size == 2)
  return (<- result items[0]!.getStr?, items[1]!)

private def verify (execution entry : Lean.Json) : IO Unit := do
  require "order schema and retained evidence"
    ((<- text entry "schemaId") == "agent.semantic-protocols.search-order-witness" &&
      (<- text entry "schemaVersion") == "1" && (<- result (<- field entry "complete").getBool?))
  require "actual runtime algorithm" ((<- text entry "runtimeAlgorithm") == "gerbil.runtime.c4")
  let receipt <- field entry "compositionReceipt"
  let original <- field execution "compositionReceipt"
  require "exact execution receipt binding" (receipt == original)
  let factors <- strings (<- field receipt "pooFactorIds")
  let roleGraph <- (<- rows entry "roleGraph").mapM fun row => do
    let (name, parents) <- pair row
    return (name, <- strings parents)
  let names := roleGraph.map (fun item => item.1)
  require "unique complete role graph" (!names.isEmpty && unique names && names.length <= 512 &&
    roleGraph.all (fun (_, parents) => unique parents && parents.all names.contains))
  let roleRoots <- (<- rows entry "roleRoots").mapM fun row => do
    let (factor, root) <- pair row
    return (factor, <- result root.getStr?)
  require "role root factor inventory" (sameSet (roleRoots.map (fun item => item.1)) factors &&
    unique (roleRoots.map (fun item => item.2)) && roleRoots.all (fun (_, root) => names.contains root))
  let runtimeOrders <- (<- rows entry "runtimeOrders").mapM fun row => do
    let (factor, order) <- pair row
    return (factor, <- strings order)
  require "runtime order inventory" (sameSet (runtimeOrders.map (fun item => item.1)) factors &&
    runtimeOrders.all (fun (_, order) => unique order && order.all names.contains))
  let roots := roleRoots.map (fun item => item.2)
  let .ok c3 := LeanPoo.Prototype.C3.certifyGraph roleGraph roots
    | throw (IO.userError "SEARCH-ORDER-FAILED: C3 source graph certification")
  let c4Graph : LeanPoo.C4.Graph := {nodes := roleGraph.map fun (name, parents) =>
    {name := name, parentOrders := if parents.isEmpty then [] else [parents]}}
  let binding : Binding := Binding.mk (<- text receipt "scope") (<- text receipt "sourceDigest")
    (<- text receipt "residentViewDigest") (<- text receipt "compositionAbiDigest")
    (<- text receipt "generationIdentity")
  for ((factor, root), order) in roleRoots.zip c3.orders do
    let some (_, actual) := runtimeOrders.find? (fun row => row.1 == factor)
      | throw (IO.userError "SEARCH-ORDER-FAILED: missing runtime order")
    let .ok c4 := LeanPoo.C4.linearizeVerified c4Graph root
      | throw (IO.userError "SEARCH-ORDER-FAILED: C4 source graph certification")
    let certificate : BoundOrders roleGraph roots c4Graph root :=
      BoundOrders.mk binding (<- text receipt "compositionIdentity")
        (<- text receipt "pooDagDigest") (POO.Flow.Composition.Orders.mk c3 c4)
    require "runtime C4 agrees with both source certificates"
      (actual == order && actual == certificate.orders.c4.output)
  IO.println s!"SEARCH-ORDER-LEAN-OK: {<- text receipt "mode"} factors={factors.length} roleNodes={names.length} actualC4=C3=C4"

def main (args : List String) : IO Unit := do
  let [executionsPath, ordersPath] := args
    | throw (IO.userError "actual execution and order witness arrays required")
  let executions <- result (Lean.Json.parse (<- IO.FS.readFile executionsPath))
  let executions := (<- result executions.getArr?).toList
  let orders <- result (Lean.Json.parse (<- IO.FS.readFile ordersPath))
  let orders := (<- result orders.getArr?).toList
  require "nonempty matched witness inventory" (!orders.isEmpty && orders.length == executions.length)
  let identities <- executions.mapM fun entry => do text (<- field entry "compositionReceipt") "compositionIdentity"
  let orderIdentities <- orders.mapM fun entry => do text (<- field entry "compositionReceipt") "compositionIdentity"
  require "exact composition inventory" (sameSet identities orderIdentities)
  for entry in orders do
    let identity <- text (<- field entry "compositionReceipt") "compositionIdentity"
    let mut found := false
    for execution in executions do
      if (<- text (<- field execution "compositionReceipt") "compositionIdentity") == identity then
        verify execution entry
        found := true
    require "matching execution witness" found

end MRR.SearchComposition.OrderChecks

def main := MRR.SearchComposition.OrderChecks.main
