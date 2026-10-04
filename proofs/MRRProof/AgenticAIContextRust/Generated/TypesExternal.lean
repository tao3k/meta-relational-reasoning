-- Explicit trusted value models for the two BTree interfaces used by forward
-- traversal. These are NOT extracted Rust stdlib implementation proofs.
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

end Aeneas.Std
