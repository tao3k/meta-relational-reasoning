import MRR.AgenticAIContext.Cbor

namespace MRR.AgenticAIContext

/-- Independent executable SHA-256 specification: FIPS 180-4 sections 4.1.2,
4.2.2, 5.1.1, 5.3.3 and 6.2. It is not a collision-resistance theorem. -/
def sha256Constants : Array UInt32 := #[
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2]

def sha256Initial : Array UInt32 := #[
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
  0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]

def rotateRight32 (value : UInt32) (bits : Nat) : UInt32 :=
  (value >>> UInt32.ofNat bits) ||| (value <<< UInt32.ofNat (32 - bits))

def sha256Block (state : Array UInt32) (block : Bytes) : Array UInt32 := Id.run do
  let mut schedule : Array UInt32 := #[]
  for index in [:16] do
    schedule := schedule.push (((block.drop (index * 4)).take 4).foldl
      (fun value byte => value * 256 + UInt32.ofNat byte.toNat) 0)
  for index in [16:64] do
    let x := schedule[index - 15]!
    let y := schedule[index - 2]!
    let sigma0 := rotateRight32 x 7 ^^^ rotateRight32 x 18 ^^^ (x >>> 3)
    let sigma1 := rotateRight32 y 17 ^^^ rotateRight32 y 19 ^^^ (y >>> 10)
    schedule := schedule.push (schedule[index - 16]! + sigma0 + schedule[index - 7]! + sigma1)
  let mut a := state[0]!
  let mut b := state[1]!
  let mut c := state[2]!
  let mut d := state[3]!
  let mut e := state[4]!
  let mut f := state[5]!
  let mut g := state[6]!
  let mut h := state[7]!
  for index in [:64] do
    let sigma1 := rotateRight32 e 6 ^^^ rotateRight32 e 11 ^^^ rotateRight32 e 25
    let choice := (e &&& f) ^^^ ((~~~ e) &&& g)
    let first := h + sigma1 + choice + sha256Constants[index]! + schedule[index]!
    let sigma0 := rotateRight32 a 2 ^^^ rotateRight32 a 13 ^^^ rotateRight32 a 22
    let majority := (a &&& b) ^^^ (a &&& c) ^^^ (b &&& c)
    let second := sigma0 + majority
    h := g
    g := f
    f := e
    e := d + first
    d := c
    c := b
    b := a
    a := first + second
  return #[state[0]! + a, state[1]! + b, state[2]! + c, state[3]! + d,
    state[4]! + e, state[5]! + f, state[6]! + g, state[7]! + h]

def sha256Padding (input : Bytes) : Bytes :=
  let bitLength := input.length * 8
  let zeroCount := (119 - input.length % 64) % 64
  let lengthBytes := (List.range 8).map (fun index => UInt8.ofNat (bitLength / 256 ^ (7 - index) % 256))
  input ++ [128] ++ List.replicate zeroCount 0 ++ lengthBytes

def sha256 (input : Bytes) : Bytes :=
  let padded := sha256Padding input
  let words := (List.range (padded.length / 64)).foldl
    (fun state index => sha256Block state ((padded.drop (index * 64)).take 64)) sha256Initial
  (List.range 32).map (fun index =>
    UInt8.ofNat ((words[index / 4]!).toNat / 256 ^ (3 - index % 4) % 256))

theorem sha256_digest_length (input : Bytes) : (sha256 input).length = 32 := by
  simp [sha256]

end MRR.AgenticAIContext
