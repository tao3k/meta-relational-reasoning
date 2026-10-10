// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Consumer cancellation identity only. POO owns graph/readiness semantics.
module SearchCancellation {
  const BUG: str
  var logical: int
  var nextAttempt: int
  var physical: Set[int]
  var lostNew: bool
  action init = all { logical' = -1, nextAttempt' = 0, physical' = Set(), lostNew' = false }
  action issue = all { logical == -1, nextAttempt < 3,
    logical' = nextAttempt, physical' = physical.union(Set(nextAttempt)),
    nextAttempt' = nextAttempt + 1, lostNew' = lostNew }
  action revise = all { logical >= 0, logical' = -1,
    physical' = physical, nextAttempt' = nextAttempt, lostNew' = lostNew }
  action cancel(id: int): bool = all { physical.contains(id),
    logical' = if (logical == id or BUG == "cancelAny") -1 else logical,
    physical' = physical.exclude(Set(id)), nextAttempt' = nextAttempt,
    lostNew' = (lostNew or (logical >= 0 and logical != id and BUG == "cancelAny")) }
  action step = any { issue, revise, cancel(0), cancel(1), cancel(2) }
  val safety = not(lostNew)
}
