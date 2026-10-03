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
  match LeanPoo.C4.certifyNode "child"
      [["a", "base"], ["b", "base"], ["a", "b"]] [["base"]] ["base"] with
  | .error error => throw (IO.userError s!"FAIL: upstream node certificate {repr error}")
  | .ok certificate =>
    let identify := fun name : String => ((name, 7) : String × Nat)
    let bound : CompositionBinding (String × Nat) "child"
        [["a", "base"], ["b", "base"], ["a", "b"]] [["base"]] := {
      certificate := certificate, identify := identify,
      injective := fun _ _ equal => congrArg Prod.fst equal,
      selected := certificate.output.map identify, binding := rfl }
    let selected : { values : List (String × Nat) // values.Nodup } :=
      ⟨bound.selected, bound.selected_nodup⟩
    check "upstream certificate maps to unique MRR identities"
      (selected.val == [identify "child", identify "a", identify "b", identify "base"])
    check "mapped parent/local order retained"
      (decide ((["a", "base"].map identify).Sublist bound.selected))
    check "mapped parent suffix retained"
      (decide ((["base"].map identify).IsSuffix bound.selected))
    let renderIdentity := fun pair : String × Nat => render pair.1
    check "mapped parent-first materialization agrees"
      (bound.selected.reverse.flatMap renderIdentity ==
        materializeOrder (fun name => renderIdentity (identify name)) certificate.output)
  check "upstream certificate rejects a foreign claimed suffix"
    (match LeanPoo.C4.certifyNode "child" [["base"]] [["base"]] ["foreign"] with
     | .error .incompatibleSuffixes => true
     | _ => false)

def main (args : List String) : IO Unit := do
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
