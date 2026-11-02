import Reflection

open MRR.SearchComposition MRR.SearchComposition.Reflection

private def binding : Binding := Binding.mk "workspace" "source" "resident" "abi" "generation"
private def snapshot : State Nat :=
  State.mk .published binding binding true false true false true (fun _ => False)
private def evidence : Evidence Nat := Evidence.mk snapshot true "query"

private def partialEvidence : Evidence Nat := {evidence with completeCoverage := false}

private instance (candidate : Nat) : Decidable (evidence.snapshot.output candidate) :=
  isFalse (fun impossible => impossible)

private instance (candidate : Nat) : Decidable (partialEvidence.snapshot.output candidate) :=
  isFalse (fun impossible => impossible)

private def require (name : String) (condition : Bool) : IO Unit :=
  if condition then IO.println s!"SEARCH-REFLECTION-CHECK-OK: {name}"
  else throw (IO.userError s!"SEARCH-REFLECTION-CHECK-FAILED: {name}")

def main : IO Unit := do
  require "exhausted proposal rejected" ((propose binding "query" 0).isNone)
  let some proposal := propose binding "query" 3 | throw (IO.userError "missing proposal")
  require "bounded premise-bearing proposal"
    (proposal.remaining == 2 && proposal.premise == binding && proposal.queryIdentity == "query")
  require "fresh published evidence bound" (decide (bound binding evidence))
  require "stale generation rejected"
    (!(decide (bound {binding with generation := "next"} evidence)))
  require "foreign source rejected"
    (!(decide (bound {binding with source := "changed"} evidence)))
  require "foreign query rejected"
    (!(decide (absenceAdmitted binding "different-query" evidence 0)))
  require "complete bound absence admitted"
    (decide (absenceAdmitted binding "query" evidence 0))
  require "partial Single still needs reflection coverage"
    (decide (truthReady (Search.mk Mode.single (fun _ : Nat => False) (fun _ => False))
      {snapshot with complete := false, truncated := true}))
  require "partial evidence lacks absence admission"
    (!(decide (absenceAdmitted binding "query" partialEvidence 0)))
  IO.println "SEARCH-REFLECTION-OK: bindings, partial coverage, proposals and budget controls"
