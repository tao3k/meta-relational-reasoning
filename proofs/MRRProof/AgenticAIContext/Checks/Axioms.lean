import MRR
import Lean.Util.CollectAxioms
import Lean.Elab.Command

open Lean Elab Command

/- Audit every declaration in this domain, including imported transitive
dependencies. Imported C4 certificates and MRR mapping laws share the same axiom gate. -/
run_cmd do
  let environment <- getEnv
  let allowed := #[`propext, `Classical.choice, `Quot.sound]
  let mut count : Nat := 0
  let mut c4Count : Nat := 0
  for (name, _) in environment.constants.toList do
    if (`MRR.AgenticAIContext).isPrefixOf name || (`LeanPoo.C4).isPrefixOf name then
      if (`LeanPoo.C4).isPrefixOf name then c4Count := c4Count + 1
      count := count + 1
      for axiomName in (<- collectAxioms name) do
        unless allowed.contains axiomName do
          throwError "{name} depends on disallowed axiom {axiomName}"
  if c4Count = 0 then throwError "Upstream C4 certificates were not loaded"
  if count = 0 then throwError "Agentic AI Context namespace was not loaded"
  logInfo m!"AXIOM-AUDIT-OK: {count} declarations ({c4Count} upstream C4); standard Lean axioms only"
