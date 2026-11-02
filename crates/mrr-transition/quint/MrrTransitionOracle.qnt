// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Finite, implementation-independent transition fixture for MRR safety parity.
module MrrTransitionOracle {
  const LAST: int
  const UNSAFE: bool
  var facts: Set[int]

  action init = facts' = Set(0)

  // Each enabled action retains its source fact and admits the next fact.
  action step = {
    nondet source = 0.to(LAST - 1).oneOf()
    all {
      facts.contains(source),
      facts' = facts.union(Set(source + 1)),
    }
  }

  val Inv = if (UNSAFE) not(facts.contains(LAST)) else not(facts.contains(LAST + 1))
}
