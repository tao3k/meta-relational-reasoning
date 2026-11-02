import MRR.AgenticAIContext.ExactIdentity
import Lean.Data.Json.Basic

namespace MRR.AgenticAIContext

/-- Definite-length CBOR argument decoding for the Context identity profile.
Nonminimal integers, tags, indefinite lengths and floating-point forms are
outside this byte-conformance profile and are refused explicitly. -/
def cborArgument (tag : Nat) (input : Bytes) : Option (Prod Nat Bytes) := do
  if tag < 24 then return (tag, input)
  let count <- match tag with
    | 24 => some 1 | 25 => some 2 | 26 => some 4 | 27 => some 8 | _ => none
  if count > input.length then none else do
    let value := (input.take count).foldl (fun value byte => value * 256 + byte.toNat) 0
    let minimum := if count == 1 then 24 else 256 ^ (count / 2)
    if value < minimum then none else some (value, input.drop count)

/-- A bounded independent decoder for unsigned integers, UTF-8 text, arrays,
string-keyed maps and booleans/null. This matches the actual manifest and
selection identity CBOR shapes, not the separate JSON Query result transport. -/
def decodeContextCbor : Nat -> Bytes -> Option (Prod Lean.Json Bytes)
  | 0, _ => none
  | fuel + 1, input => do
    let head <- input.head?
    let major := head.toNat / 32
    let tag := head.toNat % 32
    let (argument, rest) <- cborArgument tag (input.drop 1)
    match major with
    | 0 => return (.num (Lean.JsonNumber.fromNat argument), rest)
    | 3 =>
      if argument > rest.length then none else do
        let text <- String.fromUTF8? (ByteArray.mk (rest.take argument).toArray)
        return (.str text, rest.drop argument)
    | 4 =>
      if argument > rest.length then none else do
        let (values, remaining) <- (List.range argument).foldlM (fun (values, remaining) _ => do
          let (value, next) <- decodeContextCbor fuel remaining
          pure (values ++ [value], next)) ([], rest)
        return (.arr values.toArray, remaining)
    | 5 =>
      if argument * 2 > rest.length then none else do
        let (entries, remaining) <- (List.range argument).foldlM (fun (entries, remaining) _ => do
          let (key, next) <- decodeContextCbor fuel remaining
          let key <- key.getStr?.toOption
          if (entries.map Prod.fst).contains key then none else do
            let (value, next) <- decodeContextCbor fuel next
            pure (entries ++ [(key, value)], next)) ([], rest)
        return (Lean.Json.mkObj entries, remaining)
    | 7 => match argument with
      | 20 => some (.bool false, rest)
      | 21 => some (.bool true, rest)
      | 22 => some (.null, rest)
      | _ => none
    | _ => none

def decodeContextCborAll (fuel byteLimit : Nat) (bytes : Bytes) : Option Lean.Json := do
  if bytes.length > byteLimit then none else do
    let (value, remaining) <- decodeContextCbor fuel bytes
    if remaining.isEmpty then some value else none

/-- The independent decoder never silently accepts trailing bytes or input
larger than its caller ceiling. Content equality is checked by exact restoration. -/
theorem cbor_decoder_consumes_input (fuel byteLimit : Nat) (bytes : Bytes) (value : Lean.Json)
    (accepted : decodeContextCborAll fuel byteLimit bytes = some value) :
    bytes.length <= byteLimit := by
  unfold decodeContextCborAll at accepted
  by_cases tooLarge : bytes.length > byteLimit
  case pos => simp [tooLarge] at accepted
  case neg => exact Nat.le_of_not_lt tooLarge

end MRR.AgenticAIContext
