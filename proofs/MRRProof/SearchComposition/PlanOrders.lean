import SearchComposition

namespace MRR.SearchComposition

/-- Identity binding is part of the evidence; hash authenticity remains owned
by source admission. The dependent Orders binds both certificates to inputs. -/
structure BoundOrders (roles : LeanPoo.Prototype.C3.Graph) (roots : List String)
    (graph : LeanPoo.C4.Graph) (root : String) where
  binding : Binding
  compositionIdentity : String
  dagDigest : String
  orders : POO.Flow.Composition.Orders roles roots graph root

def BoundOrders.matches {roles roots graph root}
    (certificate : BoundOrders roles roots graph root)
    (expected : Binding) (composition dag : String) : Prop :=
  certificate.binding = expected /\ certificate.compositionIdentity = composition /\
    certificate.dagDigest = dag

theorem bound_orders_stale_rejected {roles roots graph root}
    (certificate : BoundOrders roles roots graph root) (expected : Binding)
    (composition dag : String)
    (stale : Not (certificate.binding.generation = expected.generation)) :
    Not (certificate.matches expected composition dag) := by
  intro matched
  exact stale (congrArg Binding.generation matched.1)

theorem bound_orders_foreign_plan_rejected {roles roots graph root}
    (certificate : BoundOrders roles roots graph root) (expected : Binding)
    (composition dag : String) (foreign : Not (certificate.dagDigest = dag)) :
    Not (certificate.matches expected composition dag) := by
  intro matched
  exact foreign matched.2.2

end MRR.SearchComposition
