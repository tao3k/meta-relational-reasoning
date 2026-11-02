-- Trusted exact String equality and primitive Bool inequality value models.
module
public import Aeneas
@[expose] public section
namespace Aeneas.Std

def alloc.string.String.Insts.CoreCmpPartialEqString.eq (left right : String) : Result Bool :=
  .ok (decide (left = right))

def Bool.Insts.CoreCmpPartialEqBool.ne (left right : Bool) : Result Bool :=
  .ok (decide (Not (left = right)))

-- Rust Option discriminants in the logical value model: None = 0, Some = 1.
def optionDiscriminant {T : Type} : Option T -> Isize
  | none => 0#isize
  | some _ => 1#isize

instance optionDiscriminantInst {T : Type} : Discriminant (Option T) Isize where
  read_discriminant := optionDiscriminant

end Aeneas.Std
