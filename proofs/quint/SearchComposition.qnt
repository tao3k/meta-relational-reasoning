// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Finite instance of MRR.SearchComposition.Protocol; C3/C4 certificates are LeanPoo-owned.
module SearchComposition {
  const BUG: str
  const MODE: str
  val primary = Set(0, 1)
  val secondary = if (BUG == "disjointTruth") Set(2) else Set(1)
  val truth = if (MODE == "intersect") primary.intersect(secondary) else primary
  var phase: str
  var current: { scope: int, source: int, resident: int, abi: int, generation: int }
  var observed: { scope: int, source: int, resident: int, abi: int, generation: int }
  var complete: bool
  var truncated: bool
  var secondaryComplete: bool
  var secondaryTruncated: bool
  var inferred: bool
  var output: Set[int]

  action init = all {
    phase' = "read",
    current' = { scope: 0, source: 0, resident: 0, abi: 0, generation: 0 },
    observed' = { scope: 0, source: 0, resident: 0, abi: 0, generation: 0 },
    complete' = true, truncated' = false, secondaryComplete' = true,
    secondaryTruncated' = false, inferred' = false, output' = Set(),
  }
  action read = all {
    phase == "read", phase' = "merge", current' = current,
    observed' = {
      scope: if (BUG == "scopeSwap") 1 else current.scope,
      source: if (BUG == "sourceSwap") 1 else current.source,
      resident: if (BUG == "residentSwap") 1 else current.resident,
      abi: if (BUG == "abiSwap") 1 else current.abi,
      generation: current.generation,
    },
    complete' = not(Set("incompleteTruth", "partialSingle").contains(BUG)),
    truncated' = Set("incompleteTruth", "partialSingle").contains(BUG),
    secondaryComplete' = and { MODE != "rankJoin", BUG != "incompleteSecondary" },
    secondaryTruncated' = or { MODE == "rankJoin", BUG == "incompleteSecondary" },
    inferred' = false, output' = Set(),
  }
  val bindingValid = observed == current
  val truthValid = MODE == "single" or and {
    complete, not(truncated),
    MODE != "intersect" or and { secondaryComplete, not(secondaryTruncated) },
  }
  val bindingMutation = Set("ignoreGeneration", "sourceSwap", "scopeSwap", "residentSwap", "abiSwap").contains(BUG)
  action merge = all {
    phase == "merge", or { bindingValid, bindingMutation },
    or { truthValid, Set("incompleteTruth", "incompleteSecondary").contains(BUG) },
    phase' = "ready", current' = current, observed' = observed,
    complete' = complete, truncated' = truncated,
    secondaryComplete' = secondaryComplete, secondaryTruncated' = secondaryTruncated,
    inferred' = BUG != "skipInference",
    output' = if (BUG == "emptyIntersection") Set()
      else if (BUG == "filterPrimary") secondary else truth,
  }
  action revise = all {
    current.generation == 0, phase != "read", phase' = "merge",
    current' = { scope: current.scope, source: current.source, resident: current.resident,
      abi: current.abi, generation: 1 },
    observed' = observed, complete' = complete, truncated' = truncated,
    secondaryComplete' = secondaryComplete, secondaryTruncated' = secondaryTruncated,
    inferred' = false, output' = Set(),
  }
  action restart = all {
    phase == "merge", not(bindingValid), phase' = "read",
    current' = current, observed' = observed, complete' = complete, truncated' = truncated,
    secondaryComplete' = secondaryComplete, secondaryTruncated' = secondaryTruncated,
    inferred' = false, output' = Set(),
  }
  action publish = all {
    phase == "ready", or { inferred, BUG == "skipInference" }, phase' = "published",
    current' = current, observed' = observed, complete' = complete, truncated' = truncated,
    secondaryComplete' = secondaryComplete, secondaryTruncated' = secondaryTruncated,
    inferred' = inferred, output' = output,
  }
  action stay = all {
    phase' = phase, current' = current, observed' = observed,
    complete' = complete, truncated' = truncated,
    secondaryComplete' = secondaryComplete, secondaryTruncated' = secondaryTruncated,
    inferred' = inferred, output' = output,
  }
  action step = any {read, merge, revise, restart, publish, stay}
  val safety = not(Set("ready", "published").contains(phase)) or and {
    bindingValid, truthValid, inferred, output == truth,
  }
}
