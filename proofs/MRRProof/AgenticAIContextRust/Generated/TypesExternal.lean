-- Explicit trusted value models for BTree values, entries and iterators used
-- by native forward/reverse traversal and reverse-index construction. These are NOT extracted Rust stdlib implementation proofs.
-- Lists describe membership/lookup; iteration order and unsafe storage are
-- outside this interface. No undefined type or custom axiom is introduced.
module
public import Aeneas
@[expose] public section
local infixr:35 " ** " => Prod

namespace Aeneas.Std

abbrev alloc.collections.btree.map.BTreeMap (K V _Allocator : Type) := List (K ** V)
abbrev alloc.collections.btree.set.BTreeSet (T _Allocator : Type) := List T

def alloc.collections.btree.set.Iter (T : Type) := List T

abbrev alloc.collections.btree.map.entry.OccupiedEntry (K V _Allocator : Type) := K ** V
abbrev alloc.collections.btree.map.entry.VacantEntry (K _V _Allocator : Type) := K
abbrev alloc.collections.btree.map.Iter (K V : Type) := List (K ** V)

abbrev alloc.collections.btree.set.IntoIter (T _Allocator : Type) := List T

end Aeneas.Std
