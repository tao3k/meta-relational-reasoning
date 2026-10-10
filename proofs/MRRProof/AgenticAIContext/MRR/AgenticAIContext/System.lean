import MRR.AgenticAIContext.Closure
import MRR.AgenticAIContext.Selection
import MRR.AgenticAIContext.Semantics
import MRR.AgenticAIContext.Revision
import MRR.AgenticAIContext.Composition

namespace MRR.AgenticAIContext

/-- Exact abstract source/provenance identities. Result row order and duplicates
are retained. Dependency tables, mandatory facts, temporal receipts and output
contracts participate in identity; roots alone cannot authenticate a record.
Labels stand for admitted identities, not a cryptographic hash theorem. -/
structure ContextSystemRecord where
  binding : SelectionBinding
  source : List SelectionRow
  rows : List SelectionRow
  roots : List Nat
  mandatory : List Nat
  temporal : List Nat
  dependencies : List (Prod Nat (List Nat))
  closure : List Nat
  renderer : Nat
  tokenizer : Nat
  deriving DecidableEq

def declaredDependencies (table : List (Prod Nat (List Nat))) (id : Nat) : List Nat :=
  ((table.find? (fun entry => entry.1 == id)).map Prod.snd).getD []

def systemRequired (record : ContextSystemRecord) : List Nat :=
  record.roots ++ record.mandatory ++ record.temporal

/-- All resource ceilings are caller inputs, never restored authority. The
complete current source and dependency declaration are established upstream. -/
def checkContextSystem (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat)
    (record : ContextSystemRecord) : Bool :=
  decide (record.source = source /\ record.dependencies = dependencies) &&
  (match admitSelection current record.binding source record.rows maxRows maxFacts with
   | .error _ => false
   | .ok roots => decide (record.roots = roots) &&
       checkRequiredClosure (source.map SelectionRow.fact) (systemRequired record)
         (declaredDependencies dependencies) fuel maxSelected record.closure)

/-- Storage re-entry checks exact caller-trusted identity and re-runs admission.
The decoded record grants neither source authority nor a larger resource budget. -/
def restoreContextSystem (expected record : ContextSystemRecord)
    (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat) : Bool :=
  decide (record = expected) &&
    checkContextSystem current source dependencies maxRows maxFacts fuel maxSelected record

private theorem system_components (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat)
    (record : ContextSystemRecord)
    (accepted : checkContextSystem current source dependencies maxRows maxFacts fuel maxSelected record = true) :
    record.source = source /\ record.dependencies = dependencies /\
      admitSelection current record.binding source record.rows maxRows maxFacts = .ok record.roots /\
      checkRequiredClosure (source.map SelectionRow.fact) (systemRequired record)
        (declaredDependencies dependencies) fuel maxSelected record.closure = true := by
  unfold checkContextSystem at accepted
  have parts := Bool.and_eq_true_iff.mp accepted
  have sourceOk := parts.1
  have selectionOk := parts.2
  have sourceBound := of_decide_eq_true sourceOk
  split at selectionOk
  case h_1 => contradiction
  case h_2 roots result =>
    have parts := Bool.and_eq_true_iff.mp selectionOk
    have rootOk := parts.1
    have closureOk := parts.2
    have exactRoots : record.roots = roots := of_decide_eq_true rootOk
    rw [<- exactRoots] at result
    exact And.intro sourceBound.1 (And.intro sourceBound.2 (And.intro result closureOk))

