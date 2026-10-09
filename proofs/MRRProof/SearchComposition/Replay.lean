import Graph
import Lean.Data.Json

namespace MRR.SearchComposition.Replay

structure Snapshot where
  phase : Phase
  current : Nat
  observed : Nat
  complete : Bool
  truncated : Bool
  secondaryComplete : Bool
  secondaryTruncated : Bool
  inferred : Bool
  output : List Nat
  deriving DecidableEq, BEq

def binding (generation : Nat) : Binding :=
  ⟨"0", "0", "0", "0", toString generation⟩

def decode (snapshot : Snapshot) : State Nat :=
  ⟨snapshot.phase, binding snapshot.current, binding snapshot.observed,
    snapshot.complete, snapshot.truncated, snapshot.secondaryComplete,
    snapshot.secondaryTruncated, snapshot.inferred, fun candidate => candidate ∈ snapshot.output⟩

def secondary (scenario : String) : List Nat :=
  if scenario = "disjointTruth" then [2] else [1]

def search (mode : Mode) (scenario : String) : Search Nat :=
  ⟨mode, fun candidate => candidate ∈ [0, 1], fun candidate => candidate ∈ secondary scenario⟩

def expected (mode : Mode) (scenario : String) : List Nat :=
  match mode with
  | .single | .rankJoin => [0, 1]
  | .intersect => [0, 1].filter (fun candidate => (secondary scenario).contains candidate)

theorem expected_exact (mode : Mode) (scenario : String) (candidate : Nat) :
    candidate ∈ expected mode scenario ↔ (search mode scenario).truth candidate := by
  cases mode <;> simp [expected, search, Search.truth, intersection]

def accepted (mode : Mode) (scenario : String) (snapshot : Snapshot) : Bool :=
  if snapshot.phase = .ready ∨ snapshot.phase = .published then
    decide (snapshot.observed = snapshot.current ∧ truthReady (search mode scenario) (decode snapshot) ∧
      snapshot.inferred = true ∧ snapshot.output = expected mode scenario)
  else true

/-- An executable replay acceptance check discharges the original protocol invariant. -/
theorem acceptance_sound (mode : Mode) (scenario : String) (snapshot : Snapshot)
    (checked : accepted mode scenario snapshot = true) :
    valid (search mode scenario) (decode snapshot) := by
  intro active
  change snapshot.phase = .ready ∨ snapshot.phase = .published at active
  have fields : snapshot.observed = snapshot.current ∧
      truthReady (search mode scenario) (decode snapshot) ∧ snapshot.inferred = true ∧
      snapshot.output = expected mode scenario := by
    apply of_decide_eq_true
    simpa [accepted, active] using checked
  refine ⟨congrArg binding fields.1, fields.2.1, fields.2.2.1, ?_⟩
  intro candidate
  change candidate ∈ snapshot.output ↔ (search mode scenario).truth candidate
  rw [fields.2.2.2]
  exact expected_exact mode scenario candidate

def initial : Snapshot := ⟨.read, 0, 0, true, false, true, false, false, []⟩

/-- Finite successors are constructed independently in Lean; TLC later checks
that every reachable Quint state belongs to this exact replay set. -/
def successors (mode : Mode) (scenario : String) (snapshot : Snapshot) : List Snapshot := Id.run do
  let mut result := [snapshot]
  if snapshot.phase == .read then
    let partialResult := scenario == "partialSingle"
    result := result ++ [{snapshot with
      phase := .merge, observed := snapshot.current,
      complete := !partialResult, truncated := partialResult,
      secondaryComplete := mode != .rankJoin, secondaryTruncated := mode == .rankJoin,
      inferred := false, output := []}]
  if snapshot.phase == .merge && snapshot.observed == snapshot.current &&
      decide (truthReady (search mode scenario) (decode snapshot)) then
    result := result ++ [{snapshot with
      phase := .ready, inferred := true,
      output := expected mode scenario}]
  if snapshot.current == 0 && snapshot.phase != .read then
    result := result ++ [{snapshot with phase := .merge, current := 1, inferred := false, output := []}]
  if snapshot.phase == .merge && snapshot.observed != snapshot.current then
    result := result ++ [{snapshot with phase := .read, inferred := false, output := []}]
  if snapshot.phase == .ready && snapshot.inferred then
    result := result ++ [{snapshot with phase := .published}]
  return result

def closure (mode : Mode) (scenario : String) : Nat → List Snapshot → List Snapshot
  | 0, states => states
  | fuel + 1, states =>
    let expanded := states.foldl (fun seen state =>
      (successors mode scenario state).foldl (fun seen next =>
        if seen.contains next then seen else seen ++ [next]) seen) states
    if expanded == states then states else closure mode scenario fuel expanded

def phaseName : Phase → String
  | .read => "read"
  | .merge => "merge"
  | .ready => "ready"
  | .published => "published"

def modeName : Mode → String
  | .single => "single"
  | .rankJoin => "rankJoin"
  | .intersect => "intersect"

def snapshotJson (snapshot : Snapshot) : Lean.Json := Lean.Json.mkObj [
  ("phase", Lean.toJson (phaseName snapshot.phase)),
  ("current", Lean.toJson snapshot.current), ("observed", Lean.toJson snapshot.observed),
  ("complete", Lean.toJson snapshot.complete), ("truncated", Lean.toJson snapshot.truncated),
  ("secondaryComplete", Lean.toJson snapshot.secondaryComplete),
  ("secondaryTruncated", Lean.toJson snapshot.secondaryTruncated),
  ("inferred", Lean.toJson snapshot.inferred), ("output", Lean.toJson snapshot.output)]

def main (args : List String) : IO UInt32 := do
  let [path] := args | throw (IO.userError "usage: search-replay RECEIPT_PATH")
  let mut rows : List Lean.Json := []
  for (mode, scenario) in [(.single, "none"), (.intersect, "none"), (.rankJoin, "none"),
      (.single, "partialSingle"), (.intersect, "disjointTruth")] do
    let states := closure mode scenario 16 [initial]
    if states.any (fun snapshot => !(accepted mode scenario snapshot)) then
      throw (IO.userError "Lean replay rejected a reachable finite state")
    if states.any (fun snapshot => (successors mode scenario snapshot).any
        (fun next => !(states.contains next))) then
      throw (IO.userError "Lean replay set is not closed under successors")
    rows := rows ++ [Lean.Json.mkObj [("mode", Lean.toJson (modeName mode)),
      ("scenario", Lean.toJson scenario), ("states", Lean.toJson (states.map snapshotJson))]]
    IO.println s!"SEARCH-LEAN-REPLAY-OK: {modeName mode}-{scenario} states={states.length}"
  IO.FS.writeFile path ((Lean.Json.mkObj [("schema", Lean.toJson "mrr.search.replay.v1"),
    ("cases", Lean.toJson rows)]).pretty ++ "\n")
  return 0

end MRR.SearchComposition.Replay

def main := MRR.SearchComposition.Replay.main
