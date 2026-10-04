import MRR.AgenticAIContext.Materialization

namespace MRR.AgenticAIContext

/-- Consumer-established identity mapping for an upstream node certificate.
This is an abstract binding premise, not Rust source admission or a graph
traversal certificate. The C4 laws come directly from lean-poo. -/
structure CompositionBinding (Element : Type) (name : String)
    (orders tails : List (List String)) where
  certificate : LeanPoo.C4.NodeCertified name orders tails
  identify : String -> Element
  injective : Function.Injective identify
  selected : List Element
  binding : certificate.output.map identify = selected

namespace CompositionBinding

variable {Element : Type} {name : String} {orders tails : List (List String)}
variable {order tail : List String}

/-- An injective identity mapping preserves the upstream no-duplicate law. -/
theorem selected_nodup (bound : CompositionBinding Element name orders tails) :
    bound.selected.Nodup := by
  rw [<- bound.binding]
  exact bound.certificate.nodup.map bound.identify (by
    intro left right different equal
    exact different (bound.injective equal))

/-- Parent/local order preservation transfers to the bound MRR identities. -/
theorem preserves_order (bound : CompositionBinding Element name orders tails)
    (member : Membership.mem orders order) :
    (order.map bound.identify).Sublist bound.selected := by
  rw [<- bound.binding]
  exact (bound.certificate.preserves member).map bound.identify

/-- Every certified parent tail remains a literal suffix after mapping. -/
theorem parent_suffix (bound : CompositionBinding Element name orders tails)
    (member : Membership.mem tails tail) :
    (tail.map bound.identify).IsSuffix bound.selected := by
  rw [<- bound.binding]
  exact (bound.certificate.parent_suffix member).map bound.identify

/-- Rendering the bound identities agrees with rendering the upstream order. -/
theorem materialization_agrees (bound : CompositionBinding Element name orders tails)
    (render : Element -> Bytes) :
    bound.selected.reverse.flatMap render =
      materializeOrder (fun node => render (bound.identify node)) bound.certificate.output := by
  rw [<- bound.binding]
  unfold materializeOrder
  rw [<- List.map_reverse, List.flatMap_map]

end CompositionBinding
end MRR.AgenticAIContext
