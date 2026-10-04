-- Trusted exact String value equality, not Rust unsafe String implementation.
module
public import Aeneas
@[expose] public section
namespace Aeneas.Std

def alloc.string.String.Insts.CoreCmpPartialEqString.eq (left right : String) : Result Bool :=
  .ok (decide (left = right))

end Aeneas.Std
