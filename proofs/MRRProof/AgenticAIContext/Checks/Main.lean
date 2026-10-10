import MRR
import Lean.Data.Json.Parser

open MRR.AgenticAIContext
open MRR.AgenticAIContext.Counterexamples

private def check (name : String) (passed : Bool) : IO Unit := do
  if !passed then throw (IO.userError s!"FAIL: {name}")
  IO.println s!"PASS: {name}"

private def isOk {Alpha : Type} [BEq Alpha] (result : Except LeanPoo.C4.Error Alpha)
    (expected : Alpha) : Bool :=
  match result with
  | .ok actual => actual == expected
  | .error _ => false

private def jsonResult {Alpha : Type} (result : Except String Alpha) : IO Alpha :=
  match result with
  | .ok value => pure value
  | .error message => throw (IO.userError message)

private def fixtureNumbers (fixture : Lean.Json) (key : String) : IO (List Nat) := do
  let value <- jsonResult (fixture.getObjVal? key)
  let values <- jsonResult value.getArr?
  values.toList.mapM (fun value => jsonResult value.getNat?)

private def fixtureNames (fixture : Lean.Json) (key : String) : IO (List String) := do
  let value <- jsonResult (fixture.getObjVal? key)
  let values <- jsonResult value.getArr?
  values.toList.mapM (fun value => jsonResult value.getStr?)

private def checkFixture (path : String) : IO Unit := do
  let fixture <- jsonResult (Lean.Json.parse (<- IO.FS.readFile path))
  let schema <- jsonResult (<- jsonResult (fixture.getObjVal? "schema")).getStr?
  check "shared fixture schema" (schema == "mrr.agentic-ai-context.materialization-fixture.v1")
  for (graph, root, orderKey, bytesKey) in [
      (oldGraph, "a", "old_precedence", "old_bytes"),
      (leafGraph, "child", "leaf_precedence", "leaf_bytes"),
      (recomposedGraph, "child", "recomposed_precedence", "recomposed_bytes")] do
    let order <- fixtureNames fixture orderKey
    let expected <- fixtureNumbers fixture bytesKey
    check s!"shared C4 order: {orderKey}" (isOk (LeanPoo.C4.linearizeChecked graph root) order)
    check s!"shared materialization: {bytesKey}"
      (match materialize graph root render with
       | .ok bytes => bytes.map UInt8.toNat == expected
       | .error _ => false)
  let segments <- jsonResult (fixture.getObjVal? "segments")
  for name in ["root", "a", "b", "child"] do
    let expected <- fixtureNumbers segments name
    check s!"shared segment: {name}" ((render name).map UInt8.toNat == expected)
  let revision <- jsonResult (fixture.getObjVal? "token_revision")
  let oldTokens <- fixtureNumbers revision "old"
  let newTokens <- fixtureNumbers revision "new"
  let expected <- fixtureNumbers revision "stable_prefix"
  let blockSize <- jsonResult (<- jsonResult (revision.getObjVal? "block_tokens")).getNat?
  let eligible <- jsonResult (<- jsonResult (revision.getObjVal? "eligible_full_block_tokens")).getNat?
  check "shared stable token prefix" (stableTokenPrefix oldTokens newTokens == expected)
  check "shared full-block eligibility" (eligibleFullBlockTokens blockSize oldTokens newTokens == some eligible)