/-- System admission combines exact Query binding and projected roots with the
least complete closure of query, mandatory and temporal roots. -/
theorem context_system_sound (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat)
    (record : ContextSystemRecord)
    (accepted : checkContextSystem current source dependencies maxRows maxFacts fuel maxSelected record = true) :
    record.binding = current /\
    (forall id, id inList record.roots <-> exists row, row inList record.rows /\ row.fact = id) /\
    (forall id, id inList record.closure <->
      Required (systemRequired record) (declaredDependencies dependencies) id) /\
    record.closure.Nodup /\ record.closure.length <= maxSelected /\
    record.rows.length <= maxRows /\ source.length <= maxFacts := by
  have parts := system_components current source dependencies maxRows maxFacts fuel maxSelected record accepted
  have selected := parts.2.2.1
  have closed := parts.2.2.2
  have budget := selection_budget current record.binding source record.rows maxRows maxFacts record.roots selected
  have closureBudget := checked_closure_source_budget (source.map SelectionRow.fact)
    (systemRequired record) (declaredDependencies dependencies) fuel maxSelected record.closure closed
  exact And.intro (selection_binding current record.binding source record.rows maxRows maxFacts record.roots selected) (And.intro (
    selection_exact_roots current record.binding source record.rows maxRows maxFacts record.roots selected) (And.intro (
    checked_closure_exact (source.map SelectionRow.fact) (systemRequired record)
      (declaredDependencies dependencies) fuel maxSelected record.closure closed) (And.intro
    closureBudget.1 (And.intro closureBudget.2.1 (And.intro budget.1 budget.2)))))

/-- Accepted restore preserves the entire trusted provenance record and grants
only a freshly admitted Context under the current caller ceilings. -/
theorem context_restore_sound (expected record : ContextSystemRecord)
    (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat)
    (accepted : restoreContextSystem expected record current source dependencies
      maxRows maxFacts fuel maxSelected = true) :
    record = expected /\
      checkContextSystem current source dependencies maxRows maxFacts fuel maxSelected record = true := by
  have parts := Bool.and_eq_true_iff.mp accepted
  have identity := parts.1
  have admitted := parts.2
  exact And.intro (of_decide_eq_true identity) admitted

theorem context_restore_identity_refused (expected record : ContextSystemRecord)
    (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat)
    (different : Not (record = expected)) :
    restoreContextSystem expected record current source dependencies maxRows maxFacts fuel maxSelected = false := by
  simp [restoreContextSystem, different]

/-- A global Context binding change includes query/snapshot/catalog/generation,
root selection and mandatory/temporal/output contracts. -/
def systemGlobalChanged (oldRecord newRecord : ContextSystemRecord) : Bool :=
  decide (Not (oldRecord.binding = newRecord.binding) \/ Not (oldRecord.roots = newRecord.roots) \/
    Not (oldRecord.mandatory = newRecord.mandatory) \/ Not (oldRecord.temporal = newRecord.temporal) \/
    Not (oldRecord.renderer = newRecord.renderer) \/ Not (oldRecord.tokenizer = newRecord.tokenizer))

/-- A full rendering frame is checked using computed reuse membership, not
supplied as equality of the old and new semantic states. Partial reuse cannot
license equality of an entire rendered Context. -/
def checkContextRevisionFrame {Value : Type} [DecidableEq Value]
    (oldRecord newRecord : ContextSystemRecord) (changed invalidated : List Nat)
    (oldValue newValue : Nat -> Value) (fuel limit : Nat) : Bool :=
  let domain := oldRecord.source.map SelectionRow.fact
  let deps := revisionDependencies (declaredDependencies oldRecord.dependencies)
    (declaredDependencies newRecord.dependencies)
  checkRevisionChanges domain changed oldValue newValue (systemGlobalChanged oldRecord newRecord) &&
    checkRequiredClosure domain changed (reverseDependencies domain deps) fuel limit invalidated &&
    decide (forall id, id inList oldRecord.closure ->
      id inList revisionReusable oldRecord.closure newRecord.closure invalidated)

