import MRR.AgenticAIContext.System

namespace MRR.AgenticAIContext

/-- Hash equality is only an early rejection filter. The independently retained,
trusted native identity is compared after decoding and full re-admission. -/
def restoreExactIdentity {Identity Digest : Type} [DecidableEq Identity] [DecidableEq Digest]
    (expected : Identity) (expectedDigest : Digest) (bytes : Bytes)
    (hash : Bytes -> Digest) (rebuild : Bytes -> Option Identity) : Option Identity :=
  if hash bytes = expectedDigest then
    match rebuild bytes with
    | none => none
    | some candidate => if candidate = expected then some candidate else none
  else none

/-- Exact-reference restoration is sound for any hash, including a constant
function. It never needs a collision-freedom or hash-injectivity premise. -/
theorem exact_restore_identity {Identity Digest : Type} [DecidableEq Identity] [DecidableEq Digest]
    (expected restored : Identity) (expectedDigest : Digest) (bytes : Bytes)
    (hash : Bytes -> Digest) (rebuild : Bytes -> Option Identity)
    (accepted : restoreExactIdentity expected expectedDigest bytes hash rebuild = some restored) :
    restored = expected /\ rebuild bytes = some expected := by
  unfold restoreExactIdentity at accepted
  split at accepted
  case isFalse => contradiction
  case isTrue =>
    split at accepted
    case h_1 => contradiction
    case h_2 candidate decoded =>
      split at accepted
      case isFalse => contradiction
      case isTrue equal =>
        have output : candidate = restored := Option.some.inj accepted
        exact And.intro (output.symm.trans equal) (by simpa only [equal] using decoded)

/-- Decoder correctness is enough for encoding injectivity; a cryptographic hash
is unnecessary. Applying this to concrete CBOR requires its round-trip law. -/
theorem encoding_injective_of_roundtrip {Identity : Type}
    (encode : Identity -> Bytes) (decode : Bytes -> Option Identity)
    (roundtrip : forall value, decode (encode value) = some value) :
    Function.Injective encode := by
  intro left right equal
  have decoded := congrArg decode equal
  rw [roundtrip left, roundtrip right] at decoded
  exact Option.some.inj decoded

/-- A digest collision cannot substitute a distinct retained identity. -/
theorem exact_restore_collision_refused {Identity Digest : Type} [DecidableEq Identity] [DecidableEq Digest]
    (expected candidate : Identity) (expectedDigest : Digest) (bytes : Bytes)
    (hash : Bytes -> Digest) (rebuild : Bytes -> Option Identity)
    (decoded : rebuild bytes = some candidate) (different : Not (candidate = expected)) :
    restoreExactIdentity expected expectedDigest bytes hash rebuild = none := by
  simp [restoreExactIdentity, decoded, different]

end MRR.AgenticAIContext