private def checkUpstreamCertificates : IO Unit := do
  let suffixGraph : LeanPoo.C4.Graph := { nodes := [
    { name := "base", suffix := true },
    { name := "a", parentOrders := [["base"]] },
    { name := "b", parentOrders := [["base"]] },
    { name := "child", parentOrders := [["a", "b"]] }] }
  check "checked C4 suffix diamond"
    (isOk (LeanPoo.C4.linearizeChecked suffixGraph "child") ["child", "a", "b", "base"])
  let conflicting : LeanPoo.C4.Graph := { nodes := [
    { name := "left", suffix := true }, { name := "right", suffix := true },
    { name := "child", parentOrders := [["left", "right"]] }] }
  check "checked C4 rejects incompatible suffix tails"
    (match LeanPoo.C4.linearizeChecked conflicting "child" with
     | .error .incompatibleSuffixes => true
     | _ => false)
  let inconsistent : LeanPoo.C4.Graph := { nodes := [
    { name := "a" }, { name := "b" },
    { name := "child", parentOrders := [["a", "b"], ["b", "a"]] }] }
  check "checked C4 rejects conflicting local orders"
    (match LeanPoo.C4.linearizeChecked inconsistent "child" with
     | .error .inconsistentOrder => true
     | _ => false)
  match LeanPoo.C4.linearizeVerified suffixGraph "child" with
  | .error error => throw (IO.userError s!"FAIL: upstream verified graph {repr error}")
  | .ok order =>
    let identify := fun name : String => ((name, 7) : Prod String Nat)
    let bound : CompositionBinding (Prod String Nat) suffixGraph "child" := {
      order := order, identify := identify,
      injective := fun _ _ equal => congrArg Prod.fst equal,
      selected := order.output.map identify, binding := rfl }
    let selected : { values : List (Prod String Nat) // values.Nodup } :=
      Subtype.mk bound.selected bound.selected_nodup
    check "upstream verified graph maps to unique MRR identities"
      (selected.val == [identify "child", identify "a", identify "b", identify "base"])
    check "upstream graph precedence query retained"
      (order.precedes "a" "base" &&
       decide (([identify "a", identify "base"]).Sublist bound.selected))
    check "upstream graph ancestor suffix retained"
      (decide ((["base"].map identify).IsSuffix bound.selected))
    let renderIdentity := fun pair : Prod String Nat => render pair.1
    check "mapped parent-first materialization agrees"
      (bound.selected.reverse.flatMap renderIdentity ==
        materializeOrder (fun name => renderIdentity (identify name)) order.output)
  check "upstream certificate rejects a foreign claimed suffix"
    (match LeanPoo.C4.certifyNode "child" [["base"]] [["base"]] ["foreign"] with
     | .error .incompatibleSuffixes => true
     | _ => false)

private def fixtureRows (fixture : Lean.Json) (key : String) : IO (List SelectionRow) := do
  let values <- jsonResult (<- jsonResult (fixture.getObjVal? key)).getArr?
  values.toList.mapM fun value => do
    let row <- jsonResult value.getArr?
    if row.size != 2 then throw (IO.userError "selection fixture row width")
    pure { fact := <- jsonResult row[0]!.getNat?, relation := <- jsonResult row[1]!.getNat? }

private def fixtureBinding (fixture : Lean.Json) (key : String) : IO SelectionBinding := do
  match <- fixtureNumbers fixture key with
  | [query, generation, catalog, snapshot] => pure { query, generation, catalog, snapshot }
  | _ => throw (IO.userError "selection fixture binding width")

private def checkSelectionFixture : IO Unit := do
  let path := "../../../fixtures/agentic-ai-context/selection.json"
  let fixture <- jsonResult (Lean.Json.parse (<- IO.FS.readFile path))
  let schema <- jsonResult (<- jsonResult (fixture.getObjVal? "schema")).getStr?
  check "shared selection schema" (schema == "mrr.agentic-ai-context.selection-fixture.v1")
  let facts <- fixtureRows fixture "facts"
  let current <- fixtureBinding fixture "current_binding"
  let cases <- jsonResult (<- jsonResult (fixture.getObjVal? "cases")).getArr?
  for value in cases do
    let name <- jsonResult (<- jsonResult (value.getObjVal? "name")).getStr?
    let claimed <- fixtureBinding value "binding"
    let rows <- fixtureRows value "rows"
    let maxRows <- jsonResult (<- jsonResult (value.getObjVal? "max_rows")).getNat?
    let maxFacts <- jsonResult (<- jsonResult (value.getObjVal? "max_facts")).getNat?
    let accepted <- jsonResult (<- jsonResult (value.getObjVal? "accepted")).getBool?
    let roots <- fixtureNumbers value "roots"
    let result := admitSelection current claimed facts rows maxRows maxFacts
    check s!"shared selection: {name}" (match result with
      | .ok actual => accepted && actual == roots
      | .error _ => !accepted)
  check "selection root change cancels semantic reuse" (selectionReusable current current [2] [3] [1, 2] == [])
  let workflow <- jsonResult (fixture.getObjVal? "workflow")
  let graph : LeanPoo.C4.Graph := { nodes := [
    { name := "n1" }, { name := "n2", parentOrders := [["n1"]] },
    { name := "n3", parentOrders := [["n2"]] }] }
  let renderSelection : SegmentRenderer := fun name =>
    if name == "n1" then [65] else if name == "n2" then [66] else [67]
  for (root, orderKey, bytesKey) in [("n2", "old_precedence", "old_bytes"),
      ("n3", "new_precedence", "new_bytes")] do
    let order <- fixtureNumbers workflow orderKey
    let bytes <- fixtureNumbers workflow bytesKey
    check s!"selection workflow checked C4: {orderKey}"
      (isOk (LeanPoo.C4.linearizeChecked graph root) (order.map (fun id => s!"n{id}")))
    check s!"selection workflow rendering: {bytesKey}" (match materialize graph root renderSelection with
      | .ok actual => actual.map UInt8.toNat == bytes
      | .error _ => false)
  let oldTokens <- fixtureNumbers workflow "old_bytes"
  let newTokens <- fixtureNumbers workflow "new_bytes"
  let prefixLength <- jsonResult (<- jsonResult (workflow.getObjVal? "stable_prefix")).getNat?
  let block <- jsonResult (<- jsonResult (workflow.getObjVal? "block_tokens")).getNat?
  let eligible <- jsonResult (<- jsonResult (workflow.getObjVal? "eligible_tokens")).getNat?
  check "selection workflow byte tokenizer prefix" ((stableTokenPrefix oldTokens newTokens).length == prefixLength)
  check "selection workflow full-block eligibility" (eligibleFullBlockTokens block oldTokens newTokens == some eligible)

private def checkContextSystemFixture : IO Unit := do
  let fixture <- jsonResult (Lean.Json.parse (<- IO.FS.readFile "../../../fixtures/agentic-ai-context/selection.json"))
  let source <- fixtureRows fixture "facts"
  let current <- fixtureBinding fixture "current_binding"
  let workflow <- jsonResult (fixture.getObjVal? "workflow")
  let rows <- fixtureRows workflow "old_rows"
  let values <- jsonResult (<- jsonResult (workflow.getObjVal? "dependencies")).getArr?
  let dependencies <- values.toList.mapM fun value => do
    let entry <- jsonResult value.getArr?
    let id <- jsonResult entry[0]!.getNat?
    let ids <- jsonResult entry[1]!.getArr?
    let ids <- ids.toList.mapM (fun id => jsonResult id.getNat?)
    pure (id, ids)
  let record : ContextSystemRecord := {
    binding := current, source, rows, roots := [2], mandatory := [], temporal := [],
    dependencies, closure := [1, 2], renderer := 1, tokenizer := 1 }
  let admit := checkContextSystem current source dependencies 8 8 3 8
  let restore := fun candidate => restoreContextSystem record candidate current source dependencies 8 8 3 8
  check "system shared Query and complete closure admission" (admit record)
  check "system source-bound provenance restore" (restore record)
  check "system rejects missing dependency" (!admit { record with closure := [2] })
  check "system rejects unrooted extra fact" (!admit { record with closure := [1, 2, 3] })
  check "system rejects duplicate closure" (!admit { record with closure := [1, 2, 2] })
  check "system refuses truncated traversal certificate"
    (!checkContextSystem current source dependencies 8 8 0 8 record)
  check "system restore enforces current closure ceiling"
    (!restoreContextSystem record record current source dependencies 8 8 3 1)
  check "system restore rejects changed row provenance"
    (!restore { record with rows := rows ++ rows })
  check "system restore rejects source declaration drift"
    (!restoreContextSystem record record current source [(2, [3])] 8 8 3 8)
  check "system restore rejects current generation drift"
    (!restoreContextSystem record record { current with generation := 2 } source dependencies 8 8 3 8)
  check "system empty Query still includes mandatory and temporal roots"
    (admit { record with rows := [], roots := [], mandatory := [2], temporal := [3], closure := [1, 2, 3] })
  check "system complete cyclic closure is accepted"
    (checkRequiredClosure [1, 2, 3] [2] (declaredDependencies [(1, [2]), (2, [1])]) 2 8 [1, 2])
  let deps := declaredDependencies dependencies
  let reverse := reverseDependencies [1, 2, 3] deps
  check "revision complete reverse dependency impact"
    (checkRequiredClosure [1, 2, 3] [1] reverse 3 8 [1, 2, 3])
  check "revision refuses incomplete impact certificate"
    (!checkRequiredClosure [1, 2, 3] [1] reverse 3 8 [1, 2])
  check "revision dependency change cancels dependent reuse"
    (revisionReusable [1, 2] [1, 2] [1, 2, 3] == [])
  check "revision unrelated change preserves selected reuse"
    (revisionReusable [1, 2] [1, 2] [3] == [1, 2])
  check "revision checks exact semantic change declaration"
    (checkRevisionChanges [1, 2, 3] [1] (fun id => id) (fun id => if id == 1 then 9 else id) false)
  check "revision rejects omitted semantic change"
    (!checkRevisionChanges [1, 2, 3] [] (fun id => id) (fun id => if id == 1 then 9 else id) false)
  check "revision global drift requires all source elements changed"
    (!checkRevisionChanges [1, 2, 3] [1] (fun id => id) (fun id => id) true)
  let unionDeps := revisionDependencies (declaredDependencies [(2, [1])])
    (declaredDependencies [(3, [2])])
  check "revision union retains removed old edge and added new edge"
    (checkRequiredClosure [1, 2, 3] [1] (reverseDependencies [1, 2, 3] unionDeps) 3 8 [1, 2, 3])
  check "revision rejects impact that omits the new-edge dependent"
    (!checkRequiredClosure [1, 2, 3] [1] (reverseDependencies [1, 2, 3] unionDeps) 3 8 [1, 2])
  let before : Nat -> SemanticElement Nat := fun id => { payload := id, dependencies := ["old"] }
  let after : Nat -> SemanticElement Nat := fun id => { payload := id, dependencies := ["new"] }
  check "revision semantic equality includes dependency declarations"
    (checkRevisionChanges [2] [2] before after false)
  check "revision refuses undeclared dependency-only change"
    (!checkRevisionChanges [2] [] before after false)
  check "system identical revision establishes full rendering frame"
    (checkContextRevisionFrame record record [] [] (fun id => id) (fun id => id) 3 8)
  check "system root drift refuses rendering frame"
    (!checkContextRevisionFrame record { record with roots := [3], closure := [1, 2, 3] }
      [1, 2, 3] [1, 2, 3] (fun id => id) (fun id => id) 3 8)
  check "system renderer drift refuses rendering frame"
    (!checkContextRevisionFrame record { record with renderer := 2 }
      [1, 2, 3] [1, 2, 3] (fun id => id) (fun id => id) 3 8)

def main (args : List String) : IO Unit := do
  checkContextSystemFixture
  checkSelectionFixture
  checkUpstreamCertificates
  check "pinned C4 leaf-extension order"
    (isOk (checkLeafExtension oldGraph leafGraph "a" "child") true)
  check "compiled parent-first byte append"
    (isOk (materialize leafGraph "child" render) [1, 2, 4])
  check "C4 recomposition rejected by leaf-extension check"
    (isOk (checkLeafExtension oldGraph recomposedGraph "a" "child") false)
  check "byte prefix does not guarantee tokenizer prefix"
    (!(mergeTokenizer [97]).isPrefixOf (mergeTokenizer [97, 98]))
  check "changed middle preserves only the earlier token prefix"
    (stableTokenPrefix [1, 2, 3] [1, 9, 3] == [1])
  check "unchanged suffix has changed causal history"
    (causalState [1, 2, 3] 2 != causalState [1, 9, 3] 2)
  check "full-block eligibility rounds down"
    (eligibleFullBlockTokens 2 [1, 2, 3, 4] [1, 2, 3, 9] == some 2)
  check "zero block size rejected"
    (eligibleFullBlockTokens 0 [1, 2] [1, 2] == none)
  checkFixture (args.headD "../../../fixtures/agentic-ai-context/materialization.json")
  IO.println "AGENTIC-AI-CONTEXT-OK"
