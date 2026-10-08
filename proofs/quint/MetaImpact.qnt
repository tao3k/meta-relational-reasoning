// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Finite support, absence, frontier, and ownership claim model.
module MetaImpact {
  const OLD_SUPPORTS: Set[(int, int)]
  const NEW_SUPPORTS: Set[(int, int)]
  const OLD_PRESENT: Set[int]
  const NEW_PRESENT: Set[int]
  const OLD_OBSERVED: Set[int]
  const NEW_OBSERVED: Set[int]
  const SUPPORT_COMPLETE: bool
  const COVERAGE_COMPLETE: bool
  const OWNER_CLAIM: str
  const BUG: str

  val inputs = 0.to(2)
  val branches = 0.to(1)
  val admitted = branches
  val negativeTarget = 2
  val oldSupports = OLD_SUPPORTS
  val newSupports = NEW_SUPPORTS
  val oldPresent = OLD_PRESENT
  val newPresent = NEW_PRESENT
  val oldObserved = OLD_OBSERVED
  val newObserved = NEW_OBSERVED
  val usedObserved = if (BUG == "ignoreNewObservation") oldObserved else newObserved
  val supportComplete = SUPPORT_COMPLETE
  val coverageComplete = COVERAGE_COMPLETE
  val ownerClaim = OWNER_CLAIM
  val changed = oldPresent.exclude(newPresent).union(newPresent.exclude(oldPresent))
  val negativeDelta = oldObserved.contains(negativeTarget) != newObserved.contains(negativeTarget)

  def touched(graph: Set[(int, int)]): bool =
    graph.exists(edge => changed.contains(edge._2))
  val frontierExpected = or { touched(oldSupports.union(newSupports)), negativeDelta }
  val usedFrontier = or {
    touched(if (BUG == "dropNewEdges") oldSupports
      else if (BUG == "dropOldEdges") newSupports
      else oldSupports.union(newSupports)),
    and { negativeDelta, BUG != "dropNegativeDelta" },
  }
  def ready(branch: int, graph: Set[(int, int)], present: Set[int]): bool = and {
    admitted.contains(branch),
    graph.exists(edge => edge._1 == branch),
    graph.filter(edge => edge._1 == branch).forall(edge => present.contains(edge._2)),
  }
  val oldAlive = branches.exists(branch => ready(branch, oldSupports, oldPresent))
  val newAlive = branches.exists(branch => ready(branch, newSupports, newPresent))

  var phase: str
  var todo: Set[int]
  var live: Set[int]
  var frontier: bool
  var verdict: str
  var absence: bool
  var claim: str

  action init = all {
    phase' = "scan",
    todo' = branches,
    live' = Set(),
    frontier' = false,
    verdict' = "pending",
    absence' = false,
    claim' = "pending",
  }
  action scan = all {
    phase == "scan",
    phase' = "evaluate",
    todo' = todo,
    live' = live,
    frontier' = usedFrontier,
    verdict' = verdict,
    absence' = absence,
    claim' = claim,
  }
  action evaluate = {
    nondet branch = todo.oneOf()
    all {
      phase == "evaluate",
      phase' = phase,
      todo' = todo.exclude(Set(branch)),
      live' = if (and {
        ready(branch, newSupports, newPresent),
        not(and { BUG == "dropAlternative", branch == 1 }),
      }) live.union(Set(branch)) else live,
      frontier' = frontier,
      verdict' = verdict,
      absence' = absence,
      claim' = claim,
    }
  }
  action seal = all {
    phase == "evaluate",
    todo.size() == 0,
    phase' = "ready",
    todo' = todo,
    live' = live,
    frontier' = frontier,
    verdict' = if (live.size() > 0) (if (oldAlive) "revalidated" else "added")
      else if (and { oldAlive, or { supportComplete, BUG == "retractIncomplete" } })
        "retracted" else "unknown",
    absence' = if (BUG == "uncertifiedAbsence") true
      else and { coverageComplete, not(usedObserved.contains(negativeTarget)) },
    claim' = if (BUG == "promoteCausal") "effect" else ownerClaim,
  }
  action publish = all {
    phase == "ready",
    phase' = "published",
    todo' = todo,
    live' = live,
    frontier' = frontier,
    verdict' = verdict,
    absence' = absence,
    claim' = claim,
  }
  action step = any { scan, evaluate, seal, publish }

  val typeOK = and {
    Set("scan", "evaluate", "ready", "published").contains(phase),
    todo.subseteq(branches),
    live.subseteq(branches),
    Set("pending", "revalidated", "added", "retracted", "unknown").contains(verdict),
    Set("pending", "structural", "temporal", "contrast", "effect").contains(claim),
  }
  val frontierSound = or { and { phase != "ready", phase != "published" },
    not(frontierExpected), frontier }
  val noFalseRetraction = or { phase != "published", verdict != "retracted", not(newAlive) }
  val retractionCertified = or {
    phase != "published", verdict != "retracted",
    and { oldAlive, supportComplete, not(newAlive) },
  }
  val noUncertifiedAbsence = or {
    phase != "published",
    not(absence),
    and { coverageComplete, not(newObserved.contains(negativeTarget)) },
  }
  val noTemporalPromotion = or { phase != "published", claim == ownerClaim }
  val supportCompleteBeforePublish = or { phase != "published", todo.size() == 0 }
  val safety = and {
    typeOK, frontierSound, noFalseRetraction, retractionCertified, noUncertifiedAbsence,
    noTemporalPromotion, supportCompleteBeforePublish,
  }
  temporal eventuallyPublished =
    step.weakFair(Set((phase, todo, live, frontier, verdict, absence, claim)))
      .implies(eventually(phase == "published"))
}
