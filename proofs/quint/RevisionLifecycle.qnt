// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Finite revision classification, impact closure, and publication protocol.
module RevisionLifecycle {
  const OLD_EDGES: Set[(int, int)]
  const NEW_EDGES: Set[(int, int)]
  const UNEQUAL: Set[int]
  const GLOBAL: bool
  const OLD_SELECTED: Set[int]
  const NEW_SELECTED: Set[int]
  const BUG: str

  val ids = 0.to(3)
  val oldEdges = OLD_EDGES
  val newEdges = NEW_EDGES
  val unequal = UNEQUAL
  val global = GLOBAL
  val oldSelected = OLD_SELECTED
  val newSelected = NEW_SELECTED
  val edges = oldEdges.union(newEdges)
  val usedEdges = if (BUG == "oldEdgesOnly") oldEdges else edges
  val requiredChanges = if (global) ids else unequal

  def users(graph: Set[(int, int)], roots: Set[int]): Set[int] =
    ids.filter(id => graph.exists(edge => and { edge._1 == id, roots.contains(edge._2) }))
  val impact1 = requiredChanges.union(users(edges, requiredChanges))
  val impact2 = impact1.union(users(edges, impact1))
  val impact3 = impact2.union(users(edges, impact2))
  val impact = impact3.union(users(edges, impact3))

  var phase: str
  var todo: Set[int]
  var changed: Set[int]
  var invalidated: Set[int]
  var reusable: Set[int]

  action init = all {
    phase' = "classify",
    todo' = ids,
    changed' = Set(),
    invalidated' = Set(),
    reusable' = Set(),
  }

  action classify = {
    nondet id = todo.oneOf()
    all {
      phase == "classify",
      phase' = phase,
      todo' = todo.exclude(Set(id)),
      changed' = if (requiredChanges.contains(id)) changed.union(Set(id)) else changed,
      invalidated' = invalidated,
      reusable' = reusable,
    }
  }
  action seed = all {
    phase == "classify",
    todo.size() == 0,
    phase' = "invalidate",
    todo' = todo,
    changed' = changed,
    invalidated' = changed,
    reusable' = reusable,
  }
  action grow = {
    nondet id = users(usedEdges, invalidated).exclude(invalidated).oneOf()
    all {
      phase == "invalidate",
      phase' = phase,
      todo' = todo,
      changed' = changed,
      invalidated' = invalidated.union(Set(id)),
      reusable' = reusable,
    }
  }
  action seal = all {
    phase == "invalidate",
    or { users(usedEdges, invalidated).subseteq(invalidated), BUG == "publishEarly" },
    phase' = "ready",
    todo' = todo,
    changed' = changed,
    invalidated' = invalidated,
    reusable' = reusable,
  }
  action publish = all {
    phase == "ready",
    phase' = "published",
    todo' = todo,
    changed' = changed,
    invalidated' = invalidated,
    reusable' = oldSelected.intersect(newSelected).exclude(invalidated),
  }
  action step = any { classify, seed, grow, seal, publish }

  val typeOK = and {
    Set("classify", "invalidate", "ready", "published").contains(phase),
    todo.subseteq(ids),
    changed.subseteq(ids),
    invalidated.subseteq(ids),
    reusable.subseteq(ids),
  }
  val classification = changed == requiredChanges.exclude(if (phase == "classify") todo else Set())
  val impactSound = invalidated.subseteq(impact)
  val completeBeforePublish = or {
    and { phase != "ready", phase != "published" },
    invalidated == impact,
  }
  val reuseSafe = or { phase != "published", and {
    reusable == oldSelected.intersect(newSelected).exclude(impact),
    or { not(global), reusable.size() == 0 },
  } }
  val safety = and { typeOK, classification, impactSound, completeBeforePublish, reuseSafe }

  temporal eventuallyPublished =
    step.weakFair(Set((phase, todo, changed, invalidated, reusable)))
      .implies(eventually(phase == "published"))
  // No action is enabled after publication; the checker may only stutter.
  temporal publishedStable = always(or { phase != "published", not(enabled(step)) })
}
