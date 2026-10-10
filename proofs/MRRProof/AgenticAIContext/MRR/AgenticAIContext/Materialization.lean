import LeanPoo.C4.Linearize

namespace MRR.AgenticAIContext

abbrev Bytes := List UInt8
abbrev TokenSequence := List Nat

/-- Render immutable, already admitted segments by their C4 node names.
The caller owns the versioned rendering policy and the name-to-payload binding. -/
abbrev SegmentRenderer := String -> Bytes

/-- C4 precedence stays most-specific-first; physical segments are parent-first. -/
def materializeOrder (render : SegmentRenderer) (precedence : List String) : Bytes :=
  precedence.reverse.flatMap render

/-- Invoke lean-poo's pinned checked C4 compiler and its existing certificates. -/
def materialize (graph : LeanPoo.C4.Graph) (root : String)
    (render : SegmentRenderer) : Except LeanPoo.C4.Error Bytes := do
  let order <- LeanPoo.C4.linearizeChecked graph root
  return materializeOrder render order

/-- The compiler must actually produce this order. Graph editing alone does not
establish append-only layout. This check accepts no changed old node order. -/
def checkLeafExtension (oldGraph newGraph : LeanPoo.C4.Graph)
    (oldRoot child : String) : Except LeanPoo.C4.Error Bool := do
  let oldOrder <- LeanPoo.C4.linearizeChecked oldGraph oldRoot
  let newOrder <- LeanPoo.C4.linearizeChecked newGraph child
  return newOrder == child :: oldOrder

theorem checkLeafExtension_sound (oldGraph newGraph : LeanPoo.C4.Graph)
    (oldRoot child : String) (order : List String)
    (oldCompiled : LeanPoo.C4.linearizeChecked oldGraph oldRoot = .ok order)
    (accepted : checkLeafExtension oldGraph newGraph oldRoot child = .ok true) :
    LeanPoo.C4.linearizeChecked newGraph child = .ok (child :: order) := by
  unfold checkLeafExtension at accepted
  rw [oldCompiled] at accepted
  cases compiled : LeanPoo.C4.linearizeChecked newGraph child with
  | error error =>
    rw [compiled] at accepted
    change Except.error error = (Except.ok true : Except LeanPoo.C4.Error Bool) at accepted
    cases accepted
  | ok newOrder =>
    rw [compiled] at accepted
    change Except.ok (newOrder == child :: order) =
      (Except.ok true : Except LeanPoo.C4.Error Bool) at accepted
    have equal : newOrder = child :: order := by
      simpa using Except.ok.inj accepted
    rw [equal]

/-- Payload preservation is an explicit frame condition, including derived
values, final-self effects, contracts, headers, and separators. -/
def RendererUnchangedOn (order : List String) (oldRender newRender : SegmentRenderer) : Prop :=
  forall name, List.Mem name order -> newRender name = oldRender name

theorem materializeOrder_congr (order : List String)
    (oldRender newRender : SegmentRenderer)
    (unchanged : RendererUnchangedOn order oldRender newRender) :
    materializeOrder newRender order = materializeOrder oldRender order := by
  unfold materializeOrder
  simp only [List.flatMap_def]
  congr 1
  apply List.map_congr_left
  intro name member
  exact unchanged name (List.mem_reverse.mp member)

theorem materializeOrder_cons (render : SegmentRenderer) (child : String)
    (order : List String) :
    materializeOrder render (child :: order) = materializeOrder render order ++ render child := by
  simp [materializeOrder]

/-- Restricted leaf extension: checked order plus preserved old payloads gives
an exact byte append, even if the renderer changes outside the old order. -/
theorem leafExtension_bytesAppend (order : List String) (child : String)
    (oldRender newRender : SegmentRenderer)
    (unchanged : RendererUnchangedOn order oldRender newRender) :
    materializeOrder newRender (child :: order) =
      materializeOrder oldRender order ++ newRender child := by
  rw [materializeOrder_cons, materializeOrder_congr order oldRender newRender unchanged]

theorem leafExtension_bytesPrefix (order : List String) (child : String)
    (oldRender newRender : SegmentRenderer)
    (unchanged : RendererUnchangedOn order oldRender newRender) :
    List.IsPrefix (materializeOrder oldRender order)
      (materializeOrder newRender (child :: order)) := by
  exact Exists.intro (newRender child) (leafExtension_bytesAppend order child oldRender newRender unchanged).symm

/-- Bridge the list theorem to two actual successful C4 compilations.
This theorem does not assert order preservation for arbitrary graph edits. -/
theorem compiledLeafExtension_bytesAppend
    (oldGraph newGraph : LeanPoo.C4.Graph) (oldRoot child : String)
    (order : List String) (oldRender newRender : SegmentRenderer)
    (oldCompiled : LeanPoo.C4.linearizeChecked oldGraph oldRoot = .ok order)
    (newCompiled : LeanPoo.C4.linearizeChecked newGraph child = .ok (child :: order))
    (unchanged : RendererUnchangedOn order oldRender newRender) :
    And (materialize oldGraph oldRoot oldRender = .ok (materializeOrder oldRender order))
      (materialize newGraph child newRender =
        .ok (materializeOrder oldRender order ++ newRender child)) := by
  constructor
  case _ =>
    unfold materialize
    rw [oldCompiled]
    rfl
  case _ =>
    unfold materialize
    rw [newCompiled]
    change Except.ok (materializeOrder newRender (child :: order)) = _
    rw [leafExtension_bytesAppend order child oldRender newRender unchanged]

end MRR.AgenticAIContext
