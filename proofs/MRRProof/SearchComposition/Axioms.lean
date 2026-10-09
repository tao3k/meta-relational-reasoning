import Replay
import ExecutionModel
import Reflection
import PlanOrders
import Dispatch
import Lean.Util.CollectAxioms
import Lean.Elab.Command

open Lean Elab Command

run_cmd do
  let environment <- getEnv
  let allowed := #[`propext, `Classical.choice, `Quot.sound]
  let mut count : Nat := 0
  let mut c3Count : Nat := 0
  let mut c4Count : Nat := 0
  for (name, _) in environment.constants.toList do
    if (`MRR.SearchComposition).isPrefixOf name || (`MRR.SearchDispatch).isPrefixOf name || (`POO.Flow.Composition).isPrefixOf name ||
        (`LeanPoo.Prototype.C3).isPrefixOf name || (`LeanPoo.C4).isPrefixOf name then
      count := count + 1
      if (`LeanPoo.Prototype.C3).isPrefixOf name then c3Count := c3Count + 1
      if (`LeanPoo.C4).isPrefixOf name then c4Count := c4Count + 1
      for axiomName in (<- collectAxioms name) do
        unless allowed.contains axiomName do
          throwError "{name} depends on disallowed axiom {axiomName}"
  if c3Count == 0 || c4Count == 0 then throwError "C3/C4 algorithms were not loaded"
  logInfo m!"SEARCH-AXIOM-AUDIT-OK: {count} declarations, C3={c3Count}, C4={c4Count}"
