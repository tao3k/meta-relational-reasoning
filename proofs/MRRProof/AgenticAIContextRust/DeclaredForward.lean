import GeneralWrapper

open Aeneas Aeneas.Std
open MRR.ContextRust
open MRR.AgenticAIContext

namespace MRR.ContextRustProofs

def nativeDeclaredGraph : NativeElements -> Nat -> List Nat
  | [], _ => []
  | (key, element) :: rest, id =>
    if factModelId key = id then element.dependencies.val.map factModelId else nativeDeclaredGraph rest id

def declaredSourceValues : NativeElements -> List Nat
  | [] => []
  | (key, element) :: rest =>
    factModelId key :: (element.dependencies.val.map factModelId ++ declaredSourceValues rest)

def declaredForwardSource (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (elements : NativeElements) : List Nat :=
  (closureRoots query contract).map factModelId ++ declaredSourceValues elements

theorem native_declared_graph_projection (elements : NativeElements) (id : NativeFactId) :
    nativeDeclaredGraph elements (factModelId id) =
      ((nativeLookup elements id).map (fun element => element.dependencies.val.map factModelId)).getD [] := by
  induction elements with
  | nil => rfl
  | cons pair rest induction =>
    cases pair with
    | mk key element =>
      by_cases same : key = id
      case pos => simp [nativeDeclaredGraph, nativeLookup, same]
      case neg =>
        have different : Not (factModelId key = factModelId id) := fun equal => same (fact_model_id_injective equal)
        simp [nativeDeclaredGraph, nativeLookup, same, different, induction]

theorem native_declared_graph_members (elements : NativeElements) (parent id : Nat)
    (member : id inList nativeDeclaredGraph elements parent) : id inList declaredSourceValues elements := by
  induction elements with
  | nil => cases member
  | cons pair rest induction =>
    cases pair with
    | mk key element =>
      simp only [nativeDeclaredGraph] at member
      by_cases same : factModelId key = parent
      case pos =>
        rw [if_pos same] at member
        exact List.mem_cons_of_mem _ (List.mem_append.mpr (Or.inl member))
      case neg =>
        rw [if_neg same] at member
        exact List.mem_cons_of_mem _ (List.mem_append.mpr (Or.inr (induction member)))

theorem native_declared_source_matches (source : List Nat) (elements : NativeElements) :
    SourceMatches source (nativeDeclaredGraph elements) elements := by
  intro id _member element lookup
  simp only [native_declared_graph_projection, lookup, Option.map_some, Option.getD_some]

theorem native_declared_source_closed (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (elements : NativeElements) :
    forall parent, parent inList declaredForwardSource query contract elements ->
      forall id, id inList nativeDeclaredGraph elements parent -> id inList declaredForwardSource query contract elements := by
  intro parent _member id edge
  exact List.mem_append.mpr (Or.inr (native_declared_graph_members elements parent id edge))

/-- End-to-end production closure-function totality relative to the named stdlib
models. Graph correspondence, finite source closure and initialization are derived
from actual roots, required/temporal receipts and element dependency values. -/
theorem native_required_closure_declared_total (query : state.AgenticAiContextQuery)
    (contract : state.AgenticAiContextContract) (elements : NativeElements)
    (budget : (closureRoots query contract).length +
      remainingEdges (nativeDeclaredGraph elements) [] (declaredForwardSource query contract elements) <= Usize.max)
    (outputCapacity : (declaredForwardSource query contract elements).length <= Usize.max) :
    exists result, state.compute_required_closure query contract elements = .ok result /\
      ClosureResultSound ((closureRoots query contract).map factModelId)
        (nativeDeclaredGraph elements) elements contract.require_complete result := by
  apply native_required_closure_wrapper_total query contract elements
    (declaredForwardSource query contract elements) (nativeDeclaredGraph elements)
  next => intro id member; exact List.mem_append.mpr (Or.inl member)
  next => exact native_declared_source_closed query contract elements
  next => exact native_declared_source_matches _ elements
  next => exact budget
  next => exact outputCapacity

end MRR.ContextRustProofs
