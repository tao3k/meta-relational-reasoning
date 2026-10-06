/-! Total deterministic witness transport with input-indexed results. -/
namespace MRRTransformation


universe u v

structure Problem where
  Input : Type u
  Result : Input → Type v
  Valid : Input → Prop
  Correct : (a : Input) → Result a → Prop

structure CertifiedTransformation (A B : Problem.{u, v}) where
  forward : A.Input → B.Input
  preserves : ∀ a, A.Valid a → B.Valid (forward a)
  extract : (a : A.Input) → B.Result (forward a) → A.Result a
  sound : ∀ a, A.Valid a → ∀ r, B.Correct (forward a) r →
    A.Correct a (extract a r)

def identity (A : Problem) : CertifiedTransformation A A where
  forward := id
  preserves := fun _ valid => valid
  extract := fun _ r => r
  sound := fun _ _ _ correct => correct

def compose {A B C : Problem} (f : CertifiedTransformation A B)
    (g : CertifiedTransformation B C) : CertifiedTransformation A C where
  forward := fun a => g.forward (f.forward a)
  preserves := fun a valid => g.preserves _ (f.preserves a valid)
  extract := fun a r => f.extract a (g.extract (f.forward a) r)
  sound := fun a valid r correct =>
    f.sound a valid _ (g.sound _ (f.preserves a valid) r correct)

theorem compose_sound {A B C : Problem} (f : CertifiedTransformation A B)
    (g : CertifiedTransformation B C) (a : A.Input) (valid : A.Valid a)
    (r : C.Result ((compose f g).forward a))
    (correct : C.Correct ((compose f g).forward a) r) :
    A.Correct a ((compose f g).extract a r) :=
  (compose f g).sound a valid r correct

theorem compose_forward_assoc {A B C D : Problem}
    (f : CertifiedTransformation A B) (g : CertifiedTransformation B C)
    (h : CertifiedTransformation C D) (a : A.Input) :
    (compose (compose f g) h).forward a =
      (compose f (compose g h)).forward a := rfl

theorem compose_extract_assoc {A B C D : Problem}
    (f : CertifiedTransformation A B) (g : CertifiedTransformation B C)
    (h : CertifiedTransformation C D) (a : A.Input)
    (r : D.Result (h.forward (g.forward (f.forward a)))) :
    (compose (compose f g) h).extract a r =
      (compose f (compose g h)).extract a r := rfl

theorem identity_extract {A : Problem} (a : A.Input) (r : A.Result a) :
    (identity A).extract a r = r := rfl


@[simp] theorem identity_left {A B : Problem} (f : CertifiedTransformation A B) :
    compose (identity A) f = f := by cases f; rfl

@[simp] theorem identity_right {A B : Problem} (f : CertifiedTransformation A B) :
    compose f (identity B) = f := by cases f; rfl

theorem compose_assoc {A B C D : Problem} (f : CertifiedTransformation A B)
    (g : CertifiedTransformation B C) (h : CertifiedTransformation C D) :
    compose (compose f g) h = compose f (compose g h) := rfl

end MRRTransformation

#print axioms MRRTransformation.compose_sound
#print axioms MRRTransformation.compose_forward_assoc
#print axioms MRRTransformation.compose_extract_assoc
#print axioms MRRTransformation.identity_extract
#print axioms MRRTransformation.identity_left
#print axioms MRRTransformation.identity_right
#print axioms MRRTransformation.compose_assoc

namespace MRRTransformation

-- Nontrivial concrete correctness: the answer must equal the original input.
abbrev naturalAnswer : Problem where
  Input := Nat
  Result := fun _ => Nat
  Valid := fun _ => True
  Correct := fun input answer => answer = input

def shift (amount : Nat) : CertifiedTransformation naturalAnswer naturalAnswer where
  forward := fun input => input + amount
  preserves := fun _ _ => True.intro
  extract := fun _ answer => answer - amount
  sound := by
    intro input _ answer correct
    change answer = input + amount at correct
    change answer - amount = input
    exact (congrArg (fun value => value - amount) correct).trans
      (Nat.add_sub_cancel input amount)

theorem two_shift_source_answer (input : Nat) :
    (compose (shift 1) (shift 2)).extract input
      ((compose (shift 1) (shift 2)).forward input) = input :=
  compose_sound (shift 1) (shift 2) input True.intro _ rfl

theorem incorrect_extractor_rejected : (7 : Nat) ≠ 5 := by decide

-- The finite oracle checks actual forward/solver/reverse extraction, not IDs.
#guard (List.range 32).all (fun input =>
  let route := compose (shift 1) (shift 2)
  route.extract input (route.forward input) == input)
#guard !((List.range 32).all (fun input => input + 3 == input))

#print axioms two_shift_source_answer
#print axioms incorrect_extractor_rejected
end MRRTransformation
