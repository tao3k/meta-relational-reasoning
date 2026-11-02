import Graph

open MRR.SearchComposition

def main : IO Unit := do
  let base : Binding := Binding.mk "workspace" "source" "resident" "abi" "generation"
  let changed := {base with generation := "stale"}
  if changed == base then throw (IO.userError "stale generation admitted")
  IO.println "SEARCH-COMPOSITION-OK: MRR binding and consumer proof contracts"

#print axioms intersection_exact
#print axioms rank_join_preserves_truth
#print axioms stale_generation_rejected
#print axioms incomplete_intersection_rejected
#print axioms truncated_intersection_rejected
#print axioms intersection_commutes
#print axioms intersection_associates
#print axioms rank_join_ignores_partial_ranking
#print axioms step_preserves_valid
#print axioms reachable_valid
#print axioms published_exact
#print axioms published_binding
#print axioms stale_cannot_publish
#print axioms empty_intersection_certifies_absence
#print axioms foreign_binding_cannot_publish
#print axioms published_intersection_complete
#print axioms influence_is_source_bound
#print axioms influence_has_causal_path
#print axioms influence_is_bounded
#print axioms parallel_no_cross_influence
