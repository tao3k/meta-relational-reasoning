import MRR
import Lean.Data.Json.Parser

open MRR.AgenticAIContext

private def result {Alpha : Type} (value : Except String Alpha) : IO Alpha :=
  match value with
  | .ok value => pure value
  | .error error => throw (IO.userError error)

private def check (name : String) (passed : Bool) : IO Unit := do
  if !passed then throw (IO.userError s!"FAIL: {name}")
  IO.println s!"PASS: {name}"

private def numbers (value : Lean.Json) : IO (List Nat) := do
  let values <- result value.getArr?
  values.toList.mapM (fun value => result value.getNat?)

private def fieldNumbers (object : Lean.Json) (key : String) : IO (List Nat) := do
  numbers (<- result (object.getObjVal? key))

private def edges (object : Lean.Json) (key : String) : IO (List (Prod Nat (List Nat))) := do
  let values <- result (<- result (object.getObjVal? key)).getArr?
  values.toList.mapM fun entry => do
    let entry <- result entry.getArr?
    if entry.size != 2 then throw (IO.userError "dependency row width")
    pure (<- result entry[0]!.getNat?, <- numbers entry[1]!)

private def bytes (object : Lean.Json) (key : String) : IO Bytes := do
  let values <- fieldNumbers object key
  if values.any (fun value => value > 255) then throw (IO.userError "byte out of range")
  pure (values.map UInt8.ofNat)

private def canonicalSet (items : List Nat) : List Nat :=
  items.eraseDups.mergeSort (fun left right => left <= right)

def main (args : List String) : IO Unit := do
  let path <- match args with
    | [path] => pure path
    | _ => throw (IO.userError "one Rust receipt path required")
  let receipt <- result (Lean.Json.parse (<- IO.FS.readFile path))
  check "Rust refinement receipt schema" ((<- result (<- result (receipt.getObjVal? "schema")).getStr?) == "mrr.context-refinement-receipts.v1")
  let closures <- result (<- result (receipt.getObjVal? "closures")).getArr?
  check "Rust worklist includes the complete finite case family" (closures.size == 68)
  for entry in closures do
    let name <- result (<- result (entry.getObjVal? "name")).getStr?
    let source <- fieldNumbers entry "source"
    let roots <- fieldNumbers entry "roots"
    let mandatory <- fieldNumbers entry "mandatory"
    let temporal <- fieldNumbers entry "temporal"
    let dependencyTable <- edges entry "dependencies"
    let selected <- fieldNumbers entry "selected"
    let deps := declaredDependencies dependencyTable
    let roots := roots ++ mandatory ++ temporal
    let fuel := roots.length + source.length + (dependencyTable.map (fun entry => entry.2.length)).foldl Nat.add 0 + 1
    let finished := worklistRun deps fuel { visited := [], pending := roots.reverse }
    check s!"Rust/Lean stack-set closure: {name}"
      (finished.pending.isEmpty && canonicalSet finished.visited == canonicalSet selected &&
        checkRequiredClosure source roots deps source.length source.length selected)
  let revision <- result (receipt.getObjVal? "revision")
  let source <- fieldNumbers revision "source"
  let changed <- fieldNumbers revision "changed"
  let oldDeps <- edges revision "old_dependencies"
  let newDeps <- edges revision "new_dependencies"
  let deps := revisionDependencies (declaredDependencies oldDeps) (declaredDependencies newDeps)
  let reverse := reverseDependencies source deps
  let impact := worklistRun reverse 128 { visited := [], pending := changed.reverse }
  let invalidated <- fieldNumbers revision "invalidated"
  check "Rust/Lean reverse dependency worklist"
    (impact.pending.isEmpty && canonicalSet impact.visited == canonicalSet invalidated)
  let oldSelected <- fieldNumbers revision "old_selected"
  let newSelected <- fieldNumbers revision "new_selected"
  let reusable <- fieldNumbers revision "reusable"
  check "Rust/Lean selected intersection minus impact"
    (canonicalSet (revisionReusable oldSelected newSelected invalidated) == canonicalSet reusable)
  check "NIST published SHA-256 abc known answer"
    (sha256 "abc".toUTF8.data.toList == [186, 120, 22, 191, 143, 1, 207, 234, 65, 65, 64, 222, 93, 174, 34, 35, 176, 3, 97, 163, 150, 23, 122, 156, 180, 16, 255, 97, 242, 0, 21, 173])
  check "NIST published SHA-256 two-block known answer"
    (sha256 "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".toUTF8.data.toList == [36, 141, 106, 97, 210, 6, 56, 184, 229, 192, 38, 147, 12, 62, 96, 57, 163, 60, 228, 89, 100, 255, 33, 103, 246, 236, 237, 212, 25, 219, 6, 193])
  let hashes <- result (<- result (receipt.getObjVal? "hash_vectors")).getArr?
  check "SHA-256 vector family includes block and padding boundaries" (hashes.size == 8)
  for index in [:hashes.size] do
    let vector := hashes[index]!
    let input <- bytes vector "bytes"
    let digest <- bytes vector "digest"
    check s!"independent Lean/Rust SHA-256: vector {index}" (sha256 input == digest)
  let identities <- result (<- result (receipt.getObjVal? "identities")).getArr?
  check "both actual CBOR identity preimages present" (identities.size == 2)
  for identity in identities do
    let name <- result (<- result (identity.getObjVal? "name")).getStr?
    let input <- bytes identity "bytes"
    let expected <- result (identity.getObjVal? "value")
    let digest <- bytes identity "digest"
    check s!"independent Lean decoder of actual Rust CBOR: {name}"
      (match decodeContextCborAll 32 65536 input with | some value => value == expected | none => false)
    check s!"actual CBOR preimage -> independently computed SHA-256: {name}" (sha256 input == digest)
    check s!"CBOR rejects truncation: {name}" ((decodeContextCborAll 32 65536 (input.dropLast)).isNone)
    check s!"CBOR rejects trailing bytes: {name}" ((decodeContextCborAll 32 65536 (input ++ [0])).isNone)
    check s!"CBOR enforces byte ceiling: {name}" ((decodeContextCborAll 32 (input.length - 1) input).isNone)
  check "exact restore refuses substitution with constant hash"
    (restoreExactIdentity (1 : Nat) (0 : Nat) [] (fun _ => 0) (fun _ => some 2) == none)
  check "exact restore permits identical value with constant hash"
    (restoreExactIdentity (1 : Nat) (0 : Nat) [] (fun _ => 0) (fun _ => some 1) == some 1)
  check "CBOR refuses nonminimal integer" ((decodeContextCborAll 8 32 [24, 1]).isNone)
  check "CBOR refuses duplicate object keys" ((decodeContextCborAll 8 32 [162,97,97,1,97,97,2]).isNone)
  check "CBOR refuses indefinite array" ((decodeContextCborAll 8 32 [159,255]).isNone)
  IO.println "CONTEXT-REFINEMENT-OK"
