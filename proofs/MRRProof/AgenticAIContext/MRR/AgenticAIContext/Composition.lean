import MRR.AgenticAIContext.Materialization
import LeanPoo.C4.OrderRelation

namespace MRR.AgenticAIContext

/-- Bind one graph-derived, checked C4 order to MRR semantic identities.
The upstream VerifiedOrder retains its graph derivation; a standalone node
certificate cannot establish that the declared graph was actually compiled. -/
structure CompositionBinding (Element : Type) (graph : LeanPoo.C4.Graph)
    (root : String) where
  order : LeanPoo.C4.VerifiedOrder graph root
  identify : String -> Element
  injective : Function.Injective identify
  selected : List Element
  binding : order.output.map identify = selected

namespace CompositionBinding

variable {Element : Type} {graph : LeanPoo.C4.Graph} {root : String}

/-- A graph-verified result cannot duplicate selected MRR identities. -/
theorem selected_nodup (bound : CompositionBinding Element graph root) :
    bound.selected.Nodup := by
  rw [<- bound.binding]
  exact bound.order.nodup.map bound.identify (by
    intro left right different equal
    exact different (bound.injective equal))

/-- A successful upstream precedence query survives injective identity mapping. -/
theorem preserves_precedence (bound : CompositionBinding Element graph root)
    (left right : String)
    (accepted : bound.order.precedes left right = true) :
    [bound.identify left, bound.identify right].Sublist bound.selected := by
  rw [<- bound.binding]
  exact (bound.order.precedes_iff.mp accepted).map bound.identify

/-- Every reachable original local order remains ordered in the mapped result. -/
theorem preserves_local_order (bound : CompositionBinding Element graph root)
    (node : String) (declaration : LeanPoo.C4.Node) (constraint : List String)
    (path : LeanPoo.C4.Ancestor graph node root)
    (found : graph.findNode? node = some declaration)
    (member : Membership.mem declaration.parentOrders constraint) :
    (constraint.map bound.identify).Sublist bound.selected := by
  rw [<- bound.binding]
  exact (bound.order.ancestor_local_order path found member).map bound.identify

/-- A flagged reachable ancestor's complete order remains a suffix. -/
theorem ancestor_suffix (bound : CompositionBinding Element graph root)
    (node : String) (declaration : LeanPoo.C4.Node)
    (path : LeanPoo.C4.Ancestor graph node root)
    (found : graph.findNode? node = some declaration)
    (flag : declaration.suffix = true) :
    exists output tail, And (LeanPoo.C4.GraphTrace graph node output tail)
      ((output.map bound.identify).IsSuffix bound.selected) := by
  have witness := bound.order.ancestor_suffix path found flag
  cases witness with
  | intro output remaining =>
    cases remaining with
    | intro tail evidence =>
      exact Exists.intro output (Exists.intro tail
        (And.intro evidence.1 (by
          rw [<- bound.binding]
          exact evidence.2.map bound.identify)))

/-- Rendering the mapped identities agrees with rendering the verified order. -/
theorem materialization_agrees (bound : CompositionBinding Element graph root)
    (render : Element -> Bytes) :
    bound.selected.reverse.flatMap render =
      materializeOrder (fun node => render (bound.identify node)) bound.order.output := by
  rw [<- bound.binding]
  unfold materializeOrder
  rw [<- List.map_reverse, List.flatMap_map]

end CompositionBinding
end MRR.AgenticAIContext