/-- Query admission, exact required closure, certified C4 identity mapping and
reverse dependency revision jointly establish the semantic renderer frame.
A fixed deterministic encoder then produces identical whole-prompt bytes and
identical tokens for any tokenizer. This grants eligibility, not a cache hit. -/
theorem context_system_revision_render_tokens {Payload : Type} [DecidableEq Payload]
    (current : SelectionBinding) (source : List SelectionRow)
    (dependencies : List (Prod Nat (List Nat))) (maxRows maxFacts fuel maxSelected : Nat)
    (oldRecord newRecord : ContextSystemRecord)
    (oldAdmitted : checkContextSystem current source dependencies maxRows maxFacts fuel maxSelected oldRecord = true)
    (newAdmitted : checkContextSystem current source dependencies maxRows maxFacts fuel maxSelected newRecord = true)
    (graph : LeanPoo.C4.Graph) (root : String)
    (composition : CompositionBinding Nat graph root)
    (compositionBinding : composition.selected = oldRecord.closure)
    (oldState newState : Nat -> SemanticElement Payload)
    (changed invalidated : List Nat) (revisionFuel revisionLimit : Nat)
    (revisionOk : checkContextRevisionFrame oldRecord newRecord changed invalidated
      oldState newState revisionFuel revisionLimit = true)
    (sourceClosed : forall parent, parent inList oldRecord.source.map SelectionRow.fact ->
      forall dependency, dependency inList revisionDependencies (declaredDependencies oldRecord.dependencies)
        (declaredDependencies newRecord.dependencies) parent ->
      dependency inList oldRecord.source.map SelectionRow.fact)
    (encode : SemanticElement Payload -> Bytes) (tokenize : Tokenizer) :
    newRecord.binding = current /\ oldRecord.binding = current /\
    (forall id, id inList oldRecord.closure <-> Required (systemRequired oldRecord)
      (declaredDependencies dependencies) id) /\
    materializeOrder (semanticRenderer encode (fun node => newState (composition.identify node)))
      composition.order.output =
      materializeOrder (semanticRenderer encode (fun node => oldState (composition.identify node)))
        composition.order.output /\
    tokenize (materializeOrder (semanticRenderer encode (fun node => newState (composition.identify node)))
      composition.order.output) =
      tokenize (materializeOrder (semanticRenderer encode (fun node => oldState (composition.identify node)))
        composition.order.output) := by
  have newSystem := context_system_sound current source dependencies maxRows maxFacts fuel maxSelected newRecord newAdmitted
  have system := context_system_sound current source dependencies maxRows maxFacts fuel maxSelected oldRecord oldAdmitted
  have parts := system_components current source dependencies maxRows maxFacts fuel maxSelected oldRecord oldAdmitted
  have sourceBound := parts.1
  have closureSource := checked_closure_source_budget (source.map SelectionRow.fact)
    (systemRequired oldRecord) (declaredDependencies dependencies) fuel maxSelected oldRecord.closure parts.2.2.2
  unfold checkContextRevisionFrame at revisionOk
  have parts := Bool.and_eq_true_iff.mp revisionOk
  have checks := parts.1
  have frameCheck := parts.2
  have parts := Bool.and_eq_true_iff.mp checks
  have changesOk := parts.1
  have impactOk := parts.2
  have frame := of_decide_eq_true frameCheck
  have unchanged : SemanticUnchangedOn composition.order.output
      (fun node => oldState (composition.identify node))
      (fun node => newState (composition.identify node)) := by
    intro node member
    have selected : composition.identify node inList oldRecord.closure := by
      rw [<- compositionBinding, <- composition.binding]
      exact List.mem_map.mpr (Exists.intro node (And.intro member rfl))
    have safe := revision_reuse_dependency_safe (oldRecord.source.map SelectionRow.fact)
      changed oldRecord.closure newRecord.closure invalidated oldState newState
      (systemGlobalChanged oldRecord newRecord)
      (revisionDependencies (declaredDependencies oldRecord.dependencies)
        (declaredDependencies newRecord.dependencies)) revisionFuel revisionLimit
      (composition.identify node) changesOk impactOk
      (by simpa only [sourceBound] using closureSource.2.2) sourceClosed (frame _ selected)
    exact (safe.2 _ (Required.root (by simp))).symm
  have bytes := semanticStability_bytes composition.order.output _ _ encode unchanged
  exact And.intro newSystem.1 (And.intro system.1 (And.intro system.2.2.1 (And.intro bytes (congrArg tokenize bytes))))

end MRR.AgenticAIContext
