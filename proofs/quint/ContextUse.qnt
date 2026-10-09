// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0
// Finite point-of-use model. Observation/source authenticity and runtime atomic
// commit are external premises. No Session or authority is manufactured here.
module ContextUse {
  const SKIP: int
  const REPLAYED: Set[Set[int]]
  val required = 0.to(5)
  var valid: Set[int]
  var phase: str
  var accepted: bool
  action init = all { valid' = required, phase' = "pending", accepted' = false }
  action change = {
    nondet check = valid.oneOf()
    all { phase == "pending", valid' = valid.exclude(Set(check)),
      phase' = phase, accepted' = accepted }
  }
  action use = all {
    phase == "pending", valid' = valid, phase' = "used",
    accepted' = required.exclude(Set(SKIP)).subseteq(valid),
  }
  action step = any { change, use }
  val typeOK = and { valid.subseteq(required), Set("pending", "used").contains(phase) }
  val useSafe = or { not(accepted), required.subseteq(valid) }
  val replayCoverage = REPLAYED.contains(valid)
  val safety = and { typeOK, useSafe, replayCoverage }
}
